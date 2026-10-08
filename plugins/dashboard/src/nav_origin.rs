//! Dashboard origin query (`from=dashboard`) and launchpad path checks.
//!
//! Registered onto [`lariv_core::components::nav_origin`] so core navigation, breadcrumbs,
//! and HTMX middleware pick up the behavior when this plugin is installed.

use axum::http::{HeaderMap, Uri};

use lariv_core::components::nav_origin::from_dashboard;

/// Apps launchpad path (`url()` adds the trailing slash).
pub use lariv_core::components::nav_origin::DASHBOARD_URL;

const FROM_PARAM: &str = "from";
const FROM_VALUE: &str = "dashboard";

/// Publish dashboard URL rules to core navigation helpers.
pub fn register() {
    lariv_core::components::nav_origin::register_nav_origin(
        arrived_from_dashboard,
        with_nav_origin,
        dashboard_app_href,
    );
}

/// True when `path` is the apps launchpad (`/dashboard`), not a nested app route.
pub fn is_dashboard_launchpad(path: &str) -> bool {
    let trimmed = path.trim_end_matches('/');
    trimmed == "/dashboard"
}

/// True when `path` is under the dashboard app namespace (`/dashboard/...`).
pub fn is_dashboard_app_path(path: &str) -> bool {
    path.starts_with("/dashboard/")
}

/// Trailing-slash app href with `from=dashboard` (dashboard tile links).
///
/// Forces the origin query even on the dashboard page, where [`from_dashboard`] is false.
pub fn dashboard_app_href(href: &str) -> String {
    let (path, query) = split_query(href);
    let mut path = path.to_string();
    if !path.ends_with('/') {
        path.push('/');
    }
    let href = match query {
        Some(q) if !q.is_empty() => format!("{path}?{q}"),
        _ => path,
    };
    append_from_dashboard(&href)
}

/// Navigation URL: trailing slash plus the current request's dashboard origin.
pub fn nav_url(path: &str) -> String {
    with_nav_origin(&lariv_core::http::trailing_slash(path))
}

/// Merge `from=dashboard` onto `href` when this request arrived from the apps grid.
pub fn with_nav_origin(href: &str) -> String {
    if !from_dashboard() {
        return href.to_owned();
    }
    append_from_dashboard(href)
}

fn append_from_dashboard(href: &str) -> String {
    let (path, query) = split_query(href);
    if is_dashboard_launchpad(path) || is_dashboard_app_path(path) {
        return href.to_owned();
    }
    if query_has_from_dashboard(query) {
        return href.to_owned();
    }
    match query.filter(|q| !q.is_empty()) {
        Some(q) => format!("{path}?{q}&{FROM_PARAM}={FROM_VALUE}"),
        None => format!("{path}?{FROM_PARAM}={FROM_VALUE}"),
    }
}

/// Origin from the request URI plus `HX-Current-URL` / `Referer`.
///
/// Always `false` on the dashboard page itself.
pub fn arrived_from_dashboard(uri: &Uri, headers: &HeaderMap) -> bool {
    let path = uri.path();
    if is_dashboard_launchpad(path) {
        return false;
    }
    if is_dashboard_app_path(path) {
        return true;
    }
    if query_has_from_dashboard(uri.query()) {
        return true;
    }
    if url_is_dashboard_origin(header_str(headers, "HX-Current-URL")) {
        return true;
    }
    url_is_dashboard_origin(header_str(headers, "Referer"))
}

fn url_is_dashboard_origin(url: Option<&str>) -> bool {
    let Some(url) = url else {
        return false;
    };
    let (path, query) = path_and_query(url);
    is_dashboard_launchpad(path) || is_dashboard_app_path(path) || query_has_from_dashboard(query)
}

fn query_has_from_dashboard(query: Option<&str>) -> bool {
    let Some(query) = query else {
        return false;
    };
    query.split('&').any(|pair| {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        k == FROM_PARAM && v == FROM_VALUE
    })
}

fn split_query(href: &str) -> (&str, Option<&str>) {
    match href.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (href, None),
    }
}

