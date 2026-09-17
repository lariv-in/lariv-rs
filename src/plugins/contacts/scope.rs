use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Select, sea_query::Expr,
};

use crate::plugins::crm::entities::company;
use crate::plugins::users::state::AuthContext;

use super::entities::contact::{self, Entity as ContactEntity};

pub fn scope_contacts(query: Select<ContactEntity>, auth: &AuthContext) -> Select<ContactEntity> {
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
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
