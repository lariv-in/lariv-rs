//! Hex color parsing and contrast helpers for colored labels/chips.

/// Parse a `#rrggbb` (or `rrggbb`) color into sRGB channels.
pub fn parse_hex_rgb(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color
        .trim()
        .strip_prefix('#')
        .unwrap_or_else(|| color.trim());
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
    let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
    let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
    Some((r, g, b))
}

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let s = f64::from(channel) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// White or near-black text so the label stays readable on `color`.
///
/// Matches the client-side `contrastText` used by many-to-many color chips.
pub fn contrasting_text_on(color: &str) -> &'static str {
    let Some((r, g, b)) = parse_hex_rgb(color) else {
        return "#ffffff";
    };
    let luminance = 0.2126 * srgb_channel_to_linear(r)
        + 0.7152 * srgb_channel_to_linear(g)
        + 0.0722 * srgb_channel_to_linear(b);
    if luminance > 0.179 {
        "#111827"
    } else {
        "#ffffff"
    }
}

#[cfg(test)]
mod tests {
    use super::{contrasting_text_on, parse_hex_rgb};

    #[test]
    fn parse_hex_rgb_accepts_hash_and_bare() {
        assert_eq!(parse_hex_rgb("#ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex_rgb("00ff00"), Some((0, 255, 0)));
        assert!(parse_hex_rgb("#fff").is_none());
        assert!(parse_hex_rgb("not-a-color").is_none());
    }

    #[test]
    fn contrasting_text_picks_dark_on_light() {
        assert_eq!(contrasting_text_on("#ffffff"), "#111827");
        assert_eq!(contrasting_text_on("#000000"), "#ffffff");
        assert_eq!(contrasting_text_on("bogus"), "#ffffff");
    }
}
