//! Central launchpad, top bar navigation, and theme toggling.
//!
//! Provides the `/dashboard/` apps grid, topbar widgets, and the `/`
//! auth redirect.
//!
//! # Templates and slots
//!
//! - [`templates::AppsPage`] — main launchpad grid (reads [`lariv_core::apps::AppsCapability`] at request time).
//! - Topbar slots: apps button, theme toggle (via [`templates::SlotsHook`]); user dropdown (via [`lariv_plugin_users::templates::SlotsHook`]).
//!
//! # Routes
//!
//! - `/` — authenticated → `/dashboard/`; guests → `/users/login/` ([`handlers::home_redirect`]).
//! - `/dashboard/` — apps grid ([`handlers::apps`]).
//!
//! Install a public-site plugin (e.g. [`lariv_plugin_website`]) **after**
//! dashboard if it should own `/` instead; later installs win on path conflicts.

pub mod handlers;
pub mod nav_origin;
pub mod routes;
pub mod state;
pub mod templates;

use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::plugin_install::define_plugin_install;
use lariv_core::traits::add::AddCapability;

pub use lariv_core::apps::{AppTile, PluginType};
pub use state::DashboardState;

/// Capability tag for the dashboard plugin.
pub struct DashboardTag;

define_passthrough_cap!(DashboardStateCap, DashboardTag, DashboardState);

define_plugin_install! {
    plugin: DashboardTag;
    /// Register dashboard templates, topbar slots, marker state, and a deferred route-mount hook.
    ///
    /// App tiles are not copied here — handlers read [`lariv_core::apps::AppsCapability`] from the
    /// App at request time.
    steps: [
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
    ];
    finish: add_capability(DashboardStateCap, {
        nav_origin::register();
        CapStore::with_items(DashboardState)
    });
}
