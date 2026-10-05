//! CodeMirror 6–backed code editor for hand-built forms and [`crate::html_form`] widgets.
//!
//! Loads a locally bundled CM6 ESM module on mount (`/codemirror.js`). A hidden `<textarea>`
//! remains the form submit source; the editor syncs into it on every change. Programmatic
//! updates: set the textarea value and dispatch `code-editor:set` on the root (detail optional
//! `{ value }`), or fire `change` on the textarea.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::label::label_hint;
use crate::components::shell::codemirror_js_href;

/// CodeMirror 6 code editor input.
pub struct CodeEditorInput<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    /// `id` on the backing textarea (for external buttons / labels).
    pub id: &'a str,
    /// Language mode key (`plaintext`, `javascript`, `markdown`, `html`, `typst`, …). Default `plaintext`.
    pub language: &'a str,
    /// Visible height in text rows (editor scrolls when content exceeds this).
    pub rows: u32,
    /// Optional CSS max-height (e.g. `"24rem"`, `"70vh"`). Defaults to the rows height.
    pub max_height: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
    pub hint: Option<&'a str>,
}

impl Default for CodeEditorInput<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            value: "",
            id: "",
            language: "plaintext",
            rows: 12,
            max_height: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
            hint: None,
        }
    }
}

fn code_editor_bootstrap() -> String {
    let href = escape_attr(&codemirror_js_href());
    format!(
        r#"<script type="module">
if (!window.LarivCodeEditor) {{
  window.LarivCodeEditor = await import("{href}");
  window.dispatchEvent(new Event("lariv-code-editor-ready"));
}}
</script>"#
    )
}

fn mount_init_attr() -> String {
    // Wait for the deferred bootstrap module, then mount (HTMX + Alpine re-init safe).
    // `import()` stays in the module script — Alpine x-init runs via Function().
    escape_attr(
        "(async () => { if (!window.LarivCodeEditor) { await new Promise((r) => { const done = () => { if (window.LarivCodeEditor) r(); }; window.addEventListener('lariv-code-editor-ready', done, { once: true }); done(); const id = setInterval(() => { if (window.LarivCodeEditor) { clearInterval(id); r(); } }, 20); }); } await window.LarivCodeEditor.mount($el); })()",
    )
}

/// Render a CodeMirror 6 editor with a hidden form textarea.
pub fn code_editor_input(opts: CodeEditorInput<'_>) -> Markup {
    let language = if opts.language.is_empty() {
        "plaintext"
    } else {
        opts.language
    };
    let rows = if opts.rows == 0 { 12 } else { opts.rows };
    let max_height_attr = if opts.max_height.is_empty() {
        String::new()
    } else {
        format!(r#" data-max-height="{}""#, escape_attr(opts.max_height))
    };
    let required_attr = if opts.required { " required" } else { "" };
    let id_attr = if opts.id.is_empty() {
        String::new()
    } else {
        format!(r#" id="{}""#, escape_attr(opts.id))
    };
    let x_init = mount_init_attr();

    let editor = html! {
        (PreEscaped(format!(
            r#"<div class="code-editor-input my-1 w-full max-w-full overflow-hidden {}" data-code-editor-root data-language="{}" data-rows="{}"{} x-data x-init="{}">"#,
            escape_attr(opts.classes),
            escape_attr(language),
            rows,
            max_height_attr,
            x_init,
        )))
        (PreEscaped(format!(
            r#"<textarea name="{}"{} rows="{}" class="hidden" data-code-editor-input{}{}>"#,
            escape_attr(opts.name),
            id_attr,
            rows,
            required_attr,
            opts.attrs.as_string(),
        )))
        (opts.value)
        (PreEscaped("</textarea>"))
        div data-code-editor-host class="h-full max-h-full w-full max-w-full overflow-hidden font-mono text-sm" {}
        (PreEscaped("</div>"))
    };

    html! {
        (PreEscaped(code_editor_bootstrap()))
        @if opts.label.is_empty() && opts.hint.is_none() {
            (editor)
        } @else {
            (label_hint(opts.label, opts.hint, editor))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_imports_vendored_codemirror() {
        let html = code_editor_input(CodeEditorInput {
            language: "typst",
            name: "Content",
            value: "= Hello",
            ..Default::default()
        })
        .into_string();
        assert!(html.contains(r#"data-language="typst""#), "{html}");
        assert!(html.contains(&codemirror_js_href()), "{html}");
        assert!(html.contains(r#"<script type="module">"#), "{html}");
        assert!(!html.contains("esm.sh"), "{html}");
        assert!(!html.contains("cdn.jsdelivr.net"), "{html}");
    }
}
