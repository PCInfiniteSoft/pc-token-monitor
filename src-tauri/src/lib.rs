mod aot_watcher;
mod config;
mod file_watcher;
mod jsonl_parser;
mod oauth_fetcher;
mod bg_sampler;
mod tray;
mod types;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use types::{AppConfig, DataSource, FrontendState, Plan, UsageData, WindowUsage};

struct AppState {
    usage: Arc<Mutex<Option<UsageData>>>,
    config: Arc<Mutex<AppConfig>>,
}

#[tauri::command]
fn get_state(state: State<AppState>) -> FrontendState {
    FrontendState {
        usage: state.usage.lock().unwrap().clone(),
        config: state.config.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn save_plan(plan_str: String, state: State<AppState>) -> Result<(), String> {
    let plan = match plan_str.as_str() {
        "Pro" => Plan::Pro,
        "Max50" => Plan::Max50,
        "Max200" => Plan::Max200,
        _ => Plan::Unknown,
    };
    let mut config = state.config.lock().unwrap();
    config.plan = plan;
    config::save_config(&config::config_path(), &config)
}

#[tauri::command]
fn set_always_on_top(value: bool, app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(value);
    }
}

fn dominant_percent(usage: &UsageData) -> u8 {
    let pct = (usage.five_hour.utilization.max(usage.seven_day.utilization) * 100.0) as u8;
    pct.min(100)
}

fn start_poll_loop(
    app: AppHandle,
    state: Arc<Mutex<Option<UsageData>>>,
    config: Arc<Mutex<AppConfig>>,
) {
    // Use tauri::async_runtime::spawn so it runs within Tauri's managed tokio runtime.
    tauri::async_runtime::spawn(async move {
        loop {
            let creds_path = oauth_fetcher::credentials_path();
            let new_usage = if let Some(token) = oauth_fetcher::load_access_token(&creds_path) {
                eprintln!("[poll] token found, fetching usage...");
                match oauth_fetcher::fetch_usage(&token).await {
                    Ok(u) => {
                        eprintln!("[poll] fetch OK: 5h={} 7d={}", u.five_hour.utilization, u.seven_day.utilization);
                        Some(u)
                    }
                    Err(e) => {
                        eprintln!("[poll] fetch ERR: {e}");
                        None
                    }
                }
            } else {
                eprintln!("[poll] no token, using fallback");
                None
            };

            let usage = if let Some(u) = new_usage {
                Some(u)
            } else if let Some(prev) = state.lock().unwrap().clone() {
                // OAuth failed (offline or rate-limited): keep the last known
                // value instead of dropping the display to 0%.
                Some(prev)
            } else {
                let dir = jsonl_parser::claude_projects_dir();
                let _events = jsonl_parser::scan_projects_dir(&dir);
                let now = chrono::Utc::now();
                let reset_5hr = now + chrono::Duration::hours(5);
                let reset_7day = now + chrono::Duration::days(7);
                Some(UsageData {
                    five_hour: WindowUsage {
                        utilization: 0.0,
                        resets_at: reset_5hr,
                    },
                    seven_day: WindowUsage {
                        utilization: 0.0,
                        resets_at: reset_7day,
                    },
                    seven_day_opus_utilization: None,
                    extra_usage_enabled: false,
                    source: DataSource::JsonlFallback,
                })
            };

            {
                let mut lock = state.lock().unwrap();
                *lock = usage.clone();
            }

            if let Some(ref u) = usage {
                let pct = dominant_percent(u);
                tray::update_tray_icon(&app, pct);

                let cfg = config.lock().unwrap().clone();
                let frontend = FrontendState {
                    usage: Some(u.clone()),
                    config: cfg,
                };
                let _ = app.emit("usage-updated", frontend);
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let mut config = config::load_config(&config::config_path());
            // Auto-detect the plan from Claude's local credentials so the user
            // never has to pick one. Credentials are the source of truth, so
            // this overrides any stale/manually-saved plan when available.
            let detected = oauth_fetcher::load_plan(&oauth_fetcher::credentials_path());
            if detected != Plan::Unknown && detected != config.plan {
                config.plan = detected;
                let _ = config::save_config(&config::config_path(), &config);
            }
            let usage_arc: Arc<Mutex<Option<UsageData>>> = Arc::new(Mutex::new(None));
            let config_arc = Arc::new(Mutex::new(config));

            app.manage(AppState {
                usage: usage_arc.clone(),
                config: config_arc.clone(),
            });

            tray::setup_tray(app)?;

            // Default the overlay to the top-right corner of the primary
            // monitor, just inside the edge.
            if let Some(win) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = win.current_monitor() {
                    let m_pos = monitor.position();
                    let m_size = monitor.size();
                    if let Ok(win_size) = win.outer_size() {
                        // Leave room at the top so the overlay clears the menu
                        // bar, and a small gap from the right edge.
                        let right_margin: i32 = 12;
                        let top_margin: i32 = 50;
                        let x = m_pos.x + m_size.width as i32 - win_size.width as i32 - right_margin;
                        let y = m_pos.y + top_margin;
                        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                }
            }

            bg_sampler::start_bg_sampler(app.handle().clone());

            let app_handle = app.handle().clone();
            let usage_for_poll = usage_arc.clone();
            let config_for_poll = config_arc.clone();
            start_poll_loop(app_handle.clone(), usage_for_poll, config_for_poll);
            aot_watcher::start_aot_watcher(app_handle.clone(), config_arc.clone());

            let watch_dir = jsonl_parser::claude_projects_dir();
            if watch_dir.exists() {
                let app_for_watch = app_handle.clone();
                // Keep the watcher alive for the process lifetime; dropping it would
                // disconnect the channel and kill the OS-level file watch.
                if let Ok(watcher) = file_watcher::start_watcher(watch_dir, move || {
                    let _ = app_for_watch.emit("jsonl-changed", ());
                }) {
                    Box::leak(Box::new(watcher));
                }
            }

            let main_win = app.get_webview_window("main").unwrap();
            // Clone because on_window_event borrows main_win while the move closure
            // also needs to own a handle to it.
            let win_for_event = main_win.clone();
            main_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win_for_event.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_state, save_plan, set_always_on_top])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
