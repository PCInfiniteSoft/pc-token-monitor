//! Samples the desktop behind the transparent overlay and decides whether the
//! background is light or dark, so the frontend can flip its text palette.

use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Decide the theme from average background luminance (0..=255), with
/// hysteresis to avoid flicker near the threshold.
///
/// Returns `"light"` when the background is light (frontend should use dark
/// text) and `"dark"` when the background is dark (light text). Within the
/// 115..=140 band the previous theme is kept.
pub fn decide_theme(prev: &str, luminance: f64) -> &'static str {
    if luminance > 140.0 {
        "light"
    } else if luminance < 115.0 {
        "dark"
    } else if prev == "light" {
        "light"
    } else {
        "dark"
    }
}

/// Average luminance (0..=255) of the desktop at several transparent points
/// inside the given physical window rect, or `None` if no pixel could be read.
#[cfg(windows)]
fn sample_luminance(x: i32, y: i32, w: i32, h: i32) -> Option<f64> {
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::Graphics::Gdi::{CLR_INVALID, GetDC, GetPixel, ReleaseDC};

    if w <= 8 || h <= 8 {
        return None;
    }
    let inset = 4;
    let points = [
        (x + inset, y + inset),
        (x + w - inset, y + inset),
        (x + inset, y + h - inset),
        (x + w - inset, y + h - inset),
        (x + w / 2, y + inset),
        (x + w / 2, y + h - inset),
        (x + inset, y + h / 2),
        (x + w - inset, y + h / 2),
    ];

    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let mut sum = 0f64;
        let mut count = 0u32;
        for (px, py) in points {
            let color = GetPixel(hdc, px, py);
            if color == COLORREF(CLR_INVALID) {
                continue;
            }
            let raw = color.0; // COLORREF: 0x00BBGGRR
            let r = (raw & 0xFF) as f64;
            let g = ((raw >> 8) & 0xFF) as f64;
            let b = ((raw >> 16) & 0xFF) as f64;
            sum += 0.299 * r + 0.587 * g + 0.114 * b;
            count += 1;
        }
        let _ = ReleaseDC(None, hdc);
        if count == 0 {
            None
        } else {
            Some(sum / count as f64)
        }
    }
}

#[cfg(not(windows))]
fn sample_luminance(_x: i32, _y: i32, _w: i32, _h: i32) -> Option<f64> {
    None
}

/// Spawn a background task that samples the desktop behind the overlay every
/// ~800ms and emits `bg-theme` ("light"/"dark") whenever the decision changes.
pub fn start_bg_sampler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut theme: &str = "dark";
        loop {
            tokio::time::sleep(Duration::from_millis(800)).await;

            let Some(win) = app.get_webview_window("main") else {
                continue;
            };
            if !win.is_visible().unwrap_or(false) {
                continue;
            }
            let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) else {
                continue;
            };

            if let Some(lum) =
                sample_luminance(pos.x, pos.y, size.width as i32, size.height as i32)
            {
                let next = decide_theme(theme, lum);
                if next != theme {
                    theme = next;
                    let _ = app.emit("bg-theme", theme);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_background_flips_to_light() {
        assert_eq!(decide_theme("dark", 200.0), "light");
        assert_eq!(decide_theme("dark", 145.0), "light");
    }

    #[test]
    fn dark_background_flips_to_dark() {
        assert_eq!(decide_theme("light", 50.0), "dark");
        assert_eq!(decide_theme("light", 110.0), "dark");
    }

    #[test]
    fn hysteresis_band_keeps_previous() {
        assert_eq!(decide_theme("dark", 130.0), "dark");
        assert_eq!(decide_theme("light", 130.0), "light");
        assert_eq!(decide_theme("dark", 115.0), "dark");
        assert_eq!(decide_theme("light", 140.0), "light");
    }
}
