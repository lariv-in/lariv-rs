use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "draft_invoice_service_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub draft_invoice_id: i64,
    /// Null after the service is deleted (`ON DELETE SET NULL`).
    pub service_id: Option<i64>,
    pub service_name: String,
    pub service_reference: Option<String>,
    pub service_remarks: Option<String>,
    pub sac_code: i64,
    /// Service variable schema JSON (name → type).
    pub variable_types: String,
    pub base_price_formula: String,
    pub sales_price_formula: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub quantity: Decimal,
    pub variable_values: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub pre_tax_amount: Decimal,
    pub remarks: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
