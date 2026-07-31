//! Theme asset inject/strip (Go `theme_inject.go`).

use crate::grapesjs::GrapesJsTheme;

fn theme_head_html(theme_id: &str, theme: &GrapesJsTheme) -> String {
    if theme_id.is_empty() {
        return String::new();
    }
    let escaped_id = html_escape(theme_id);
    let mut b = String::new();
    for href in &theme.stylesheets {
        let href = href.trim();
        if href.is_empty() {
            continue;
        }
        b.push_str("<link rel=\"stylesheet\" href=\"");
        b.push_str(&html_escape(href));
        b.push_str("\" data-lariv-theme=\"");
        b.push_str(&escaped_id);
        b.push_str("\">\n");
    }
    let css = theme.css.trim();
    if !css.is_empty() {
        b.push_str("<style data-lariv-theme=\"");
        b.push_str(&escaped_id);
        b.push_str("\">\n");
        b.push_str(css);
        b.push_str("\n</style>\n");
    }
    b
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn strip_lariv_theme_assets(html_doc: &str) -> String {
    let mut out = html_doc.to_string();
    loop {
        let lower = out.to_lowercase();
        if let Some((start, end)) = find_tagged_block(&lower, "<style", "data-lariv-theme", "</style>")
        {
            out.replace_range(start..end, "");
            continue;
        }
        if let Some((start, end)) = find_link_tag(&lower, "data-lariv-theme") {
            out.replace_range(start..end, "");
            continue;
        }
        break;
    }
    out
}

fn find_tagged_block(lower: &str, open: &str, marker: &str, close: &str) -> Option<(usize, usize)> {
    let mut search = 0;
    while let Some(rel) = lower[search..].find(open) {
        let start = search + rel;
        let after_open = start + open.len();
        let gt = lower[after_open..].find('>')?;
        let tag_end = after_open + gt + 1;
        if lower[start..tag_end].contains(marker)
            && let Some(crel) = lower[tag_end..].find(close)
        {
            let mut end = tag_end + crel + close.len();
            if lower.as_bytes().get(end) == Some(&b'\n') {
                end += 1;
            }
            return Some((start, end));
        }
        search = tag_end;
    }
    None
}

fn find_link_tag(lower: &str, marker: &str) -> Option<(usize, usize)> {
    let mut search = 0;
    while let Some(rel) = lower[search..].find("<link") {
        let start = search + rel;
        let after_open = start + 5;
        let gt = lower[after_open..].find('>')?;
        let mut end = after_open + gt + 1;
        if lower[start..end].contains(marker) {
            if lower.as_bytes().get(end) == Some(&b'\n') {
                end += 1;
            }
            return Some((start, end));
        }
        search = end;
    }
    None
}

/// Insert or replace theme stylesheets/CSS into an HTML document.
pub fn inject_theme_assets(html_doc: &str, theme_id: &str, theme: Option<&GrapesJsTheme>) -> String {
    let html_doc = strip_lariv_theme_assets(html_doc);
    let Some(theme) = theme else {
        return html_doc;
    };
    let block = theme_head_html(theme_id, theme);
    if block.is_empty() {
        return html_doc;
    }

    let lower = html_doc.to_lowercase();
    if let Some(idx) = lower.find("</head>") {
        return format!("{}{}{}", &html_doc[..idx], block, &html_doc[idx..]);
    }
    if let Some(idx) = lower.find("<body") {
        return format!(
            "{}<head>\n{}</head>\n{}",
            &html_doc[..idx],
            block,
            &html_doc[idx..]
        );
    }
    if lower.contains("<html") {
        return format!("{block}{html_doc}");
    }
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n{block}</head>\n<body>\n{html_doc}\n</body>\n</html>\n"
    )
}
