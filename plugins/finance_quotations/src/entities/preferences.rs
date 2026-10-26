use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quotation_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub quotation_number_format: Option<String>,
    pub quotation_pdf_template: Option<String>,
    pub quotation_email_subject: Option<String>,
    pub quotation_email_body: Option<String>,
    pub company_name: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_gstin: Option<String>,
    pub place_of_supply: Option<String>,
    pub default_bank_account: Option<String>,
    pub company_logo_vnode_id: Option<i64>,
    pub company_signature_vnode_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
