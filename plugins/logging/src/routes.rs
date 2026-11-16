//! Audit log HTTP routes.

use lariv_core::define_plugin_routes;
use lariv_plugin_users::roles::Admin;

use super::handlers;
use super::keys::LOG_TABLE;

/// Reading the audit log. `admin` is the fallback; superuser always may.
pub struct LogsView;

define_plugin_routes! {
    plugin: LoggingTag;
    prefix: "/dashboard";
    routes: [
        get LogListRouteTag, "/logs", handlers::list, fragment(LOG_TABLE), authorize(LogsView, [Admin]);
    ]
}
