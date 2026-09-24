use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const APPLICANT_GENDER_MALE: &str = "male";
pub const APPLICANT_GENDER_FEMALE: &str = "female";
pub const APPLICANT_GENDER_OTHER: &str = "other";
pub const APPLICANT_GENDER_PREFER_NOT_TO_SAY: &str = "prefer_not_to_say";

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum ApplicantGender {
    #[sea_orm(string_value = "male")]
    Male,
    #[sea_orm(string_value = "female")]
    Female,
    #[sea_orm(string_value = "other")]
    Other,
    #[sea_orm(string_value = "prefer_not_to_say")]
    PreferNotToSay,
}

impl ApplicantGender {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Male => APPLICANT_GENDER_MALE,
            Self::Female => APPLICANT_GENDER_FEMALE,
            Self::Other => APPLICANT_GENDER_OTHER,
            Self::PreferNotToSay => APPLICANT_GENDER_PREFER_NOT_TO_SAY,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
            Self::Other => "Other",
            Self::PreferNotToSay => "Prefer not to say",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            APPLICANT_GENDER_MALE => Some(Self::Male),
            APPLICANT_GENDER_FEMALE => Some(Self::Female),
            APPLICANT_GENDER_OTHER => Some(Self::Other),
            APPLICANT_GENDER_PREFER_NOT_TO_SAY => Some(Self::PreferNotToSay),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (APPLICANT_GENDER_MALE, "Male"),
            (APPLICANT_GENDER_FEMALE, "Female"),
            (APPLICANT_GENDER_OTHER, "Other"),
            (APPLICANT_GENDER_PREFER_NOT_TO_SAY, "Prefer not to say"),
        ]
    }
}

impl fmt::Display for ApplicantGender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for ApplicantGender {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid gender: {s:?}"))
    }
}

impl From<ApplicantGender> for String {
    fn from(v: ApplicantGender) -> Self {
        v.as_str().into()
    }
}
