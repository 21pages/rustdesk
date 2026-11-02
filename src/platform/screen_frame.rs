pub(crate) struct Style {
    pub color: [u8; 3],
    pub width: u8,
    pub transparency: u8,
}

impl Style {
    #[cfg(any(windows, target_os = "macos"))]
    pub fn for_display(name: &str) -> Self {
        let mut style = Self::load();
        if !crate::ui_cm_interface::is_screen_frame_display_viewed(name) {
            style.color = parse_color(
                &crate::ui_interface::get_option(
                    base::config::keys::OPTION_SCREEN_FRAME_INACTIVE_COLOR,
                ),
                0x808080,
            );
        }
        style
    }

    #[cfg(any(windows, target_os = "macos"))]
    pub fn load() -> Self {
        use crate::ui_interface::get_option;
        use base::config::keys::{
            OPTION_SCREEN_FRAME_COLOR, OPTION_SCREEN_FRAME_TRANSPARENCY, OPTION_SCREEN_FRAME_WIDTH,
        };

        // CM receives the server's options through IPC; its own Config can be stale.
        Self::from_options(
            &get_option(OPTION_SCREEN_FRAME_COLOR),
            &get_option(OPTION_SCREEN_FRAME_WIDTH),
            &get_option(OPTION_SCREEN_FRAME_TRANSPARENCY),
        )
    }

    fn from_options(color: &str, width: &str, transparency: &str) -> Self {
        // Enforce visibility and size limits here too, since configuration can bypass the UI.
        Self {
            color: parse_color(color, 0xEF4438),
            width: width.parse::<i64>().unwrap_or(6).clamp(2, 12) as u8,
            transparency: transparency.parse::<i64>().unwrap_or(25).clamp(0, 80) as u8,
        }
    }
}

fn parse_color(value: &str, fallback: u32) -> [u8; 3] {
    let rgb = value
        .strip_prefix('#')
        .filter(|value| value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .and_then(|value| u32::from_str_radix(value, 16).ok())
        .unwrap_or(fallback);
    [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8]
}

#[cfg(test)]
mod tests {
    use super::Style;

    #[test]
    fn unviewed_color_defaults_to_gray_and_accepts_custom_rgb() {
        for invalid in ["", "#00FFFFFF", "invalid"] {
            assert_eq!(super::parse_color(invalid, 0x808080), [128, 128, 128]);
        }
        assert_eq!(super::parse_color("#aBcD12", 0x808080), [171, 205, 18]);
    }

    #[test]
    fn missing_or_invalid_style_preserves_the_original_appearance() {
        for (color, width, transparency) in [("", "", ""), ("#GG4438", "NaN", "infinity")] {
            let style = Style::from_options(color, width, transparency);
            assert_eq!(
                (style.color, style.width, style.transparency),
                ([239, 68, 56], 6, 25)
            );
        }
    }

    #[test]
    fn style_limits_apply_even_to_manually_edited_configuration() {
        for (width, transparency, expected) in [("-1", "-10", (2, 0)), ("99999", "100", (12, 80))] {
            let style = Style::from_options("#12aBcD", width, transparency);
            assert_eq!(style.color, [0x12, 0xAB, 0xCD]);
            assert_eq!((style.width, style.transparency), expected);
        }
    }

    #[test]
    fn color_cannot_override_the_separate_transparency_limit() {
        for color in ["#00EF4438", "#fff", "#+FFFFF", "EF4438"] {
            assert_eq!(Style::from_options(color, "6", "25").color, [239, 68, 56]);
        }
    }
}
