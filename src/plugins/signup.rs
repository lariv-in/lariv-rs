//! Public self-service signup for Lariv.
//!
//! Adds GET/POST `/users/signup` and replaces users login /
//! unauthenticated pages so they link to signup. Install after
//! [`crate::plugins::users`].

pub mod forms;
pub mod handlers;
pub mod routes;
pub mod templates;

use crate::plugin_install::define_plugin_install;

/// Capability tag for the signup addon (hook identity only).
pub struct SignupTag;

define_plugin_install! {
    plugin: SignupTag;
    /// Register signup templates, login/unauthenticated patches, and routes.
    steps: [templates(templates::Hook, LoginIdx, UnauthIdx), http(routes::Hook)]
}
