use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const CUSTOM_SOURCE_DOC_TYPE: &str = "p_finance_accounts.CustomSourceDoc";

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "custom_source_docs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub created_by_id: i64,
    #[sea_orm(unique)]
    pub journal_entry_id: i64,
    /// Attached file or folder in the filesystem.
    pub attached_vnode_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::journal_entry::Entity",
        from = "Column::JournalEntryId",
        to = "super::journal_entry::Column::Id"
    )]
    JournalEntry,
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::AttachedVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_delete = "SetNull"
    )]
    Attached,
}

impl Related<super::journal_entry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

impl Related<lariv_plugin_filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Attached.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
