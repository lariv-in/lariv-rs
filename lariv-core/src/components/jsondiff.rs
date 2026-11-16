//! Side-by-side JSON diff rendered by the vendored jsondiffpatch HTML formatter.
//!
//! Both documents are embedded as JSON scripts. Alpine `x-init` mounts the diff after
//! HTMX swaps, and unchanged properties stay visible.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::shell::{jsondiffpatch_css_href, jsondiffpatch_js_href};

/// Two JSON documents to diff. Each string is JSON text (`null` is an empty side).
pub struct JsonDiff<'a> {
    pub left: &'a str,
    pub right: &'a str,
}

fn embed_json(json: &str) -> String {
    json.replace('<', "\\u003c")
}

fn bootstrap() -> String {
    let js = escape_attr(&jsondiffpatch_js_href());
    let css = escape_attr(&jsondiffpatch_css_href());
    format!(
        r#"<script>
if (!window.__larivJsonDiff) {{
  window.__larivJsonDiff = true;
  var link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = "{css}";
  document.head.appendChild(link);
  var script = document.createElement("script");
  script.src = "{js}";
  script.onload = function () {{ window.dispatchEvent(new Event("lariv-jsondiff-ready")); }};
  document.head.appendChild(script);
}}
</script>"#
    )
}

fn mount_init_attr() -> String {
    escape_attr(
        "(async () => { if (!window.LarivJsonDiff) { await new Promise((r) => { const done = () => { if (window.LarivJsonDiff) r(); }; window.addEventListener('lariv-jsondiff-ready', done, { once: true }); done(); const id = setInterval(() => { if (window.LarivJsonDiff) { clearInterval(id); r(); } }, 20); }); } window.LarivJsonDiff.mount($el); })()",
    )
}

/// Render a jsondiffpatch HTML diff of `left` and `right`.
///
/// Text that is not JSON is shown as-is when the browser cannot parse it.
pub fn json_diff(opts: JsonDiff<'_>) -> Markup {
    let left = embed_json(opts.left);
    let right = embed_json(opts.right);
    let x_init = mount_init_attr();
    html! {
        (PreEscaped(bootstrap()))
        div class="jsondiff text-xs" x-data x-init=(x_init) {
            (PreEscaped(format!(
                r#"<script type="application/json" data-jsondiff-side="left">{left}</script>"#
            )))
            (PreEscaped(format!(
                r#"<script type="application/json" data-jsondiff-side="right">{right}</script>"#
            )))
            div data-jsondiff-host {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JsonDiff;
    use super::json_diff;

    #[test]
    fn markup_embeds_both_documents_and_mounts() {
        let html = json_diff(JsonDiff {
            left: r#"{"name":"a"}"#,
            right: r#"{"name":"b","note":"</script>"}"#,
        })
        .into_string();
        assert!(html.contains(r#"data-jsondiff-side="left""#));
        assert!(html.contains(r#"data-jsondiff-side="right""#));
        assert!(html.contains(r#"{"name":"a"}"#));
        assert!(html.contains(r"\u003c/script>"));
        assert!(!html.contains("</script></script>"));
        assert!(html.contains("LarivJsonDiff.mount"));
        assert!(html.contains("/jsondiffpatch.js"));
        assert!(html.contains("/jsondiffpatch.css"));
    }
}
