use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const GENDER_MALE: &str = "male";
pub const GENDER_FEMALE: &str = "female";
pub const GENDER_TRANSGENDER: &str = "transgender";

/// Gender printed on an identity document. Stored as text so only `document_type` is a Postgres enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum Gender {
    #[sea_orm(string_value = "male")]
    Male,
    #[sea_orm(string_value = "female")]
    Female,
    #[sea_orm(string_value = "transgender")]
    Transgender,
}

impl Gender {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Male => GENDER_MALE,
            Self::Female => GENDER_FEMALE,
            Self::Transgender => GENDER_TRANSGENDER,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
            Self::Transgender => "Transgender",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            GENDER_MALE => Some(Self::Male),
            GENDER_FEMALE => Some(Self::Female),
            GENDER_TRANSGENDER => Some(Self::Transgender),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (GENDER_MALE, "Male"),
            (GENDER_FEMALE, "Female"),
            (GENDER_TRANSGENDER, "Transgender"),
        ]
    }
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Gender {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid gender: {s:?}"))
    }
}

impl From<Gender> for String {
    fn from(v: Gender) -> Self {
        v.as_str().into()
    }
}
