//! SeaORM entities for database-backed website routes and preferences.
pub mod db_route;
pub mod route_reference;
pub mod website_preferences;

pub use db_route::Entity as DbRouteEntity;
pub use db_route::Model as DbRoute;
pub use route_reference::Entity as RouteReferenceEntity;
pub use route_reference::Model as RouteReference;
pub use website_preferences::Entity as WebsitePreferencesEntity;
pub use website_preferences::Model as WebsitePreferences;
