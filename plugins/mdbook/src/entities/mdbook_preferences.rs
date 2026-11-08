use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "mdbook_preferences")]
/// Singleton mdBook preferences (`id = 1`).
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// `filesystem_nodes.id` of the directory that contains `book.toml` and `src/`.
    #[sea_orm(indexed)]
    pub source_vnode_id: i64,
    /// `filesystem_nodes.id` of the directory that receives the built HTML site.
    #[sea_orm(indexed)]
    pub output_vnode_id: i64,
    /// Public URL prefix. `index.html` is also redirected from this path.
    pub url_prefix: String,
    /// Rebuild and publish when the server starts.
    pub build_on_serve: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::SourceVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Source,
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::OutputVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Output,
}

impl ActiveModelBehavior for ActiveModel {}

pub type MdBookPreferences = Model;
