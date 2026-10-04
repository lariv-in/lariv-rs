//! Users HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.

use crate::define_plugin_routes;

/// User and role administration. Empty allowlist: superuser only until another plugin patches it.
pub struct UsersAdmin;

/// User picker. Empty allowlist: superuser only until a deployment patches it.
pub struct UsersPick;

use super::{
    handlers,
    keys::{UserDeleteModalKey, UserSelectModalKey, UserSelectTableKey, UserTableKey},
};

define_plugin_routes! {
    plugin: UsersTag;
    prefix: "/dashboard";
    routes: [
        get UsersLoginGetRouteTag, "/users/login", root handlers::auth::login_get;
        post UsersLoginPostRouteTag, "/users/login", root handlers::auth::login_post;
        get UsersLogoutGetRouteTag, "/users/logout", root bare handlers::auth::logout, redirect;
        post UsersLogoutPostRouteTag, "/users/logout", root bare handlers::auth::logout, redirect;
        get UsersUnauthenticatedRouteTag, "/users/unauthenticated", root handlers::auth::unauthenticated;
        get UsersLoginSuccessRouteTag, "/users/success", root bare handlers::auth::login_success, redirect;
        get UsersSelfRouteTag, "/users/self", handlers::self_profile::detail;
        get UsersSelfEditGetRouteTag, "/users/self/edit", handlers::self_profile::edit_get, modal;
        post UsersSelfEditPostRouteTag, "/users/self/edit", handlers::self_profile::edit_post;
        get UsersSelfChangePasswordGetRouteTag, "/users/self/change-password", handlers::self_profile::change_password_get;
        post UsersSelfChangePasswordPostRouteTag, "/users/self/change-password", handlers::self_profile::change_password_post;
        get UsersListRouteTag, "/users", handlers::users::list, fragment(UserTableKey), authorize(UsersAdmin, []);
        get UsersSelectRouteTag, "/users/select", handlers::users::select, fk_select(UserSelectTableKey, UserSelectModalKey), authorize(UsersPick, []);
        get UsersCreateGetRouteTag, "/users/create", handlers::users::create_get, modal, authorize(UsersAdmin, []);
        post UsersCreatePostRouteTag, "/users/create", handlers::users::create_post, authorize(UsersAdmin, []);
        get UsersDetailRouteTag, "/users/u/{id}", handlers::users::detail, authorize(UsersAdmin, []);
        get UsersEditGetRouteTag, "/users/u/{id}/edit", handlers::users::edit_get, modal, authorize(UsersAdmin, []);
        post UsersEditPostRouteTag, "/users/u/{id}/edit", handlers::users::edit_post, authorize(UsersAdmin, []);
        get UsersDeleteGetRouteTag, "/users/u/{id}/delete", handlers::users::delete_get, modal, authorize(UsersAdmin, []);
        post UsersDeletePostRouteTag, "/users/u/{id}/delete", bare handlers::users::delete_post, fragment(UserDeleteModalKey), authorize(UsersAdmin, []);
        get UsersChangePasswordGetRouteTag, "/users/u/{id}/change-password", handlers::users::change_password_get, authorize(UsersAdmin, []);
        post UsersChangePasswordPostRouteTag, "/users/u/{id}/change-password", handlers::users::change_password_post, authorize(UsersAdmin, []);
    ]
}
