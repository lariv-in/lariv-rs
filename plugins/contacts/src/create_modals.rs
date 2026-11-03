//! Typed [`CreateModal`] wiring for contact and company swap keys.

use lariv_core::web::CreateModal;

use super::keys::{COMPANY_CREATE_MODAL, CONTACT_CREATE_MODAL};
use super::routes::{
    CompanyCreateGetRouteTag, CompanyCreatePostRouteTag, ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
};

pub const CONTACT_CREATE: CreateModal<ContactCreateGetRouteTag, ContactCreatePostRouteTag> =
    CreateModal {
        key: CONTACT_CREATE_MODAL,
        form_name: "p_contacts.ContactCreateForm",
        get: ContactCreateGetRouteTag,
        post: ContactCreatePostRouteTag,
    };

pub const COMPANY_CREATE: CreateModal<CompanyCreateGetRouteTag, CompanyCreatePostRouteTag> =
    CreateModal {
        key: COMPANY_CREATE_MODAL,
        form_name: "p_contacts.CompanyCreateForm",
        get: CompanyCreateGetRouteTag,
        post: CompanyCreatePostRouteTag,
    };
