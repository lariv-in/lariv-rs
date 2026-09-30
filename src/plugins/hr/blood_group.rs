use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const BLOOD_GROUP_A_POSITIVE: &str = "A+";
pub const BLOOD_GROUP_A_NEGATIVE: &str = "A-";
pub const BLOOD_GROUP_B_POSITIVE: &str = "B+";
pub const BLOOD_GROUP_B_NEGATIVE: &str = "B-";
pub const BLOOD_GROUP_AB_POSITIVE: &str = "AB+";
pub const BLOOD_GROUP_AB_NEGATIVE: &str = "AB-";
pub const BLOOD_GROUP_O_POSITIVE: &str = "O+";
pub const BLOOD_GROUP_O_NEGATIVE: &str = "O-";

/// Postgres enum `hr_blood_group`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "hr_blood_group")]
pub enum BloodGroup {
    #[sea_orm(string_value = "A+")]
    APositive,
    #[sea_orm(string_value = "A-")]
    ANegative,
    #[sea_orm(string_value = "B+")]
    BPositive,
    #[sea_orm(string_value = "B-")]
    BNegative,
    #[sea_orm(string_value = "AB+")]
    AbPositive,
    #[sea_orm(string_value = "AB-")]
    AbNegative,
    #[sea_orm(string_value = "O+")]
    OPositive,
    #[sea_orm(string_value = "O-")]
    ONegative,
}

impl BloodGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::APositive => BLOOD_GROUP_A_POSITIVE,
            Self::ANegative => BLOOD_GROUP_A_NEGATIVE,
            Self::BPositive => BLOOD_GROUP_B_POSITIVE,
            Self::BNegative => BLOOD_GROUP_B_NEGATIVE,
            Self::AbPositive => BLOOD_GROUP_AB_POSITIVE,
            Self::AbNegative => BLOOD_GROUP_AB_NEGATIVE,
            Self::OPositive => BLOOD_GROUP_O_POSITIVE,
            Self::ONegative => BLOOD_GROUP_O_NEGATIVE,
        }
    }

    pub fn label(self) -> &'static str {
        self.as_str()
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            BLOOD_GROUP_A_POSITIVE => Some(Self::APositive),
            BLOOD_GROUP_A_NEGATIVE => Some(Self::ANegative),
            BLOOD_GROUP_B_POSITIVE => Some(Self::BPositive),
            BLOOD_GROUP_B_NEGATIVE => Some(Self::BNegative),
            BLOOD_GROUP_AB_POSITIVE => Some(Self::AbPositive),
            BLOOD_GROUP_AB_NEGATIVE => Some(Self::AbNegative),
            BLOOD_GROUP_O_POSITIVE => Some(Self::OPositive),
            BLOOD_GROUP_O_NEGATIVE => Some(Self::ONegative),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (BLOOD_GROUP_A_POSITIVE, "A+"),
            (BLOOD_GROUP_A_NEGATIVE, "A-"),
            (BLOOD_GROUP_B_POSITIVE, "B+"),
            (BLOOD_GROUP_B_NEGATIVE, "B-"),
            (BLOOD_GROUP_AB_POSITIVE, "AB+"),
            (BLOOD_GROUP_AB_NEGATIVE, "AB-"),
            (BLOOD_GROUP_O_POSITIVE, "O+"),
            (BLOOD_GROUP_O_NEGATIVE, "O-"),
        ]
    }
}

impl fmt::Display for BloodGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for BloodGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid blood group: {s:?}"))
    }
}
