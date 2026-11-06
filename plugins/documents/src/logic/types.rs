//! Shared document field payloads and helpers.

use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
use sea_orm::{DatabaseConnection, EntityTrait};

use chrono::NaiveDate;
use lariv_core::datetime::parse_date;

/// Fields shown on the document form and detail page, loaded from the type table.
#[derive(Clone, Debug)]
pub struct TypeFields {
    pub vnode_id: i64,
    pub vnode_name: String,
    pub aadhar_number: String,
    pub pan_number: String,
    pub passport_number: String,
    pub name: String,
    pub gender: String,
    pub gender_label: String,
    pub date_of_birth: String,
    pub address: String,
    pub nationality: String,
    pub expiry_date: String,
}

/// Name and number for a document list row.
#[derive(Clone, Debug, Default)]
pub struct TypeSummary {
    pub name: String,
    pub number: String,
}

pub(crate) fn require_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("Name is required".into())
    } else {
        Ok(name.to_string())
    }
}

pub(crate) fn require_dob(raw: &str) -> Result<NaiveDate, String> {
    parse_date(raw).ok_or_else(|| "Date of birth must be DD/MM/YYYY".to_string())
}

pub(crate) async fn require_file(
    db: &DatabaseConnection,
    vnode_id: i64,
    label: &str,
) -> Result<(), String> {
    if vnode_id <= 0 {
        return Err(format!("{label} is required"));
    }
    match VNodeEntity::find_by_id(vnode_id)
        .one(db)
        .await
        .map_err(db_message)?
    {
        Some(node) if node.is_directory => Err("Choose a file, not a folder".into()),
        Some(_) => Ok(()),
        None => Err(format!("{label} was not found")),
    }
}

pub(crate) async fn vnode_name(db: &DatabaseConnection, vnode_id: i64) -> String {
    if vnode_id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(
        VNodeEntity::find_by_id(vnode_id).one(db).await,
        "find document file",
    )
    .map(|node| node.name)
    .unwrap_or_else(|| format!("File #{vnode_id}"))
}

pub(crate) async fn finish_txn<T>(
    txn: sea_orm::DatabaseTransaction,
    result: Result<T, String>,
) -> Result<T, String> {
    match result {
        Ok(value) => {
            txn.commit().await.map_err(db_message)?;
            Ok(value)
        }
        Err(err) => {
            if let Err(rollback) = txn.rollback().await {
                tracing::error!(error = %rollback, "document transaction rollback failed");
            }
            Err(err)
        }
    }
}

pub(crate) fn db_message(err: sea_orm::DbErr) -> String {
    let text = err.to_string();
    if text.contains("uix_aadhar_cards_aadhar_number") {
        "This Aadhar number is already saved".into()
    } else if text.contains("uix_pan_cards_pan_number") {
        "This PAN is already saved".into()
    } else if text.contains("uix_passports_passport_number") {
        "This passport number is already saved".into()
    } else {
        text
    }
}
