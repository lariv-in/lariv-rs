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
