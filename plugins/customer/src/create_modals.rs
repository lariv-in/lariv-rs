//! Typed create-modal wiring for a customer.

use lariv_core::web::CreateModal;

use super::keys::CUSTOMER_CREATE_MODAL;
use super::routes::{CustomerCreateGetRouteTag, CustomerCreatePostRouteTag};

pub const CUSTOMER_CREATE: CreateModal<CustomerCreateGetRouteTag, CustomerCreatePostRouteTag> =
    CreateModal {
        key: CUSTOMER_CREATE_MODAL,
        form_name: "p_customer.CustomerForm",
        get: CustomerCreateGetRouteTag,
        post: CustomerCreatePostRouteTag,
    };
