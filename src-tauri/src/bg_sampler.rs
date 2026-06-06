//! Samples the desktop behind the transparent overlay and decides whether the
//! background is light or dark, so the frontend can flip its text palette.

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
