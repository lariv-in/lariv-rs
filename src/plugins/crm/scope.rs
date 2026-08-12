use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Select, sea_query::Expr,
};

use crate::plugins::users::state::AuthContext;

use super::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
    converted_lead::{self, Entity as ConvertedLeadEntity},
    deal::{self, Entity as DealEntity},
    failed_lead::{self, Entity as FailedLeadEntity},
    lead::{self, Entity as LeadEntity},
};

pub fn sql_lead_not_converted() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM crm_converted_leads c WHERE c.lead_id = crm_leads.id)",
    )
}

pub fn sql_lead_not_failed() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust("NOT EXISTS (SELECT 1 FROM crm_failed_leads f WHERE f.lead_id = crm_leads.id)")
}

pub fn sql_lead_active() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust(
        "NOT EXISTS (SELECT 1 FROM crm_converted_leads c WHERE c.lead_id = crm_leads.id) \
         AND NOT EXISTS (SELECT 1 FROM crm_failed_leads f WHERE f.lead_id = crm_leads.id)",
    )
}

pub fn scope_superuser<T>(query: Select<T>, auth: &AuthContext) -> Select<T>
where
    T: EntityTrait,
{
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub async fn find_active_lead(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead::Model> {
    scope_superuser(LeadEntity::find_by_id(id), auth)
        .filter(sql_lead_active())
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead::Model> {
    scope_superuser(LeadEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_converted_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<converted_lead::Model> {
    scope_superuser(ConvertedLeadEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_failed_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<failed_lead::Model> {
    scope_superuser(FailedLeadEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_company_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<company::Model> {
    scope_superuser(CompanyEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_contact_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<contact::Model> {
    scope_superuser(ContactEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_deal_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<deal::Model> {
    scope_superuser(DealEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub fn apply_lead_filters(
    mut query: Select<LeadEntity>,
    company: Option<&str>,
    email: Option<&str>,
) -> Select<LeadEntity> {
    if let Some(c) = company.filter(|s| !s.is_empty()) {
        query = query.filter(lead::Column::CompanyName.contains(c));
    }
    if let Some(e) = email.filter(|s| !s.is_empty()) {
        query = query.filter(lead::Column::Email.contains(e));
    }
    query
}

pub fn apply_company_filters(
    mut query: Select<CompanyEntity>,
    name: Option<&str>,
) -> Select<CompanyEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(company::Column::Name.contains(n));
    }
    query
}

pub fn apply_contact_filters(
    mut query: Select<ContactEntity>,
    company_id: Option<i64>,
    name: Option<&str>,
) -> Select<ContactEntity> {
    if let Some(cid) = company_id.filter(|id| *id > 0) {
        query = query.filter(contact::Column::CompanyId.eq(cid));
    }
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(contact::Column::FirstName.contains(n));
    }
    query
}

pub fn apply_deal_filters(
    mut query: Select<DealEntity>,
    company_id: Option<i64>,
    name: Option<&str>,
) -> Select<DealEntity> {
    if let Some(cid) = company_id.filter(|id| *id > 0) {
        query = query.filter(deal::Column::CompanyId.eq(cid));
    }
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(deal::Column::Name.contains(n));
    }
    query
}

pub async fn contact_belongs_to_company(
    db: &DatabaseConnection,
    contact_id: i64,
    company_id: i64,
) -> bool {
    ContactEntity::find_by_id(contact_id)
        .filter(contact::Column::CompanyId.eq(company_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
}

pub async fn company_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    CompanyEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_default()
}

pub async fn contact_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    ContactEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.display_name())
        .unwrap_or_default()
}
