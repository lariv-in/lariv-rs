use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "journal_entry_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub datetime: DateTime<Utc>,
    pub account_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub amount: Decimal,
    pub journal_entry_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id"
    )]
    Account,
    #[sea_orm(
        belongs_to = "super::journal_entry::Entity",
        from = "Column::JournalEntryId",
        to = "super::journal_entry::Column::Id"
    )]
    JournalEntry,
}

impl ActiveModelBehavior for ActiveModel {}
