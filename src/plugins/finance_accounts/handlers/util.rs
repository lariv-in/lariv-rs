use axum::http::Uri;

pub fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub fn parse_i64(s: &str) -> Option<i64> {
    if s.is_empty() { None } else { s.parse().ok() }
}

pub fn parse_i32(s: &str) -> Option<i32> {
    if s.is_empty() { None } else { s.parse().ok() }
}

pub fn checkbox_on(s: &str) -> bool {
    s == "on" || s == "true" || s == "1"
}

pub fn query_param(path_and_query: &str, key: &str) -> Option<String> {
    path_and_query
        .split('?')
        .nth(1)?
        .split('&')
        .find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            if k == key { Some(v.to_string()) } else { None }
        })
}
