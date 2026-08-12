use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::finance_accounts::journal_type::JournalType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "journals")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub is_active: bool,
    pub is_mutable: bool,
    pub currency_id: i64,
    pub journal_type: JournalType,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::currency::Entity",
        from = "Column::CurrencyId",
        to = "super::currency::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Currency,
}

impl Related<super::currency::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Currency.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
