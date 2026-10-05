//! Typed [`CreateModal`] / [`PickerModal`] wiring for tax swap keys.

use super::keys::{TaxCreateModalKey, TaxMultiSelectModalKey, TaxMultiSelectTableKey};
use super::routes::{TaxCreateGetRouteTag, TaxCreatePostRouteTag};

lariv_core::impl_create_modal!(
    TaxCreateModalKey,
    TaxCreateGetRouteTag,
    TaxCreatePostRouteTag,
    "p_taxes.TaxCreateForm"
);
lariv_core::impl_picker_modal!(TaxMultiSelectModalKey, TaxMultiSelectTableKey);
