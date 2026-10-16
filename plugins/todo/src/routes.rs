#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

//! ToDo HTTP routes.

use lariv_core::define_plugin_routes;

use super::keys::{TodoPageDeleteModalKey, TodoPageTableKey};
use super::{handlers, ws};

define_plugin_routes! {
    plugin: TodoTag;
    prefix: "/dashboard";
    routes: [
        get TodoListRouteTag, "/todo", handlers::list, fragment(TodoPageTableKey);
        get TodoPageCreateGetRouteTag, "/todo/create", handlers::create_get, modal;
        post TodoPageCreatePostRouteTag, "/todo/create", handlers::create_post;
        get TodoPageDetailRouteTag, "/todo/{id}", handlers::detail;
        get TodoPageEditGetRouteTag, "/todo/{id}/edit", handlers::edit_get, modal;
        post TodoPageEditPostRouteTag, "/todo/{id}/edit", handlers::edit_post;
        get TodoPageDeleteGetRouteTag, "/todo/{id}/delete", handlers::delete_get, modal;
        post TodoPageDeletePostRouteTag, "/todo/{id}/delete", bare handlers::delete_post, fragment(TodoPageDeleteModalKey);
        get TodoWsRouteTag, "/todo/{id}/ws", bare ws::upgrade, raw;
    ]
}
