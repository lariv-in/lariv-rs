//! Packed 24-bit RGB colors (`0xRRGGBB`) stored as `u32`.

pub const COLOR_MASK: u32 = 0x00FF_FFFF;
pub const DEFAULT_ACCENT_COLOR: u32 = 0x0063_66F1;

/// Parse `#rrggbb` (or `rrggbb`) into a 24-bit RGB value.
pub fn hex_to_u24(raw: &str) -> u32 {
    let s = raw.trim();
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 {
        if let Ok(value) = u32::from_str_radix(hex, 16) {
            return value & COLOR_MASK;
        }
    }
    DEFAULT_ACCENT_COLOR
}

/// Text color that stays readable on `color` (`#ffffff` or `#111827`).
pub fn contrast_content_hex(color: u32) -> &'static str {
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;
    if 0.299 * r + 0.587 * g + 0.114 * b > 160.0 {
        "#111827"
    } else {
        "#ffffff"
    }
}

/// Format a packed RGB value as `#rrggbb`.
pub fn u24_to_hex(color: u32) -> String {
    format!("#{:06x}", color & COLOR_MASK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex_to_u24("#6366f1"), 0x0063_66F1);
        assert_eq!(hex_to_u24("22c55e"), 0x0022_C55E);
        assert_eq!(u24_to_hex(0x00F5_9E0B), "#f59e0b");
    }

    #[test]
    fn invalid_hex_uses_default() {
        assert_eq!(hex_to_u24("not-a-color"), DEFAULT_ACCENT_COLOR);
    }

    #[test]
    fn contrast_picks_light_or_dark_text() {
        assert_eq!(contrast_content_hex(0x00FF_FF00), "#111827");
        assert_eq!(contrast_content_hex(0x001E_3A8A), "#ffffff");
    }
}
