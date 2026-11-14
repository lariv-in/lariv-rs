//! Foreign keys from customers to CRM companies and contacts.
//!
//! `crm_customers` is created before the rename to `customers`. Both names are
//! retried after every installed plugin has migrated, because CRM may migrate first.

use lariv_core::db::migration_sql::{ForeignKey, fk_restrict};

pub const FK_CRM_CUSTOMERS_COMPANY: ForeignKey = fk_restrict(
    "fk_crm_customers_company_id",
    "crm_customers",
    "company_id",
    "crm_companies",
);
pub const FK_CRM_CUSTOMERS_INDIVIDUAL: ForeignKey = fk_restrict(
    "fk_crm_customers_individual_id",
    "crm_customers",
    "individual_id",
    "crm_contacts",
);
pub const FK_CUSTOMERS_COMPANY: ForeignKey = fk_restrict(
    "fk_customers_company_id",
    "customers",
    "company_id",
    "crm_companies",
);
pub const FK_CUSTOMERS_INDIVIDUAL: ForeignKey = fk_restrict(
    "fk_customers_individual_id",
    "customers",
    "individual_id",
    "crm_contacts",
);

pub const DEFERRED_FOREIGN_KEYS: &[ForeignKey] = &[
    FK_CRM_CUSTOMERS_COMPANY,
    FK_CRM_CUSTOMERS_INDIVIDUAL,
    FK_CUSTOMERS_COMPANY,
    FK_CUSTOMERS_INDIVIDUAL,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deferred_foreign_keys_are_unique_identifiers() {
        assert!(lariv_core::db::migration_sql::foreign_keys_are_unique(
            DEFERRED_FOREIGN_KEYS
        ));
    }
}
