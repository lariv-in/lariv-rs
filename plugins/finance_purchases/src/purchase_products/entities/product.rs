use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Stock this product is kept in.
    pub stock_id: i64,
    pub reference: Option<String>,
    pub remarks: Option<String>,
    pub name: String,
    pub hsn_code: i64,
    /// JSON object of variable name to type (`length`, `weight`, …).
    pub variables: String,
    /// Rune expression for the cost of one product. Purchase quantity multiplies it.
    pub base_price_formula: String,
    /// Rune expression for the sales price of one product before tax. Purchase quantity multiplies it.
    pub sales_price_formula: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::product_tax::Entity")]
    ProductTaxes,
}

impl Related<super::product_tax::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductTaxes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
