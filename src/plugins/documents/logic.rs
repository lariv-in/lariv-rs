//! Load and save type-specific rows behind a document.
//!
//! Callers only pass a [`document::Model`]. `document_type` selects the table
//! and `document_type_id` is that table's primary key.

use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::datetime::{format_date, parse_date};
use crate::plugins::filesystem::entities::filesystem_node::Entity as VNodeEntity;

use super::document_type::DocumentType;
use super::entities::{
    aadhar_card::{self, Entity as AadharCardEntity},
    document::{self, Entity as DocumentEntity},
    pan_card::{self, Entity as PanCardEntity},
    passport::{self, Entity as PassportEntity},
};
use super::forms::DocumentForm;
use super::gender::Gender;

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

struct ParsedAadhar {
    vnode_id: i64,
    aadhar_number: String,
    name: String,
    gender: Gender,
    date_of_birth: NaiveDate,
    address: String,
}

pub fn normalize_aadhar_number(raw: &str) -> Result<String, String> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.len() == 12 && digits.chars().all(|c| c.is_ascii_digit()) {
        Ok(digits)
    } else {
        Err("Aadhar number must be 12 digits".into())
    }
}

pub async fn load_type_fields(
    db: &DatabaseConnection,
    doc: &document::Model,
) -> Result<TypeFields, String> {
    match doc.document_type {
        DocumentType::AadharCard => load_aadhar_fields(db, doc.document_type_id).await,
        DocumentType::Pan => load_pan_fields(db, doc.document_type_id).await,
        DocumentType::Passport => load_passport_fields(db, doc.document_type_id).await,
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
    let aadhar_by_id = load_aadhar_map(db, aadhar_ids).await?;
    let pan_by_id = load_pan_map(db, pan_ids).await?;
    let passport_by_id = load_passport_map(db, passport_ids).await?;
    let mut out = std::collections::HashMap::new();
    for doc in docs {
        let summary = match doc.document_type {
            DocumentType::AadharCard => aadhar_by_id.get(&doc.document_type_id).map(|card| {
                TypeSummary {
                    name: card.name.clone(),
                    number: card.aadhar_number.clone(),
                }
            }),
            DocumentType::Pan => pan_by_id.get(&doc.document_type_id).map(|card| TypeSummary {
                name: card.name.clone(),
                number: card.pan_number.clone(),
            }),
            DocumentType::Passport => {
                passport_by_id.get(&doc.document_type_id).map(|card| TypeSummary {
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

async fn load_aadhar_map(
    db: &DatabaseConnection,
    ids: Vec<i32>,
) -> Result<std::collections::HashMap<i32, aadhar_card::Model>, sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    Ok(AadharCardEntity::find()
        .filter(aadhar_card::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect())
}

async fn load_pan_map(
    db: &DatabaseConnection,
    ids: Vec<i32>,
) -> Result<std::collections::HashMap<i32, pan_card::Model>, sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    Ok(PanCardEntity::find()
        .filter(pan_card::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect())
}

async fn load_passport_map(
    db: &DatabaseConnection,
    ids: Vec<i32>,
) -> Result<std::collections::HashMap<i32, passport::Model>, sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    Ok(PassportEntity::find()
        .filter(passport::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect())
}

/// Check type and Aadhaar fields before any database work.
pub fn validate_document_form(form: &DocumentForm) -> Result<(), String> {
    let document_type = DocumentType::parse(&form.document_type)
        .ok_or_else(|| format!("Unknown document type: {}", form.document_type.trim()))?;
    match document_type {
        DocumentType::AadharCard => {
            parse_aadhar(form)?;
        }
        DocumentType::Pan => {
            parse_pan(form)?;
        }
        DocumentType::Passport => {
            parse_passport(form)?;
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
                let parsed = parse_aadhar(form)?;
                require_file(db, parsed.vnode_id, "Aadhar card file").await?;
                insert_aadhar(&txn, &parsed, now).await?
            }
            DocumentType::Pan => {
                let parsed = parse_pan(form)?;
                require_file(db, parsed.vnode_id, "PAN file").await?;
                insert_pan(&txn, &parsed, now).await?
            }
            DocumentType::Passport => {
                let parsed = parse_passport(form)?;
                require_file(db, parsed.vnode_id, "Passport file").await?;
                insert_passport(&txn, &parsed, now).await?
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
                let parsed = parse_aadhar(form)?;
                require_file(db, parsed.vnode_id, "Aadhar card file").await?;
                update_aadhar(&txn, existing.document_type_id, &parsed, now).await?;
            }
            DocumentType::Pan => {
                let parsed = parse_pan(form)?;
                require_file(db, parsed.vnode_id, "PAN file").await?;
                update_pan(&txn, existing.document_type_id, &parsed, now).await?;
            }
            DocumentType::Passport => {
                let parsed = parse_passport(form)?;
                require_file(db, parsed.vnode_id, "Passport file").await?;
                update_passport(&txn, existing.document_type_id, &parsed, now).await?;
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

async fn load_aadhar_fields(db: &DatabaseConnection, id: i32) -> Result<TypeFields, String> {
    let card = AadharCardEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_message)?
        .ok_or_else(|| "Aadhar card record for this document was not found".to_string())?;
    let vnode_name = vnode_name(db, card.vnode_id).await;
    Ok(TypeFields {
        vnode_id: card.vnode_id,
        vnode_name,
        aadhar_number: card.aadhar_number,
        pan_number: String::new(),
        passport_number: String::new(),
        name: card.name,
        gender: card.gender.as_str().to_string(),
        gender_label: card.gender.label().to_string(),
        date_of_birth: format_date(card.date_of_birth),
        address: card.address,
        nationality: String::new(),
        expiry_date: String::new(),
    })
}

async fn load_pan_fields(db: &DatabaseConnection, id: i32) -> Result<TypeFields, String> {
    let card = PanCardEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_message)?
        .ok_or_else(|| "PAN record for this document was not found".to_string())?;
    let vnode_name = vnode_name(db, card.vnode_id).await;
    Ok(TypeFields {
        vnode_id: card.vnode_id,
        vnode_name,
        aadhar_number: String::new(),
        pan_number: card.pan_number,
        passport_number: String::new(),
        name: card.name,
        gender: String::new(),
        gender_label: String::new(),
        date_of_birth: format_date(card.date_of_birth),
        address: String::new(),
        nationality: String::new(),
        expiry_date: String::new(),
    })
}

async fn load_passport_fields(db: &DatabaseConnection, id: i32) -> Result<TypeFields, String> {
    let card = PassportEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_message)?
        .ok_or_else(|| "Passport record for this document was not found".to_string())?;
    let vnode_name = vnode_name(db, card.vnode_id).await;
    Ok(TypeFields {
        vnode_id: card.vnode_id,
        vnode_name,
        aadhar_number: String::new(),
        pan_number: String::new(),
        passport_number: card.passport_number,
        name: card.name,
        gender: card.gender.as_str().to_string(),
        gender_label: card.gender.label().to_string(),
        date_of_birth: format_date(card.date_of_birth),
        address: String::new(),
        nationality: card.nationality,
        expiry_date: format_date(card.expiry_date),
    })
}

fn parse_aadhar(form: &DocumentForm) -> Result<ParsedAadhar, String> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err("Name is required".into());
    }
    let address = form.address.trim();
    if address.is_empty() {
        return Err("Address is required".into());
    }
    let gender = Gender::parse(&form.gender).ok_or_else(|| "Choose a gender".to_string())?;
    let date_of_birth = parse_date(&form.date_of_birth)
        .ok_or_else(|| "Date of birth must be DD/MM/YYYY".to_string())?;
    Ok(ParsedAadhar {
        vnode_id: form.vnode_id,
        aadhar_number: normalize_aadhar_number(&form.aadhar_number)?,
        name: name.to_string(),
        gender,
        date_of_birth,
        address: address.to_string(),
    })
}

fn normalize_pan_number(raw: &str) -> Result<String, String> {
    let pan: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect();
    let bytes = pan.as_bytes();
    let ok = bytes.len() == 10
        && bytes[..5].iter().all(|b| b.is_ascii_uppercase())
        && bytes[5..9].iter().all(|b| b.is_ascii_digit())
        && bytes[9].is_ascii_uppercase();
    if ok {
        Ok(pan)
    } else {
        Err("PAN must be 5 letters, 4 digits, and 1 letter".into())
    }
}

fn normalize_passport_number(raw: &str) -> Result<String, String> {
    let number: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect();
    let ok = (6..=12).contains(&number.len()) && number.chars().all(|c| c.is_ascii_alphanumeric());
    if ok {
        Ok(number)
    } else {
        Err("Passport number must be 6 to 12 letters or digits".into())
    }
}

struct ParsedPan {
    vnode_id: i64,
    pan_number: String,
    name: String,
    date_of_birth: NaiveDate,
}

struct ParsedPassport {
    vnode_id: i64,
    passport_number: String,
    name: String,
    gender: Gender,
    date_of_birth: NaiveDate,
    nationality: String,
    expiry_date: NaiveDate,
}

fn require_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("Name is required".into())
    } else {
        Ok(name.to_string())
    }
}

fn require_dob(raw: &str) -> Result<NaiveDate, String> {
    parse_date(raw).ok_or_else(|| "Date of birth must be DD/MM/YYYY".to_string())
}

fn parse_pan(form: &DocumentForm) -> Result<ParsedPan, String> {
    Ok(ParsedPan {
        vnode_id: form.vnode_id,
        pan_number: normalize_pan_number(&form.pan_number)?,
        name: require_name(&form.name)?,
        date_of_birth: require_dob(&form.date_of_birth)?,
    })
}

fn parse_passport(form: &DocumentForm) -> Result<ParsedPassport, String> {
    let nationality = form.nationality.trim();
    if nationality.is_empty() {
        return Err("Nationality is required".into());
    }
    let gender = Gender::parse(&form.gender).ok_or_else(|| "Choose a gender".to_string())?;
    let expiry_date =
        parse_date(&form.expiry_date).ok_or_else(|| "Expiry date must be DD/MM/YYYY".to_string())?;
    Ok(ParsedPassport {
        vnode_id: form.vnode_id,
        passport_number: normalize_passport_number(&form.passport_number)?,
        name: require_name(&form.name)?,
        gender,
        date_of_birth: require_dob(&form.date_of_birth)?,
        nationality: nationality.to_string(),
        expiry_date,
    })
}

async fn require_file(db: &DatabaseConnection, vnode_id: i64, label: &str) -> Result<(), String> {
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

async fn vnode_name(db: &DatabaseConnection, vnode_id: i64) -> String {
    if vnode_id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(
        VNodeEntity::find_by_id(vnode_id).one(db).await,
        "find document file",
    )
    .map(|node| node.name)
    .unwrap_or_else(|| format!("File #{vnode_id}"))
}

async fn insert_aadhar(
    db: &impl sea_orm::ConnectionTrait,
    parsed: &ParsedAadhar,
    now: chrono::DateTime<Utc>,
) -> Result<i32, String> {
    let card = aadhar_card::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        vnode_id: Set(parsed.vnode_id),
        aadhar_number: Set(parsed.aadhar_number.clone()),
        name: Set(parsed.name.clone()),
        gender: Set(parsed.gender),
        date_of_birth: Set(parsed.date_of_birth),
        address: Set(parsed.address.clone()),
        ..Default::default()
    };
    card.insert(db)
        .await
        .map(|saved| saved.id)
        .map_err(db_message)
}

async fn update_aadhar(
    db: &impl sea_orm::ConnectionTrait,
    id: i32,
    parsed: &ParsedAadhar,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let card = aadhar_card::ActiveModel {
        id: Set(id),
        updated_at: Set(Some(now)),
        vnode_id: Set(parsed.vnode_id),
        aadhar_number: Set(parsed.aadhar_number.clone()),
        name: Set(parsed.name.clone()),
        gender: Set(parsed.gender),
        date_of_birth: Set(parsed.date_of_birth),
        address: Set(parsed.address.clone()),
        ..Default::default()
    };
    card.update(db).await.map(|_| ()).map_err(db_message)
}

async fn finish_txn<T>(
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

async fn insert_pan(
    db: &impl sea_orm::ConnectionTrait,
    parsed: &ParsedPan,
    now: chrono::DateTime<Utc>,
) -> Result<i32, String> {
    let card = pan_card::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        vnode_id: Set(parsed.vnode_id),
        pan_number: Set(parsed.pan_number.clone()),
        name: Set(parsed.name.clone()),
        date_of_birth: Set(parsed.date_of_birth),
        ..Default::default()
    };
    card.insert(db)
        .await
        .map(|saved| saved.id)
        .map_err(db_message)
}

async fn update_pan(
    db: &impl sea_orm::ConnectionTrait,
    id: i32,
    parsed: &ParsedPan,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let card = pan_card::ActiveModel {
        id: Set(id),
        updated_at: Set(Some(now)),
        vnode_id: Set(parsed.vnode_id),
        pan_number: Set(parsed.pan_number.clone()),
        name: Set(parsed.name.clone()),
        date_of_birth: Set(parsed.date_of_birth),
        ..Default::default()
    };
    card.update(db).await.map(|_| ()).map_err(db_message)
}

async fn insert_passport(
    db: &impl sea_orm::ConnectionTrait,
    parsed: &ParsedPassport,
    now: chrono::DateTime<Utc>,
) -> Result<i32, String> {
    let card = passport::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        vnode_id: Set(parsed.vnode_id),
        passport_number: Set(parsed.passport_number.clone()),
        name: Set(parsed.name.clone()),
        gender: Set(parsed.gender),
        date_of_birth: Set(parsed.date_of_birth),
        nationality: Set(parsed.nationality.clone()),
        expiry_date: Set(parsed.expiry_date),
        ..Default::default()
    };
    card.insert(db)
        .await
        .map(|saved| saved.id)
        .map_err(db_message)
}

async fn update_passport(
    db: &impl sea_orm::ConnectionTrait,
    id: i32,
    parsed: &ParsedPassport,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let card = passport::ActiveModel {
        id: Set(id),
        updated_at: Set(Some(now)),
        vnode_id: Set(parsed.vnode_id),
        passport_number: Set(parsed.passport_number.clone()),
        name: Set(parsed.name.clone()),
        gender: Set(parsed.gender),
        date_of_birth: Set(parsed.date_of_birth),
        nationality: Set(parsed.nationality.clone()),
        expiry_date: Set(parsed.expiry_date),
        ..Default::default()
    };
    card.update(db).await.map(|_| ()).map_err(db_message)
}

fn db_message(err: sea_orm::DbErr) -> String {
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

#[cfg(test)]
mod tests {
    use super::{normalize_aadhar_number, normalize_pan_number, normalize_passport_number};

    #[test]
    fn aadhar_number_accepts_twelve_digits() {
        assert_eq!(
            normalize_aadhar_number("1234 5678 9012").as_deref(),
            Ok("123456789012")
        );
        assert!(normalize_aadhar_number("123").is_err());
    }

    #[test]
    fn pan_number_accepts_standard_shape() {
        assert_eq!(
            normalize_pan_number("abcde1234f").as_deref(),
            Ok("ABCDE1234F")
        );
        assert!(normalize_pan_number("ABCD1234F").is_err());
    }

    #[test]
    fn passport_number_rejects_short_values() {
        assert!(normalize_passport_number("A123").is_err());
        assert_eq!(
            normalize_passport_number("a1234567").as_deref(),
            Ok("A1234567")
        );
    }
}
