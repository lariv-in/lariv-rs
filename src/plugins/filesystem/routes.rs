//! Filesystem HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{
    handlers,
    templates::{
        VNodeConfirmDeletePage, VNodeConfirmDeletePageTag, VNodeDetailPage, VNodeDetailPageTag,
        VNodeFormPage, VNodeFormPageTag, VNodeListPage, VNodeListPageTag, VNodeMoveFormPage,
        VNodeMoveFormPageTag, VNodeMultiUploadFormPage, VNodeMultiUploadFormPageTag,
        VNodeSelectPage, VNodeSelectPageTag, VNodeZipUploadFormPage, VNodeZipUploadFormPageTag,
    },
};

define_plugin_routes! {
    plugin: FilesystemTag;
    proof: FilesystemRoutesProof;
    pages: [
        pane ListIdx, ListP => VNodeListPageTag, VNodeListPage;
        pane DetailIdx, DetailP => VNodeDetailPageTag, VNodeDetailPage;
        pane FormIdx, FormP => VNodeFormPageTag, VNodeFormPage;
        pane MoveIdx, MoveP => VNodeMoveFormPageTag, VNodeMoveFormPage;
        pane MultiIdx, MultiP => VNodeMultiUploadFormPageTag, VNodeMultiUploadFormPage;
        pane ZipIdx, ZipP => VNodeZipUploadFormPageTag, VNodeZipUploadFormPage;
        page SelectIdx, SelectP => VNodeSelectPageTag, VNodeSelectPage;
        page ConfirmIdx, ConfirmP => VNodeConfirmDeletePageTag, VNodeConfirmDeletePage;
    ];
    routes: [
        get VNodeListRouteTag, "/filesystem", handlers::nodes::list;
        get VNodeBrowseRouteTag, "/filesystem/browse/{parent_id}", handlers::nodes::browse;
        get VNodeCreateGetRouteTag, "/filesystem/create", handlers::nodes::create_get;
        get VNodeCreateGetInRouteTag, "/filesystem/create/in/{parent_id}", handlers::nodes::create_get_in;
        post VNodeCreatePostRouteTag, "/filesystem/create", handlers::nodes::create_post;
        post VNodeCreatePostInRouteTag, "/filesystem/create/in/{parent_id}", handlers::nodes::create_post_in;
        get VNodeUploadGetRouteTag, "/filesystem/upload", handlers::nodes::upload_get;
        get VNodeUploadGetInRouteTag, "/filesystem/upload/in/{parent_id}", handlers::nodes::upload_get_in;
        post VNodeUploadPostRouteTag, "/filesystem/upload", handlers::nodes::upload_post;
        post VNodeUploadPostInRouteTag, "/filesystem/upload/in/{parent_id}", handlers::nodes::upload_post_in;
        get VNodeZipUploadGetRouteTag, "/filesystem/zip-upload", handlers::nodes::zip_upload_get;
        get VNodeZipUploadGetInRouteTag, "/filesystem/zip-upload/in/{parent_id}", handlers::nodes::zip_upload_get_in;
        post VNodeZipUploadPostRouteTag, "/filesystem/zip-upload", handlers::nodes::zip_upload_post;
        post VNodeZipUploadPostInRouteTag, "/filesystem/zip-upload/in/{parent_id}", handlers::nodes::zip_upload_post_in;
        get VNodeSelectRouteTag, "/filesystem/select", handlers::nodes::select;
        get VNodeSelectInRouteTag, "/filesystem/select/in/{parent_id}", handlers::nodes::select_in;
        get VNodeFileSelectRouteTag, "/filesystem/file-select", handlers::nodes::file_select;
        get VNodeFileSelectInRouteTag, "/filesystem/file-select/in/{parent_id}", handlers::nodes::file_select_in;
        post ChatUploadRouteTag, "/filesystem/chat-upload", bare handlers::chat_upload::chat_upload;
        get VNodeMoveSelectRouteTag, "/filesystem/move-select", handlers::nodes::move_select;
        get VNodeMoveSelectInRouteTag, "/filesystem/move-select/in/{parent_id}", handlers::nodes::move_select_in;
        get VNodeDownloadRootRouteTag, "/filesystem/download", bare handlers::nodes::download_root;
        get VNodeDetailRouteTag, "/filesystem/{id}", handlers::nodes::detail;
        get VNodeEditGetRouteTag, "/filesystem/{id}/edit", handlers::nodes::edit_get;
        post VNodeEditPostRouteTag, "/filesystem/{id}/edit", handlers::nodes::edit_post;
        get VNodeDeleteGetRouteTag, "/filesystem/{id}/delete", handlers::nodes::delete_get;
        post VNodeDeletePostRouteTag, "/filesystem/{id}/delete", bare handlers::nodes::delete_post;
        get VNodeMoveGetRouteTag, "/filesystem/{id}/move", handlers::nodes::move_get;
        post VNodeMovePostRouteTag, "/filesystem/{id}/move", handlers::nodes::move_post;
        get VNodeDownloadRouteTag, "/filesystem/{id}/download", bare handlers::nodes::download;
    ]
}
