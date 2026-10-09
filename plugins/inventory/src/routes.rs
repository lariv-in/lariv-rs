#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

use lariv_plugin_users::roles::{Admin, Unassigned};

use super::{
    handlers,
    keys::{
        MovementDeleteModalKey, MovementTableKey, StockDeleteModalKey, StockLinesTableKey,
        StockSelectModalKey, StockSelectTableKey, StockTableKey,
    },
};

pub struct InventoryView;

pub struct InventoryMutate;

lariv_core::define_plugin_routes! {
    plugin: InventoryTag;
    prefix: "/dashboard";
    routes: [
        get StockDefaultRouteTag, "/inventory", handlers::stocks::list, fragment(StockTableKey), authorize(InventoryView, [Unassigned, Admin]);
        get StockCreateGetRouteTag, "/inventory/create", handlers::stocks::create_get, modal, authorize(InventoryMutate, [Admin]);
        post StockCreatePostRouteTag, "/inventory/create", handlers::stocks::create_post, authorize(InventoryMutate, [Admin]);
        get StockFkSelectRouteTag, "/inventory/pick", handlers::stocks::select, fk_select(StockSelectTableKey, StockSelectModalKey), authorize(InventoryView, [Unassigned, Admin]);

        get InventoryPrefsGetRouteTag, "/inventory/preferences", handlers::preferences::get, authorize(InventoryMutate, [Admin]);
        post InventoryPrefsPostRouteTag, "/inventory/preferences", handlers::preferences::post, authorize(InventoryMutate, [Admin]);
        post MovementPdfPreviewInRouteTag, "/inventory/preferences/preview-in", bare handlers::pdf_preview::preview_in, modal, authorize(InventoryMutate, [Admin]);
        post MovementPdfPreviewOutRouteTag, "/inventory/preferences/preview-out", bare handlers::pdf_preview::preview_out, modal, authorize(InventoryMutate, [Admin]);
        get MovementPdfPreviewFileRouteTag, "/inventory/preferences/preview/{token}", bare handlers::pdf_preview::pdf_get, file, param token: String, authorize(InventoryMutate, [Admin]);

        get MovementDefaultRouteTag, "/inventory/movements", handlers::movements::list, fragment(MovementTableKey), authorize(InventoryView, [Unassigned, Admin]);
        get MovementCreateGetRouteTag, "/inventory/movements/create", handlers::movements::create_get, modal, authorize(InventoryMutate, [Admin]);
        post MovementCreatePostRouteTag, "/inventory/movements/create", handlers::movements::create_post, authorize(InventoryMutate, [Admin]);
        get MovementPdfModalRouteTag, "/inventory/movements/{id}/pdf", bare handlers::pdf::modal, modal, authorize(InventoryView, [Unassigned, Admin]);
        get MovementPdfFileRouteTag, "/inventory/movements/{id}/pdf/file", bare handlers::pdf::file, file, authorize(InventoryView, [Unassigned, Admin]);
        get MovementDetailRouteTag, "/inventory/movements/{id}", handlers::movements::detail, authorize(InventoryView, [Unassigned, Admin]);
        get MovementEditGetRouteTag, "/inventory/movements/{id}/edit", handlers::movements::edit_get, modal, authorize(InventoryMutate, [Admin]);
        post MovementEditPostRouteTag, "/inventory/movements/{id}/edit", handlers::movements::edit_post, authorize(InventoryMutate, [Admin]);
        get MovementDeleteGetRouteTag, "/inventory/movements/{id}/delete", handlers::movements::delete_get, modal, authorize(InventoryMutate, [Admin]);
        post MovementDeletePostRouteTag, "/inventory/movements/{id}/delete", bare handlers::movements::delete_post, fragment(MovementDeleteModalKey), authorize(InventoryMutate, [Admin]);

        get StockDetailRouteTag, "/inventory/{id}", handlers::stocks::detail, fragment(StockLinesTableKey), authorize(InventoryView, [Unassigned, Admin]);
        get StockEditGetRouteTag, "/inventory/{id}/edit", handlers::stocks::edit_get, modal, authorize(InventoryMutate, [Admin]);
        post StockEditPostRouteTag, "/inventory/{id}/edit", handlers::stocks::edit_post, authorize(InventoryMutate, [Admin]);
        get StockDeleteGetRouteTag, "/inventory/{id}/delete", handlers::stocks::delete_get, modal, authorize(InventoryMutate, [Admin]);
        post StockDeletePostRouteTag, "/inventory/{id}/delete", bare handlers::stocks::delete_post, fragment(StockDeleteModalKey), authorize(InventoryMutate, [Admin]);
    ]
}
