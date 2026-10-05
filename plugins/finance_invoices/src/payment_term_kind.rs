use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum PaymentTermDateKind {
    #[sea_orm(string_value = "absolute")]
    Absolute,
    #[default]
    #[sea_orm(string_value = "relative")]
    Relative,
    #[sea_orm(string_value = "relative_delivery")]
    RelativeDelivery,
}

impl PaymentTermDateKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absolute => "absolute",
            Self::Relative => "relative",
            Self::RelativeDelivery => "relative_delivery",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "absolute" => Some(Self::Absolute),
            "relative" => Some(Self::Relative),
            "relative_delivery" => Some(Self::RelativeDelivery),
            _ => None,
        }
    }

    pub const fn uses_duration(self) -> bool {
        match self {
            Self::Absolute => false,
            Self::Relative => true,
            Self::RelativeDelivery => true,
        }
    }
}

impl fmt::Display for PaymentTermDateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PaymentTermDateKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid date kind: {s}"))
    }
}

impl From<PaymentTermDateKind> for String {
    fn from(v: PaymentTermDateKind) -> Self {
        v.as_str().into()
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum PaymentTermAmountKind {
    #[sea_orm(string_value = "absolute")]
    Absolute,
    #[default]
    #[sea_orm(string_value = "relative")]
    Relative,
}

impl PaymentTermAmountKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absolute => "absolute",
            Self::Relative => "relative",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "absolute" => Some(Self::Absolute),
            "relative" => Some(Self::Relative),
            _ => None,
        }
    }
}

impl fmt::Display for PaymentTermAmountKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PaymentTermAmountKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid amount kind: {s}"))
    }
}

impl From<PaymentTermAmountKind> for String {
    fn from(v: PaymentTermAmountKind) -> Self {
        v.as_str().into()
    }
}
