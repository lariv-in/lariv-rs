//! Dashboard app state — chrome only; tiles come from [`crate::apps::AppsCapability`].

/// Marker state for the dashboard plugin (tiles live on [`crate::apps::AppsCapability`]).
#[derive(Clone, Debug, Default)]
pub struct DashboardState;
