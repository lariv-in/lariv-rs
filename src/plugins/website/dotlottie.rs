//! DotLottie CDN inject.

pub const DOTLOTTIE_CDN_VERSION: &str = "0.9.17";
pub const DOTLOTTIE_CDN_URL: &str =
    "https://unpkg.com/@lottiefiles/dotlottie-wc@0.9.17/dist/dotlottie-wc.js";
pub const DOTLOTTIE_SCRIPT_ATTR: &str = "data-lariv-dotlottie";

fn script_tag() -> String {
    format!(r#"<script type="module" src="{DOTLOTTIE_CDN_URL}" {DOTLOTTIE_SCRIPT_ATTR}></script>"#)
}

/// Insert pinned DotLottie CDN script when HTML uses `<dotlottie-wc>` and loader is absent.
pub fn inject_dotlottie_script(html: &str) -> String {
    let lower = html.to_lowercase();
    if !lower.contains("dotlottie-wc") {
        return html.to_string();
    }
    if lower.contains(DOTLOTTIE_SCRIPT_ATTR) {
        return html.to_string();
    }
    let tag = script_tag();
    if let Some(idx) = lower.rfind("</body>") {
        format!("{}{}\n{}", &html[..idx], tag, &html[idx..])
    } else {
        format!("{html}\n{tag}\n")
    }
}
