use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cancelled_invoices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub posted_invoice_id: i64,
    pub posted_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub number: String,
    pub reference: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub account_receivable_id: i64,
    pub account_revenue_id: i64,
    pub account_tax_payable_id: i64,
    pub journal_id: i64,
    pub datetime: DateTime<Utc>,
    pub customer_id: i64,
    pub credit_note_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
