//! Packed 24-bit RGB colors (`0xRRGGBB`) stored as `u32`.

pub const COLOR_MASK: u32 = 0x00FF_FFFF;
pub const DEFAULT_STATUS_COLOR: u32 = 0x0063_66F1;

const STATUS_COLORS: &[u32] = &[
    0x00EF_4444,
    0x00F9_7316,
    0x00F5_9E0B,
    0x0084_CC16,
    0x0022_C55E,
    0x0014_B8A6,
    0x0006_B6D4,
    0x003B_82F6,
    0x0063_66F1,
    0x008B_5CF6,
    0x00A8_55F7,
    0x00EC_4899,
];

/// Built-in status created for every new task.
pub const STATUS_TODO: &str = "To Do";
/// Built-in status for work that has started.
pub const STATUS_IN_PROGRESS: &str = "In Progress";
/// Built-in status for finished work.
pub const STATUS_DONE: &str = "Done";

/// Seeded "To Do" color (`#6366f1`).
pub const SEED_TODO: u32 = 0x0063_66F1;
/// Seeded "In Progress" color (`#f59e0b`).
pub const SEED_IN_PROGRESS: u32 = 0x00F5_9E0B;
/// Seeded "Done" color (`#22c55e`).
pub const SEED_DONE: u32 = 0x0022_C55E;

/// Parse `#rrggbb` (or `rrggbb`) into a 24-bit RGB value. Invalid input yields a palette color.
pub fn hex_to_u24(raw: &str) -> u32 {
    let s = raw.trim();
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 {
        if let Ok(value) = u32::from_str_radix(hex, 16) {
            return value & COLOR_MASK;
        }
    }
    random_status_color()
}

/// Format a packed RGB value as `#rrggbb`.
pub fn u24_to_hex(color: u32) -> String {
    format!("#{:06x}", color & COLOR_MASK)
}

/// Path slug for a built-in status button. Unknown names are not actionable.
pub fn status_slug(name: &str) -> Option<&'static str> {
    match name {
        STATUS_TODO => Some("todo"),
        STATUS_IN_PROGRESS => Some("in-progress"),
        STATUS_DONE => Some("done"),
        _ => None,
    }
}

/// Built-in status name for a button slug.
pub fn status_name_from_slug(slug: &str) -> Option<&'static str> {
    match slug {
        "todo" => Some(STATUS_TODO),
        "in-progress" => Some(STATUS_IN_PROGRESS),
        "done" => Some(STATUS_DONE),
        _ => None,
    }
}

/// The three statuses a task button can set, in display order.
pub fn builtin_status_names() -> &'static [&'static str] {
    &[STATUS_TODO, STATUS_IN_PROGRESS, STATUS_DONE]
}

/// Pick a palette color for new statuses.
pub fn random_status_color() -> u32 {
    use rand::seq::IndexedRandom;
    STATUS_COLORS
        .choose(&mut rand::rng())
        .copied()
        .unwrap_or(DEFAULT_STATUS_COLOR)
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
    fn builtin_status_slugs_round_trip() {
        for name in builtin_status_names() {
            let slug = status_slug(name).expect("slug");
            assert_eq!(status_name_from_slug(slug), Some(*name));
        }
        assert_eq!(status_name_from_slug("later"), None);
    }

    #[test]
    fn invalid_hex_falls_back_to_palette() {
        let color = hex_to_u24("not-a-color");
        assert!(STATUS_COLORS.contains(&color));
    }
}
