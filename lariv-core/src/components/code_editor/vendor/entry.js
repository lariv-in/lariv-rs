// Bundled CodeMirror 6 runtime for Lariv's code editor (no CDN).
// Rebuild: npm install && npm run build
import { basicSetup } from "codemirror";
import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { javascript } from "@codemirror/lang-javascript";
import { markdown } from "@codemirror/lang-markdown";
import { html } from "@codemirror/lang-html";
import { typst_lezer } from "codemirror-lang-typst/lezer";

const views = new WeakMap();

function languageExtensions(lang) {
  if (lang === "javascript") return [javascript()];
  if (lang === "markdown") return [markdown()];
  if (lang === "typst") return [typst_lezer()];
  if (lang === "html") return [html()];
  return [];
}

export async function mount(root) {
  if (!root || views.get(root)) return;
  const ta = root.querySelector("textarea[data-code-editor-input]");
  const host = root.querySelector("[data-code-editor-host]");
  if (!ta || !host) return;
  // Claim the slot before await so concurrent Alpine inits do not double-mount.
  views.set(root, { destroy() {} });
  try {
    if (!EditorView || !EditorState || !basicSetup) {
      throw new Error(
        "CodeMirror 6 modules missing exports (EditorView/EditorState/basicSetup)",
      );
    }
    const rows = Number(root.dataset.rows || "12");
    const lang = root.dataset.language || "plaintext";
    const langExt = languageExtensions(lang);
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
}
