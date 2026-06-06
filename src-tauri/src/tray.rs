use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{ImageBuffer, Rgba};
use imageproc::drawing::draw_text_mut;
use tauri::{App, Manager};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItemBuilder};

const FONT_BYTES: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");

pub fn icon_rgba_for_percent(percent: u8) -> Vec<u8> {
    let size = 32u32;
    // Backgrounds are kept dark/saturated so the white number stays high
    // contrast at tiny tray sizes; a bright fill (e.g. light cyan) washes the
    // digits out even with an outline.
    let bg_color = if percent >= 90 {
        Rgba([211u8, 47, 47, 255]) // deep red
    } else if percent >= 70 {
        Rgba([216u8, 110, 0, 255]) // deep amber
    } else {
        Rgba([0u8, 103, 184, 255]) // deep azure
    };

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, bg_color);

    let font = FontRef::try_from_slice(FONT_BYTES).expect("invalid font");
    let label = format!("{percent}");

    // Pick the largest font scale whose rendered glyphs still fit inside the
    // icon (minus a 1px margin for the outline). Search from large to small so
    // 1–2 digit values fill the icon and "100" shrinks just enough to fit.
    let margin = 1.0f32;
    let avail = size as f32 - margin * 2.0;
    let mut scale_val = 8.0f32;
    let mut text_w = 0.0f32;
    let mut text_h = 0.0f32;
    let mut s = 34.0f32;
    while s > 8.0 {
        let scaled = font.as_scaled(PxScale::from(s));
        let w: f32 = label.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
        let h = scaled.ascent() - scaled.descent();
        if w <= avail && h <= size as f32 {
            scale_val = s;
            text_w = w;
            text_h = h;
            break;
        }
        s -= 1.0;
    }
    let scale = PxScale::from(scale_val);

    let x = ((size as f32 - text_w) / 2.0).round() as i32;
    // Center the text box vertically; digits have no descender so nudge up
    // slightly so the visible glyphs sit optically centered.
    let y = (((size as f32 - text_h) / 2.0) - 1.0).round() as i32;

    // Draw a 1px black outline by stamping the glyphs at the 8 surrounding
    // offsets, then the white fill on top. This keeps the number legible on
    // every background color regardless of the percent band.
    let outline = Rgba([0u8, 0, 0, 255]);
    for (dx, dy) in [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0),           (1, 0),
        (-1, 1),  (0, 1),  (1, 1),
    ] {
        draw_text_mut(&mut img, outline, x + dx, y + dy, scale, &font, &label);
    }
    draw_text_mut(&mut img, Rgba([255u8, 255, 255, 255]), x, y, scale, &font, &label);
    img.into_raw()
}

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let show_hide = MenuItemBuilder::new("Show / Hide").id("show_hide").build(app)?;
    let settings = MenuItemBuilder::new("Settings").id("settings").build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;

    let menu = Menu::with_items(app, &[&show_hide, &settings, &quit])?;

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
            "settings" => {
                crate::open_settings_window(app);
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
        assert_eq!(rgba[0], 211);
        assert_eq!(rgba[1], 47);
    }

    #[test]
    fn icon_rgba_orange_at_70_to_89() {
        let rgba = icon_rgba_for_percent(75);
        assert_eq!(rgba[0], 216);
        assert_eq!(rgba[1], 110);
    }

    #[test]
    fn icon_rgba_blue_below_70() {
        let rgba = icon_rgba_for_percent(50);
        assert_eq!(rgba[0], 0);
        assert_eq!(rgba[1], 103);
    }
}
