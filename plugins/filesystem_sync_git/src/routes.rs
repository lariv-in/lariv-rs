//! Git sync routes under the filesystem app.

use lariv_core::define_plugin_routes;

use super::handlers;
use super::keys::{GIT_SYNC_DELETE_MODAL, GIT_SYNC_TABLE};

/// Who may edit git sync rows. Empty extra roles would be superuser only; admin may.
pub struct FilesystemSyncGitAdmin;

define_plugin_routes! {
    plugin: FilesystemSyncGitTag;
    prefix: "/dashboard";
    routes: [
        get GitSyncListRouteTag, "/filesystem/git-sync", handlers::records::list, fragment(GIT_SYNC_TABLE), authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        get GitSyncCreateGetRouteTag, "/filesystem/git-sync/create", handlers::records::create_get, modal, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        post GitSyncCreatePostRouteTag, "/filesystem/git-sync/create", handlers::records::create_post, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        get GitSyncCreateForGetRouteTag, "/filesystem/{directory_id}/git-sync/create", handlers::records::create_for_get, modal, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        get GitSyncRunGetRouteTag, "/filesystem/{directory_id}/git-sync/run", handlers::sync::git_sync_get, modal;
        post GitSyncRunPostRouteTag, "/filesystem/{directory_id}/git-sync/run", handlers::sync::git_sync_post;
        get GitSyncConfirmGetRouteTag, "/filesystem/{directory_id}/git-sync/force", handlers::sync::confirm_get, modal;
        post GitSyncConfirmPostRouteTag, "/filesystem/{directory_id}/git-sync/force", handlers::sync::confirm_post;
        get GitSyncDetailRouteTag, "/filesystem/{directory_id}/git-sync", handlers::records::detail, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        get GitSyncEditGetRouteTag, "/filesystem/{directory_id}/git-sync/edit", handlers::records::edit_get, modal, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        post GitSyncEditPostRouteTag, "/filesystem/{directory_id}/git-sync/edit", handlers::records::edit_post, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        get GitSyncDeleteGetRouteTag, "/filesystem/{directory_id}/git-sync/delete", handlers::records::delete_get, modal, authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
        post GitSyncDeletePostRouteTag, "/filesystem/{directory_id}/git-sync/delete", bare handlers::records::delete_post, fragment(GIT_SYNC_DELETE_MODAL), authorize(FilesystemSyncGitAdmin, [lariv_plugin_users::roles::Admin]);
    ]
}
