use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PAYMENT_SOURCE_DOC_TYPE: &str = "p_finance_payments.Payment";

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub customer_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub amount: Decimal,
    pub datetime: DateTime<Utc>,
    #[sea_orm(unique)]
    pub number: String,
    pub journal_entry_id: i64,
    pub is_gst_advance: bool,
    pub gst_pos: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
