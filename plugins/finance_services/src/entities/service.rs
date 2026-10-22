use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "services")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub reference: Option<String>,
    pub remarks: Option<String>,
    pub name: String,
    pub sac_code: i64,
    /// JSON object of variable name to type (`length`, `weight`, …).
    pub variables: String,
    /// Rune expression for the cost of one service. Invoice quantity multiplies it.
    pub base_price_formula: String,
    /// Rune expression for the sales price of one service before tax. Invoice quantity multiplies it.
    pub sales_price_formula: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::service_tax::Entity")]
    ServiceTaxes,
}

impl Related<super::service_tax::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServiceTaxes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
