use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::finance_accounts::balance_type::BalanceType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub code: i32,
    pub is_group: bool,
    pub balance_type: BalanceType,
    pub parent_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentId",
        to = "Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Parent,
}

impl ActiveModelBehavior for ActiveModel {}

pub const BALANCE_TYPE_DEBIT: &str = "Debit";
pub const BALANCE_TYPE_CREDIT: &str = "Credit";
