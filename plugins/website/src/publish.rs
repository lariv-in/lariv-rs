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

/// Ensure navbar logo `<img>` `src`/`alt` and link lists match `data-*` attrs on `<nav>`.
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
    let mut out = nav.to_string();
    let logo = extract_attr_value(nav, "data-logo-src").unwrap_or_default();
    if !logo.is_empty() {
        let alt = extract_attr_value(nav, "data-logo-alt").unwrap_or_else(|| "Logo".into());
        if let Some((start, end)) = find_navbar_logo_img(&out) {
            let fixed_img = build_navbar_logo_img(&out[start..end], &logo, &alt);
            out = format!("{}{}{}", &out[..start], fixed_img, &out[end..]);
        }
    }
    if let Some(links_json) = extract_attr_value(nav, "data-nav-links") {
        if let Some(links_html) = nav_links_html(&links_json) {
            out = replace_nav_list_inner(&out, "gjs-navbar-links", &links_html);
            out = replace_nav_list_inner(&out, "gjs-navbar-mobile-links", &links_html);
        }
    }
    out
}

fn nav_links_html(raw: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(raw).ok()?;
    let arr = parsed.as_array()?;
    let mut html = String::new();
    for item in arr {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let path = item
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if title.is_empty() && path.is_empty() {
            continue;
        }
        let title = if title.is_empty() { "Link" } else { title };
        let path = if path.is_empty() { "#" } else { path };
        html.push_str(&format!(
            "<li><a class=\"nav-link\" href=\"{}\">{}</a></li>",
            escape_html_attr(path),
            escape_html_text(title),
        ));
    }
    Some(html)
}

fn replace_nav_list_inner(nav: &str, class_token: &str, inner: &str) -> String {
    let mut search_from = 0;
    while let Some(ul_rel) = nav[search_from..].find("<ul") {
        let start = search_from + ul_rel;
        let tag_end = match nav[start..].find('>') {
            Some(i) => start + i + 1,
            None => return nav.to_string(),
        };
        let tag = &nav[start..tag_end];
        if tag.contains(class_token) {
            let close = match nav[tag_end..].find("</ul>") {
                Some(i) => tag_end + i,
                None => return nav.to_string(),
            };
            return format!("{}{}{}", &nav[..tag_end], inner, &nav[close..]);
        }
        search_from = tag_end;
    }
    nav.to_string()
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
    let raw = if let Some(start) = tag.find(&dq) {
        let value_start = start + dq.len();
        let end = tag[value_start..].find('"')?;
        Some(tag[value_start..value_start + end].to_string())
    } else if let Some(start) = tag.find(&sq) {
        let value_start = start + sq.len();
        let end = tag[value_start..].find('\'')?;
        Some(tag[value_start..value_start + end].to_string())
    } else {
        None
    }?;
    Some(decode_html_attr(&raw))
}

fn decode_html_attr(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

    #[test]
    fn fix_navbar_syncs_links_from_data_attr() {
        let html = r##"<nav data-gjs-type="p_website.navbar" data-nav-links="[{&quot;title&quot;:&quot;CNC&quot;,&quot;path&quot;:&quot;#cnc&quot;},{&quot;title&quot;:&quot;Contact&quot;,&quot;path&quot;:&quot;#contact&quot;}]" class="gjs-navbar">
<ul class="gjs-navbar-links nav-menu"><li><a href="#old">Old</a></li></ul>
</nav>"##;
        let fixed = fix_navbar_logos(html);
        assert!(fixed.contains("href=\"#cnc\""));
        assert!(fixed.contains(">CNC</a>"));
        assert!(fixed.contains("href=\"#contact\""));
        assert!(!fixed.contains("#old"));
    }
}
