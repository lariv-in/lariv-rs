//! Type-safe FK / many-to-many picker responses.
//!
//! Picker routes return either a table fragment (pagination/filter) or a modal dialog
//! wrapping that table (initial open). [`respond_picker_select`] is the single entry
//! point handlers should use.
//!
//! # Filter forms inside modals
//!
//! FK picker modals set `hx-target="this"` on the `<dialog>`. Filter and pagination
//! forms inside the modal must use [`form_hx_get_picker_route`](crate::components::form_hx_get_picker_route)
//! (modal `outerHTML` swap), not [`form_hx_get_route`](crate::components::form_hx_get_route)
//! targeting the inner table fragment.
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
use crate::web::Htmx;

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

/// Dispatch picker HTMX response: table fragment when targeting `K`, modal otherwise.
pub fn respond_picker_select<K, M, P>(htmx: &Htmx, page: &P) -> Markup
where
    K: SwapKey,
    M: SwapKey,
    P: RenderPickerSelect<K, M>,
{
    if htmx.targets::<K>() {
        page.render_table()
    } else {
        page.render_modal().into_inner()
    }
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
        let out = respond_picker_select::<TestPickerTableKey, TestPickerModalKey, _>(
            &htmx,
            &DummyPicker,
        );
        assert!(out.into_string().contains("<dialog"));
    }
}
