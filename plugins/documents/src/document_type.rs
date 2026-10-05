use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const DOCUMENT_TYPE_AADHAR_CARD: &str = "aadhar_card";
pub const DOCUMENT_TYPE_PAN: &str = "pan";
pub const DOCUMENT_TYPE_PASSPORT: &str = "passport";

/// Postgres enum `document_type`. Add a variant here when a new document table is introduced.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "document_type")]
pub enum DocumentType {
    #[default]
    #[sea_orm(string_value = "aadhar_card")]
    AadharCard,
    #[sea_orm(string_value = "pan")]
    Pan,
    #[sea_orm(string_value = "passport")]
    Passport,
}

impl DocumentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AadharCard => DOCUMENT_TYPE_AADHAR_CARD,
            Self::Pan => DOCUMENT_TYPE_PAN,
            Self::Passport => DOCUMENT_TYPE_PASSPORT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AadharCard => "Aadhar card",
            Self::Pan => "PAN",
            Self::Passport => "Passport",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            DOCUMENT_TYPE_AADHAR_CARD => Some(Self::AadharCard),
            DOCUMENT_TYPE_PAN => Some(Self::Pan),
            DOCUMENT_TYPE_PASSPORT => Some(Self::Passport),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (DOCUMENT_TYPE_AADHAR_CARD, "Aadhar card"),
            (DOCUMENT_TYPE_PAN, "PAN"),
            (DOCUMENT_TYPE_PASSPORT, "Passport"),
        ]
    }
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for DocumentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid document type: {s:?}"))
    }
}

impl From<DocumentType> for String {
    fn from(v: DocumentType) -> Self {
        v.as_str().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aadhar_card() {
        assert_eq!(
            DocumentType::parse(DOCUMENT_TYPE_AADHAR_CARD),
            Some(DocumentType::AadharCard)
        );
        assert_eq!(DocumentType::AadharCard.as_str(), "aadhar_card");
        assert_eq!(DocumentType::parse("pan"), Some(DocumentType::Pan));
        assert_eq!(
            DocumentType::parse("passport"),
            Some(DocumentType::Passport)
        );
        assert!(DocumentType::parse("voter_id").is_none());
    }
}
