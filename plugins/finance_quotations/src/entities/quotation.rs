use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const QUOTATION_STATUS_DRAFT: &str = "draft";
pub const QUOTATION_STATUS_ACCEPTED: &str = "accepted";
pub const QUOTATION_STATUS_CANCELLED: &str = "cancelled";

/// Lifecycle of a quotation row.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "quotation_status")]
pub enum QuotationStatus {
    #[default]
    #[sea_orm(string_value = "draft")]
    Draft,
    #[sea_orm(string_value = "accepted")]
    Accepted,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

impl QuotationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => QUOTATION_STATUS_DRAFT,
            Self::Accepted => QUOTATION_STATUS_ACCEPTED,
            Self::Cancelled => QUOTATION_STATUS_CANCELLED,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Accepted => "Accepted",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            QUOTATION_STATUS_DRAFT => Some(Self::Draft),
            QUOTATION_STATUS_ACCEPTED => Some(Self::Accepted),
            QUOTATION_STATUS_CANCELLED => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (QUOTATION_STATUS_DRAFT, "Draft"),
            (QUOTATION_STATUS_ACCEPTED, "Accepted"),
            (QUOTATION_STATUS_CANCELLED, "Cancelled"),
        ]
    }
}

impl From<QuotationStatus> for String {
    fn from(value: QuotationStatus) -> Self {
        value.as_str().into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quotations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub date: NaiveDate,
    pub customer_id: i64,
    pub number: String,
    pub status: QuotationStatus,
    pub bank_account: Option<String>,
    pub remarks: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
