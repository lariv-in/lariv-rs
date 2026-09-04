//! Context-window fill for the chat composer (used / max tokens).

use super::config::DEFAULT_INPUT_TOKEN_LIMIT;

/// Snapshot of how full the current model's context window is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextUsageView {
    pub used: u32,
    pub max: u32,
}

impl Default for ContextUsageView {
    fn default() -> Self {
        Self {
            used: 0,
            max: DEFAULT_INPUT_TOKEN_LIMIT,
        }
    }
}

impl ContextUsageView {
    pub fn new(used: u32, max: u32) -> Self {
        Self {
            used,
            max: max.max(1),
        }
    }

    pub fn percent(&self) -> u32 {
        let max = u64::from(self.max.max(1));
        ((u64::from(self.used) * 100) / max).min(100) as u32
    }

    /// True when fill is at or above `threshold` (clamped to 1–100).
    pub fn at_or_over_threshold(&self, threshold: u32) -> bool {
        let threshold = threshold.clamp(1, 100);
        self.percent() >= threshold
    }

    pub fn progress_class(&self) -> &'static str {
        match self.percent() {
            90..=100 => "progress-error",
            70..=89 => "progress-warning",
            _ => "progress-success",
        }
    }
}

/// Compact token count for the composer meter (`842`, `12.4k`, `1.0M`).
pub fn format_token_count(n: u32) -> String {
    if n >= 1_000_000 {
        let m = f64::from(n) / 1_000_000.0;
        if n % 1_000_000 == 0 {
            format!("{}M", n / 1_000_000)
        } else {
            format!("{m:.1}M")
        }
    } else if n >= 10_000 {
        format!("{}k", n / 1000)
    } else if n >= 1000 {
        let k = f64::from(n) / 1000.0;
        format!("{k:.1}k")
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_compact_counts() {
        assert_eq!(format_token_count(0), "0");
        assert_eq!(format_token_count(842), "842");
        assert_eq!(format_token_count(1240), "1.2k");
        assert_eq!(format_token_count(12_450), "12k");
        assert_eq!(format_token_count(1_048_576), "1.0M");
        assert_eq!(format_token_count(2_000_000), "2M");
    }

    #[test]
    fn percent_and_progress_class() {
        let low = ContextUsageView::new(100, 1000);
        assert_eq!(low.percent(), 10);
        assert_eq!(low.progress_class(), "progress-success");
        let warn = ContextUsageView::new(750, 1000);
        assert_eq!(warn.percent(), 75);
        assert_eq!(warn.progress_class(), "progress-warning");
        let high = ContextUsageView::new(950, 1000);
        assert_eq!(high.percent(), 95);
        assert_eq!(high.progress_class(), "progress-error");
        assert_eq!(ContextUsageView::new(0, 0).percent(), 0);
        assert!(ContextUsageView::new(800, 1000).at_or_over_threshold(80));
        assert!(!ContextUsageView::new(799, 1000).at_or_over_threshold(80));
        // invalid 0 clamps to 1%
        assert!(ContextUsageView::new(10, 1000).at_or_over_threshold(0));
        assert!(!ContextUsageView::new(0, 1000).at_or_over_threshold(0));
    }
}
