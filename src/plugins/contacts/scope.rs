use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Select, sea_query::Expr,
};

use crate::plugins::users::state::AuthContext;

use super::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};

pub fn scope_superuser<T>(query: Select<T>, auth: &AuthContext) -> Select<T>
where
    T: EntityTrait,
{
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub fn scope_contacts(query: Select<ContactEntity>, auth: &AuthContext) -> Select<ContactEntity> {
    scope_superuser(query, auth)
}

pub async fn find_contact_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<contact::Model> {
    crate::web::opt_or_log(
        scope_contacts(ContactEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find contact scoped",
    )
}

pub async fn find_company_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<company::Model> {
    crate::web::opt_or_log(
        scope_superuser(CompanyEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find company scoped",
    )
}

fn sort_key(sort: &str) -> &str {
    sort.trim().split_whitespace().next().unwrap_or("")
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
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
        _ => query.order_by_desc(contact::Column::Id),
    }
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
        _ => query.order_by_desc(company::Column::Id),
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

pub async fn contact_belongs_to_company(
    db: &DatabaseConnection,
    contact_id: i64,
    company_id: i64,
) -> bool {
    crate::web::opt_or_log(
        ContactEntity::find_by_id(contact_id)
            .filter(contact::Column::CompanyId.eq(company_id))
            .one(db)
            .await,
        "find by id",
    )
    .is_some()
}

pub async fn contact_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(ContactEntity::find_by_id(id).one(db).await, "find by id")
        .map(|c| c.display_name())
        .unwrap_or_default()
}

pub async fn company_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(CompanyEntity::find_by_id(id).one(db).await, "find by id")
        .map(|c| c.name)
        .unwrap_or_default()
}
