use chrono::{NaiveDate, Utc};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Select,
    sea_query::{Expr, Query as SeaQuery, SelectStatement},
};

use crate::datetime::parse_timezone;
use crate::plugins::users::{
    entities::user::{self, Entity as UserEntity},
    state::AuthContext,
};

use super::entities::{
    company::{self, Entity as CompanyEntity},
    completed_task::{self, Entity as CompletedTaskEntity},
    contact::{self, Entity as ContactEntity},
    converted_lead::{self, Entity as ConvertedLeadEntity},
    failed_lead::{self, Entity as FailedLeadEntity},
    lead::{self, Entity as LeadEntity},
    lead_tag::{self, Entity as LeadTagEntity},
    lead_tag_link,
    lead_update::{self, Entity as LeadUpdateEntity},
    task::{self, Entity as TaskEntity},
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

pub async fn find_lead_update_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead_update::Model> {
    scope_superuser(LeadUpdateEntity::find_by_id(id), auth)
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

pub async fn find_lead_tag_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<lead_tag::Model> {
    scope_superuser(LeadTagEntity::find_by_id(id), auth)
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

pub fn sql_task_uncompleted() -> sea_orm::sea_query::SimpleExpr {
    Expr::cust("NOT EXISTS (SELECT 1 FROM crm_completed_tasks c WHERE c.task_id = crm_tasks.id)")
}

pub async fn find_uncompleted_task(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<task::Model> {
    scope_superuser(TaskEntity::find_by_id(id), auth)
        .filter(sql_task_uncompleted())
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_task_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<task::Model> {
    scope_superuser(TaskEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn find_completed_task_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<completed_task::Model> {
    scope_superuser(CompletedTaskEntity::find_by_id(id), auth)
        .one(db)
        .await
        .ok()
        .flatten()
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
        _ => query
            .order_by_desc(lead::Column::CreatedAt)
            .order_by_desc(lead::Column::Id),
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
        _ => query
            .order_by_desc(converted_lead::Column::ConvertedAt)
            .order_by_desc(converted_lead::Column::Id),
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
        _ => query
            .order_by_desc(failed_lead::Column::FailedAt)
            .order_by_desc(failed_lead::Column::Id),
    }
}

pub fn apply_company_sort(
    query: Select<CompanyEntity>,
    sort: Option<&str>,
) -> Select<CompanyEntity> {
    let sort = sort.unwrap_or("").trim();
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Name") => {
            if sort_desc(sort) {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        _ => query
            .order_by_desc(company::Column::CreatedAt)
            .order_by_desc(company::Column::Id),
    }
}

pub fn apply_contact_sort(
    mut query: Select<ContactEntity>,
    sort: Option<&str>,
) -> Select<ContactEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    if key.eq_ignore_ascii_case("Company") {
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
        _ => query
            .order_by_desc(contact::Column::CreatedAt)
            .order_by_desc(contact::Column::Id),
    }
}

pub fn apply_task_sort(
    mut query: Select<TaskEntity>,
    sort: Option<&str>,
    today: NaiveDate,
) -> Select<TaskEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    if key.eq_ignore_ascii_case("AssignedTo") {
        query = query.join(JoinType::LeftJoin, task::Relation::AssignedTo.def());
    }
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Title") => {
            if desc {
                query.order_by_desc(task::Column::Title)
            } else {
                query.order_by_asc(task::Column::Title)
            }
        }
        s if s.eq_ignore_ascii_case("AssignedTo") => {
            if desc {
                query.order_by_desc(user::Column::Name)
            } else {
                query.order_by_asc(user::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("DueDate") => {
            if desc {
                query.order_by_desc(task::Column::DueDate)
            } else {
                query.order_by_asc(task::Column::DueDate)
            }
        }
        s if s.eq_ignore_ascii_case("Status") => {
            let expr = Expr::cust_with_values(
                "(CASE WHEN crm_tasks.due_date IS NOT NULL AND crm_tasks.due_date < ? THEN 0 ELSE 1 END)",
                [today],
            );
            if desc {
                query.order_by_desc(expr)
            } else {
                query.order_by_asc(expr)
            }
        }
        _ => query
            .order_by_desc(task::Column::CreatedAt)
            .order_by_desc(task::Column::Id),
    }
}

pub fn apply_completed_task_filters(
    mut query: Select<CompletedTaskEntity>,
    title: Option<&str>,
    assigned_to_id: Option<i64>,
    sort: Option<&str>,
) -> Select<CompletedTaskEntity> {
    let title = title.filter(|s| !s.is_empty());
    let assigned_to_id = assigned_to_id.filter(|id| *id > 0);
    let sort_col = sort_key(sort.unwrap_or(""));
    let need_task = title.is_some()
        || assigned_to_id.is_some()
        || sort_col.eq_ignore_ascii_case("Title")
        || sort_col.eq_ignore_ascii_case("DueDate")
        || sort_col.eq_ignore_ascii_case("AssignedTo");
    let need_user = sort_col.eq_ignore_ascii_case("AssignedTo");
    if need_task {
        query = query.join(JoinType::LeftJoin, completed_task::Relation::Task.def());
    }
    if need_user {
        query = query.join(JoinType::LeftJoin, task::Relation::AssignedTo.def());
    }
    if let Some(t) = title {
        query = query.filter(task::Column::Title.contains(t));
    }
    if let Some(uid) = assigned_to_id {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
    query
}

pub fn apply_completed_task_sort(
    query: Select<CompletedTaskEntity>,
    sort: Option<&str>,
) -> Select<CompletedTaskEntity> {
    let sort = sort.unwrap_or("").trim();
    let desc = sort_desc(sort);
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Title") => {
            if desc {
                query.order_by_desc(task::Column::Title)
            } else {
                query.order_by_asc(task::Column::Title)
            }
        }
        s if s.eq_ignore_ascii_case("AssignedTo") => {
            if desc {
                query.order_by_desc(user::Column::Name)
            } else {
                query.order_by_asc(user::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("DueDate") => {
            if desc {
                query.order_by_desc(task::Column::DueDate)
            } else {
                query.order_by_asc(task::Column::DueDate)
            }
        }
        s if s.eq_ignore_ascii_case("CompletedAt") => {
            if desc {
                query.order_by_desc(completed_task::Column::CompletedAt)
            } else {
                query.order_by_asc(completed_task::Column::CompletedAt)
            }
        }
        _ => query
            .order_by_desc(completed_task::Column::CompletedAt)
            .order_by_desc(completed_task::Column::Id),
    }
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
        query = query.filter(contact::Column::Name.contains(n));
    }
    query
}

pub fn apply_task_filters(
    mut query: Select<TaskEntity>,
    title: Option<&str>,
    assigned_to_id: Option<i64>,
) -> Select<TaskEntity> {
    if let Some(t) = title.filter(|s| !s.is_empty()) {
        query = query.filter(task::Column::Title.contains(t));
    }
    if let Some(uid) = assigned_to_id.filter(|id| *id > 0) {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
    query
}

pub fn today_in_timezone(tz: &str) -> NaiveDate {
    Utc::now().with_timezone(&parse_timezone(tz)).date_naive()
}

/// Status for an uncompleted task: overdue when due before today.
pub fn open_task_status(due_date: Option<NaiveDate>, today: NaiveDate) -> &'static str {
    if due_date.is_some_and(|d| d < today) {
        "Overdue"
    } else {
        "Not Completed"
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
    UserEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
}

pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    UserEntity::find_by_id(id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|u| u.name)
        .unwrap_or_default()
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
    LeadContactView {
        display_name: contact.display_name(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overdue_when_due_before_today() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        let due = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
        assert_eq!(open_task_status(Some(due), today), "Overdue");
    }

    #[test]
    fn not_completed_when_due_today_or_missing() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 13).unwrap();
        assert_eq!(open_task_status(Some(today), today), "Not Completed");
        assert_eq!(open_task_status(None, today), "Not Completed");
    }
}
