use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::customer::customer_type::CustomerType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub customer_type: CustomerType,
    pub name: String,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub pincode: Option<String>,
    pub state: Option<String>,
    pub gstin: Option<String>,
    pub cin: Option<String>,
    pub pan: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Joins non-empty address parts into a multi-line string for display and PDF templates.
    pub fn formatted_address(&self) -> Option<String> {
        let mut lines: Vec<String> = Vec::new();
        for part in [
            self.address_line_1.as_deref(),
            self.address_line_2.as_deref(),
        ] {
            if let Some(s) = part.map(str::trim).filter(|s| !s.is_empty()) {
                lines.push(s.to_string());
            }
        }
        let city = self
            .city
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let pincode = self
            .pincode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match (city, pincode) {
            (Some(c), Some(p)) => lines.push(format!("{c} {p}")),
            (Some(c), None) => lines.push(c.to_string()),
            (None, Some(p)) => lines.push(p.to_string()),
            (None, None) => {}
        }
        if let Some(s) = self
            .state
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            lines.push(s.to_string());
        }
        if !lines.is_empty() {
            lines.push("India".to_string());
        }
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Address lines joined with Typst line breaks (` \`) for invoice PDF templates.
    pub fn formatted_address_for_typst(&self) -> Option<String> {
        self.formatted_address().map(|a| typst_line_breaks(&a))
    }
}

fn typst_line_breaks(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" \\ ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatted_address_joins_city_and_pincode() {
        let c = Model {
            id: 1,
            created_at: None,
            updated_at: None,
            customer_type: CustomerType::Business,
            name: "WIPRO PARI PRIVATE LIMITED".into(),
            address_line_1: Some(
                "GAT NO. 463/A/2/8 to 463/A/2/11, 463/A/2/15 and 463/A/2/16,".into(),
            ),
            address_line_2: Some(
                "PUNE - BANGLORE HIGHWAY, MOUJE DHANGARWADI, TALUKA KHANDALA".into(),
            ),
            city: Some("Satara".into()),
            pincode: Some("412801".into()),
            state: Some("Maharashtra MH".into()),
            gstin: Some("27AABCP2572Q1ZW".into()),
            cin: None,
            pan: None,
            phone: None,
            email: None,
            website: None,
        };
        assert_eq!(
            c.formatted_address().as_deref(),
            Some(
                "GAT NO. 463/A/2/8 to 463/A/2/11, 463/A/2/15 and 463/A/2/16,\n\
                 PUNE - BANGLORE HIGHWAY, MOUJE DHANGARWADI, TALUKA KHANDALA\n\
                 Satara 412801\n\
                 Maharashtra MH\n\
                 India"
            )
        );
    }

    #[test]
    fn formatted_address_for_typst_uses_line_breaks() {
        let c = Model {
            id: 1,
            created_at: None,
            updated_at: None,
            customer_type: CustomerType::Business,
            name: "Acme".into(),
            address_line_1: Some("Line one".into()),
            address_line_2: None,
            city: Some("Mumbai".into()),
            pincode: Some("400001".into()),
            state: Some("Maharashtra".into()),
            gstin: None,
            cin: None,
            pan: None,
            phone: None,
            email: None,
            website: None,
        };
        assert_eq!(
            c.formatted_address_for_typst().as_deref(),
            Some("Line one \\ Mumbai 400001 \\ Maharashtra \\ India")
        );
    }
}
