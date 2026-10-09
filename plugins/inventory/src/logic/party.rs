//! Which party a stock movement is for: one contact or one company.

use std::collections::HashMap;

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BillTo {
    pub bill_to_individual: bool,
    pub customer_individual: Option<i64>,
    pub customer_company: Option<i64>,
}

impl BillTo {
    pub fn new(
        bill_to_individual: bool,
        customer_individual: Option<i64>,
        customer_company: Option<i64>,
    ) -> Self {
        Self {
            bill_to_individual,
            customer_individual,
            customer_company,
        }
    }

    pub fn party_id(self) -> i64 {
        if self.bill_to_individual {
            self.customer_individual.unwrap_or(0)
        } else {
            self.customer_company.unwrap_or(0)
        }
    }

    pub fn from_row(
        bill_to_individual: bool,
        customer_individual: Option<i64>,
        customer_company: Option<i64>,
    ) -> Self {
        Self {
            bill_to_individual,
            customer_individual,
            customer_company,
        }
    }
}

pub fn checkbox_on(raw: &str) -> bool {
    raw == "on" || raw == "true" || raw == "1"
}

/// Exactly one side is set. `individual` and `company` are form ids (`<= 0` means unset).
pub fn require_bill_to(
    bill_to_individual: bool,
    individual: i64,
    company: i64,
) -> Result<BillTo, String> {
    if bill_to_individual {
        if individual <= 0 {
            return Err("select a contact".to_string());
        }
        Ok(BillTo::new(true, Some(individual), None))
    } else if company <= 0 {
        Err("select a company".to_string())
    } else {
        Ok(BillTo::new(false, None, Some(company)))
    }
}

pub struct PartyLabels {
    contacts: HashMap<i64, String>,
    companies: HashMap<i64, String>,
}

impl PartyLabels {
    pub async fn load(db: &DatabaseConnection, parties: &[BillTo]) -> Self {
        let contact_ids: Vec<i64> = parties
            .iter()
            .filter(|p| p.bill_to_individual)
            .filter_map(|p| p.customer_individual)
            .filter(|id| *id > 0)
            .collect();
        let company_ids: Vec<i64> = parties
            .iter()
            .filter(|p| !p.bill_to_individual)
            .filter_map(|p| p.customer_company)
            .filter(|id| *id > 0)
            .collect();
        Self {
            contacts: contact_names(db, &contact_ids).await,
            companies: company_names(db, &company_ids).await,
        }
    }

    pub fn name(&self, party: BillTo) -> String {
        let id = party.party_id();
        if id <= 0 {
            return "—".into();
        }
        if party.bill_to_individual {
            self.contacts
                .get(&id)
                .cloned()
                .unwrap_or_else(|| format!("#{id}"))
        } else {
            self.companies
                .get(&id)
                .cloned()
                .unwrap_or_else(|| format!("#{id}"))
        }
    }
}

pub async fn contact_names(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    ContactEntity::find()
        .filter(contact::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect()
}

pub async fn company_names(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    CompanyEntity::find()
        .filter(company::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect()
}

pub async fn party_displays(
    db: &DatabaseConnection,
    individual: i64,
    company: i64,
) -> (String, String) {
    let labels = PartyLabels::load(
        db,
        &[
            BillTo::new(true, Some(individual), None),
            BillTo::new(false, None, Some(company)),
        ],
    )
    .await;
    (
        if individual > 0 {
            labels.name(BillTo::new(true, Some(individual), None))
        } else {
            String::new()
        },
        if company > 0 {
            labels.name(BillTo::new(false, None, Some(company)))
        } else {
            String::new()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_bill_to_keeps_one_side() {
        let company = require_bill_to(false, 4, 9).expect("company side");
        assert!(!company.bill_to_individual);
        assert_eq!(company.customer_company, Some(9));
        assert_eq!(company.customer_individual, None);

        let person = require_bill_to(true, 4, 9).expect("contact side");
        assert!(person.bill_to_individual);
        assert_eq!(person.customer_individual, Some(4));
        assert_eq!(person.customer_company, None);

        require_bill_to(true, 0, 9).unwrap_err();
        require_bill_to(false, 4, 0).unwrap_err();
    }
}
