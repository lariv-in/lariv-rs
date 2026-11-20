//! Company writes used by HTTP handlers and assistant tools.

use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

use crate::entities::company::{self, Entity as CompanyEntity};
use crate::entities::company_membership::{self, Entity as MembershipEntity};
use crate::entities::company_user_pref::{self, Entity as PrefEntity};

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

pub async fn create_company<C: ConnectionTrait>(
    db: &C,
    input: CompanyWrite,
    owner_user_id: i64,
) -> Result<company::Model, String> {
    if owner_user_id <= 0 {
        return Err("create_company requires a signed-in user".to_string());
    }
    let name = require_name(&input.name)?;
    let now = chrono::Utc::now();
    let company = company::ActiveModel {
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
    .map_err(|e| e.to_string())?;
    add_membership(db, company.id, owner_user_id, true).await?;
    Ok(company)
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
    crate::scope::visible_companies(CompanyEntity::find_by_id(company_id))
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
    let mut select = crate::scope::visible_companies(CompanyEntity::find());
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

pub async fn add_membership<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
    user_id: i64,
    owned_by: bool,
) -> Result<company_membership::Model, String> {
    if company_id <= 0 || user_id <= 0 {
        return Err("membership requires a company and a user".to_string());
    }
    let now = chrono::Utc::now();
    company_membership::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        company_id: Set(company_id),
        owned_by: Set(owned_by),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn remove_membership<C: ConnectionTrait>(
    db: &C,
    membership_id: i64,
) -> Result<(), String> {
    if membership_id <= 0 {
        return Err("remove_membership requires a positive id".to_string());
    }
    MembershipEntity::delete_by_id(membership_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn set_membership_owned_by<C: ConnectionTrait>(
    db: &C,
    membership_id: i64,
    owned_by: bool,
) -> Result<company_membership::Model, String> {
    let existing = MembershipEntity::find_by_id(membership_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("membership {membership_id} not found"))?;
    let now = chrono::Utc::now();
    let mut am: company_membership::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.owned_by = Set(owned_by);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn membership_for_user<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
    user_id: i64,
) -> Result<Option<company_membership::Model>, String> {
    MembershipEntity::find()
        .filter(company_membership::Column::CompanyId.eq(company_id))
        .filter(company_membership::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn user_owns_company<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
    user_id: i64,
) -> Result<bool, String> {
    Ok(membership_for_user(db, company_id, user_id)
        .await?
        .is_some_and(|m| m.owned_by))
}

pub async fn list_memberships_for_company<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
) -> Result<Vec<company_membership::Model>, String> {
    MembershipEntity::find()
        .filter(company_membership::Column::CompanyId.eq(company_id))
        .order_by_asc(company_membership::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_memberships_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<company_membership::Model>, String> {
    MembershipEntity::find()
        .filter(company_membership::Column::UserId.eq(user_id))
        .order_by_desc(company_membership::Column::OwnedBy)
        .order_by_asc(company_membership::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

/// Remember which company this user acts as. RLS reads this when stamping
/// `app.company_id` on checkout. Pass `None` for superuser "no company" mode.
pub async fn set_active_company<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    company_id: Option<i64>,
) -> Result<(), String> {
    if let Some(company_id) = company_id {
        if membership_for_user(db, company_id, user_id)
            .await?
            .is_none()
        {
            return Err("not a member of that company".to_string());
        }
    }
    let now = chrono::Utc::now();
    match PrefEntity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(existing) => {
            let mut am: company_user_pref::ActiveModel = existing.into();
            am.active_company_id = Set(company_id);
            am.updated_at = Set(Some(now));
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        None => {
            company_user_pref::ActiveModel {
                user_id: Set(user_id),
                active_company_id: Set(company_id),
                updated_at: Set(Some(now)),
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub async fn active_company_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Option<i64>, String> {
    let prefs = PrefEntity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    // Explicit prefs row with NULL means "no company" — do not fall back.
    if let Some(row) = &prefs {
        if row.active_company_id.is_none() {
            return Ok(None);
        }
    }
    let preferred = prefs.and_then(|row| row.active_company_id);
    let memberships = list_memberships_for_user(db, user_id).await?;
    if memberships.is_empty() {
        return Ok(None);
    }
    if let Some(preferred) = preferred {
        if memberships.iter().any(|m| m.company_id == preferred) {
            return Ok(Some(preferred));
        }
    }
    if let Some(owned) = memberships.iter().find(|m| m.owned_by) {
        return Ok(Some(owned.company_id));
    }
    Ok(memberships.first().map(|m| m.company_id))
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
