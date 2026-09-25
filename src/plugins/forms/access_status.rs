use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const ACCESS_STATUS_NOT_ACCEPTING_SUBMISSIONS: &str = "NotAcceptingSubmissions";
pub const ACCESS_STATUS_ANYONE_WITH_LINK: &str = "AnyoneWithLink";

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "access_status")]
pub enum AccessStatus {
    #[default]
    #[sea_orm(string_value = "NotAcceptingSubmissions")]
    NotAcceptingSubmissions,
    #[sea_orm(string_value = "AnyoneWithLink")]
    AnyoneWithLink,
}

impl AccessStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotAcceptingSubmissions => ACCESS_STATUS_NOT_ACCEPTING_SUBMISSIONS,
            Self::AnyoneWithLink => ACCESS_STATUS_ANYONE_WITH_LINK,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NotAcceptingSubmissions => "Not accepting submissions",
            Self::AnyoneWithLink => "Anyone with the link",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            ACCESS_STATUS_NOT_ACCEPTING_SUBMISSIONS => Some(Self::NotAcceptingSubmissions),
            ACCESS_STATUS_ANYONE_WITH_LINK => Some(Self::AnyoneWithLink),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (
                ACCESS_STATUS_NOT_ACCEPTING_SUBMISSIONS,
                "Not accepting submissions",
            ),
            (ACCESS_STATUS_ANYONE_WITH_LINK, "Anyone with the link"),
        ]
    }

    pub fn accepts_submissions(self) -> bool {
        matches!(self, Self::AnyoneWithLink)
    }
}

impl fmt::Display for AccessStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for AccessStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid AccessStatus: {s:?}"))
    }
}

impl From<AccessStatus> for String {
    fn from(v: AccessStatus) -> Self {
        v.as_str().into()
    }
}
