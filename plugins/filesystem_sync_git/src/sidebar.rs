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
    fn root_url(&self) -> Option<String> {
        admin().then(|| GitSyncListRouteTag.url())
    }

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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use lariv_plugin_users::entities::user::Model as User;
    use lariv_plugin_users::role_authorization::{RoleAuthorizationRegistry, with_principal};
    use lariv_plugin_users::roles::Admin;
    use lariv_plugin_users::state::AuthContext;

    use crate::routes::{FilesystemSyncGitAdmin, GitSyncListRouteTag};

    fn auth() -> AuthContext {
        AuthContext {
            user: User {
                id: 1,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                name: "Ada".into(),
                email: "ada@example.com".into(),
                phone: "1".into(),
                role: Admin::NAME.into(),
                password_hash: Some(vec![]),
                password_salt: Some(vec![]),
                timezone: "UTC".into(),
                last_login_time: None,
            },
            role: Admin::NAME.into(),
            timezone: "UTC".into(),
        }
    }

    fn link_active(html: &str, label: &str) -> bool {
        let needle = format!(">{label}</a>");
        let Some(end) = html.find(&needle) else {
            return false;
        };
        let Some(prefix) = html.get(..end) else {
            return false;
        };
        let Some(open) = prefix.rfind("<a ") else {
            return false;
        };
        html.get(open..end)
            .is_some_and(|anchor| anchor.contains("menu-active"))
    }

    #[test]
    fn git_sync_list_does_not_activate_all_files() {
        super::register();
        let registry = RoleAuthorizationRegistry::new()
            .allow::<FilesystemSyncGitAdmin>(vec![Admin::NAME.into()]);
        let git_sync = with_principal(auth(), registry.clone(), || {
            lariv_plugin_filesystem::templates::main_menu(&GitSyncListRouteTag.url()).into_string()
        });
        assert!(
            git_sync.contains(">All Files</a>"),
            "filesystem menu should list All Files"
        );
        assert!(
            git_sync.contains(">Git Sync</a>"),
            "filesystem menu should list Git Sync"
        );
        assert!(
            !link_active(&git_sync, "All Files"),
            "All Files should be inactive on the git sync page: {git_sync}"
        );
        assert!(
            link_active(&git_sync, "Git Sync"),
            "Git Sync should be active on its page: {git_sync}"
        );

        let files = with_principal(auth(), registry, || {
            lariv_plugin_filesystem::templates::main_menu(
                &lariv_plugin_filesystem::routes::VNodeListRouteTag.url(),
            )
            .into_string()
        });
        assert!(
            link_active(&files, "All Files"),
            "All Files should stay active on the file list: {files}"
        );
        assert!(
            !link_active(&files, "Git Sync"),
            "Git Sync should be inactive on the file list: {files}"
        );
    }
}
