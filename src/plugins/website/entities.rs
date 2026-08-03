//! SeaORM entities for database-backed website routes.
pub mod db_route;
pub mod route_reference;

pub use db_route::Entity as DbRouteEntity;
pub use db_route::Model as DbRoute;
pub use route_reference::Entity as RouteReferenceEntity;
pub use route_reference::Model as RouteReference;
