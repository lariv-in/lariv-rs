//! Signup HTTP routes — tagged entries on [`lariv_core::http::HttpCapability`]'s route HList.
//!
//! Login and unauthenticated paths duplicate users routes so this plugin's
//! handlers win when installed later (see [`lariv_core::http::MountRoutes`]).

use lariv_core::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: SignupTag;
    routes: [
        get SignupGetRouteTag, "/users/signup", handlers::signup_get;
        post SignupPostRouteTag, "/users/signup", handlers::signup_post;
        get SignupLoginGetRouteTag, "/users/login", handlers::login_get;
        post SignupLoginPostRouteTag, "/users/login", handlers::login_post;
        get SignupUnauthenticatedRouteTag, "/users/unauthenticated", handlers::unauthenticated;
    ]
}
