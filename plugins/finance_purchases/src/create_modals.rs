//! Typed [`PickerModal`] wiring for purchase swap keys.

use super::keys::{
    DraftPurchaseSelectModalKey, DraftPurchaseSelectTableKey, VendorCreateModalKey,
    VendorSelectModalKey, VendorSelectTableKey,
};
use super::routes::{VendorCreateGetRouteTag, VendorCreatePostRouteTag};

lariv_core::impl_picker_modal!(DraftPurchaseSelectModalKey, DraftPurchaseSelectTableKey);
lariv_core::impl_picker_modal!(VendorSelectModalKey, VendorSelectTableKey);
lariv_core::impl_create_modal!(
    VendorCreateModalKey,
    VendorCreateGetRouteTag,
    VendorCreatePostRouteTag,
    "p_finance_purchases.VendorCreateForm"
);
