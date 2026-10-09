//! Which party an invoice is billed to: one contact or one company.

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

    pub fn checkbox_value(self) -> String {
        if self.bill_to_individual {
            "on".into()
        } else {
            String::new()
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

/// Bulk edit: neither id leaves the existing party. When both are set, the checkbox wins.
pub fn optional_bill_to(
    bill_to_individual: bool,
    individual: i64,
    company: i64,
) -> Result<Option<BillTo>, String> {
    if individual <= 0 && company <= 0 {
        return Ok(None);
    }
    let individual_side = if individual > 0 && company > 0 {
        bill_to_individual
    } else {
        individual > 0
    };
    require_bill_to(individual_side, individual, company).map(Some)
}

pub fn customer_name_sql(invoice_table: &str) -> String {
    format!(
        "(CASE WHEN {invoice_table}.bill_to_individual THEN \
           (SELECT name FROM crm_contacts WHERE crm_contacts.id = {invoice_table}.customer_individual) \
         ELSE \
           (SELECT name FROM crm_companies WHERE crm_companies.id = {invoice_table}.customer_company) \
         END)"
    )
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
        let contacts = if contact_ids.is_empty() {
            HashMap::new()
        } else {
            ContactEntity::find()
                .filter(contact::Column::Id.is_in(contact_ids))
                .all(db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|c| (c.id, c.name))
                .collect()
        };
        let companies = if company_ids.is_empty() {
            HashMap::new()
        } else {
            CompanyEntity::find()
                .filter(company::Column::Id.is_in(company_ids))
                .all(db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|c| (c.id, c.name))
                .collect()
        };
        Self {
            contacts,
            companies,
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

    pub async fn one(db: &DatabaseConnection, party: BillTo) -> String {
        Self::load(db, &[party]).await.name(party)
    }
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
        let company = require_bill_to(false, 4, 9).unwrap();
        assert!(!company.bill_to_individual);
        assert_eq!(company.customer_company, Some(9));
        assert_eq!(company.customer_individual, None);

        let person = require_bill_to(true, 4, 9).unwrap();
        assert!(person.bill_to_individual);
        assert_eq!(person.customer_individual, Some(4));
        assert_eq!(person.customer_company, None);

        assert!(require_bill_to(true, 0, 9).is_err());
        assert!(require_bill_to(false, 4, 0).is_err());
    }

    #[test]
    fn optional_bill_to_skips_when_both_empty() {
        assert_eq!(optional_bill_to(true, 0, 0).unwrap(), None);
        let picked = optional_bill_to(false, 3, 8).unwrap().unwrap();
        assert!(!picked.bill_to_individual);
        assert_eq!(picked.customer_company, Some(8));
    }
}
