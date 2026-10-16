use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Singleton HR preferences (`id = 1`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Minijinja template rendered with the employee and user when an employee
    /// is created. Empty uses the default year-of-joining plus padded user id.
    pub employee_id_format: Option<String>,
    /// Minijinja template that expands to Typst for an employee ID card.
    /// Empty uses the shipped example.
    pub id_card_format: Option<String>,
    /// PKCS#8 PEM P-256 private key. Empty is replaced with a generated key before use.
    pub id_card_signing_key: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
