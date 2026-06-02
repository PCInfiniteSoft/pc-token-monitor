use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{ImageBuffer, Rgba};
use imageproc::drawing::draw_text_mut;
use tauri::{App, Manager};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItemBuilder, CheckMenuItemBuilder};

const FONT_BYTES: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");

pub fn icon_rgba_for_percent(percent: u8) -> Vec<u8> {
    let size = 32u32;
    let bg_color = if percent >= 90 {
        Rgba([220u8, 50, 50, 255])
    } else if percent >= 70 {
        Rgba([255u8, 140, 0, 255])
    } else {
        Rgba([0u8, 180, 220, 255])
    };

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, bg_color);

    let font = FontRef::try_from_slice(FONT_BYTES).expect("invalid font");
    let label = format!("{percent}");
    let scale = PxScale::from(if label.len() >= 3 { 11.0 } else { 14.0 });

    let scaled = font.as_scaled(scale);
    let text_w: f32 = label.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
    let x = ((size as f32 - text_w) / 2.0).max(0.0) as i32;
    let y = 8i32;

    draw_text_mut(&mut img, Rgba([255u8, 255, 255, 255]), x, y, scale, &font, &label);
    img.into_raw()
}

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let show_hide = MenuItemBuilder::new("Show / Hide").id("show_hide").build(app)?;
    let always_on_top = CheckMenuItemBuilder::new("Always on Top")
        .id("always_on_top")
        .checked(true)
        .build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;

    let menu = Menu::with_items(app, &[&show_hide, &always_on_top, &quit])?;

    let initial_rgba = icon_rgba_for_percent(0);
    let initial_icon = tauri::image::Image::new(&initial_rgba, 32, 32);

    TrayIconBuilder::with_id("main")
        .icon(initial_icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_hide" => {
                let win = app.get_webview_window("main").unwrap();
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                } else {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "always_on_top" => {
                let win = app.get_webview_window("main").unwrap();
                let current = win.is_always_on_top().unwrap_or(false);
                let _ = win.set_always_on_top(!current);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let win = app.get_webview_window("main").unwrap();
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                } else {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray_icon(app: &tauri::AppHandle, percent: u8) {
    if let Some(tray) = app.tray_by_id("main") {
        let rgba = icon_rgba_for_percent(percent);
        let icon = tauri::image::Image::new(&rgba, 32, 32);
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_tooltip(Some(&format!("PC Token Monitor — {percent}%")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_rgba_correct_size() {
        let rgba = icon_rgba_for_percent(73);
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }

    #[test]
    fn icon_rgba_red_at_90_plus() {
        let rgba = icon_rgba_for_percent(90);
        assert_eq!(rgba[0], 220);
        assert_eq!(rgba[1], 50);
    }

    #[test]
    fn icon_rgba_orange_at_70_to_89() {
        let rgba = icon_rgba_for_percent(75);
        assert_eq!(rgba[0], 255);
        assert_eq!(rgba[1], 140);
    }

    #[test]
    fn icon_rgba_cyan_below_70() {
        let rgba = icon_rgba_for_percent(50);
        assert_eq!(rgba[0], 0);
        assert_eq!(rgba[1], 180);
    }
}
