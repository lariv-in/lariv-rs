//! Request-scoped "arrived from the apps dashboard" flag for breadcrumbs.
//!
//! [`htmx_middleware`](crate::web::htmx_middleware) scopes [`from_dashboard`] for the
//! request. Tile links use [`dashboard_app_href`] so the first hop carries `from=dashboard`.

use std::future::Future;

use axum::http::{HeaderMap, Uri};

/// Apps launchpad path (`url()` adds the trailing slash).
pub const DASHBOARD_URL: &str = "/dashboard/";

const FROM_PARAM: &str = "from";
const FROM_VALUE: &str = "dashboard";

tokio::task_local! {
    static FROM_DASHBOARD: bool;
}

/// `true` when this request should show a Dashboard origin crumb.
///
/// `false` outside a request (unit tests) and when `plugin-dashboard` is off.
pub fn from_dashboard() -> bool {
    #[cfg(feature = "plugin-dashboard")]
    {
        FROM_DASHBOARD.try_with(|v| *v).unwrap_or(false)
    }
    #[cfg(not(feature = "plugin-dashboard"))]
    {
        false
    }
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
    with_nav_origin(&crate::http::trailing_slash(path))
}

/// Merge `from=dashboard` onto `href` when this request arrived from the apps grid.
///
/// No-op when origin is unset, the target is the dashboard itself, or the query
/// already carries the param. Idempotent.
pub fn with_nav_origin(href: &str) -> String {
    if !from_dashboard() {
        return href.to_owned();
    }
    append_from_dashboard(href)
}

fn append_from_dashboard(href: &str) -> String {
    let (path, query) = split_query(href);
    if is_dashboard_path(path) {
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
    if is_dashboard_path(uri.path()) {
        return false;
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
    is_dashboard_path(path) || query_has_from_dashboard(query)
}

fn is_dashboard_path(path: &str) -> bool {
    let trimmed = path.trim_end_matches('/');
    trimmed == "/dashboard"
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

    fn uri(s: &str) -> Uri {
        s.parse().unwrap()
    }

    #[test]
    fn dashboard_app_href_adds_query() {
        assert_eq!(dashboard_app_href("/users"), "/users/?from=dashboard");
        assert_eq!(dashboard_app_href("/users/"), "/users/?from=dashboard");
        assert_eq!(
            dashboard_app_href("/users/?page=1"),
            "/users/?page=1&from=dashboard"
        );
        assert_eq!(
            dashboard_app_href("/users/?from=dashboard"),
            "/users/?from=dashboard"
        );
        assert_eq!(dashboard_app_href("/dashboard"), "/dashboard/");
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
    fn query_param_marks_origin() {
        let headers = HeaderMap::new();
        assert!(arrived_from_dashboard(
            &uri("/users/?from=dashboard"),
            &headers
        ));
        assert!(arrived_from_dashboard(
            &uri("/users/list/?page=2&from=dashboard"),
            &headers
        ));
        assert!(!arrived_from_dashboard(&uri("/users/"), &headers));
        assert!(!arrived_from_dashboard(
            &uri("/users/?from=other"),
            &headers
        ));
    }

    #[test]
    fn hx_current_url_dashboard_marks_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HX-Current-URL",
            HeaderValue::from_static("http://localhost:3000/dashboard/"),
        );
        assert!(arrived_from_dashboard(&uri("/users/"), &headers));
    }

    #[test]
    fn hx_current_url_with_from_query_marks_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HX-Current-URL",
            HeaderValue::from_static("http://localhost:3000/users/?from=dashboard"),
        );
        assert!(arrived_from_dashboard(&uri("/users/1/"), &headers));
    }

    #[test]
    fn referer_dashboard_marks_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Referer",
            HeaderValue::from_static("http://localhost:3000/dashboard"),
        );
        assert!(arrived_from_dashboard(&uri("/clients/"), &headers));
    }

    #[test]
    fn unrelated_referer_is_not_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "HX-Current-URL",
            HeaderValue::from_static("http://localhost:3000/users/"),
        );
        headers.insert(
            "Referer",
            HeaderValue::from_static("http://localhost:3000/users/"),
        );
        assert!(!arrived_from_dashboard(&uri("/users/1/"), &headers));
    }

    #[test]
    fn from_dashboard_defaults_false_outside_scope() {
        assert!(!from_dashboard());
    }

    #[tokio::test]
    async fn scope_sets_from_dashboard() {
        scope_from_dashboard(true, async {
            #[cfg(feature = "plugin-dashboard")]
            assert!(from_dashboard());
        })
        .await;
        assert!(!from_dashboard());
    }

    #[tokio::test]
    async fn with_nav_origin_merges_onto_typed_style_urls() {
        assert_eq!(with_nav_origin("/crm/contacts/"), "/crm/contacts/");
        scope_from_dashboard(true, async {
            #[cfg(feature = "plugin-dashboard")]
            {
                assert_eq!(
                    with_nav_origin("/crm/contacts/"),
                    "/crm/contacts/?from=dashboard"
                );
                assert_eq!(
                    with_nav_origin("/crm/contacts/?from=dashboard"),
                    "/crm/contacts/?from=dashboard"
                );
                assert_eq!(with_nav_origin("/dashboard/"), "/dashboard/");
                assert_eq!(nav_url("/crm/contacts"), "/crm/contacts/?from=dashboard");
            }
        })
        .await;
    }
}
