//! Typed [`CreateModal`] wiring for CRM swap keys.

use lariv_core::web::CreateModal;

use super::keys::{CUSTOMER_CREATE_MODAL, LEAD_CREATE_MODAL, LEAD_TAG_CREATE_MODAL};
use super::routes::{
    CustomerCreateGetRouteTag, CustomerCreatePostRouteTag, LeadCreateGetRouteTag,
    LeadCreatePostRouteTag, LeadTagCreateGetRouteTag, LeadTagCreatePostRouteTag,
};

pub const LEAD_CREATE: CreateModal<LeadCreateGetRouteTag, LeadCreatePostRouteTag> = CreateModal {
    key: LEAD_CREATE_MODAL,
    form_name: "p_crm.LeadCreateForm",
    get: LeadCreateGetRouteTag,
    post: LeadCreatePostRouteTag,
};

pub const LEAD_TAG_CREATE: CreateModal<LeadTagCreateGetRouteTag, LeadTagCreatePostRouteTag> =
    CreateModal {
        key: LEAD_TAG_CREATE_MODAL,
        form_name: "p_crm.LeadTagCreateForm",
        get: LeadTagCreateGetRouteTag,
        post: LeadTagCreatePostRouteTag,
    };

pub const CUSTOMER_CREATE: CreateModal<CustomerCreateGetRouteTag, CustomerCreatePostRouteTag> =
    CreateModal {
        key: CUSTOMER_CREATE_MODAL,
        form_name: "p_crm.CustomerCreateForm",
        get: CustomerCreateGetRouteTag,
        post: CustomerCreatePostRouteTag,
    };
