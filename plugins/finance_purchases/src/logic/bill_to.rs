//! Which party an purchase is billed to: one contact or one company.

use std::collections::HashMap;

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BillTo {
    pub vendor_is_individual: bool,
    pub vendor_contact_id: Option<i64>,
    pub vendor_company_id: Option<i64>,
}

impl BillTo {
    pub fn new(
        vendor_is_individual: bool,
        vendor_contact_id: Option<i64>,
        vendor_company_id: Option<i64>,
    ) -> Self {
        Self {
            vendor_is_individual,
            vendor_contact_id,
            vendor_company_id,
        }
    }

    pub fn party_id(self) -> i64 {
        if self.vendor_is_individual {
            self.vendor_contact_id.unwrap_or(0)
        } else {
            self.vendor_company_id.unwrap_or(0)
        }
    }

    pub fn checkbox_value(self) -> String {
        if self.vendor_is_individual {
            "on".into()
        } else {
            String::new()
        }
    }
}

pub fn checkbox_on(raw: &str) -> bool {
    raw == "on" || raw == "true" || raw == "1"
}

/// Contact is required when billing an individual; company is required otherwise.
/// A company id sent with an individual is kept (the contact's company, or an override).
pub fn require_bill_to(
    vendor_is_individual: bool,
    individual: i64,
    company: i64,
) -> Result<BillTo, String> {
    let company_id = if company > 0 { Some(company) } else { None };
    if vendor_is_individual {
        if individual <= 0 {
            return Err("select a contact".to_string());
        }
        Ok(BillTo::new(true, Some(individual), company_id))
    } else if company_id.is_none() {
        Err("select a company".to_string())
    } else {
        Ok(BillTo::new(false, None, company_id))
    }
}

/// When an individual is selected and no company was posted, copy the contact's company.
pub async fn fill_company_from_contact(db: &DatabaseConnection, bill: BillTo) -> BillTo {
    if !bill.vendor_is_individual || bill.vendor_company_id.filter(|id| *id > 0).is_some() {
        return bill;
    }
    let Some(contact_id) = bill.vendor_contact_id.filter(|id| *id > 0) else {
        return bill;
    };
    let company_id = ContactEntity::find_by_id(contact_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .and_then(|c| c.company_id)
        .filter(|id| *id > 0);
    BillTo {
        vendor_company_id: company_id,
        ..bill
    }
}

/// Bulk edit: neither id leaves the existing party. When both are set, the checkbox wins.
pub fn optional_bill_to(
    vendor_is_individual: bool,
    individual: i64,
    company: i64,
) -> Result<Option<BillTo>, String> {
    if individual <= 0 && company <= 0 {
        return Ok(None);
    }
    let individual_side = if individual > 0 && company > 0 {
        vendor_is_individual
    } else {
        individual > 0
    };
    require_bill_to(individual_side, individual, company).map(Some)
}

pub fn vendor_name_sql(purchase_table: &str) -> String {
    format!(
        "(CASE WHEN {purchase_table}.vendor_is_individual THEN \
           (SELECT name FROM crm_contacts WHERE crm_contacts.id = {purchase_table}.vendor_contact_id) \
         ELSE \
           (SELECT name FROM crm_companies WHERE crm_companies.id = {purchase_table}.vendor_company_id) \
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
            .filter(|p| p.vendor_is_individual)
            .filter_map(|p| p.vendor_contact_id)
            .filter(|id| *id > 0)
            .collect();
        let company_ids: Vec<i64> = parties
            .iter()
            .filter(|p| !p.vendor_is_individual)
            .filter_map(|p| p.vendor_company_id)
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
        if party.vendor_is_individual {
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
        assert!(!company.vendor_is_individual);
        assert_eq!(company.vendor_company_id, Some(9));
        assert_eq!(company.vendor_contact_id, None);

        let person = require_bill_to(true, 4, 9).unwrap();
        assert!(person.vendor_is_individual);
        assert_eq!(person.vendor_contact_id, Some(4));
        assert_eq!(person.vendor_company_id, Some(9));

        let person_only = require_bill_to(true, 4, 0).unwrap();
        assert_eq!(person_only.vendor_contact_id, Some(4));
        assert_eq!(person_only.vendor_company_id, None);

        assert!(require_bill_to(true, 0, 9).is_err());
        assert!(require_bill_to(false, 4, 0).is_err());
    }

    #[test]
    fn optional_bill_to_skips_when_both_empty() {
        assert_eq!(optional_bill_to(true, 0, 0).unwrap(), None);
        let picked = optional_bill_to(false, 3, 8).unwrap().unwrap();
        assert!(!picked.vendor_is_individual);
        assert_eq!(picked.vendor_company_id, Some(8));
    }
}
