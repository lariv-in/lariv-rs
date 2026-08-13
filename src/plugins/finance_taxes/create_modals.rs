//! Typed [`CreateModal`] / [`PickerModal`] wiring for tax swap keys.

use super::keys::{TaxCreateModalKey, TaxMultiSelectModalKey, TaxMultiSelectTableKey};
use super::routes::{TaxCreateGetRouteTag, TaxCreatePostRouteTag};

crate::impl_create_modal!(
    TaxCreateModalKey,
    TaxCreateGetRouteTag,
    TaxCreatePostRouteTag,
    "p_taxes.TaxCreateForm"
);
crate::impl_picker_modal!(TaxMultiSelectModalKey, TaxMultiSelectTableKey);
