use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const CUSTOMER_TYPE_BUSINESS: &str = "business";
pub const CUSTOMER_TYPE_INDIVIDUAL: &str = "individual";

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum CustomerType {
    #[default]
    #[sea_orm(string_value = "business")]
    Business,
    #[sea_orm(string_value = "individual")]
    Individual,
}

impl CustomerType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Business => CUSTOMER_TYPE_BUSINESS,
            Self::Individual => CUSTOMER_TYPE_INDIVIDUAL,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Business => "Business",
            Self::Individual => "Individual",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            CUSTOMER_TYPE_BUSINESS => Some(Self::Business),
            CUSTOMER_TYPE_INDIVIDUAL => Some(Self::Individual),
            _ => None,
        }
    }
}

impl fmt::Display for CustomerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for CustomerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid CustomerType: {s:?}"))
    }
}

impl From<CustomerType> for String {
    fn from(v: CustomerType) -> Self {
        v.as_str().into()
    }
}
