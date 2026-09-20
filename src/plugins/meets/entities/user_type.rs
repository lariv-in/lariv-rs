use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const JOINED_USER_REGISTERED: &str = "registered";
pub const JOINED_USER_ANONYMOUS: &str = "anonymous";

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum JoinedUserType {
    #[default]
    #[sea_orm(string_value = "registered")]
    Registered,
    #[sea_orm(string_value = "anonymous")]
    Anonymous,
}

impl JoinedUserType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registered => JOINED_USER_REGISTERED,
            Self::Anonymous => JOINED_USER_ANONYMOUS,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Registered => "Registered",
            Self::Anonymous => "Anonymous",
        }
    }
}

impl fmt::Display for JoinedUserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for JoinedUserType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            JOINED_USER_REGISTERED => Ok(Self::Registered),
            JOINED_USER_ANONYMOUS => Ok(Self::Anonymous),
            _ => Err(()),
        }
    }
}
