//! Typed [`CreateModal`] / [`PickerModal`] wiring for contact and company swap keys.

use super::keys::{
    CompanyCreateModalKey, CompanySelectModalKey, CompanySelectTableKey, ContactCreateModalKey,
    ContactSelectModalKey, ContactSelectTableKey,
};
use super::routes::{
    CompanyCreateGetRouteTag, CompanyCreatePostRouteTag, ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
};

crate::impl_create_modal!(
    ContactCreateModalKey,
    ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
    "p_contacts.ContactCreateForm"
);
crate::impl_picker_modal!(ContactSelectModalKey, ContactSelectTableKey);

crate::impl_create_modal!(
    CompanyCreateModalKey,
    CompanyCreateGetRouteTag,
    CompanyCreatePostRouteTag,
    "p_contacts.CompanyCreateForm"
);
crate::impl_picker_modal!(CompanySelectModalKey, CompanySelectTableKey);
