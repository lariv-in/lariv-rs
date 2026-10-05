//! Request-scoped "arrived from the apps dashboard" flag.
//!
//! The dashboard plugin registers URL rewriting. Until it does, [`from_dashboard`]
//! stays false unless a test scopes it, and href helpers leave URLs unchanged.

use std::future::Future;
use std::sync::OnceLock;

use axum::http::{HeaderMap, Uri};

/// Apps launchpad path (`url()` adds the trailing slash).
pub const DASHBOARD_URL: &str = "/dashboard/";

tokio::task_local! {
    static FROM_DASHBOARD: bool;
}

type ArrivedFn = fn(&Uri, &HeaderMap) -> bool;
type HrefFn = fn(&str) -> String;

static ARRIVED: OnceLock<ArrivedFn> = OnceLock::new();
static WITH_ORIGIN: OnceLock<HrefFn> = OnceLock::new();
static APP_HREF: OnceLock<HrefFn> = OnceLock::new();

pub fn register_nav_origin(arrived: ArrivedFn, with_origin: HrefFn, app_href: HrefFn) {
    let _ = ARRIVED.set(arrived);
    let _ = WITH_ORIGIN.set(with_origin);
    let _ = APP_HREF.set(app_href);
}

/// `true` when this request should show a Dashboard origin crumb.
///
/// `false` outside a request.
pub fn from_dashboard() -> bool {
    FROM_DASHBOARD.try_with(|v| *v).unwrap_or(false)
}

/// Run `fut` with [`from_dashboard`] set to `from`.
pub async fn scope_from_dashboard<F>(from: bool, fut: F) -> F::Output
where
    F: Future,
{
    FROM_DASHBOARD.scope(from, fut).await
}

/// Trailing-slash app href with `from=dashboard` (dashboard tile links).
///
/// Identity until the dashboard plugin registers.
pub fn dashboard_app_href(href: &str) -> String {
    if let Some(f) = APP_HREF.get() {
        f(href)
    } else {
        href.to_owned()
    }
}

/// Navigation URL: trailing slash plus the current request's dashboard origin.
pub fn nav_url(path: &str) -> String {
    with_nav_origin(&crate::http::trailing_slash(path))
}

/// Merge `from=dashboard` onto `href` when this request arrived from the apps grid.
///
/// Identity until the dashboard plugin registers.
pub fn with_nav_origin(href: &str) -> String {
    if let Some(f) = WITH_ORIGIN.get() {
        f(href)
    } else {
        href.to_owned()
    }
}

/// Origin from the request URI plus `HX-Current-URL` / `Referer`.
///
/// Always `false` until the dashboard plugin registers.
pub fn arrived_from_dashboard(uri: &Uri, headers: &HeaderMap) -> bool {
    ARRIVED.get().is_some_and(|f| f(uri, headers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_dashboard_defaults_false_outside_scope() {
        assert!(!from_dashboard());
    }

    #[tokio::test]
    async fn scope_sets_from_dashboard() {
        scope_from_dashboard(true, async {
            assert!(from_dashboard());
        })
        .await;
        assert!(!from_dashboard());
    }

    #[test]
    fn href_helpers_are_identity_until_registered() {
        assert_eq!(with_nav_origin("/crm/contacts/"), "/crm/contacts/");
        assert_eq!(dashboard_app_href("/dashboard/tasks"), "/dashboard/tasks");
    }
}
