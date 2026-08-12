use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "journal_type")]
pub enum JournalType {
    #[sea_orm(string_value = "Credit")]
    Credit,
    #[sea_orm(string_value = "Debit")]
    Debit,
}

impl JournalType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Credit => "Credit",
            Self::Debit => "Debit",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Credit" => Some(Self::Credit),
            "Debit" => Some(Self::Debit),
            _ => None,
        }
    }
}

impl fmt::Display for JournalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JournalType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid JournalType: {s:?}"))
    }
}

impl Default for JournalType {
    fn default() -> Self {
        Self::Debit
    }
}
