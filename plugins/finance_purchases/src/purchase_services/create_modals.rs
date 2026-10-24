//! Typed [`CreateModal`] / [`PickerModal`] wiring for service swap keys.

use super::keys::{ServiceCreateModalKey, ServiceSelectModalKey, ServiceSelectTableKey};
use crate::routes::{PurchaseServiceCreateGetRouteTag, PurchaseServiceCreatePostRouteTag};

lariv_core::impl_create_modal!(
    ServiceCreateModalKey,
    PurchaseServiceCreateGetRouteTag,
    PurchaseServiceCreatePostRouteTag,
    "p_finance_purchases.PurchaseServiceCreateForm"
);
lariv_core::impl_picker_modal!(ServiceSelectModalKey, ServiceSelectTableKey);
