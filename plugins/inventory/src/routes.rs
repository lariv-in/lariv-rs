#![allow(
    clippy::redundant_field_names,
    reason = "define_plugin_routes expands path params as field: field"
)]

use lariv_plugin_users::roles::{Admin, Unassigned};

use super::{
    handlers,
    keys::{
        MovementDeleteModalKey, MovementInSelectModalKey, MovementInSelectTableKey,
        MovementProcessSelectModalKey, MovementProcessSelectTableKey, MovementTableKey,
        StockDeleteModalKey, StockLinesTableKey, StockSelectModalKey, StockSelectTableKey,
        StockTableKey,
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
        get MovementInFkSelectRouteTag, "/inventory/movements/in/pick", handlers::movements::select_in, multi_select(MovementInSelectTableKey, MovementInSelectModalKey), authorize(InventoryView, [Unassigned, Admin]);
        get MovementProcessFkSelectRouteTag, "/inventory/movements/process/pick", handlers::movements::select_process, multi_select(MovementProcessSelectTableKey, MovementProcessSelectModalKey), authorize(InventoryView, [Unassigned, Admin]);
        get MovementCreateGetRouteTag, "/inventory/movements/{kind}/create", handlers::movements::create_get, modal, authorize(InventoryMutate, [Admin]);
        post MovementCreateInPostRouteTag, "/inventory/movements/create/in", handlers::movements::create_in_post, authorize(InventoryMutate, [Admin]);
        post MovementCreateOutPostRouteTag, "/inventory/movements/create/out", handlers::movements::create_out_post, authorize(InventoryMutate, [Admin]);
        post MovementCreateProcessPostRouteTag, "/inventory/movements/create/process", handlers::movements::create_process_post, authorize(InventoryMutate, [Admin]);
        get MovementPdfModalRouteTag, "/inventory/movements/{kind}/{id}/pdf", bare handlers::pdf::modal, modal, authorize(InventoryView, [Unassigned, Admin]);
        get MovementPdfFileRouteTag, "/inventory/movements/{kind}/{id}/pdf/file", bare handlers::pdf::file, file, authorize(InventoryView, [Unassigned, Admin]);
        get MovementDetailRouteTag, "/inventory/movements/{kind}/{id}", handlers::movements::detail, authorize(InventoryView, [Unassigned, Admin]);
        get MovementEditGetRouteTag, "/inventory/movements/{kind}/{id}/edit", handlers::movements::edit_get, modal, authorize(InventoryMutate, [Admin]);
        post MovementEditInPostRouteTag, "/inventory/movements/edit/in/{id}", handlers::movements::edit_in_post, authorize(InventoryMutate, [Admin]);
        post MovementEditOutPostRouteTag, "/inventory/movements/edit/out/{id}", handlers::movements::edit_out_post, authorize(InventoryMutate, [Admin]);
        post MovementEditProcessPostRouteTag, "/inventory/movements/edit/process/{id}", handlers::movements::edit_process_post, authorize(InventoryMutate, [Admin]);
        get MovementDeleteGetRouteTag, "/inventory/movements/{kind}/{id}/delete", handlers::movements::delete_get, modal, authorize(InventoryMutate, [Admin]);
        post MovementDeletePostRouteTag, "/inventory/movements/{kind}/{id}/delete", bare handlers::movements::delete_post, fragment(MovementDeleteModalKey), authorize(InventoryMutate, [Admin]);

        get StockDetailRouteTag, "/inventory/{id}", handlers::stocks::detail, fragment(StockLinesTableKey), authorize(InventoryView, [Unassigned, Admin]);
        get StockEditGetRouteTag, "/inventory/{id}/edit", handlers::stocks::edit_get, modal, authorize(InventoryMutate, [Admin]);
        post StockEditPostRouteTag, "/inventory/{id}/edit", handlers::stocks::edit_post, authorize(InventoryMutate, [Admin]);
        get StockDeleteGetRouteTag, "/inventory/{id}/delete", handlers::stocks::delete_get, modal, authorize(InventoryMutate, [Admin]);
        post StockDeletePostRouteTag, "/inventory/{id}/delete", bare handlers::stocks::delete_post, fragment(StockDeleteModalKey), authorize(InventoryMutate, [Admin]);
    ]
}
