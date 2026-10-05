//! Dashboard app state — chrome only; tiles come from [`lariv_core::apps::AppsCapability`].

/// Marker state for the dashboard plugin (tiles live on [`lariv_core::apps::AppsCapability`]).
#[derive(Clone, Debug, Default)]
pub struct DashboardState;
