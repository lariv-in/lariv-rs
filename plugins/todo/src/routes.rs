#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

//! ToDo HTTP routes.

use lariv_core::define_plugin_routes;

use super::{
    handlers,
    keys::{TodoDeleteModalKey, TodoTableKey},
};

define_plugin_routes! {
    plugin: TodoTag;
    prefix: "/dashboard";
    routes: [
        get TodoListRouteTag, "/todo", handlers::list, fragment(TodoTableKey);
        get TodoCreateGetRouteTag, "/todo/create", handlers::create_get, modal;
        post TodoCreatePostRouteTag, "/todo/create", handlers::create_post;
        get TodoDetailRouteTag, "/todo/{id}", handlers::detail;
        get TodoEditGetRouteTag, "/todo/{id}/edit", handlers::edit_get, modal;
        post TodoEditPostRouteTag, "/todo/{id}/edit", handlers::edit_post;
        get TodoDeleteGetRouteTag, "/todo/{id}/delete", handlers::delete_get, modal;
        post TodoDeletePostRouteTag, "/todo/{id}/delete", bare handlers::delete_post, fragment(TodoDeleteModalKey);
        post TodoToggleRouteTag, "/todo/{id}/checked", handlers::toggle, fragment(TodoTableKey);
    ]
}
