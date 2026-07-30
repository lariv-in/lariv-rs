//! No-signup addon — disables public signup routes and UI links.
//!
//! Port of Go `p_no_signup`: removes GET/POST `/users/signup` and replaces
//! the users login / unauthenticated pages so they no longer link to signup.

pub mod routes;
pub mod templates;

use crate::plugin_install::define_plugin_install;

/// Capability tag for the no-signup addon (hook identity only).
pub struct NoSignupTag;

define_plugin_install! {
    plugin: NoSignupTag;
    /// Register deferred template replacements and route removals.
    steps: [templates, http]
}
