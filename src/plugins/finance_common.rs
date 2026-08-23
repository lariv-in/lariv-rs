//! Shared helpers for finance plugins (decimal formatting, Typst PDF, schema utilities).

pub mod decimal;
pub mod fiscal_year;
pub mod schema;
pub mod typst;

use crate::components::document_title;
use crate::plugins::users::state::AuthContext;

/// Page title suffix from PWA config (`PWA_APP_NAME`) or `"Lariv"`.
pub fn finance_page_title(page: &str) -> String {
    format!("{page} — {}", document_title())
}

/// Whether the user has superuser access (finance apps require this).
pub fn is_superuser(auth: &AuthContext) -> bool {
    auth.user.is_superuser
}

/// Deny write access for non-superuser users.
pub fn require_superuser(auth: &AuthContext) -> bool {
    is_superuser(auth)
}
