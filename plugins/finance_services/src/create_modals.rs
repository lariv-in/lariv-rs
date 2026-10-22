//! Typed [`CreateModal`] / [`PickerModal`] wiring for service swap keys.

use super::keys::{ServiceCreateModalKey, ServiceSelectModalKey, ServiceSelectTableKey};
use super::routes::{ServiceCreateGetRouteTag, ServiceCreatePostRouteTag};

lariv_core::impl_create_modal!(
    ServiceCreateModalKey,
    ServiceCreateGetRouteTag,
    ServiceCreatePostRouteTag,
    "p_finance_services.ServiceCreateForm"
);
lariv_core::impl_picker_modal!(ServiceSelectModalKey, ServiceSelectTableKey);
