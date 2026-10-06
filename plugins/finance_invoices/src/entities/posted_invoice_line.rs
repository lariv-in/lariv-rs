use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posted_invoice_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub posted_invoice_id: i64,
    pub product_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub quantity: Decimal,
    /// Raw variable values copied from the draft line.
    pub variable_values: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub pre_tax_amount: Decimal,
    /// Optional note copied from the draft line.
    pub remarks: Option<String>,
    pub journal_entry_item_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