fn path_and_query(url: &str) -> (&str, Option<&str>) {
    let url = url.trim();
    let without_fragment = url.split_once('#').map(|(p, _)| p).unwrap_or(url);
    let (before_query, query) = split_query(without_fragment);
    let path = if let Some(scheme_end) = before_query.find("://") {
        let after_scheme = &before_query[scheme_end + 3..];
        match after_scheme.find('/') {
            Some(i) => &after_scheme[i..],
            None => "/",
        }
    } else {
        before_query
    };
    (path, query)
}

fn header_str<'a>(headers: &'a HeaderMap, name: &'static str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Uri};
    use lariv_core::components::nav_origin::scope_from_dashboard;

    fn uri(s: &str) -> Uri {
        s.parse().unwrap()
    }

    #[test]
    fn dashboard_app_href_normalizes_dashboard_app_paths() {
        assert_eq!(dashboard_app_href("/dashboard/tasks"), "/dashboard/tasks/");
        assert_eq!(dashboard_app_href("/dashboard/tasks/"), "/dashboard/tasks/");
        assert_eq!(
            dashboard_app_href("/dashboard/tasks/?page=1"),
            "/dashboard/tasks/?page=1"
        );
        assert_eq!(dashboard_app_href("/dashboard"), "/dashboard/");
    }

    #[test]
    fn dashboard_app_path_marks_origin() {
        let headers = HeaderMap::new();
        assert!(arrived_from_dashboard(&uri("/dashboard/tasks/"), &headers));
        assert!(arrived_from_dashboard(
            &uri("/dashboard/crm/leads/"),
            &headers
        ));
        assert!(!arrived_from_dashboard(&uri("/users/login"), &headers));
    }

    #[test]
    fn dashboard_path_is_not_origin() {
        let headers = HeaderMap::new();
        assert!(!arrived_from_dashboard(&uri("/dashboard/"), &headers));
        assert!(!arrived_from_dashboard(&uri("/dashboard"), &headers));
        assert!(!arrived_from_dashboard(
            &uri("/dashboard/?from=dashboard"),
            &headers
        ));
    }

    #[test]
    fn query_param_still_marks_legacy_paths() {
        let headers = HeaderMap::new();
        assert!(arrived_from_dashboard(
            &uri("/crm/leads/?from=dashboard"),
            &headers
        ));
        assert!(!arrived_from_dashboard(&uri("/crm/leads/"), &headers));
    }

    #[test]
    fn hx_current_url_dashboard_marks_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HX-Current-URL",
            HeaderValue::from_static("http://localhost:3000/dashboard/"),
        );
        assert!(arrived_from_dashboard(&uri("/dashboard/users/"), &headers));
    }

    #[test]
    fn hx_current_url_with_from_query_marks_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HX-Current-URL",
            HeaderValue::from_static("http://localhost:3000/dashboard/users/?from=dashboard"),
        );
        assert!(arrived_from_dashboard(
            &uri("/dashboard/users/u/1/"),
            &headers
        ));
    }

    #[test]
    fn referer_dashboard_marks_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("http://localhost:3000/dashboard"),
        );
        assert!(arrived_from_dashboard(
            &uri("/dashboard/clients/"),
            &headers
        ));
    }

    #[test]
    fn unrelated_referer_on_dashboard_app_path_is_still_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HX-Current-URL",
            HeaderValue::from_static("http://localhost:3000/users/"),
        );
        headers.insert(
            "Referer",
            HeaderValue::from_static("http://localhost:3000/users/"),
        );
        assert!(arrived_from_dashboard(
            &uri("/dashboard/users/u/1/"),
            &headers
        ));
    }

    #[tokio::test]
    async fn with_nav_origin_merges_onto_typed_style_urls() {
        assert_eq!(with_nav_origin("/crm/contacts/"), "/crm/contacts/");
        scope_from_dashboard(true, async {
            assert_eq!(
                with_nav_origin("/dashboard/crm/contacts/"),
                "/dashboard/crm/contacts/"
            );
            assert_eq!(
                with_nav_origin("/dashboard/crm/contacts/?from=dashboard"),
                "/dashboard/crm/contacts/?from=dashboard"
            );
            assert_eq!(with_nav_origin("/dashboard/"), "/dashboard/");
            assert_eq!(
                nav_url("/dashboard/crm/contacts"),
                "/dashboard/crm/contacts/"
            );
            assert_eq!(
                with_nav_origin("/crm/contacts/"),
                "/crm/contacts/?from=dashboard"
            );
        })
        .await;
    }
}
