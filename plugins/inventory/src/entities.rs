//! SeaORM entities for stocks, movements, movement lines, and preferences.

pub mod preferences;
pub mod stock;
pub mod stock_movement_in;
pub mod stock_movement_in_line;
pub mod stock_movement_out;
pub mod stock_movement_out_in;
pub mod stock_movement_out_line;
pub mod stock_movement_out_process;
pub mod stock_movement_process;
pub mod stock_movement_process_line;

pub use stock::Entity as StockEntity;
pub use stock::Model as Stock;
pub use stock_movement_in::Entity as StockMovementInEntity;
pub use stock_movement_in::Model as StockMovementIn;
pub use stock_movement_in_line::Entity as StockMovementInLineEntity;
pub use stock_movement_in_line::Model as StockMovementInLine;
pub use stock_movement_out::Entity as StockMovementOutEntity;
pub use stock_movement_out::Model as StockMovementOut;
pub use stock_movement_out_line::Entity as StockMovementOutLineEntity;
pub use stock_movement_out_line::Model as StockMovementOutLine;
pub use stock_movement_process::Entity as StockMovementProcessEntity;
pub use stock_movement_process::Model as StockMovementProcess;
pub use stock_movement_process_line::Entity as StockMovementProcessLineEntity;
pub use stock_movement_process_line::Model as StockMovementProcessLine;
