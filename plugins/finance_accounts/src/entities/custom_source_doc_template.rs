use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "custom_source_doc_templates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub remarks: Option<String>,
    /// JSON object of variable name to type (`length`, `weight`, …).
    pub variables: String,
    /// Attached file or folder in the filesystem.
    pub attached_vnode_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::custom_source_doc_journal_entry_item_template::Entity")]
    JournalEntryItemTemplates,
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::AttachedVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_delete = "SetNull"
    )]
    Attached,
}

impl Related<super::custom_source_doc_journal_entry_item_template::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntryItemTemplates.def()
    }
}

impl Related<lariv_plugin_filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Attached.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
