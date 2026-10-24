//! Typed [`CreateModal`] / [`PickerModal`] wiring for product swap keys.

use super::keys::{ProductCreateModalKey, ProductSelectModalKey, ProductSelectTableKey};
use crate::routes::{PurchaseProductCreateGetRouteTag, PurchaseProductCreatePostRouteTag};

lariv_core::impl_create_modal!(
    ProductCreateModalKey,
    PurchaseProductCreateGetRouteTag,
    PurchaseProductCreatePostRouteTag,
    "p_finance_purchases.PurchaseProductCreateForm"
);
lariv_core::impl_picker_modal!(ProductSelectModalKey, ProductSelectTableKey);
