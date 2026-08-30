//! Shared list-filter fields (page size) appended to every table filter panel.
//!
//! Use [`with_list_filter_common`] in list/picker filter forms and
//! [`page_size_only_filter_form`] when the panel has no entity filters.

use maud::{Markup, html};

use crate::components::button::{ButtonClear, ButtonSubmit, button_clear, button_submit};
use crate::components::container::container_row;
use crate::components::form::{FormOpts, form};
use crate::components::swap::{SwapKey, form_hx_get_route};
use crate::components::table::{clamp_page_size, page_size_choice_pairs};
use crate::html_form::{FormCtx, HtmlForm, html_form, widgets::Select};
use crate::http::{FragmentGet, RouteUrl};

/// Common fields rendered on every list/picker filter form.
#[html_form]
pub struct ListFilterCommonForm {
    #[form(
        label = "Page size",
        widget = Select,
        choices = "page_size",
        required,
        name = "page_size"
    )]
    pub page_size: String,
}

/// Render the shared page-size Select for the current value.
pub fn list_filter_common_inputs(page_size: u32) -> Markup {
    let page_size = clamp_page_size(Some(page_size));
    let choices = page_size_choice_pairs();
    let value = page_size.to_string();
    ListFilterCommonForm::render_inputs(
        &FormCtx::form::<ListFilterCommonForm>()
            .value(ListFilterCommonFormField::PageSize, value.as_str())
            .choices(ListFilterCommonFormField::PageSize, &choices),
    )
}

/// Append shared list-filter fields after entity-specific filter inputs.
pub fn with_list_filter_common(entity_inputs: Markup, page_size: u32) -> Markup {
    html! {
        (entity_inputs)
        (list_filter_common_inputs(page_size))
    }
}

fn filter_form_actions(apply_label: &str) -> Markup {
    html! {
        (container_row("flex gap-2", html! {
            (button_submit(ButtonSubmit { label: apply_label, ..Default::default() }))
            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
        }))
    }
}

/// Page-size-only filter panel for list views that have no other filters.
///
/// `extra_inputs` can carry hidden fields (e.g. `tab`) that must survive Apply.
pub fn page_size_only_filter_form_with_extras<K, R>(page_size: u32, extra_inputs: Markup) -> Markup
where
    K: SwapKey,
    R: FragmentGet<K> + RouteUrl + Copy + Default,
{
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: html! {
            (list_filter_common_inputs(page_size))
            (extra_inputs)
        },
        actions: filter_form_actions("Apply Filters"),
        ..Default::default()
    })
}

/// Page-size-only filter panel for list views that have no other filters.
pub fn page_size_only_filter_form<K, R>(page_size: u32) -> Markup
where
    K: SwapKey,
    R: FragmentGet<K> + RouteUrl + Copy + Default,
{
    page_size_only_filter_form_with_extras::<K, R>(page_size, Markup::default())
}

/// Page-size-only filter panel for FK picker modals (`hx-push-url=false`).
pub fn page_size_only_picker_filter_form<K, R>(page_size: u32) -> Markup
where
    K: SwapKey,
    R: FragmentGet<K> + RouteUrl + Copy + Default,
{
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()).set("hx-push-url", "false"),
        inputs: list_filter_common_inputs(page_size),
        actions: filter_form_actions("Apply"),
        ..Default::default()
    })
}
