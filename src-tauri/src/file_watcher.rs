use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

pub fn start_watcher(
    watch_dir: PathBuf,
    on_change: impl Fn() + Send + 'static,
) -> Result<RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| format!("watcher init failed: {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("watch failed: {e}"))?;

    std::thread::spawn(move || {
        loop {
            match rx.recv_timeout(Duration::from_secs(60)) {
                Ok(Ok(event)) => {
                    let is_jsonl = event.paths.iter().any(|p| {
                        p.extension().map_or(false, |e| e == "jsonl")
                    });
                    let is_write = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_)
                    );
                    if is_jsonl && is_write {
                        on_change();
                    }
                }
                Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
            }
        }
    });

    Ok(watcher)
}
