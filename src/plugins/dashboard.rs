//! Dashboard plugin — apps launchpad, topbar, and home redirects.

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
    /// App at request time (Go `App.Plugins`).
    steps: [
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
    ];
    finish: add_capability(DashboardStateCap, CapStore::with_items(DashboardState));
}
