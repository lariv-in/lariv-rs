use chrono::NaiveDate;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Select,
    sea_query::{Expr, Query as SeaQuery, SelectStatement},
};

use crate::plugins::contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use crate::plugins::users::{entities::user::Entity as UserEntity, state::AuthContext};

use super::entities::{
    converted_lead::{self, Entity as ConvertedLeadEntity},
    failed_lead::{self, Entity as FailedLeadEntity},
    lead::{self, Entity as LeadEntity},
    lead_tag::{self, Entity as LeadTagEntity},
    lead_tag_link,
    lead_update::{self, Entity as LeadUpdateEntity},
};

pub fn sql_lead_not_converted() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust("NOT EXISTS (SELECT 1 FROM crm_converted_leads c WHERE c.lead_id = crm_leads.id)")
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
    crate::web::opt_or_log(
        scope_superuser(LeadEntity::find_by_id(id), auth)
            .filter(sql_lead_active())
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead::Model> {
    crate::web::opt_or_log(
        scope_superuser(LeadEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_lead_update_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead_update::Model> {
    crate::web::opt_or_log(
        scope_superuser(LeadUpdateEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_converted_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<converted_lead::Model> {
    crate::web::opt_or_log(
        scope_superuser(ConvertedLeadEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_failed_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<failed_lead::Model> {
    crate::web::opt_or_log(
        scope_superuser(FailedLeadEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_lead_tag_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead_tag::Model> {
    crate::web::opt_or_log(
        scope_superuser(LeadTagEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub fn apply_lead_filters(
    mut query: Select<LeadEntity>,
    company_id: Option<i64>,
    contact: Option<&str>,
    tag_ids: &[i64],
    sort: Option<&str>,
) -> Select<LeadEntity> {
    let company_id = company_id.filter(|id| *id > 0);
    let contact = contact.filter(|s| !s.is_empty());
    let sort_col = sort_key(sort.unwrap_or(""));
    let need_contact = company_id.is_some()
        || contact.is_some()
        || sort_col.eq_ignore_ascii_case("Name")
        || sort_col.eq_ignore_ascii_case("Email")
        || sort_col.eq_ignore_ascii_case("Company");
    let need_company = sort_col.eq_ignore_ascii_case("Company");
    if need_contact {
        query = query.join(JoinType::LeftJoin, lead::Relation::Contact.def());
    }
    if need_company {
        query = query.join(JoinType::LeftJoin, contact::Relation::Company.def());
    }
    if let Some(cid) = company_id {
        query = query.filter(contact::Column::CompanyId.eq(cid));
    }
    if let Some(n) = contact {
        query = query.filter(
            Condition::any()
                .add(contact::Column::Name.contains(n))
                .add(contact::Column::Email.contains(n)),
        );
    }
    apply_lead_tag_id_filter(query, lead::Column::Id, tag_ids)
}

/// Leads that have any of the selected tags.
pub fn apply_lead_tag_id_filter<E, C>(
    query: Select<E>,
    lead_id_col: C,
    tag_ids: &[i64],
) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait,
{
    let tag_ids: Vec<i64> = tag_ids.iter().copied().filter(|id| *id > 0).collect();
    if tag_ids.is_empty() {
        return query;
    }
    query.filter(lead_id_col.in_subquery(lead_ids_with_tags_subquery(&tag_ids)))
}

fn lead_ids_with_tags_subquery(tag_ids: &[i64]) -> SelectStatement {
    let mut sub = SeaQuery::select();
    sub.column(lead_tag_link::Column::LeadId)
        .from(lead_tag_link::Entity)
        .and_where(lead_tag_link::Column::LeadTagId.is_in(tag_ids.to_vec()));
    sub
}

fn sort_key(sort: &str) -> &str {
    sort.trim().split_whitespace().next().unwrap_or("")
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
}

pub fn apply_lead_sort(query: Select<LeadEntity>, sort: Option<&str>) -> Select<LeadEntity> {
    let sort = sort.unwrap_or("").trim();
    let desc = sort_desc(sort);
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Name") => {
            if desc {
                query.order_by_desc(contact::Column::Name)
            } else {
                query.order_by_asc(contact::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        s if s.eq_ignore_ascii_case("Source") => {
            if desc {
                query.order_by_desc(lead::Column::Source)
            } else {
                query.order_by_asc(lead::Column::Source)
            }
        }
        _ => query.order_by_desc(lead::Column::Id),
    }
}

pub fn apply_converted_lead_sort(
    mut query: Select<ConvertedLeadEntity>,
    sort: Option<&str>,
) -> Select<ConvertedLeadEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    if key.eq_ignore_ascii_case("Name") || key.eq_ignore_ascii_case("Email") {
        query = query.join(JoinType::LeftJoin, converted_lead::Relation::Contact.def());
    } else if key.eq_ignore_ascii_case("Company") {
        query = query.join(JoinType::LeftJoin, converted_lead::Relation::Company.def());
    } else if key.eq_ignore_ascii_case("Source") {
        query = query.join(JoinType::LeftJoin, converted_lead::Relation::Lead.def());
    }
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Name") => {
            if desc {
                query.order_by_desc(contact::Column::Name)
            } else {
                query.order_by_asc(contact::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        s if s.eq_ignore_ascii_case("Source") => {
            if desc {
                query.order_by_desc(lead::Column::Source)
            } else {
                query.order_by_asc(lead::Column::Source)
            }
        }
        _ => query.order_by_desc(converted_lead::Column::Id),
    }
}

pub fn apply_failed_lead_sort(
    mut query: Select<FailedLeadEntity>,
    sort: Option<&str>,
) -> Select<FailedLeadEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    let need_lead = key.eq_ignore_ascii_case("Name")
        || key.eq_ignore_ascii_case("Email")
        || key.eq_ignore_ascii_case("Company")
        || key.eq_ignore_ascii_case("Source");
    let need_contact = key.eq_ignore_ascii_case("Name")
        || key.eq_ignore_ascii_case("Email")
        || key.eq_ignore_ascii_case("Company");
    let need_company = key.eq_ignore_ascii_case("Company");
    if need_lead {
        query = query.join(JoinType::LeftJoin, failed_lead::Relation::Lead.def());
    }
    if need_contact {
        query = query.join(JoinType::LeftJoin, lead::Relation::Contact.def());
    }
    if need_company {
        query = query.join(JoinType::LeftJoin, contact::Relation::Company.def());
    }
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Name") => {
            if desc {
                query.order_by_desc(contact::Column::Name)
            } else {
                query.order_by_asc(contact::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        s if s.eq_ignore_ascii_case("Source") => {
            if desc {
                query.order_by_desc(lead::Column::Source)
            } else {
                query.order_by_asc(lead::Column::Source)
            }
        }
        _ => query.order_by_desc(failed_lead::Column::Id),
    }
}

pub fn format_due_date(due_date: Option<NaiveDate>) -> String {
    due_date
        .map(crate::datetime::format_date)
        .unwrap_or_default()
}

pub async fn user_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    crate::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id").is_some()
}

pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .map(|u| u.name)
        .unwrap_or_default()
}

pub use crate::plugins::contacts::scope::company_display_label;

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
    let Some(contact) = crate::web::opt_or_log(
        ContactEntity::find_by_id(contact_id).one(db).await,
        "find by id",
    ) else {
        return LeadContactView {
            display_name: format!("Contact #{contact_id}"),
            contact_id,
            ..Default::default()
        };
    };
    let company = match contact.company_id {
        Some(id) => {
            crate::web::opt_or_log(CompanyEntity::find_by_id(id).one(db).await, "find by id")
                .map(|c| c.name)
                .unwrap_or_default()
        }
        None => String::new(),
    };
    LeadContactView {
        display_name: contact.display_name(),
        company,
        email: contact.email.unwrap_or_default(),
        contact_id: contact.id,
        company_id: contact.company_id.unwrap_or(0),
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
