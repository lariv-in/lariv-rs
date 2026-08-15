//! Modal dialog overlay.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::swap::SwapKey;
use crate::components::text::icon;

pub struct Modal<'a> {
    pub uid: &'a str,
    pub classes: &'a str,
    pub children: Markup,
}

impl Default for Modal<'_> {
    fn default() -> Self {
        Self {
            uid: "modal",
            classes: "",
            children: Markup::default(),
        }
    }
}

pub fn modal(opts: Modal<'_>) -> Markup {
    modal_with_uid(opts.uid, opts.classes, opts.children, true)
}

/// Modal rooted at a compile-time [`SwapKey`] id.
pub fn modal_keyed<K: SwapKey>(classes: &str, children: Markup) -> Markup {
    modal_with_uid(K::ID, classes, children, true)
}

/// Picker/select dialog without the DaisyUI page dim (`background-color: oklch(0% 0 0/.4)`).
pub fn modal_keyed_undimmed<K: SwapKey>(classes: &str, children: Markup) -> Markup {
    modal_with_uid(K::ID, classes, children, false)
}

fn modal_with_uid(uid: &str, classes: &str, children: Markup, dim: bool) -> Markup {
    let onclick = format!("document.getElementById('{}').remove()", uid);
    let dialog_class = if dim {
        "modal modal-open fk-modal-container"
    } else {
        "modal modal-open fk-modal-container !bg-transparent"
    };
    html! {
        (PreEscaped(format!(
            r#"<dialog id="{}" class="{}" hx-push-url="false" hx-target="this" hx-swap="outerHTML">"#,
            escape_attr(uid),
            dialog_class
        )))
        div class=(format!(
            "modal-box max-w-4xl max-h-[90vh] overflow-y-auto bg-base-200 border border-base-content/10 {}",
            classes
        )) {
            form method="dialog" {
                (PreEscaped(format!(
                    r#"<button type="button" class="btn btn-sm btn-circle btn-outline btn-error absolute right-3 top-3 z-10" onclick="{}">"#,
                    escape_attr(&onclick)
                )))
                (icon("x-mark", ""))
                (PreEscaped("</button>"))
            }
            div class="mt-8" { (children) }
        }
        form method="dialog" class="modal-backdrop" {
            (PreEscaped(format!(
                r#"<button onclick="{}">close</button>"#,
                escape_attr(&onclick)
            )))
        }
        (PreEscaped("</dialog>"))
    }
}
