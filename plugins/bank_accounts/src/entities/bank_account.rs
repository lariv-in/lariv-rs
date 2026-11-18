use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bank_accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub micr_code: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::bank::Entity",
        from = "Column::IfscCode",
        to = "super::bank::Column::Ifsc"
    )]
    Bank,
}

impl Related<super::bank::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Bank.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
