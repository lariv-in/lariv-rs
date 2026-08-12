//! Central launchpad, top bar navigation, and theme toggling.
//!
//! Provides the `/dashboard/` apps grid, topbar widgets, and the `/`
//! auth redirect.
//!
//! # Templates and slots
//!
//! - [`templates::AppsPage`] — main launchpad grid (reads [`crate::apps::AppsCapability`] at request time).
//! - Topbar slots: apps button, theme toggle (via [`templates::SlotsHook`]); user dropdown (via [`crate::plugins::users::templates::SlotsHook`]).
//!
//! # Routes
//!
//! - `/` — authenticated → `/dashboard/`; guests → `/users/login/` ([`handlers::home_redirect`]).
//! - `/dashboard/` — apps grid ([`handlers::apps`]).
//!
//! Install a public-site plugin (e.g. [`crate::plugins::website`]) **after**
//! dashboard if it should own `/` instead; later installs win on path conflicts.

pub mod handlers;
pub mod routes;
pub mod state;
pub mod templates;

use crate::plugin_install::define_plugin_install;
use crate::{
    capability::{CapStore, define_passthrough_cap},
    traits::add::AddCapability,
};

pub use crate::apps::{AppTile, PluginType};
pub use state::DashboardState;

/// Capability tag for the dashboard plugin.
pub struct DashboardTag;

define_passthrough_cap!(DashboardStateCap, DashboardTag, DashboardState);

define_plugin_install! {
    plugin: DashboardTag;
    /// Register dashboard templates, topbar slots, marker state, and a deferred route-mount hook.
    ///
    /// App tiles are not copied here — handlers read [`crate::apps::AppsCapability`] from the
    /// App at request time.
    steps: [
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
    ];
    finish: add_capability(DashboardStateCap, CapStore::with_items(DashboardState));
}
