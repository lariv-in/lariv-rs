//! Typed create-modal wiring for a customer.

use super::keys::CustomerCreateModalKey;
use super::routes::{CustomerCreateGetRouteTag, CustomerCreatePostRouteTag};

lariv_core::impl_create_modal!(
    CustomerCreateModalKey,
    CustomerCreateGetRouteTag,
    CustomerCreatePostRouteTag,
    "p_customer.CustomerForm"
);
