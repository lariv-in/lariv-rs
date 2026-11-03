//! Typed [`CreateModal`] wiring for inventory swap keys.

use lariv_core::web::CreateModal;

use super::keys::STOCK_CREATE_MODAL;
use super::routes::{StockCreateGetRouteTag, StockCreatePostRouteTag};

pub const STOCK_CREATE: CreateModal<StockCreateGetRouteTag, StockCreatePostRouteTag> =
    CreateModal {
        key: STOCK_CREATE_MODAL,
        form_name: "p_inventory.StockCreateForm",
        get: StockCreateGetRouteTag,
        post: StockCreatePostRouteTag,
    };
