use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::customer_type::CustomerType;

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
                push_address_text(&mut lines, s);
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
        let city_new = city.filter(|c| !address_has_place(&lines, c));
        let pincode_new = pincode.filter(|p| !address_has_digits(&lines, p));
        match (city_new, pincode_new) {
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
            .filter(|s| !address_has_place(&lines, s))
        {
            lines.push(s.to_string());
        }
        if !lines.is_empty() && !address_has_place(&lines, "India") {
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

/// A newline before `- Pune` is a wrapped "Dist. - Pune", not a new address line.
fn push_address_text(lines: &mut Vec<String>, text: &str) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if is_dash_continuation(line) {
            if let Some(prev) = lines.last_mut() {
                if !prev.ends_with(' ') {
                    prev.push(' ');
                }
                prev.push_str(line);
                continue;
            }
        }
        lines.push(line.to_string());
    }
}

fn is_dash_continuation(line: &str) -> bool {
    let mut chars = line.chars();
    match chars.next() {
        Some('-' | '+') => matches!(chars.next(), Some(' ' | '\t') | None),
        _ => false,
    }
}

fn address_has_digits(lines: &[String], digits: &str) -> bool {
    let digits: String = digits.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return false;
    }
    lines.iter().any(|line| {
        let found: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
        found.contains(&digits)
    })
}

fn address_has_place(lines: &[String], place: &str) -> bool {
    let words: Vec<String> = place
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .map(|w| w.to_lowercase())
        .collect();
    if words.is_empty() {
        return false;
    }
    words.iter().any(|word| {
        lines.iter().any(|line| {
            line.split(|c: char| !c.is_alphanumeric())
                .any(|token| places_match(word, &token.to_lowercase()))
        })
    })
}

fn places_match(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let (short, long) = if a.chars().count() <= b.chars().count() {
        (a, b)
    } else {
        (b, a)
    };
    if short.chars().count() >= 4 && long.contains(short) {
        return true;
    }
    let shared = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    shared >= 6 && shared * 2 >= a.chars().count().min(b.chars().count())
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

    #[test]
    fn formatted_address_keeps_district_dash_and_skips_repeated_locality() {
        let c = Model {
            id: 1,
            created_at: None,
            updated_at: None,
            customer_type: CustomerType::Business,
            name: "KDS".into(),
            address_line_1: Some("Plot No. : D-5, D-7, D-9, C-63, C-1 & F5".into()),
            address_line_2: Some("MIDC Jejuri – 412 303, Dist.\n- Pune, Maharashtra, India".into()),
            city: Some("Pune".into()),
            pincode: Some("412303".into()),
            state: Some("Maharastra".into()),
            gstin: None,
            cin: None,
            pan: None,
            phone: None,
            email: None,
            website: None,
        };
        assert_eq!(
            c.formatted_address().as_deref(),
            Some(
                "Plot No. : D-5, D-7, D-9, C-63, C-1 & F5\n\
                 MIDC Jejuri – 412 303, Dist. - Pune, Maharashtra, India"
            )
        );
        assert_eq!(
            c.formatted_address_for_typst().as_deref(),
            Some(
                "Plot No. : D-5, D-7, D-9, C-63, C-1 & F5 \\ \
                 MIDC Jejuri – 412 303, Dist. - Pune, Maharashtra, India"
            )
        );
    }
}
