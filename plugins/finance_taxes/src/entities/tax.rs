use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "tax_kind")]
pub enum TaxKind {
    #[sea_orm(string_value = "levied")]
    Levied,
    #[sea_orm(string_value = "withholding")]
    Withholding,
}

impl TaxKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Levied => "levied",
            Self::Withholding => "withholding",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Levied => "Levied",
            Self::Withholding => "Withholding",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "levied" => Some(Self::Levied),
            "withholding" => Some(Self::Withholding),
            _ => None,
        }
    }
}

impl From<TaxKind> for String {
    fn from(v: TaxKind) -> Self {
        v.as_str().into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "taxes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub percentage: Decimal,
    pub tax_type: TaxKind,
    pub account_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type Tax = Model;
