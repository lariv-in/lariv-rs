//! SeaORM entities for stocks, movements, movement lines, and preferences.

pub mod preferences;
pub mod stock;
pub mod stock_movement;
pub mod stock_movement_line;

pub use stock::Entity as StockEntity;
pub use stock::Model as Stock;
pub use stock_movement::Entity as StockMovementEntity;
pub use stock_movement::Model as StockMovement;
pub use stock_movement_line::Entity as StockMovementLineEntity;
pub use stock_movement_line::Model as StockMovementLine;
