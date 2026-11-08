//! URL prefix checks for a published book.

/// Normalize a site prefix to `/` or `/book/section`.
pub fn normalize_prefix(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return Ok("/".into());
    }
    let mut parts = Vec::new();
    for part in trimmed.split('/') {
        if part.is_empty() {
            continue;
        }
        if part == "." || part == ".." {
            return Err(format!("invalid url prefix \"{raw}\""));
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return Ok("/".into());
    }
    Ok(format!("/{}", parts.join("/")))
}

/// Public path for a file relative to the book build root (`css/general.css`).
pub fn book_file_url(prefix: &str, relative: &str) -> String {
    let relative = relative.trim_matches('/');
    if prefix == "/" {
        format!("/{relative}")
    } else {
        format!("{prefix}/{relative}")
    }
}

/// `site-url` passed to mdBook. Includes a trailing slash so the 404 page can use it.
pub fn site_url(prefix: &str) -> String {
    if prefix == "/" {
        "/".into()
    } else {
        format!("{prefix}/")
    }
}

/// True when `path` is the prefix itself or a page under it.
pub fn route_under_prefix(prefix: &str, path: &str) -> bool {
    if prefix == "/" {
        path.starts_with('/')
    } else {
        path == prefix || path.starts_with(&format!("{prefix}/"))
    }
}

/// HTML document that sends `/book` to `/book/index.html`.
///
/// Serving the index bytes at `/book` would make relative links resolve above the
/// prefix, because the URL has no trailing file name. An absolute redirect keeps
/// chapter, CSS, and search URLs on the paths mdBook wrote.
pub fn entry_redirect_html(index_url: &str) -> String {
    let attr = html_attr(index_url);
    let script = js_string(index_url);
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Redirect</title>\
<meta http-equiv=\"refresh\" content=\"0;url={attr}\"><link rel=\"canonical\" href=\"{attr}\">\
<script>location.replace({script})</script></head><body></body></html>"
    )
}

fn html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

fn js_string(value: &str) -> String {
    let mut out = String::from('"');
    for c in value.chars() {
        match c {
            '\\' | '"' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_prefix_and_vfs_paths() {
        assert_eq!(normalize_prefix("/book/").expect("prefix"), "/book");
        assert_eq!(normalize_prefix("/").expect("root"), "/");
        normalize_prefix("/../book").expect_err("reject parent segment");
    }

    #[test]
    fn file_urls_keep_the_book_directory() {
        assert_eq!(book_file_url("/book", "index.html"), "/book/index.html");
        assert_eq!(
            book_file_url("/book", "css/general.css"),
            "/book/css/general.css"
        );
        assert_eq!(book_file_url("/", "index.html"), "/index.html");
        assert_eq!(site_url("/book"), "/book/");
        assert!(route_under_prefix("/book", "/book/intro.html"));
        assert!(!route_under_prefix("/book", "/other"));
    }

    #[test]
    fn redirect_points_at_the_index() {
        let html = entry_redirect_html("/book/index.html");
        assert!(html.contains("url=/book/index.html"));
        assert!(html.contains("location.replace(\"/book/index.html\")"));
    }
}
