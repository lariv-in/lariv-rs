use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "banks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub bank_name: Option<String>,
    pub bank_code: String,
    #[sea_orm(unique, indexed)]
    pub ifsc: String,
    pub branch_name: Option<String>,
    pub center: Option<String>,
    pub district: Option<String>,
    pub state: Option<String>,
    pub address: Option<String>,
    pub contact: Option<String>,
    pub imps: bool,
    pub rtgs: bool,
    pub city: Option<String>,
    pub iso3166: Option<String>,
    pub neft: bool,
    pub micr: Option<String>,
    pub upi: bool,
    pub swift: Option<String>,
    pub ach_credit: bool,
    pub ach_debit: bool,
    pub apbs: bool,
    pub nach_debit: bool,
    pub bank_type: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::bank_account::Entity")]
    BankAccount,
}

impl Related<super::bank_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BankAccount.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Label shown in FK pickers and account forms.
    pub fn display_label(&self) -> String {
        let bank = self.bank_name.as_deref().unwrap_or("").trim();
        let branch = self.branch_name.as_deref().unwrap_or("").trim();
        match (bank.is_empty(), branch.is_empty()) {
            (true, true) => self.ifsc.clone(),
            (false, true) => format!("{} — {}", self.ifsc, bank),
            (true, false) => format!("{} — {}", self.ifsc, branch),
            (false, false) => format!("{} — {} / {}", self.ifsc, bank, branch),
        }
    }
}
