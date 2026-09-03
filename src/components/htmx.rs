//! Shared HTMX attribute helpers built on [`crate::components::swap::SwapKey`].

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::swap::{
    ModalHostKey, SwapKey, form_hx_get_for_url, form_hx_get_route, form_hx_post_for_url,
    form_hx_post_route, nav_main_attrs,
};
use crate::http::{FkSelectGet, FragmentGet, FragmentPost, RouteUrl};

/// HTMX target/swap for appending modals as children of `document.body`.
pub const HTMX_TARGET_BODY_MODAL: &str = ModalHostKey::SELECTOR;
pub const HTMX_SWAP_BODY_MODAL: &str = "beforeend";

/// Element id prefix for FK typeahead dropdown swap targets (`fk-dropdown-{field}`).
pub const FK_DROPDOWN_ID_PREFIX: &str = "fk-dropdown-";

/// Declarative POST form attrs targeting a typed region (replaces form bubbling).
pub fn form_post_region<K: SwapKey>(action: &str) -> HtmlAttrs {
    form_hx_post_for_url::<K>(action)
}

/// Typed POST form attrs for a fragment route value.
pub fn form_post_region_route<K: SwapKey, R: RouteUrl + FragmentPost<K>>(route: R) -> HtmlAttrs {
    form_hx_post_route::<K, R>(route)
}

/// Declarative GET filter form attrs targeting the closest data table.
pub fn form_get_region<K: SwapKey>(action: &str) -> HtmlAttrs {
    let _ = K::ID;
    form_hx_get_for_url(action)
}

/// Typed GET filter form attrs for a fragment route value.
pub fn form_get_region_route<K: SwapKey, R: RouteUrl + FragmentGet<K>>(route: R) -> HtmlAttrs {
    form_hx_get_route::<K, R>(route)
}

/// Row click attrs that navigate into [`AppLayoutKey`](crate::components::swap::AppLayoutKey)
/// via HTMX (updates sidebar + main on detail pages).
pub fn row_attr_navigate(url: &str) -> HtmlAttrs {
    nav_main_attrs(url)
}

/// Typed row click navigation for a route value.
pub fn row_attr_navigate_route(route: impl RouteUrl) -> HtmlAttrs {
    row_attr_navigate(&route.url())
}

/// Row click attrs that dispatch `fk-select` and close the picker modal.
///
/// Typeahead rows live inside `.fk-picker-results` on the parent form, so those
/// clicks must not `remove()` the enclosing create/edit dialog.
pub fn row_attr_select(name: &str, value: &str, display: &str) -> HtmlAttrs {
    row_attr_select_extra(name, value, display, &[])
}

fn fk_select_click_js(name: &str, value: &str, display: &str, extra: &[(&str, &str)]) -> String {
    let value_json = if let Ok(n) = value.parse::<u64>() {
        serde_json::Value::from(n)
    } else {
        serde_json::Value::from(value)
    };
    let mut detail = serde_json::json!({
        "name": name,
        "value": value_json,
        "display": display,
    });
    if let Some(map) = detail.as_object_mut() {
        for (k, v) in extra {
            map.insert((*k).to_string(), serde_json::Value::from(*v));
        }
    }
    format!(
        "$dispatch('fk-select', {detail}); const el=$event.currentTarget; if(el.closest('.fk-picker-results')) return; el.closest('dialog.modal')?.remove()"
    )
}

/// Like [`row_attr_select`], merging `extra` key/value pairs into the event detail.
///
/// Use this when the consumer needs payload beyond id/label (e.g. product
/// `sales_price` for invoice line rate autofill).
pub fn row_attr_select_extra(
    name: &str,
    value: &str,
    display: &str,
    extra: &[(&str, &str)],
) -> HtmlAttrs {
    HtmlAttrs::new()
        .set(
            "class",
            "cursor-pointer hover:bg-base-200 transition-colors",
        )
        .set("@click", fk_select_click_js(name, value, display, extra))
}

/// Action button that fills an FK field with `value`/`display` and closes the picker.
pub fn button_fk_select(label: &str, name: &str, value: &str, display: &str) -> Markup {
    let js = fk_select_click_js(name, value, display, &[]);
    html! {
        (PreEscaped(format!(
            r#"<button type="button" class="btn btn-outline btn-sm" @click="{}">"#,
            escape_attr(&js),
        )))
        (label)
        (PreEscaped("</button>"))
    }
}

/// Row click attrs for many-to-many pickers.
pub fn row_attr_select_multi(name: &str, value: &str, display: &str) -> HtmlAttrs {
    row_attr_select_multi_extra(name, value, display, &[])
}

/// Like [`row_attr_select_multi`], merging `extra` key/value pairs into the event detail.
pub fn row_attr_select_multi_extra(
    name: &str,
    value: &str,
    display: &str,
    extra: &[(&str, &str)],
) -> HtmlAttrs {
    let mut detail = serde_json::json!({
        "name": name,
        "value": value,
        "display": display,
    });
    if let Some(map) = detail.as_object_mut() {
        for (k, v) in extra {
            map.insert((*k).to_string(), serde_json::Value::from(*v));
        }
    }
    let js = format!("$dispatch('fk-multi-select', {})", detail);
    let class_expr = format!(
        "((Alpine.store('m2mSelections') && Alpine.store('m2mSelections')[{name:?}]) || []).some(item => item.Key === {value:?}) ? 'bg-success text-success-content hover:bg-success border-success' : 'hover:bg-base-200'"
    );
    HtmlAttrs::new()
        .set("class", "cursor-pointer transition-colors")
        .set(":class", class_expr)
        .set("@click", js)
}

/// Response `<head>` for hx-head append swaps.
pub fn hx_head_append(children: Markup) -> Markup {
    html! {
        head hx-head="append" {
            (children)
        }
    }
}

/// Prepend an append-only response head before a partial body fragment.
pub fn hx_partial_with_head(head: Markup, body: Markup) -> Markup {
    html! {
        (hx_head_append(head))
        (body)
    }
}

/// Attrs to open a modal into [`ModalHostKey`] (`body` / beforeend).
pub fn modal_open_attrs(href: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set("hx-get", href)
        .set("hx-target", ModalHostKey::SELECTOR)
        .set("hx-swap", HTMX_SWAP_BODY_MODAL)
        .set("hx-push-url", "false")
}

/// Typed picker open attrs for routes declared with `fk_select` / `multi_select`.
pub fn modal_picker_open_route<R, K, M>(route: R) -> HtmlAttrs
where
    R: RouteUrl + FkSelectGet<K, M>,
    K: SwapKey,
    M: SwapKey,
{
    modal_open_attrs(&route.url())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_attr_select_skips_removing_dialog_from_typeahead() {
        let html = row_attr_select("CustomerId", "1", "Acme").as_string();
        assert!(html.contains("fk-select"), "{html}");
        assert!(html.contains("fk-picker-results"), "{html}");
        assert!(html.contains("dialog.modal"), "{html}");
    }
}
