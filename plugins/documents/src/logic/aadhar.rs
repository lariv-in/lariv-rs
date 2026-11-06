//! Aadhar card type-row helpers.

use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use lariv_core::datetime::{format_date, parse_date};

use crate::entities::aadhar_card::{self, Entity as AadharCardEntity};
use crate::forms::DocumentForm;
use crate::gender::Gender;
use crate::logic::types::{TypeFields, db_message, vnode_name};

pub(crate) struct ParsedAadhar {
    pub vnode_id: i64,
    pub aadhar_number: String,
    pub name: String,
    pub gender: Gender,
    pub date_of_birth: NaiveDate,
    pub address: String,
}

pub fn normalize_aadhar_number(raw: &str) -> Result<String, String> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.len() == 12 && digits.chars().all(|c| c.is_ascii_digit()) {
        Ok(digits)
    } else {
        Err("Aadhar number must be 12 digits".into())
    }
}

pub(crate) async fn load_map(
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

pub(crate) async fn load_fields(db: &DatabaseConnection, id: i32) -> Result<TypeFields, String> {
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

pub(crate) fn parse(form: &DocumentForm) -> Result<ParsedAadhar, String> {
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

pub(crate) async fn insert(
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

pub(crate) async fn update(
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

#[cfg(test)]
mod tests {
    use super::normalize_aadhar_number;

    #[test]
    fn aadhar_number_accepts_twelve_digits() {
        assert_eq!(
            normalize_aadhar_number("1234 5678 9012").as_deref(),
            Ok("123456789012")
        );
        assert!(normalize_aadhar_number("123").is_err());
    }
}
