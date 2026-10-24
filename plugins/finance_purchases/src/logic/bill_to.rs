//! Vendor on a purchase: one `purchase_vendors` row, shown as a contact or a company.

use std::collections::HashMap;

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entities::vendor::{self, Entity as VendorEntity};

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

    pub fn from_vendor(row: &vendor::Model) -> Self {
        if row.is_individual {
            Self::new(true, row.individual_id, None)
        } else {
            Self::new(false, None, row.company_id)
        }
    }

    pub fn party_id(self) -> i64 {
        if self.vendor_is_individual {
            self.vendor_contact_id.unwrap_or(0)
        } else {
            self.vendor_company_id.unwrap_or(0)
        }
    }
}

pub fn vendor_name_sql(purchase_table: &str) -> String {
    format!(
        "(SELECT CASE WHEN purchase_vendors.is_individual THEN \
           (SELECT name FROM crm_contacts WHERE crm_contacts.id = purchase_vendors.individual_id) \
         ELSE \
           (SELECT name FROM crm_companies WHERE crm_companies.id = purchase_vendors.company_id) \
         END FROM purchase_vendors WHERE purchase_vendors.id = {purchase_table}.vendor_id)"
    )
}

pub async fn require_vendor_id<C: ConnectionTrait>(db: &C, vendor_id: i64) -> Result<i64, String> {
    if vendor_id <= 0 {
        return Err("select a vendor".to_string());
    }
    match VendorEntity::find_by_id(vendor_id).one(db).await {
        Ok(Some(_)) => Ok(vendor_id),
        Ok(None) => Err("select a vendor".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

pub async fn vendor_label(db: &DatabaseConnection, vendor_id: i64) -> String {
    if vendor_id <= 0 {
        return String::new();
    }
    let name = PartyLabels::one(db, load_bill_to(db, vendor_id).await).await;
    if name == "—" { String::new() } else { name }
}

pub async fn load_bill_to<C: ConnectionTrait>(db: &C, vendor_id: i64) -> BillTo {
    let Ok(Some(row)) = VendorEntity::find_by_id(vendor_id).one(db).await else {
        return BillTo::new(false, None, None);
    };
    BillTo::from_vendor(&row)
}

pub async fn load_bill_tos<C: ConnectionTrait>(db: &C, vendor_ids: &[i64]) -> HashMap<i64, BillTo> {
    if vendor_ids.is_empty() {
        return HashMap::new();
    }
    VendorEntity::find()
        .filter(vendor::Column::Id.is_in(vendor_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, BillTo::from_vendor(&row)))
        .collect()
}

pub async fn parties_in_order<C: ConnectionTrait>(db: &C, vendor_ids: &[i64]) -> Vec<BillTo> {
    let loaded = load_bill_tos(db, vendor_ids).await;
    vendor_ids
        .iter()
        .map(|id| {
            loaded
                .get(id)
                .copied()
                .unwrap_or_else(|| BillTo::new(false, None, None))
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn vendor(
        is_individual: bool,
        company_id: Option<i64>,
        individual_id: Option<i64>,
    ) -> vendor::Model {
        vendor::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            is_individual,
            company_id,
            individual_id,
        }
    }

    #[test]
    fn vendor_row_becomes_one_party() {
        let company = BillTo::from_vendor(&vendor(false, Some(9), None));
        assert!(!company.vendor_is_individual);
        assert_eq!(company.vendor_company_id, Some(9));
        assert_eq!(company.party_id(), 9);

        let person = BillTo::from_vendor(&vendor(true, None, Some(4)));
        assert!(person.vendor_is_individual);
        assert_eq!(person.vendor_contact_id, Some(4));
        assert_eq!(person.party_id(), 4);
    }

    #[test]
    fn vendor_name_sql_reads_vendor() {
        let sql = vendor_name_sql("draft_purchases");
        assert!(sql.contains("draft_purchases.vendor_id"));
        assert!(sql.contains("purchase_vendors"));
    }
}
