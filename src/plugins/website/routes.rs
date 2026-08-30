//! Website HTTP routes.

use crate::define_plugin_routes;

use super::{
    builder_assets, handlers,
    keys::{RouteDeleteModalKey, RoutesTableKey},
};

define_plugin_routes! {
    plugin: WebsiteTag;
    routes: [
        get WebsiteCatchAllRouteTag, "/{*path}", bare handlers::dynamic::catch_all, raw;
        get WebsiteHomeRouteTag, "/", bare handlers::dynamic::home, raw;
        get WebsiteRoutesListRouteTag, "/website", handlers::routes::list, fragment(RoutesTableKey);
        get WebsitePrefsGetRouteTag, "/website/preferences", handlers::preferences::get;
        post WebsitePrefsPostRouteTag, "/website/preferences", handlers::preferences::post;
        get WebsiteRoutesCreateGetRouteTag, "/website/create", handlers::routes::create_get, modal;
        post WebsiteRoutesCreatePostRouteTag, "/website/create", handlers::routes::create_post;
        get WebsiteRoutesDetailRouteTag, "/website/{id}", handlers::routes::detail;
        get WebsiteRoutesEditGetRouteTag, "/website/{id}/edit", handlers::routes::edit_get, modal;
        post WebsiteRoutesEditPostRouteTag, "/website/{id}/edit", handlers::routes::edit_post;
        get WebsiteRoutesDeleteGetRouteTag, "/website/{id}/delete", handlers::routes::delete_get, modal;
        post WebsiteRoutesDeletePostRouteTag, "/website/{id}/delete", bare handlers::routes::delete_post, fragment(RouteDeleteModalKey);
        get WebsiteBuilderRouteTag, "/website/{id}/builder", handlers::builder::builder_page;
        get WebsiteBuilderProjectGetRouteTag, "/website/{id}/builder/project", bare handlers::builder::project_load, raw;
        post WebsiteBuilderProjectPostRouteTag, "/website/{id}/builder/project", bare handlers::builder::project_store, raw;
        post WebsiteBuilderThemeRouteTag, "/website/{id}/builder/theme", bare handlers::builder::theme_store, raw;
        post WebsiteBuilderAssetsRouteTag, "/website/builder/assets", bare builder_assets::builder_asset_upload, raw;
        get WebsitePublicAssetRouteTag, "/media/{id}", bare builder_assets::public_asset, file;
    ]
}
