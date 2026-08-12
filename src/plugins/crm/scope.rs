use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QuerySelect,
    RelationTrait, Select, sea_query::Expr,
};

use crate::plugins::users::state::AuthContext;

use super::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
    converted_lead::{self, Entity as ConvertedLeadEntity},
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

pub fn apply_lead_filters(
    mut query: Select<LeadEntity>,
    company_id: Option<i64>,
    contact: Option<&str>,
) -> Select<LeadEntity> {
    let company_id = company_id.filter(|id| *id > 0);
    let contact = contact.filter(|s| !s.is_empty());
    if company_id.is_some() || contact.is_some() {
        query = query.join(JoinType::InnerJoin, lead::Relation::Contact.def());
    }
    if let Some(cid) = company_id {
        query = query.filter(contact::Column::CompanyId.eq(cid));
    }
    if let Some(n) = contact {
        query = query.filter(
            Condition::any()
                .add(contact::Column::FirstName.contains(n))
                .add(contact::Column::LastName.contains(n))
                .add(contact::Column::Email.contains(n)),
        );
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

/// Resolved contact/company fields for lead list and detail views.
#[derive(Clone, Debug, Default)]
pub struct LeadContactView {
    pub display_name: String,
    pub company: String,
    pub email: String,
    pub contact_id: i64,
    pub company_id: i64,
}

pub async fn lead_contact_view(db: &DatabaseConnection, contact_id: i64) -> LeadContactView {
    if contact_id <= 0 {
        return LeadContactView::default();
    }
    let Some(contact) = ContactEntity::find_by_id(contact_id)
        .one(db)
        .await
        .ok()
        .flatten()
    else {
        return LeadContactView {
            display_name: format!("Contact #{contact_id}"),
            contact_id,
            ..Default::default()
        };
    };
    let company = CompanyEntity::find_by_id(contact.company_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_default();
    let person = contact.display_name();
    let display_name = if company.is_empty() {
        person.clone()
    } else {
        format!("{person} ({company})")
    };
    LeadContactView {
        display_name,
        company,
        email: contact.email.unwrap_or_default(),
        contact_id: contact.id,
        company_id: contact.company_id,
    }
}

pub async fn lead_display_name(db: &DatabaseConnection, lead: &lead::Model) -> String {
    let view = lead_contact_view(db, lead.contact_id).await;
    if view.display_name.is_empty() {
        format!("Lead #{}", lead.id)
    } else {
        view.display_name
    }
}
