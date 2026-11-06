//! PAN card type-row helpers.

use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use lariv_core::datetime::format_date;

use crate::entities::pan_card::{self, Entity as PanCardEntity};
use crate::forms::DocumentForm;
use crate::logic::types::{TypeFields, db_message, require_dob, require_name, vnode_name};

pub(crate) struct ParsedPan {
    pub vnode_id: i64,
    pub pan_number: String,
    pub name: String,
    pub date_of_birth: NaiveDate,
}

pub(crate) fn normalize_pan_number(raw: &str) -> Result<String, String> {
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

pub(crate) async fn load_map(
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

pub(crate) async fn load_fields(db: &DatabaseConnection, id: i32) -> Result<TypeFields, String> {
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

pub(crate) fn parse(form: &DocumentForm) -> Result<ParsedPan, String> {
    Ok(ParsedPan {
        vnode_id: form.vnode_id,
        pan_number: normalize_pan_number(&form.pan_number)?,
        name: require_name(&form.name)?,
        date_of_birth: require_dob(&form.date_of_birth)?,
    })
}

pub(crate) async fn insert(
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

pub(crate) async fn update(
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

#[cfg(test)]
mod tests {
    use super::normalize_pan_number;

    #[test]
    fn pan_number_accepts_standard_shape() {
        assert_eq!(
            normalize_pan_number("abcde1234f").as_deref(),
            Ok("ABCDE1234F")
        );
        assert!(normalize_pan_number("ABCD1234F").is_err());
    }
}
