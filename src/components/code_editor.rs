//! CodeMirror 6–backed code editor for hand-built forms and [`crate::html_form`] widgets.
//!
//! Loads CM6 via ESM CDN on mount. A hidden `<textarea>` remains the form submit source;
//! the editor syncs into it on every change. Programmatic updates: set the textarea value
//! and dispatch `code-editor:set` on the root (detail optional `{ value }`), or fire
//! `change` on the textarea.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::label::label_hint;

/// CodeMirror 6 code editor input.
pub struct CodeEditorInput<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    /// `id` on the backing textarea (for external buttons / labels).
    pub id: &'a str,
    /// Language mode key (`plaintext`, `javascript`, `markdown`, …). Default `plaintext`.
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

/// Bootstrap that defines `window.LarivCodeEditor.mount` once (ESM CDN).
const CODE_EDITOR_BOOTSTRAP: &str = r#"
<script type="module">
if (!window.LarivCodeEditor) {
  const views = new WeakMap();
  async function languageExtensions(lang) {
    if (lang === "javascript") {
      const { javascript } = await import("https://esm.sh/@codemirror/lang-javascript@6");
      return [javascript()];
    }
    if (lang === "markdown") {
      const { markdown } = await import("https://esm.sh/@codemirror/lang-markdown@6");
      return [markdown()];
    }
    return [];
  }
  window.LarivCodeEditor = {
    async mount(root) {
      if (!root || views.get(root)) return;
      const ta = root.querySelector("textarea[data-code-editor-input]");
      const host = root.querySelector("[data-code-editor-host]");
      if (!ta || !host) return;
      // Claim the slot before await so concurrent Alpine inits do not double-mount.
      views.set(root, { destroy() {} });
      try {
        // Pin codemirror@6.0.2 — bare `codemirror@6` on esm.sh resolves to legacy CM5 (6.65.x).
        const [cm, viewMod, stateMod] = await Promise.all([
          import("https://esm.sh/codemirror@6.0.2"),
          import("https://esm.sh/@codemirror/view@6"),
          import("https://esm.sh/@codemirror/state@6"),
        ]);
        const { basicSetup } = cm;
        const { EditorView } = viewMod;
        const { EditorState } = stateMod;
        if (!EditorView || !EditorState || !basicSetup) {
          throw new Error(
            "CodeMirror 6 modules missing exports (EditorView/EditorState/basicSetup)",
          );
        }
        const rows = Number(root.dataset.rows || "12");
        const lang = root.dataset.language || "plaintext";
        const langExt = await languageExtensions(lang);
        const rowHeight = `${Math.max(rows, 4) * 1.5}rem`;
        const maxHeight = root.dataset.maxHeight || rowHeight;
        const sync = EditorView.updateListener.of((v) => {
          if (v.docChanged) {
            ta.value = v.state.doc.toString();
          }
        });
        root.style.height = rowHeight;
        root.style.maxHeight = maxHeight;
        host.style.height = "100%";
        host.style.maxHeight = "100%";
        host.replaceChildren();
        const view = new EditorView({
          parent: host,
          state: EditorState.create({
            doc: ta.value,
            extensions: [
              basicSetup,
              ...langExt,
              sync,
              EditorView.theme({
                "&": {
                  width: "100%",
                  maxWidth: "100%",
                  height: "100%",
                  maxHeight: "100%",
                  border: "1px solid color-mix(in oklab, CanvasText 20%, transparent)",
                  borderRadius: "0.5rem",
                  fontSize: "0.875rem",
                },
                "&.cm-focused": {
                  outline: "2px solid color-mix(in oklab, CanvasText 35%, transparent)",
                },
                ".cm-scroller": {
                  width: "100%",
                  height: "100%",
                  overflow: "auto",
                  fontFamily: "Roboto Mono, ui-monospace, SFMono-Regular, Menlo, monospace",
                  lineHeight: "1.5",
                },
                ".cm-content": {
                  width: "100%",
                },
              }),
            ],
          }),
        });
        views.set(root, view);
        const applyText = (text) => {
          const next = text == null ? ta.value : String(text);
          ta.value = next;
          view.dispatch({
            changes: { from: 0, to: view.state.doc.length, insert: next },
          });
        };
        root.addEventListener("code-editor:set", (e) => {
          applyText(e.detail && e.detail.value != null ? e.detail.value : ta.value);
        });
        ta.addEventListener("change", () => {
          if (ta.value !== view.state.doc.toString()) applyText(ta.value);
        });
      } catch (err) {
        views.delete(root);
        console.error("LarivCodeEditor.mount failed", err);
      }
    },
  };
  window.dispatchEvent(new Event("lariv-code-editor-ready"));
}
</script>
"#;

fn mount_init_attr() -> String {
    // Wait for deferred bootstrap module, then mount (HTMX + Alpine re-init safe).
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
        (PreEscaped(CODE_EDITOR_BOOTSTRAP))
        @if opts.label.is_empty() && opts.hint.is_none() {
            (editor)
        } @else {
            (label_hint(opts.label, opts.hint, editor))
        }
    }
}
