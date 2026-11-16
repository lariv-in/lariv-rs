//! Per-user audit log route.

use lariv_core::define_plugin_routes;
use lariv_plugin_logging::keys::LOG_TABLE;
use lariv_plugin_users::roles::Admin;

use super::handlers;

/// Reading one user's audit log. `admin` is the fallback; superuser always may.
pub struct UserLogsView;

define_plugin_routes! {
    plugin: UserLogTag;
    prefix: "/dashboard";
    routes: [
        get UserLogListRouteTag, "/users/u/{id}/logs", handlers::list, fragment(LOG_TABLE), authorize(UserLogsView, [Admin]);
    ]
}
