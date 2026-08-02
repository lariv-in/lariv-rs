//! Users HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::{
    handlers,
    keys::{RoleDeleteModalKey, RoleSelectTableKey, RoleTableKey, UserDeleteModalKey,
           UserSelectTableKey, UserTableKey},
};

define_plugin_routes! {
    plugin: UsersTag;
    routes: [
        get UsersLoginGetRouteTag, "/users/login", handlers::auth::login_get;
        post UsersLoginPostRouteTag, "/users/login", handlers::auth::login_post;
        get UsersSignupGetRouteTag, "/users/signup", handlers::auth::signup_get;
        post UsersSignupPostRouteTag, "/users/signup", handlers::auth::signup_post;
        get UsersLogoutGetRouteTag, "/users/logout", bare handlers::auth::logout, redirect;
        post UsersLogoutPostRouteTag, "/users/logout", bare handlers::auth::logout, redirect;
        get UsersUnauthenticatedRouteTag, "/users/unauthenticated", handlers::auth::unauthenticated;
        get UsersLoginSuccessRouteTag, "/users/success", bare handlers::auth::login_success, redirect;
        get UsersSelfRouteTag, "/users/self", handlers::self_profile::detail;
        get UsersSelfEditGetRouteTag, "/users/self/edit", handlers::self_profile::edit_get;
        post UsersSelfEditPostRouteTag, "/users/self/edit", handlers::self_profile::edit_post;
        get UsersSelfChangePasswordGetRouteTag, "/users/self/change-password", handlers::self_profile::change_password_get;
        post UsersSelfChangePasswordPostRouteTag, "/users/self/change-password", handlers::self_profile::change_password_post;
        get UsersListRouteTag, "/users", handlers::users::list, fragment(UserTableKey);
        get UsersSelectRouteTag, "/users/select", handlers::users::select, fragment(UserSelectTableKey);
        get UsersCreateGetRouteTag, "/users/create", handlers::users::create_get;
        post UsersCreatePostRouteTag, "/users/create", handlers::users::create_post;
        get UsersDetailRouteTag, "/users/u/{id}", handlers::users::detail;
        get UsersEditGetRouteTag, "/users/u/{id}/edit", handlers::users::edit_get;
        post UsersEditPostRouteTag, "/users/u/{id}/edit", handlers::users::edit_post;
        get UsersDeleteGetRouteTag, "/users/u/{id}/delete", handlers::users::delete_get, modal;
        post UsersDeletePostRouteTag, "/users/u/{id}/delete", bare handlers::users::delete_post, fragment(UserDeleteModalKey);
        get UsersChangePasswordGetRouteTag, "/users/u/{id}/change-password", handlers::users::change_password_get;
        post UsersChangePasswordPostRouteTag, "/users/u/{id}/change-password", handlers::users::change_password_post;
        get UsersRolesListRouteTag, "/users/roles", handlers::roles::list, fragment(RoleTableKey);
        get UsersRolesSelectRouteTag, "/users/roles/select", handlers::roles::select, fragment(RoleSelectTableKey);
        get UsersRolesCreateGetRouteTag, "/users/roles/create", handlers::roles::create_get;
        post UsersRolesCreatePostRouteTag, "/users/roles/create", handlers::roles::create_post;
        get UsersRolesDetailRouteTag, "/users/roles/{id}", handlers::roles::detail;
        get UsersRolesEditGetRouteTag, "/users/roles/{id}/edit", handlers::roles::edit_get;
        post UsersRolesEditPostRouteTag, "/users/roles/{id}/edit", handlers::roles::edit_post;
        get UsersRolesDeleteGetRouteTag, "/users/roles/{id}/delete", handlers::roles::delete_get, modal;
        post UsersRolesDeletePostRouteTag, "/users/roles/{id}/delete", bare handlers::roles::delete_post, fragment(RoleDeleteModalKey);
    ]
}
