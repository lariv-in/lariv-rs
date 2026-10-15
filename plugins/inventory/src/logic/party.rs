//! Stock-movement customer: one `crm_customers` row.

use std::collections::HashMap;

use lariv_plugin_contacts::entities::contact::{self, Entity as ContactEntity};
use lariv_plugin_crm::entities::customer::{self, Entity as CustomerEntity};
use lariv_plugin_crm::logic::customer::{self as customers, detail_href};
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn require_customer<C: ConnectionTrait>(db: &C, customer_id: i64) -> Result<i64, String> {
    customers::require_existing(db, customer_id).await
}

pub struct CustomerLabel {
    pub name: String,
    pub href: String,
}

pub async fn labels(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, CustomerLabel> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    let rows = CustomerEntity::find()
        .filter(customer::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default();
    let mut out = HashMap::new();
    for row in rows {
        let name = customers::display_name(db, row.id).await;
        let href = detail_href(&row);
        out.insert(row.id, CustomerLabel { name, href });
    }
    out
}

pub async fn label(db: &DatabaseConnection, customer_id: i64) -> CustomerLabel {
    labels(db, &[customer_id])
        .await
        .remove(&customer_id)
        .unwrap_or(CustomerLabel {
            name: if customer_id > 0 {
                format!("#{customer_id}")
            } else {
                String::new()
            },
            href: String::new(),
        })
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
