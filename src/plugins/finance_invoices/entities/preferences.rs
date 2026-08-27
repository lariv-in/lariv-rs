use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "invoice_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub account_receivable_id: Option<i64>,
    pub account_revenue_id: Option<i64>,
    pub account_tax_payable_id: Option<i64>,
    pub journal_id: Option<i64>,
    pub invoice_number_format: Option<String>,
    pub invoice_pdf_template: Option<String>,
    pub invoice_logo_vnode_id: Option<i64>,
    pub invoice_signature_vnode_id: Option<i64>,
    pub company_name: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_gstin: Option<String>,
    pub place_of_supply: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
