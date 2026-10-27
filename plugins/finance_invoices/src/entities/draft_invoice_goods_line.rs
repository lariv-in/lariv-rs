use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "draft_invoice_goods_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub draft_invoice_id: i64,
    /// Null after the product is deleted (`ON DELETE SET NULL`).
    pub product_id: Option<i64>,
    pub product_name: String,
    pub product_reference: Option<String>,
    pub product_remarks: Option<String>,
    pub hsn_code: i64,
    /// Product variable schema JSON (name → type).
    pub variable_types: String,
    pub base_price_formula: String,
    pub sales_price_formula: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate: Decimal,
    /// Number of products. Multiplies `rate`, the price of one product.
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub quantity: Decimal,
    /// Raw variable values typed on the line, as a JSON object.
    pub variable_values: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub pre_tax_amount: Decimal,
    /// Optional note shown with this product line.
    pub remarks: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
