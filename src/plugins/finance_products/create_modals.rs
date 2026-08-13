//! Typed [`CreateModal`] / [`PickerModal`] wiring for product swap keys.

use super::keys::{ProductCreateModalKey, ProductSelectModalKey, ProductSelectTableKey};
use super::routes::{ProductCreateGetRouteTag, ProductCreatePostRouteTag};

crate::impl_create_modal!(
    ProductCreateModalKey,
    ProductCreateGetRouteTag,
    ProductCreatePostRouteTag,
    "p_finance_products.ProductCreateForm"
);
crate::impl_picker_modal!(ProductSelectModalKey, ProductSelectTableKey);
