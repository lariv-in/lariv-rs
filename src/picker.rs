//! Type-safe FK / many-to-many picker responses.
//!
//! Picker routes return either a table fragment (pagination/filter, typeahead dropdown)
//! or a modal dialog wrapping that table (initial open). [`respond_picker_select`] is
//! the single entry point handlers should use.
//!
//! # Filter forms inside modals
//!
//! Filter and pagination inside a picker table target
//! [`HX_TARGET_CLOSEST_TABLE`](crate::components::HX_TARGET_CLOSEST_TABLE) so two
//! pickers of the same entity do not swap each other. [`form_hx_get_picker_route`](crate::components::form_hx_get_picker_route)
//! also sets `outerHTML` (not `outerMorph`) on that closest table.
//!
//! # Typeahead dropdown
//!
//! [`input_foreign_key`](crate::components::input_foreign_key) GETs the picker URL on
//! debounced input and swaps the list table into a dropdown whose id starts with
//! [`FK_DROPDOWN_ID_PREFIX`]. The typed value is sent as the picker filter key
//! (default `Name`). [`respond_picker_select`] returns the table fragment for those
//! targets so the widget can show matching rows.
//!
//! When the picker table includes a create-modal button (`.fk-modal-host`), the widget
//! shows a **Create New…** footer that opens that create modal. After a successful create,
//! [`respond_create_modal_done_fk`](crate::web::respond_create_modal_done_fk) writes the
//! new row into the FK field via `lariv-fk-created` (it does not refresh the picker table).
//!
//! # Query parameters
//!
//! List filter structs nested via `#[serde(flatten)]` in picker [`Query`] types must use
//! [`QueryPage`](crate::web::QueryPage) for `page` fields, [`QueryI64`](crate::web::QueryI64)
//! for optional ID filter fields, and [`QueryStr`](crate::web::QueryStr) for optional text
//! filters — see [`crate::web::query`].

use std::marker::PhantomData;

use maud::Markup;

use crate::components::modal::modal_keyed;
use crate::components::swap::SwapKey;
use crate::components::{ButtonModalForm, button_modal_form};
use crate::web::{CreateModal, Htmx, modal_create_get_for_picker};

pub use crate::components::htmx::FK_DROPDOWN_ID_PREFIX;

fn targets_fk_search_dropdown(htmx: &Htmx) -> bool {
    htmx.target_id
        .as_deref()
        .is_some_and(|id| id.starts_with(FK_DROPDOWN_ID_PREFIX))
}

/// HTMX response: modal dialog appended to `document.body`.
#[derive(Debug, Clone)]
pub struct ModalHtml(Markup);

impl ModalHtml {
    /// Wrap `inner` in a keyed `<dialog class="modal …">`.
    pub fn wrap<M: SwapKey>(inner: Markup) -> Self {
        Self(modal_keyed::<M>("", inner))
    }

    pub fn into_inner(self) -> Markup {
        self.0
    }
}

/// HTMX response: table fragment for picker pagination/filter swaps.
pub struct TableFragmentHtml<K: SwapKey>(Markup, PhantomData<K>);

impl<K: SwapKey> TableFragmentHtml<K> {
    pub fn new(inner: Markup) -> Self {
        Self(inner, PhantomData)
    }

    pub fn into_inner(self) -> Markup {
        self.0
    }
}

/// Page type for FK or M2M picker routes.
///
/// Implementors provide `render_table` only; `render_modal` wraps it by default.
pub trait RenderPickerSelect<K: SwapKey, M: SwapKey> {
    fn render_table(&self) -> Markup;

    fn render_modal(&self) -> ModalHtml {
        ModalHtml::wrap::<M>(self.render_table())
    }
}

/// FK / M2M picker modal swap key paired with its inner table fragment key.
pub trait PickerModal: SwapKey {
    type Table: SwapKey;
}

/// Implement [`PickerModal`] pairing a picker dialog key with its inner table key.
#[macro_export]
macro_rules! impl_picker_modal {
    ($modal:ty, $table:ty) => {
        impl $crate::picker::PickerModal for $modal {
            type Table = $table;
        }
    };
}

/// Dispatch picker HTMX response: table fragment when targeting `K` or an FK
/// typeahead dropdown, modal otherwise.
pub fn respond_picker_select<K, M, P>(htmx: &Htmx, page: &P) -> Markup
where
    K: SwapKey,
    M: SwapKey,
    P: RenderPickerSelect<K, M>,
{
    if htmx.targets::<K>() || htmx.sourced_from::<K>() || targets_fk_search_dropdown(htmx) {
        page.render_table()
    } else {
        page.render_modal().into_inner()
    }
}

/// Create button for an FK picker modal; fills field `target_input` after create.
pub fn picker_create_button<M: CreateModal>(
    target_input: &str,
    icon_name: Option<&str>,
    classes: &str,
) -> maud::Markup {
    let href = modal_create_get_for_picker::<M>(target_input);
    button_modal_form(ButtonModalForm {
        name: "",
        href: &href,
        form_post_url: "",
        modal_uid: M::ID,
        icon_name,
        classes,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use maud::html;

    use super::*;
    use crate::components::swap::SwapKey;

    lariv_rs::swap_key!(TestPickerTableKey, "test-picker-table");
    lariv_rs::swap_key!(TestPickerModalKey, "test-picker-modal");

    struct DummyPicker;

    impl RenderPickerSelect<TestPickerTableKey, TestPickerModalKey> for DummyPicker {
        fn render_table(&self) -> Markup {
            html! { p { "pick me" } }
        }
    }

    #[test]
    fn modal_wrap_contains_dialog() {
        let m = ModalHtml::wrap::<TestPickerModalKey>(html! { "x" });
        let s = m.into_inner().into_string();
        assert!(s.contains("<dialog"));
        assert!(s.contains("modal"));
    }

    #[test]
    fn respond_picker_select_open_returns_modal() {
        let htmx = Htmx::default();
        let out =
            respond_picker_select::<TestPickerTableKey, TestPickerModalKey, _>(&htmx, &DummyPicker);
        assert!(out.into_string().contains("<dialog"));
    }

    #[test]
    fn respond_picker_select_table_refresh_from_source_id() {
        let mut htmx = Htmx::default();
        htmx.request = true;
        htmx.source_id = Some(TestPickerTableKey::ID.to_string());
        let out =
            respond_picker_select::<TestPickerTableKey, TestPickerModalKey, _>(&htmx, &DummyPicker);
        let s = out.into_string();
        assert!(s.contains("pick me"));
        assert!(!s.contains("<dialog"));
    }

    #[test]
    fn respond_picker_select_table_refresh_from_instance_source_id() {
        let mut htmx = Htmx::default();
        htmx.request = true;
        htmx.source_id = Some(format!("{}--abc123", TestPickerTableKey::ID));
        let out =
            respond_picker_select::<TestPickerTableKey, TestPickerModalKey, _>(&htmx, &DummyPicker);
        let s = out.into_string();
        assert!(s.contains("pick me"));
        assert!(!s.contains("<dialog"));
    }

    #[test]
    fn respond_picker_select_typeahead_dropdown_returns_table() {
        let mut htmx = Htmx::default();
        htmx.request = true;
        htmx.target_id = Some(format!("{FK_DROPDOWN_ID_PREFIX}role_id"));
        let out =
            respond_picker_select::<TestPickerTableKey, TestPickerModalKey, _>(&htmx, &DummyPicker);
        let s = out.into_string();
        assert!(s.contains("pick me"));
        assert!(!s.contains("<dialog"));
    }
}
