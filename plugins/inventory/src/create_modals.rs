//! Typed [`CreateModal`] wiring for inventory swap keys.

use super::keys::{
    MovementCreateModalKey, StockCreateModalKey, StockSelectModalKey, StockSelectTableKey,
};
use super::routes::{
    MovementCreateGetRouteTag, MovementCreatePostRouteTag, StockCreateGetRouteTag,
    StockCreatePostRouteTag,
};

lariv_core::impl_create_modal!(
    StockCreateModalKey,
    StockCreateGetRouteTag,
    StockCreatePostRouteTag,
    "p_inventory.StockCreateForm"
);
lariv_core::impl_create_modal!(
    MovementCreateModalKey,
    MovementCreateGetRouteTag,
    MovementCreatePostRouteTag,
    "p_inventory.MovementCreateForm"
);
lariv_core::impl_picker_modal!(StockSelectModalKey, StockSelectTableKey);
