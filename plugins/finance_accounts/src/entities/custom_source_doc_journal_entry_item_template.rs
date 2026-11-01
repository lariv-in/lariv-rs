use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "custom_source_doc_journal_entry_item_templates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub custom_source_doc_template_id: i64,
    pub account_id: i64,
    /// Rune expression. May reference variables defined on the parent template.
    pub formula: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::custom_source_doc_template::Entity",
        from = "Column::CustomSourceDocTemplateId",
        to = "super::custom_source_doc_template::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    CustomSourceDocTemplate,
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Account,
}

impl Related<super::custom_source_doc_template::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomSourceDocTemplate.def()
    }
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
