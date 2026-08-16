//! Published HTML assembly.

use crate::grapesjs::GrapesJsTheme;

use super::{dotlottie::inject_dotlottie_script, theme::inject_theme_assets};

pub fn build_published_html(html: &str, css: &str) -> String {
    let html = html.trim();
    let css = css.trim();

    let style_block = if css.is_empty() {
        String::new()
    } else {
        format!("<style>\n{css}\n</style>\n")
    };

    let lower = html.to_lowercase();
    if lower.contains("<html") {
        if let Some(idx) = lower.find("</head>") {
            return format!("{}{}{}", &html[..idx], style_block, &html[idx..]);
        }
        if let Some(idx) = lower.find("<body") {
            return format!(
                "{}<head>\n{}</head>\n{}",
                &html[..idx],
                style_block,
                &html[idx..]
            );
        }
        return format!("{style_block}{html}");
    }

    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n{style_block}</head>\n<body>\n{html}\n</body>\n</html>\n"
    )
}

pub fn build_published_html_with_theme(
    html: &str,
    css: &str,
    theme_id: &str,
    theme: Option<&GrapesJsTheme>,
) -> String {
    inject_theme_assets(&build_published_html(html, css), theme_id, theme)
}

pub fn finalize_published_html(
    html: &str,
    css: &str,
    theme_id: &str,
    theme: Option<&GrapesJsTheme>,
) -> String {
    inject_dotlottie_script(&build_published_html_with_theme(html, css, theme_id, theme))
}

/// Ensure navbar logo `<img>` `src` matches `data-logo-src` on the parent `<nav>`.
pub fn fix_navbar_logos(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    loop {
        let start = match rest.find("<nav") {
            Some(i) => i,
            None => {
                out.push_str(rest);
                break;
            }
        };
        out.push_str(&rest[..start]);
        let nav_start = &rest[start..];
        let end_rel = nav_start.find("</nav>");
        let (nav_block, close_len) = if let Some(i) = end_rel {
            (&nav_start[..i], 6)
        } else {
            (nav_start, 0)
        };
        out.push_str(&fix_one_navbar(nav_block));
        if close_len > 0 {
            out.push_str("</nav>");
        }
        rest = &nav_start[nav_block.len() + close_len..];
    }
    out
}

fn fix_one_navbar(nav: &str) -> String {
    let logo = extract_attr_value(nav, "data-logo-src").unwrap_or_default();
    if logo.is_empty() {
        return nav.to_string();
    }
    let alt = extract_attr_value(nav, "data-logo-alt").unwrap_or_else(|| "Logo".into());
    if let Some((start, end)) = find_navbar_logo_img(nav) {
        let fixed_img = build_navbar_logo_img(&nav[start..end], &logo, &alt);
        let mut out = String::with_capacity(nav.len());
        out.push_str(&nav[..start]);
        out.push_str(&fixed_img);
        out.push_str(&nav[end..]);
        out
    } else {
        nav.to_string()
    }
}

fn find_navbar_logo_img(nav: &str) -> Option<(usize, usize)> {
    let mut search_from = 0;
    while let Some(img_rel) = nav[search_from..].find("<img") {
        let start = search_from + img_rel;
        let tag_end = nav[start..].find('>').map(|i| start + i + 1)?;
        let tag = &nav[start..tag_end];
        if tag.contains("gjs-navbar-logo") {
            return Some((start, tag_end));
        }
        search_from = tag_end;
    }
    None
}

fn build_navbar_logo_img(old_tag: &str, src: &str, alt: &str) -> String {
    let class = extract_attr_value(old_tag, "class")
        .unwrap_or_else(|| "gjs-navbar-logo h-12 w-auto max-w-[10rem] object-contain".into());
    let id = extract_attr_value(old_tag, "id");
    let mut img = format!(
        "<img alt=\"{}\" data-gjs-type=\"p_website.navbar-logo\" class=\"{}\" src=\"{}\"",
        escape_html_attr(alt),
        escape_html_attr(&class),
        escape_html_attr(src),
    );
    if let Some(id) = id {
        img.push_str(&format!(" id=\"{}\"", escape_html_attr(&id)));
    }
    img.push('>');
    img
}

fn extract_attr_value(tag: &str, name: &str) -> Option<String> {
    let dq = format!("{name}=\"");
    let sq = format!("{name}='");
    if let Some(start) = tag.find(&dq) {
        let value_start = start + dq.len();
        if let Some(end) = tag[value_start..].find('"') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    if let Some(start) = tag.find(&sq) {
        let value_start = start + sq.len();
        if let Some(end) = tag[value_start..].find('\'') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    None
}

fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fix_navbar_logos_replaces_placeholder_src() {
        let html = r#"<nav data-gjs-type="p_website.navbar" data-logo-src="https://example.com/logo.svg" data-logo-alt="Logo" class="gjs-navbar navbar">
<a href="/"><img alt="Logo" class="gjs-navbar-logo h-8" src="data:image/svg+xml;base64,PHN2Zy"></a>
</nav>"#;
        let fixed = fix_navbar_logos(html);
        assert!(fixed.contains("src=\"https://example.com/logo.svg\""));
        assert!(!fixed.contains("data:image/svg+xml"));
    }

    #[test]
    fn fix_navbar_logos_skips_nav_without_logo_src() {
        let html = r#"<nav data-logo-src="" class="gjs-navbar"><img class="gjs-navbar-logo" src="data:image/svg+xml"></nav>"#;
        let fixed = fix_navbar_logos(html);
        assert!(fixed.contains("data:image/svg+xml"));
    }
}
