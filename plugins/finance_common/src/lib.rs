//! Shared helpers for finance plugins (decimal formatting, Typst PDF, schema utilities).

pub mod decimal;
pub mod environment;
pub mod fiscal_year;
pub mod schema;
#[cfg(feature = "typst")]
pub mod typst;

use lariv_core::components::document_title;

/// Page title suffix from PWA config (`PWA_APP_NAME`) or `"Lariv"`.
pub fn finance_page_title(page: &str) -> String {
    format!("{page} — {}", document_title())
}
