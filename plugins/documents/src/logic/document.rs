//! Document CRUD that dispatches to the type-specific table.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, TransactionTrait,
};

use crate::document_type::DocumentType;
use crate::entities::{
    aadhar_card::Entity as AadharCardEntity,
    document::{self, Entity as DocumentEntity},
    pan_card::Entity as PanCardEntity,
    passport::Entity as PassportEntity,
};
use crate::forms::DocumentForm;
use crate::logic::{
    aadhar, pan, passport,
    types::{TypeFields, TypeSummary, db_message, finish_txn, require_file},
};

pub async fn load_type_fields(
    db: &DatabaseConnection,
    doc: &document::Model,
) -> Result<TypeFields, String> {
    match doc.document_type {
        DocumentType::AadharCard => aadhar::load_fields(db, doc.document_type_id).await,
        DocumentType::Pan => pan::load_fields(db, doc.document_type_id).await,
        DocumentType::Passport => passport::load_fields(db, doc.document_type_id).await,
    }
}

pub async fn load_type_summaries(
    db: &DatabaseConnection,
    docs: &[document::Model],
) -> Result<std::collections::HashMap<i64, TypeSummary>, sea_orm::DbErr> {
    let mut aadhar_ids = Vec::new();
    let mut pan_ids = Vec::new();
    let mut passport_ids = Vec::new();
    for doc in docs {
        match doc.document_type {
            DocumentType::AadharCard => aadhar_ids.push(doc.document_type_id),
            DocumentType::Pan => pan_ids.push(doc.document_type_id),
            DocumentType::Passport => passport_ids.push(doc.document_type_id),
        }
    }
    let aadhar_by_id = aadhar::load_map(db, aadhar_ids).await?;
    let pan_by_id = pan::load_map(db, pan_ids).await?;
    let passport_by_id = passport::load_map(db, passport_ids).await?;
    let mut out = std::collections::HashMap::new();
    for doc in docs {
        let summary = match doc.document_type {
            DocumentType::AadharCard => {
                aadhar_by_id
                    .get(&doc.document_type_id)
                    .map(|card| TypeSummary {
                        name: card.name.clone(),
                        number: card.aadhar_number.clone(),
                    })
            }
            DocumentType::Pan => pan_by_id
                .get(&doc.document_type_id)
                .map(|card| TypeSummary {
                    name: card.name.clone(),
                    number: card.pan_number.clone(),
                }),
            DocumentType::Passport => {
                passport_by_id
                    .get(&doc.document_type_id)
                    .map(|card| TypeSummary {
                        name: card.name.clone(),
                        number: card.passport_number.clone(),
                    })
            }
        };
        if let Some(summary) = summary {
            out.insert(doc.id, summary);
        }
    }
    Ok(out)
}

/// Check type and Aadhaar fields before any database work.
pub fn validate_document_form(form: &DocumentForm) -> Result<(), String> {
    let document_type = DocumentType::parse(&form.document_type)
        .ok_or_else(|| format!("Unknown document type: {}", form.document_type.trim()))?;
    match document_type {
        DocumentType::AadharCard => {
            aadhar::parse(form)?;
        }
        DocumentType::Pan => {
            pan::parse(form)?;
        }
        DocumentType::Passport => {
            passport::parse(form)?;
        }
    }
    Ok(())
}

pub async fn create_document(
    db: &DatabaseConnection,
    form: &DocumentForm,
) -> Result<document::Model, String> {
    let document_type = DocumentType::parse(&form.document_type)
        .ok_or_else(|| format!("Unknown document type: {}", form.document_type.trim()))?;
    let now = Utc::now();
    let txn = db.begin().await.map_err(db_message)?;
    let saved = async {
        let type_id = match document_type {
            DocumentType::AadharCard => {
                let parsed = aadhar::parse(form)?;
                require_file(db, parsed.vnode_id, "Aadhar card file").await?;
                aadhar::insert(&txn, &parsed, now).await?
            }
            DocumentType::Pan => {
                let parsed = pan::parse(form)?;
                require_file(db, parsed.vnode_id, "PAN file").await?;
                pan::insert(&txn, &parsed, now).await?
            }
            DocumentType::Passport => {
                let parsed = passport::parse(form)?;
                require_file(db, parsed.vnode_id, "Passport file").await?;
                passport::insert(&txn, &parsed, now).await?
            }
        };
        let doc = document::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            document_type: Set(document_type),
            document_type_id: Set(type_id),
            ..Default::default()
        };
        doc.insert(&txn).await.map_err(db_message)
    }
    .await;
    finish_txn(txn, saved).await
}

pub async fn update_document(
    db: &DatabaseConnection,
    existing: &document::Model,
    form: &DocumentForm,
) -> Result<(), String> {
    let document_type = DocumentType::parse(&form.document_type)
        .ok_or_else(|| format!("Unknown document type: {}", form.document_type.trim()))?;
    if document_type != existing.document_type {
        return Err("Document type cannot be changed".into());
    }
    let now = Utc::now();
    let txn = db.begin().await.map_err(db_message)?;
    let saved = async {
        match document_type {
            DocumentType::AadharCard => {
                let parsed = aadhar::parse(form)?;
                require_file(db, parsed.vnode_id, "Aadhar card file").await?;
                aadhar::update(&txn, existing.document_type_id, &parsed, now).await?;
            }
            DocumentType::Pan => {
                let parsed = pan::parse(form)?;
                require_file(db, parsed.vnode_id, "PAN file").await?;
                pan::update(&txn, existing.document_type_id, &parsed, now).await?;
            }
            DocumentType::Passport => {
                let parsed = passport::parse(form)?;
                require_file(db, parsed.vnode_id, "Passport file").await?;
                passport::update(&txn, existing.document_type_id, &parsed, now).await?;
            }
        }
        let doc = document::ActiveModel {
            id: Set(existing.id),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        doc.update(&txn).await.map_err(db_message)?;
        Ok(())
    }
    .await;
    finish_txn(txn, saved).await
}

pub async fn delete_document(
    db: &DatabaseConnection,
    existing: &document::Model,
) -> Result<(), String> {
    let txn = db.begin().await.map_err(db_message)?;
    let saved = async {
        DocumentEntity::delete_by_id(existing.id)
            .exec(&txn)
            .await
            .map_err(db_message)?;
        match existing.document_type {
            DocumentType::AadharCard => {
                AadharCardEntity::delete_by_id(existing.document_type_id)
                    .exec(&txn)
                    .await
                    .map_err(db_message)?;
            }
            DocumentType::Pan => {
                PanCardEntity::delete_by_id(existing.document_type_id)
                    .exec(&txn)
                    .await
                    .map_err(db_message)?;
            }
            DocumentType::Passport => {
                PassportEntity::delete_by_id(existing.document_type_id)
                    .exec(&txn)
                    .await
                    .map_err(db_message)?;
            }
        }
        Ok(())
    }
    .await;
    finish_txn(txn, saved).await
}
