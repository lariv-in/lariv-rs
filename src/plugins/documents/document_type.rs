use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const DOCUMENT_TYPE_AADHAR_CARD: &str = "aadhar_card";

/// Postgres enum `document_type`. Add a variant here when a new document table is introduced.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "document_type")]
pub enum DocumentType {
    #[default]
    #[sea_orm(string_value = "aadhar_card")]
    AadharCard,
}

impl DocumentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AadharCard => DOCUMENT_TYPE_AADHAR_CARD,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AadharCard => "Aadhar card",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            DOCUMENT_TYPE_AADHAR_CARD => Some(Self::AadharCard),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[(DOCUMENT_TYPE_AADHAR_CARD, "Aadhar card")]
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
        assert!(DocumentType::parse("passport").is_none());
    }
}
