//! Small URL / header string helpers used by navigation and HTMX origin checks.

use axum::http::HeaderMap;

/// Split `href` into path and optional query (without the `?`).
pub fn split_query(href: &str) -> (&str, Option<&str>) {
    match href.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (href, None),
    }
}

/// Path and query from an absolute or relative URL string (fragment stripped).
pub fn url_path_and_query(url: &str) -> (&str, Option<&str>) {
    let url = url.trim();
    let without_fragment = url.split_once('#').map(|(p, _)| p).unwrap_or(url);
    let (before_query, query) = split_query(without_fragment);
    let path = if let Some(scheme_end) = before_query.find("://") {
        let after_scheme = before_query.get(scheme_end + 3..).unwrap_or("");
        match after_scheme.find('/') {
            Some(i) => after_scheme.get(i..).unwrap_or("/"),
            None => "/",
        }
    } else {
        before_query
    };
    (path, query)
}

/// UTF-8 value of a request header, if present and valid.
pub fn header_str<'a>(headers: &'a HeaderMap, name: &'static str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn split_query_separates_path() {
        assert_eq!(split_query("/a/b"), ("/a/b", None));
        assert_eq!(split_query("/a/b?x=1"), ("/a/b", Some("x=1")));
    }

    #[test]
    fn url_path_and_query_handles_absolute_and_relative() {
        assert_eq!(
            url_path_and_query("http://localhost:3000/dashboard/?from=dashboard"),
            ("/dashboard/", Some("from=dashboard"))
        );
        assert_eq!(
            url_path_and_query("/crm/leads/?q=1#top"),
            ("/crm/leads/", Some("q=1"))
        );
        assert_eq!(url_path_and_query("https://example.com"), ("/", None));
    }

    #[test]
    fn header_str_reads_utf8() {
        let mut headers = HeaderMap::new();
        headers.insert("Referer", HeaderValue::from_static("http://localhost/"));
        assert_eq!(header_str(&headers, "Referer"), Some("http://localhost/"));
        assert_eq!(header_str(&headers, "HX-Current-URL"), None);
    }
}
