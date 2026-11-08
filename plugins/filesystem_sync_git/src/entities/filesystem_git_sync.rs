use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "filesystem_git_syncs")]
/// One git checkout bound to a filesystem directory.
pub struct Model {
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(column_type = "Text")]
    pub git_url: String,
    /// Branch name, tag name, or full commit id.
    #[sea_orm(column_type = "Text")]
    pub git_identifier: String,
    /// Peeled commit id written by the last successful sync.
    #[sea_orm(column_type = "Text", nullable)]
    pub last_synced_commit_hash: Option<String>,
    /// Path inside the repository. Empty is the repository root.
    #[sea_orm(column_type = "Text")]
    pub subpath: String,
    /// `filesystem_nodes.id` of the directory that receives the tree.
    /// One directory has at most one git sync row.
    #[sea_orm(primary_key, auto_increment = false)]
    pub directory_vnode_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::DirectoryVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Directory,
}

impl ActiveModelBehavior for ActiveModel {}

pub type FilesystemGitSync = Model;
