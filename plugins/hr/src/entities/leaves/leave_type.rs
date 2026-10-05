use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const LEAVE_TYPE_CASUAL: &str = "casual";
pub const LEAVE_TYPE_SICK: &str = "sick";
pub const LEAVE_TYPE_PRIVILEGE: &str = "privilege";

/// Leave category stored as `varchar(32)` on `hr_leave_applications.leave_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum LeaveType {
    #[sea_orm(string_value = "casual")]
    Casual,
    #[sea_orm(string_value = "sick")]
    Sick,
    #[sea_orm(string_value = "privilege")]
    Privilege,
}

impl LeaveType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Casual => LEAVE_TYPE_CASUAL,
            Self::Sick => LEAVE_TYPE_SICK,
            Self::Privilege => LEAVE_TYPE_PRIVILEGE,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Casual => "Casual",
            Self::Sick => "Sick",
            Self::Privilege => "Privilege Leave",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            LEAVE_TYPE_CASUAL => Some(Self::Casual),
            LEAVE_TYPE_SICK => Some(Self::Sick),
            LEAVE_TYPE_PRIVILEGE => Some(Self::Privilege),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (LEAVE_TYPE_CASUAL, "Casual"),
            (LEAVE_TYPE_SICK, "Sick"),
            (LEAVE_TYPE_PRIVILEGE, "Privilege Leave"),
        ]
    }
}

impl fmt::Display for LeaveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for LeaveType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid leave type: {s:?}"))
    }
}

impl From<LeaveType> for String {
    fn from(v: LeaveType) -> Self {
        v.as_str().into()
    }
}
