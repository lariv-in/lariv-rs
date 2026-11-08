//! Filesystem sidebar links for git sync.

use std::sync::Arc;

use maud::Markup;

use lariv_core::components::{SidebarMenuItem, sidebar_menu_item_pane};
use lariv_plugin_filesystem::sidebar_items::FilesystemSidebarItem;

use crate::routes::{GitSyncDetailRouteTag, GitSyncListRouteTag};

pub fn register() {
    lariv_plugin_filesystem::sidebar_items::register("git-sync", Arc::new(GitSyncSidebar));
}

struct GitSyncSidebar;

fn admin() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        crate::routes::FilesystemSyncGitAdmin,
    >())
}

impl FilesystemSidebarItem for GitSyncSidebar {
    fn root_item(&self, current_path: &str) -> Markup {
        if !admin() {
            return maud::html! {};
        }
        let url = GitSyncListRouteTag.url();
        let path = current_path
            .split_once('?')
            .map_or(current_path, |(path, _)| path);
        sidebar_menu_item_pane(SidebarMenuItem {
            title: "Git Sync",
            url: &url,
            active: path == url,
            ..Default::default()
        })
    }

    fn directory_item(&self, directory_id: i64, active: &str) -> Markup {
        if !admin() {
            return maud::html! {};
        }
        let url = GitSyncDetailRouteTag::new(directory_id).url();
        sidebar_menu_item_pane(SidebarMenuItem {
            title: "Git Sync",
            url: &url,
            active: active == "git-sync",
            ..Default::default()
        })
    }
}
