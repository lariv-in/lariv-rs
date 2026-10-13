//! Typed [`CreateModal`] wiring for inventory swap keys.

use super::keys::{
    MovementInSelectModalKey, MovementInSelectTableKey, MovementProcessSelectModalKey,
    MovementProcessSelectTableKey, StockCreateModalKey, StockSelectModalKey, StockSelectTableKey,
};
use super::routes::{StockCreateGetRouteTag, StockCreatePostRouteTag};

lariv_core::impl_create_modal!(
    StockCreateModalKey,
    StockCreateGetRouteTag,
    StockCreatePostRouteTag,
    "p_inventory.StockCreateForm"
);
lariv_core::impl_picker_modal!(StockSelectModalKey, StockSelectTableKey);
lariv_core::impl_picker_modal!(MovementInSelectModalKey, MovementInSelectTableKey);
lariv_core::impl_picker_modal!(MovementProcessSelectModalKey, MovementProcessSelectTableKey);
