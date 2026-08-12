use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::crm::deal_stage::DealStage;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_deals")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub company_id: i64,
    pub primary_contact_id: i64,
    pub name: String,
    pub amount: Option<Decimal>,
    pub stage: DealStage,
    pub expected_close_date: Option<NaiveDate>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::company::Entity",
        from = "Column::CompanyId",
        to = "super::company::Column::Id"
    )]
    Company,
    #[sea_orm(
        belongs_to = "super::contact::Entity",
        from = "Column::PrimaryContactId",
        to = "super::contact::Column::Id"
    )]
    PrimaryContact,
}

impl Related<super::company::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Company.def()
    }
}

impl Related<super::contact::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PrimaryContact.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
