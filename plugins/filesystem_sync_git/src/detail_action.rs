//! Directory detail button shown when a git sync row exists.

use std::sync::Arc;

use async_trait::async_trait;
use maud::Markup;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use lariv_core::components::button_modal_route;
use lariv_plugin_filesystem::detail_actions::{
    VNodeDetailAction, VNodeDetailActionRegistrar, VNodeDetailActionRegistry,
};
use lariv_plugin_filesystem::entities::VNode;

use crate::entities::filesystem_git_sync::{Column, Entity};
use crate::routes::{GitSyncConfirmGetRouteTag, GitSyncRunGetRouteTag};

/// Registers the sync button on filesystem directory pages.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl VNodeDetailActionRegistrar for Hook {
    fn register_detail_actions(
        self,
        registry: VNodeDetailActionRegistry,
    ) -> VNodeDetailActionRegistry {
        crate::sidebar::register();
        registry.push(Arc::new(SyncButton))
    }
}

struct SyncButton;

#[async_trait]
impl VNodeDetailAction for SyncButton {
    async fn header_actions(
        &self,
        db: &DatabaseConnection,
        node: &VNode,
        can_write: bool,
    ) -> Markup {
        if !can_write || !node.is_directory {
            return maud::html! {};
        }
        let found = Entity::find()
            .filter(Column::DirectoryVnodeId.eq(node.id))
            .one(db)
            .await
            .ok()
            .flatten();
        if found.is_none() {
            return maud::html! {};
        }
        maud::html! {
            (button_modal_route(
                GitSyncRunGetRouteTag::new(node.id),
                "Git Sync",
                "btn-outline",
            ))
            (button_modal_route(
                GitSyncConfirmGetRouteTag::new(node.id),
                "Force Sync",
                "btn-outline",
            ))
        }
    }
}
