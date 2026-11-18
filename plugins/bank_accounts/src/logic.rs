//! Bank account writes used by HTTP handlers.

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use crate::entities::bank::{self, Entity as BankEntity};
use crate::entities::bank_account::{self, Entity as BankAccountEntity};

pub struct BankAccountWrite {
    pub name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub micr_code: Option<String>,
}

pub fn require_nonempty(label: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(value.to_string())
    }
}

pub async fn ensure_bank_ifsc<C: ConnectionTrait>(db: &C, ifsc: &str) -> Result<String, String> {
    let ifsc = require_nonempty("IFSC code", ifsc)?;
    let found = BankEntity::find()
        .filter(bank::Column::Ifsc.eq(&ifsc))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("bank IFSC {ifsc} not found"));
    }
    Ok(ifsc)
}

pub async fn ensure_known_micr<C: ConnectionTrait>(
    db: &C,
    micr: Option<String>,
) -> Result<Option<String>, String> {
    let Some(micr) = micr else {
        return Ok(None);
    };
    let micr = micr.trim();
    if micr.is_empty() {
        return Ok(None);
    }
    let found = BankEntity::find()
        .filter(bank::Column::Micr.eq(micr))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("bank MICR {micr} not found"));
    }
    Ok(Some(micr.to_string()))
}

pub async fn create_bank_account<C: ConnectionTrait>(
    db: &C,
    input: BankAccountWrite,
) -> Result<bank_account::Model, String> {
    let name = require_nonempty("name", &input.name)?;
    let account_number = require_nonempty("account number", &input.account_number)?;
    let ifsc_code = ensure_bank_ifsc(db, &input.ifsc_code).await?;
    let micr_code = ensure_known_micr(db, input.micr_code).await?;
    let now = chrono::Utc::now();
    bank_account::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        account_number: Set(account_number),
        ifsc_code: Set(ifsc_code),
        micr_code: Set(micr_code),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_bank_account<C: ConnectionTrait>(
    db: &C,
    account_id: i64,
    input: BankAccountWrite,
) -> Result<bank_account::Model, String> {
    let name = require_nonempty("name", &input.name)?;
    let account_number = require_nonempty("account number", &input.account_number)?;
    let ifsc_code = ensure_bank_ifsc(db, &input.ifsc_code).await?;
    let micr_code = ensure_known_micr(db, input.micr_code).await?;
    let existing = BankAccountEntity::find_by_id(account_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("bank account {account_id} not found"))?;
    let now = chrono::Utc::now();
    let mut am: bank_account::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.account_number = Set(account_number);
    am.ifsc_code = Set(ifsc_code);
    am.micr_code = Set(micr_code);
    am.update(db).await.map_err(|e| e.to_string())
}
