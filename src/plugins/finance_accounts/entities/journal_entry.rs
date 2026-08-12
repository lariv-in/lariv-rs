use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "journal_entries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub datetime: DateTime<Utc>,
    pub source_doc_id: i64,
    pub journal_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::journal::Entity",
        from = "Column::JournalId",
        to = "super::journal::Column::Id"
    )]
    Journal,
    #[sea_orm(
        belongs_to = "super::source_doc::Entity",
        from = "Column::SourceDocId",
        to = "super::source_doc::Column::Id"
    )]
    SourceDoc,
}

impl Related<super::journal::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Journal.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
