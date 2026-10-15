//! Which party an invoice is billed to: one contact or one company.

use std::collections::HashMap;

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use lariv_plugin_crm::entities::customer::{self, Entity as CustomerEntity};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
};

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

/// Contact is required when billing an individual; company is required otherwise.
/// A company id sent with an individual is kept (the contact's company, or an override).
pub fn require_bill_to(
    bill_to_individual: bool,
    individual: i64,
    company: i64,
) -> Result<BillTo, String> {
    let company_id = if company > 0 { Some(company) } else { None };
    if bill_to_individual {
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
    if !bill.bill_to_individual || bill.customer_company.filter(|id| *id > 0).is_some() {
        return bill;
    }
    let Some(contact_id) = bill.customer_individual.filter(|id| *id > 0) else {
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
        customer_company: company_id,
        ..bill
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
        "(SELECT CASE WHEN crm_customers.is_individual THEN \
           (SELECT name FROM crm_contacts WHERE crm_contacts.id = crm_customers.individual_id) \
         ELSE \
           (SELECT name FROM crm_companies WHERE crm_companies.id = crm_customers.company_id) \
         END FROM crm_customers WHERE crm_customers.id = {invoice_table}.customer_id)"
    )
}

impl BillTo {
    pub fn from_customer(row: &customer::Model) -> Self {
        if row.is_individual {
            Self::new(true, row.individual_id, None)
        } else {
            Self::new(false, None, row.company_id)
        }
    }
}

pub async fn require_customer_id<C: ConnectionTrait>(
    db: &C,
    customer_id: i64,
) -> Result<i64, String> {
    lariv_plugin_crm::logic::customer::require_existing(db, customer_id).await
}

pub async fn customer_label(db: &DatabaseConnection, customer_id: i64) -> String {
    if customer_id <= 0 {
        return String::new();
    }
    let name = PartyLabels::one(db, load_bill_to(db, customer_id).await).await;
    if name == "—" { String::new() } else { name }
}

pub async fn load_bill_to<C: ConnectionTrait>(db: &C, customer_id: i64) -> BillTo {
    let Ok(Some(row)) = CustomerEntity::find_by_id(customer_id).one(db).await else {
        return BillTo::new(false, None, None);
    };
    BillTo::from_customer(&row)
}

pub async fn load_bill_tos<C: ConnectionTrait>(
    db: &C,
    customer_ids: &[i64],
) -> HashMap<i64, BillTo> {
    if customer_ids.is_empty() {
        return HashMap::new();
    }
    CustomerEntity::find()
        .filter(customer::Column::Id.is_in(customer_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, BillTo::from_customer(&row)))
        .collect()
}

pub async fn parties_in_order<C: ConnectionTrait>(db: &C, customer_ids: &[i64]) -> Vec<BillTo> {
    let loaded = load_bill_tos(db, customer_ids).await;
    customer_ids
        .iter()
        .map(|id| {
            loaded
                .get(id)
                .copied()
                .unwrap_or_else(|| BillTo::new(false, None, None))
        })
        .collect()
}

/// Find the customer for this party, or insert one. Company and individual customers stay distinct.
pub async fn ensure_customer_id<C: ConnectionTrait>(db: &C, bill: BillTo) -> Result<i64, String> {
    if bill.bill_to_individual {
        let individual_id = bill
            .customer_individual
            .filter(|id| *id > 0)
            .ok_or_else(|| "select a contact".to_string())?;
        find_or_insert_customer(db, true, None, Some(individual_id)).await
    } else {
        let company_id = bill
            .customer_company
            .filter(|id| *id > 0)
            .ok_or_else(|| "select a company".to_string())?;
        find_or_insert_customer(db, false, Some(company_id), None).await
    }
}

async fn find_or_insert_customer<C: ConnectionTrait>(
    db: &C,
    is_individual: bool,
    company_id: Option<i64>,
    individual_id: Option<i64>,
) -> Result<i64, String> {
    if let Some(existing) = find_customer(db, is_individual, company_id, individual_id).await? {
        return Ok(existing);
    }
    let now = chrono::Utc::now();
    let inserted = customer::ActiveModel {
        is_individual: Set(is_individual),
        company_id: Set(company_id),
        individual_id: Set(individual_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await;
    match inserted {
        Ok(row) => Ok(row.id),
        Err(err) => find_customer(db, is_individual, company_id, individual_id)
            .await?
            .ok_or_else(|| err.to_string()),
    }
}

async fn find_customer<C: ConnectionTrait>(
    db: &C,
    is_individual: bool,
    company_id: Option<i64>,
    individual_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let mut query = CustomerEntity::find().filter(customer::Column::IsIndividual.eq(is_individual));
    if let Some(company_id) = company_id {
        query = query.filter(customer::Column::CompanyId.eq(company_id));
    }
    if let Some(individual_id) = individual_id {
        query = query.filter(customer::Column::IndividualId.eq(individual_id));
    }
    query
        .one(db)
        .await
        .map(|row| row.map(|row| row.id))
        .map_err(|e| e.to_string())
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
        assert_eq!(person.customer_company, Some(9));

        let person_only = require_bill_to(true, 4, 0).unwrap();
        assert_eq!(person_only.customer_individual, Some(4));
        assert_eq!(person_only.customer_company, None);

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
