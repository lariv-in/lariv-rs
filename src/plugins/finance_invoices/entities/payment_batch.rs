use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PAYMENT_BATCH_SOURCE_DOC_TYPE: &str = "p_finance_invoices.PaymentBatch";

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payment_batches")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub datetime: DateTime<Utc>,
    pub account_id: i64,
    pub journal_entry_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub total_amount: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
