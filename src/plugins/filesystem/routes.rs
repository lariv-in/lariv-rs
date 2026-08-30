//! Filesystem HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::{
    handlers,
    keys::{VNodeDeleteModalKey, VNodeSelectModalKey, VNodeSelectTableKey, VNodeTableKey},
};

define_plugin_routes! {
    plugin: FilesystemTag;
    routes: [
        get VNodeListRouteTag, "/filesystem", handlers::nodes::list, fragment(VNodeTableKey);
        get VNodeBrowseRouteTag, "/filesystem/browse/{parent_id}", handlers::nodes::browse;
        get VNodeCreateGetRouteTag, "/filesystem/create", handlers::nodes::create_get, modal;
        get VNodeCreateGetInRouteTag, "/filesystem/create/in/{parent_id}", handlers::nodes::create_get_in, modal;
        post VNodeCreatePostRouteTag, "/filesystem/create", handlers::nodes::create_post;
        post VNodeCreatePostInRouteTag, "/filesystem/create/in/{parent_id}", handlers::nodes::create_post_in;
        get VNodeUploadGetRouteTag, "/filesystem/upload", handlers::nodes::upload_get, modal;
        get VNodeUploadGetInRouteTag, "/filesystem/upload/in/{parent_id}", handlers::nodes::upload_get_in, modal;
        post VNodeUploadPostRouteTag, "/filesystem/upload", handlers::nodes::upload_post;
        post VNodeUploadPostInRouteTag, "/filesystem/upload/in/{parent_id}", handlers::nodes::upload_post_in;
        get VNodeZipUploadGetRouteTag, "/filesystem/zip-upload", handlers::nodes::zip_upload_get, modal;
        get VNodeZipUploadGetInRouteTag, "/filesystem/zip-upload/in/{parent_id}", handlers::nodes::zip_upload_get_in, modal;
        post VNodeZipUploadPostRouteTag, "/filesystem/zip-upload", handlers::nodes::zip_upload_post;
        post VNodeZipUploadPostInRouteTag, "/filesystem/zip-upload/in/{parent_id}", handlers::nodes::zip_upload_post_in;
        get VNodeSelectRouteTag, "/filesystem/select", handlers::nodes::select, fk_select(VNodeSelectTableKey, VNodeSelectModalKey);
        get VNodeSelectInRouteTag, "/filesystem/select/in/{parent_id}", handlers::nodes::select_in, fk_select(VNodeSelectTableKey, VNodeSelectModalKey);
        get VNodeFileSelectRouteTag, "/filesystem/file-select", handlers::nodes::file_select, fk_select(VNodeSelectTableKey, VNodeSelectModalKey);
        get VNodeFileSelectInRouteTag, "/filesystem/file-select/in/{parent_id}", handlers::nodes::file_select_in, fk_select(VNodeSelectTableKey, VNodeSelectModalKey);
        get VNodeMoveSelectRouteTag, "/filesystem/move-select", handlers::nodes::move_select, fk_select(VNodeSelectTableKey, VNodeSelectModalKey);
        get VNodeMoveSelectInRouteTag, "/filesystem/move-select/in/{parent_id}", handlers::nodes::move_select_in, fk_select(VNodeSelectTableKey, VNodeSelectModalKey);
        get VNodeDownloadRootRouteTag, "/filesystem/download", bare handlers::nodes::download_root, file;
        get VNodeDetailRouteTag, "/filesystem/{id}", handlers::nodes::detail;
        get VNodeEditGetRouteTag, "/filesystem/{id}/edit", handlers::nodes::edit_get, modal;
        post VNodeEditPostRouteTag, "/filesystem/{id}/edit", handlers::nodes::edit_post;
        get VNodeDeleteGetRouteTag, "/filesystem/{id}/delete", handlers::nodes::delete_get, modal;
        post VNodeDeletePostRouteTag, "/filesystem/{id}/delete", bare handlers::nodes::delete_post, fragment(VNodeDeleteModalKey);
        get VNodeMoveGetRouteTag, "/filesystem/{id}/move", handlers::nodes::move_get;
        post VNodeMovePostRouteTag, "/filesystem/{id}/move", handlers::nodes::move_post;
        get VNodeDownloadRouteTag, "/filesystem/{id}/download", bare handlers::nodes::download, file;
    ]
}
