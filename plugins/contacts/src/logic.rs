//! Shared contact and company writes used by HTTP handlers and assistant tools.

use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};

pub struct ContactWrite {
    pub company_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_primary: bool,
}

pub struct CompanyWrite {
    pub name: String,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub pincode: Option<String>,
    pub state: Option<String>,
    pub website: Option<String>,
    pub gstin: Option<String>,
    pub cin: Option<String>,
    pub pan: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

pub fn require_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("name is required".to_string())
    } else {
        Ok(name.to_string())
    }
}

pub async fn ensure_company<C: ConnectionTrait>(
    db: &C,
    company_id: Option<i64>,
) -> Result<(), String> {
    let Some(id) = company_id else {
        return Ok(());
    };
    let found = CompanyEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err("company not found".to_string());
    }
    Ok(())
}

pub async fn create_contact<C: ConnectionTrait>(
    db: &C,
    input: ContactWrite,
) -> Result<contact::Model, String> {
    let name = require_name(&input.name)?;
    ensure_company(db, input.company_id).await?;
    let now = chrono::Utc::now();
    contact::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_id: Set(input.company_id),
        name: Set(name),
        email: Set(input.email),
        phone: Set(input.phone),
        is_primary: Set(input.is_primary),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_contact<C: ConnectionTrait>(
    db: &C,
    contact_id: i64,
    input: ContactWrite,
) -> Result<contact::Model, String> {
    let name = require_name(&input.name)?;
    ensure_company(db, input.company_id).await?;
    let existing = ContactEntity::find_by_id(contact_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("contact {contact_id} not found"))?;
    let now = chrono::Utc::now();
    let mut am: contact::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.company_id = Set(input.company_id);
    am.name = Set(name);
    am.email = Set(input.email);
    am.phone = Set(input.phone);
    am.is_primary = Set(input.is_primary);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_contact<C: ConnectionTrait>(db: &C, contact_id: i64) -> Result<(), String> {
    if contact_id <= 0 {
        return Err("delete_contact requires a positive contact id".to_string());
    }
    let found = ContactEntity::find_by_id(contact_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("contact {contact_id} not found"));
    }
    ContactEntity::delete_by_id(contact_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_contact<C: ConnectionTrait>(
    db: &C,
    contact_id: i64,
) -> Result<contact::Model, String> {
    if contact_id <= 0 {
        return Err("get_contact requires a positive contact id".to_string());
    }
    ContactEntity::find_by_id(contact_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("contact {contact_id} not found"))
}

pub async fn search_contacts<C: ConnectionTrait>(
    db: &C,
    query: &str,
    limit: u64,
) -> Result<Vec<contact::Model>, String> {
    let mut select = ContactEntity::find();
    let needle = query.trim();
    if !needle.is_empty() {
        let like = like_pattern(needle);
        select = select.filter(
            Condition::any()
                .add(Func::lower(Expr::col(contact::Column::Name)).like(like.clone()))
                .add(Func::lower(Expr::col(contact::Column::Email)).like(like.clone()))
                .add(Func::lower(Expr::col(contact::Column::Phone)).like(like)),
        );
    }
    select
        .order_by_desc(contact::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_company<C: ConnectionTrait>(
    db: &C,
    input: CompanyWrite,
) -> Result<company::Model, String> {
    let name = require_name(&input.name)?;
    let now = chrono::Utc::now();
    company::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        address_line_1: Set(input.address_line_1),
        address_line_2: Set(input.address_line_2),
        city: Set(input.city),
        pincode: Set(input.pincode),
        state: Set(input.state),
        website: Set(input.website),
        gstin: Set(input.gstin),
        cin: Set(input.cin),
        pan: Set(input.pan),
        phone: Set(input.phone),
        email: Set(input.email),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_company<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
    input: CompanyWrite,
) -> Result<company::Model, String> {
    let name = require_name(&input.name)?;
    let existing = CompanyEntity::find_by_id(company_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("company {company_id} not found"))?;
    let now = chrono::Utc::now();
    let mut am: company::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.address_line_1 = Set(input.address_line_1);
    am.address_line_2 = Set(input.address_line_2);
    am.city = Set(input.city);
    am.pincode = Set(input.pincode);
    am.state = Set(input.state);
    am.website = Set(input.website);
    am.gstin = Set(input.gstin);
    am.cin = Set(input.cin);
    am.pan = Set(input.pan);
    am.phone = Set(input.phone);
    am.email = Set(input.email);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_company<C: ConnectionTrait>(db: &C, company_id: i64) -> Result<(), String> {
    if company_id <= 0 {
        return Err("delete_company requires a positive company id".to_string());
    }
    let found = CompanyEntity::find_by_id(company_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("company {company_id} not found"));
    }
    CompanyEntity::delete_by_id(company_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_company<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
) -> Result<company::Model, String> {
    if company_id <= 0 {
        return Err("get_company requires a positive company id".to_string());
    }
    CompanyEntity::find_by_id(company_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("company {company_id} not found"))
}

pub async fn search_companies<C: ConnectionTrait>(
    db: &C,
    query: &str,
    limit: u64,
) -> Result<Vec<company::Model>, String> {
    let mut select = CompanyEntity::find();
    let needle = query.trim();
    if !needle.is_empty() {
        let like = like_pattern(needle);
        select = select.filter(
            Condition::any()
                .add(Func::lower(Expr::col(company::Column::Name)).like(like.clone()))
                .add(Func::lower(Expr::col(company::Column::Email)).like(like.clone()))
                .add(Func::lower(Expr::col(company::Column::City)).like(like)),
        );
    }
    select
        .order_by_desc(company::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

fn like_pattern(needle: &str) -> String {
    format!(
        "%{}%",
        needle
            .to_lowercase()
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}
