#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

use lariv_plugin_users::roles::{Admin, Unassigned};

use super::{handlers, keys::CustomerTableKey};

pub struct CustomerView;

pub struct CustomerMutate;

lariv_core::define_plugin_routes! {
    plugin: CustomerTag;
    prefix: "/dashboard";
    routes: [
        get CustomerDefaultRouteTag, "/customers", handlers::list, fragment(CustomerTableKey), authorize(CustomerView, [Unassigned, Admin]);
        get CustomerCreateGetRouteTag, "/customers/create", handlers::create_get, modal, authorize(CustomerMutate, [Admin]);
        post CustomerCreatePostRouteTag, "/customers/create", handlers::create_post, authorize(CustomerMutate, [Admin]);
    ]
}
