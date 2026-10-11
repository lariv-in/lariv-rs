use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cancelled_purchases")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub posted_purchase_id: i64,
    pub posted_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub number: String,
    pub reference: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub remarks: Option<String>,
    pub account_payable_id: i64,
    pub account_expense_id: i64,
    pub account_input_tax_id: i64,
    pub journal_id: i64,
    pub datetime: DateTime<Utc>,
    pub delivery_date: Option<NaiveDate>,
    pub vendor_is_individual: bool,
    pub vendor_contact_id: Option<i64>,
    pub vendor_company_id: Option<i64>,
    pub reversed_journal_entry_id: i64,
    /// Why the posted purchase was cancelled.
    pub reason: Option<String>,
    pub posted_payment_term_id: Option<i64>,
}

pub const PURCHASE_REVERSAL_SOURCE_DOC_TYPE: &str = "p_finance_purchases.PurchaseReversal";

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
