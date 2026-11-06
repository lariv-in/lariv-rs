//! Passport type-row helpers.

use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use lariv_core::datetime::{format_date, parse_date};

use crate::entities::passport::{self, Entity as PassportEntity};
use crate::forms::DocumentForm;
use crate::gender::Gender;
use crate::logic::types::{TypeFields, db_message, require_dob, require_name, vnode_name};

pub(crate) struct ParsedPassport {
    pub vnode_id: i64,
    pub passport_number: String,
    pub name: String,
    pub gender: Gender,
    pub date_of_birth: NaiveDate,
    pub nationality: String,
    pub expiry_date: NaiveDate,
}

pub(crate) fn normalize_passport_number(raw: &str) -> Result<String, String> {
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

pub(crate) async fn load_map(
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

pub(crate) async fn load_fields(db: &DatabaseConnection, id: i32) -> Result<TypeFields, String> {
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

pub(crate) fn parse(form: &DocumentForm) -> Result<ParsedPassport, String> {
    let nationality = form.nationality.trim();
    if nationality.is_empty() {
        return Err("Nationality is required".into());
    }
    let gender = Gender::parse(&form.gender).ok_or_else(|| "Choose a gender".to_string())?;
    let expiry_date = parse_date(&form.expiry_date)
        .ok_or_else(|| "Expiry date must be DD/MM/YYYY".to_string())?;
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

pub(crate) async fn insert(
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

pub(crate) async fn update(
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

#[cfg(test)]
mod tests {
    use super::normalize_passport_number;

    #[test]
    fn passport_number_rejects_short_values() {
        assert!(normalize_passport_number("A123").is_err());
        assert_eq!(
            normalize_passport_number("a1234567").as_deref(),
            Ok("A1234567")
        );
    }
}
