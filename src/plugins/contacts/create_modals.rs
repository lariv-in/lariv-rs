//! Typed [`CreateModal`] / [`PickerModal`] wiring for contact swap keys.

use super::keys::{ContactCreateModalKey, ContactSelectModalKey, ContactSelectTableKey};
use super::routes::{ContactCreateGetRouteTag, ContactCreatePostRouteTag};

crate::impl_create_modal!(
    ContactCreateModalKey,
    ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
    "p_contacts.ContactCreateForm"
);
crate::impl_picker_modal!(ContactSelectModalKey, ContactSelectTableKey);
