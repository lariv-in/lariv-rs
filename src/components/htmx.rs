//! Shared HTMX attribute helpers built on [`crate::components::swap::SwapKey`].

use crate::components::attrs::HtmlAttrs;
use crate::components::swap::{ModalHostKey, SwapKey, form_hx_get, form_hx_post, nav_main_attrs};

/// HTMX target/swap for appending modals as children of `document.body`.
pub const HTMX_TARGET_BODY_MODAL: &str = ModalHostKey::SELECTOR;
pub const HTMX_SWAP_BODY_MODAL: &str = "beforeend";

/// Opt out of body-inherited `hx-select="#app-layout"` so fragment responses
/// (modals, tables, FK pickers) are swapped as-is.
///
/// HTMX 4 has no `unset` keyword: a present empty `hx-select=""` overrides
/// inheritance, and a falsy select skips response filtering. The literal
/// `"unset"` would be treated as a CSS selector and empty the swap.
pub const HTMX_SELECT_UNSET: &str = "";

/// Declarative POST form attrs targeting a typed region (replaces form bubbling).
pub fn form_post_region<K: SwapKey>(action: &str) -> HtmlAttrs {
    form_hx_post::<K>(action)
}

/// Declarative GET filter form attrs targeting a typed region.
pub fn form_get_region<K: SwapKey>(action: &str) -> HtmlAttrs {
    form_hx_get::<K>(action)
}

/// Row click attrs that navigate into [`AppLayoutKey`] via HTMX
/// (sidebar may change, e.g. list → detail).
pub fn row_attr_navigate(url: &str) -> HtmlAttrs {
    nav_main_attrs(url)
}

/// Row click attrs that dispatch `fk-select` and close the enclosing modal.
///
/// Alpine is retained only for this local FK display update; the select modal
/// table itself uses typed [`SwapKey`] regions.
pub fn row_attr_select(name: &str, value: &str, display: &str) -> HtmlAttrs {
    // Go `getters.Select` JSON-marshals numeric PKs as numbers, not strings.
    let value_json = if let Ok(n) = value.parse::<u64>() {
        serde_json::Value::from(n)
    } else {
        serde_json::Value::from(value)
    };
    let detail = serde_json::json!({
        "name": name,
        "value": value_json,
        "display": display,
    });
    let js = format!(
        "$dispatch('fk-select', {}); $event.currentTarget.closest('dialog.modal')?.remove()",
        detail
    );
    HtmlAttrs::new()
        .set(
            "class",
            "cursor-pointer hover:bg-base-200 transition-colors",
        )
        .set("@click", js)
}

/// Row click attrs for many-to-many pickers (Go `RowAttrSelectMulti`).
///
/// Dispatches `fk-multi-select` without closing the modal so multiple tags can
/// be toggled; selected styling comes from the Alpine `m2mSelections` store.
pub fn row_attr_select_multi(name: &str, value: &str, display: &str) -> HtmlAttrs {
    let detail = serde_json::json!({
        "name": name,
        "value": value,
        "display": display,
    });
    let js = format!("$dispatch('fk-multi-select', {})", detail);
    let class_expr = format!(
        "((Alpine.store('m2mSelections') && Alpine.store('m2mSelections')[{name:?}]) || []).some(item => item.Key === {value:?}) ? 'bg-success text-success-content hover:bg-success border-success' : 'hover:bg-base-200'"
    );
    HtmlAttrs::new()
        .set("class", "cursor-pointer transition-colors")
        .set(":class", class_expr)
        .set("@click", js)
}

/// Attrs to open a modal into [`ModalHostKey`] (`body` / beforeend).
pub fn modal_open_attrs(href: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("hx-get", href)
        .set("hx-target", ModalHostKey::SELECTOR)
        .set("hx-select", HTMX_SELECT_UNSET)
        .set("hx-swap", HTMX_SWAP_BODY_MODAL)
        .set("hx-push-url", "false")
}
