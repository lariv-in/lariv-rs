//! Compile-time HTMX swap keys for the website plugin.

use lariv_core::components::SwapKey;

pub const ROUTES_TABLE: SwapKey = SwapKey("website-routes-table");
pub const ROUTE_CREATE_MODAL: SwapKey = SwapKey("website-route-create-modal");
pub const ROUTE_EDIT_MODAL: SwapKey = SwapKey("website-route-edit-modal");
pub const ROUTE_DELETE_MODAL: SwapKey = SwapKey("website-route-delete-modal");
pub const ROUTE_FK_PAGE: SwapKey = SwapKey("fk-website-page");
pub const ROUTE_FK_REFS: SwapKey = SwapKey("fk-website-refs");
pub const CUSTOM_THEME_CSS_FK: SwapKey = SwapKey("fk-website-custom-theme-css");
