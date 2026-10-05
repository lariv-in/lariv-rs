//! Typed [`CreateModal`] / [`PickerModal`] wiring for contact and company swap keys.

use super::keys::{
    CompanyCreateModalKey, CompanySelectModalKey, CompanySelectTableKey, ContactCreateModalKey,
    ContactSelectModalKey, ContactSelectTableKey,
};
use super::routes::{
    CompanyCreateGetRouteTag, CompanyCreatePostRouteTag, ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
};

lariv_core::impl_create_modal!(
    ContactCreateModalKey,
    ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
    "p_contacts.ContactCreateForm"
);
lariv_core::impl_picker_modal!(ContactSelectModalKey, ContactSelectTableKey);

lariv_core::impl_create_modal!(
    CompanyCreateModalKey,
    CompanyCreateGetRouteTag,
    CompanyCreatePostRouteTag,
    "p_contacts.CompanyCreateForm"
);
lariv_core::impl_picker_modal!(CompanySelectModalKey, CompanySelectTableKey);
