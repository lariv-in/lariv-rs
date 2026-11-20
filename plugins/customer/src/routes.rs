#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

use lariv_plugin_users::roles::Admin;

use super::{
    handlers,
    keys::{CUSTOMER_SELECT_MODAL, CUSTOMER_SELECT_TABLE},
};

pub struct CustomerMutate;

/// Customer lookup. Empty allowlist: superuser only until a deployment patches it.
pub struct CustomerSelect;

lariv_core::define_plugin_routes! {
    plugin: CustomerTag;
    prefix: "/dashboard";
    routes: [
        get CustomerFkSelectRouteTag, "/customers/pick", handlers::select, fk_select(CUSTOMER_SELECT_TABLE, CUSTOMER_SELECT_MODAL), authorize(CustomerSelect, []);
        get CustomerCreateGetRouteTag, "/customers/create", handlers::create_get, modal, authorize(CustomerMutate, [Admin]);
        post CustomerCreatePostRouteTag, "/customers/create", handlers::create_post, authorize(CustomerMutate, [Admin]);
    ]
}
