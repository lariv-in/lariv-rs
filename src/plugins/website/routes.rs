//! Website HTTP routes.

use crate::plugin_routes::define_plugin_routes;

use super::{
    builder_assets, handlers,
    templates::{
        ConfirmDeletePage, RouteDetailPage, RouteDetailPageTag, RouteFormPage, RouteFormPageTag,
        RouteListPage, RouteListPageTag, RoutesBuilderPage, RoutesBuilderPageTag,
        WebsiteConfirmDeletePageTag,
    },
};

define_plugin_routes! {
    plugin: WebsiteTag;
    proof: WebsiteRoutesProof;
    pages: [
        pane RouteListIdx, RouteListP => RouteListPageTag, RouteListPage;
        pane RouteFormIdx, RouteFormP => RouteFormPageTag, RouteFormPage;
        pane RouteDetailIdx, RouteDetailP => RouteDetailPageTag, RouteDetailPage;
        page ConfirmDeleteIdx, ConfirmDeleteP => WebsiteConfirmDeletePageTag, ConfirmDeletePage;
        page BuilderIdx, BuilderP => RoutesBuilderPageTag, RoutesBuilderPage;
    ];
    routes: [
        get WebsiteHomeRouteTag, "/", bare handlers::dynamic::home;
        get WebsiteCatchAllRouteTag, "/{*path}", bare handlers::dynamic::catch_all;
        get WebsiteRoutesListRouteTag, "/website", handlers::routes::list;
        get WebsiteRoutesCreateGetRouteTag, "/website/create", handlers::routes::create_get;
        post WebsiteRoutesCreatePostRouteTag, "/website/create", handlers::routes::create_post;
        get WebsiteRoutesDetailRouteTag, "/website/{id}", handlers::routes::detail;
        get WebsiteRoutesEditGetRouteTag, "/website/{id}/edit", handlers::routes::edit_get;
        post WebsiteRoutesEditPostRouteTag, "/website/{id}/edit", handlers::routes::edit_post;
        get WebsiteRoutesDeleteGetRouteTag, "/website/{id}/delete", handlers::routes::delete_get;
        post WebsiteRoutesDeletePostRouteTag, "/website/{id}/delete", bare handlers::routes::delete_post;
        get WebsiteBuilderRouteTag, "/website/{id}/builder", handlers::builder::builder_page;
        get WebsiteBuilderProjectGetRouteTag, "/website/{id}/builder/project", bare handlers::builder::project_load;
        post WebsiteBuilderProjectPostRouteTag, "/website/{id}/builder/project", bare handlers::builder::project_store;
        post WebsiteBuilderThemeRouteTag, "/website/{id}/builder/theme", bare handlers::builder::theme_store;
        post WebsiteBuilderAssetsRouteTag, "/website/builder/assets", bare builder_assets::builder_asset_upload;
        get WebsitePublicAssetRouteTag, "/media/{id}", bare builder_assets::public_asset;
    ]
}
