//! Find, create, and list `customers` rows.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, JoinType, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    Select,
    sea_query::{
        Expr, ExprTrait, Func, Order, extension::postgres::PgExpr,
    },
};

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};

use crate::entities::customer::{self, Entity as CustomerEntity};

#[derive(Debug, Clone, FromQueryResult)]
struct CustomerSqlRow {
    id: i64,
    name: String,
    is_individual: bool,
    company_id: Option<i64>,
    individual_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ListedCustomer {
    pub id: i64,
    pub name: String,
    pub kind: &'static str,
    pub href: String,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct NamedCustomer {
    pub id: i64,
    pub name: String,
}

/// Display name: contact or company name, else `#<customer id>`.
fn customer_name_expr() -> Expr {
    Expr::case(
        Expr::col((CustomerEntity, customer::Column::IsIndividual)).eq(true),
        party_display_name(Expr::col((ContactEntity, contact::Column::Name))),
    )
    .finally(party_display_name(Expr::col((
        CompanyEntity,
        company::Column::Name,
    ))))
    .into()
}

fn party_display_name(name: Expr) -> Expr {
    Expr::case(name.clone().eq("").or(name.clone().is_null()), fallback_id_label())
        .finally(name)
        .into()
}

fn fallback_id_label() -> Expr {
    Expr::val("#").concatenate(
        Expr::col((CustomerEntity, customer::Column::Id)).cast_as("text"),
    )
}

fn customers_with_parties() -> Select<CustomerEntity> {
    CustomerEntity::find()
        .join(JoinType::LeftJoin, customer::Relation::Individual.def())
        .join(JoinType::LeftJoin, customer::Relation::Company.def())
}

fn apply_name_search(query: Select<CustomerEntity>, needle: &str, like: &str) -> Select<CustomerEntity> {
    if needle.is_empty() {
        query
    } else {
        query.filter(Func::lower(customer_name_expr()).like(like))
    }
}

/// `customer_id` must already point at `customers`.
pub async fn require_existing<C: ConnectionTrait>(db: &C, customer_id: i64) -> Result<i64, String> {
    if customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    match CustomerEntity::find_by_id(customer_id).one(db).await {
        Ok(Some(_)) => Ok(customer_id),
        Ok(None) => Err("select a customer".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

/// Exactly one of `company_id` and `individual_id` is positive.
pub async fn create_for_party<C: ConnectionTrait>(
    db: &C,
    company_id: i64,
    individual_id: i64,
) -> Result<i64, String> {
    let company = company_id > 0;
    let individual = individual_id > 0;
    if company == individual {
        return Err("select a company or a contact".to_string());
    }
    if individual {
        find_or_insert(db, true, None, Some(individual_id)).await
    } else {
        find_or_insert(db, false, Some(company_id), None).await
    }
}

pub fn detail_href(row: &customer::Model) -> String {
    party_href(row.is_individual, row.company_id, row.individual_id)
}

pub async fn display_name(db: &DatabaseConnection, customer_id: i64) -> String {
    if customer_id <= 0 {
        return String::new();
    }
    let Ok(Some(row)) = CustomerEntity::find_by_id(customer_id).one(db).await else {
        return String::new();
    };
    party_name(db, &row).await
}

pub async fn list_customers(
    db: &DatabaseConnection,
    needle: &str,
    sort: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<ListedCustomer>, u64), String> {
    let needle = needle.trim().to_string();
    let like = format!("%{}%", needle.to_lowercase().replace('%', "\\%"));
    let name_order = if sort.to_ascii_uppercase().as_str().contains("DESC") {
        Order::Desc
    } else {
        Order::Asc
    };
    let total = count_matching(db, &needle, &like).await?;
    let listed = apply_name_search(customers_with_parties(), &needle, &like)
        .select_only()
        .column(customer::Column::Id)
        .column_as(customer_name_expr(), "name")
        .column(customer::Column::IsIndividual)
        .column(customer::Column::CompanyId)
        .column(customer::Column::IndividualId)
        .order_by(customer_name_expr(), name_order)
        .order_by_asc(customer::Column::Id)
        .limit(limit)
        .offset(offset)
        .into_model::<CustomerSqlRow>()
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let rows = listed
        .into_iter()
        .map(|row| ListedCustomer {
            id: row.id,
            kind: if row.is_individual {
                "Contact"
            } else {
                "Company"
            },
            href: party_href(row.is_individual, row.company_id, row.individual_id),
            name: row.name,
        })
        .collect();
    Ok((rows, total))
}

pub async fn search(
    db: &DatabaseConnection,
    needle: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<NamedCustomer>, u64), String> {
    let needle = needle.trim().to_string();
    let like = format!("%{}%", needle.to_lowercase().replace('%', "\\%"));
    let total = count_matching(db, &needle, &like).await?;
    let listed = apply_name_search(customers_with_parties(), &needle, &like)
        .select_only()
        .column(customer::Column::Id)
        .column_as(customer_name_expr(), "name")
        .order_by_asc(customer_name_expr())
        .order_by_asc(customer::Column::Id)
        .limit(limit)
        .offset(offset)
        .into_model::<NamedCustomer>()
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok((listed, total))
}

fn party_href(is_individual: bool, company_id: Option<i64>, individual_id: Option<i64>) -> String {
    if is_individual {
        individual_id
            .filter(|id| *id > 0)
            .map(|id| ContactDetailRouteTag::new(id).url())
            .unwrap_or_default()
    } else {
        company_id
            .filter(|id| *id > 0)
            .map(|id| CompanyDetailRouteTag::new(id).url())
            .unwrap_or_default()
    }
}

async fn party_name(db: &DatabaseConnection, row: &customer::Model) -> String {
    if row.is_individual {
        let Some(id) = row.individual_id.filter(|id| *id > 0) else {
            return format!("#{}", row.id);
        };
        ContactEntity::find_by_id(id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|contact| contact.name)
            .unwrap_or_else(|| format!("#{id}"))
    } else {
        let Some(id) = row.company_id.filter(|id| *id > 0) else {
            return format!("#{}", row.id);
        };
        CompanyEntity::find_by_id(id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|company| company.name)
            .unwrap_or_else(|| format!("#{id}"))
    }
}

async fn count_matching(db: &DatabaseConnection, needle: &str, like: &str) -> Result<u64, String> {
    apply_name_search(customers_with_parties(), needle, like)
        .count(db)
        .await
        .map_err(|err| err.to_string())
}

async fn find_or_insert<C: ConnectionTrait>(
    db: &C,
    is_individual: bool,
    company_id: Option<i64>,
    individual_id: Option<i64>,
) -> Result<i64, String> {
    if let Some(existing) = find_customer(db, is_individual, company_id, individual_id).await? {
        return Ok(existing);
    }
    let now = chrono::Utc::now();
    let inserted = customer::ActiveModel {
        is_individual: sea_orm::ActiveValue::Set(is_individual),
        company_id: sea_orm::ActiveValue::Set(company_id),
        individual_id: sea_orm::ActiveValue::Set(individual_id),
        created_at: sea_orm::ActiveValue::Set(Some(now)),
        updated_at: sea_orm::ActiveValue::Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await;
    match inserted {
        Ok(row) => Ok(row.id),
        Err(err) => find_customer(db, is_individual, company_id, individual_id)
            .await?
            .ok_or_else(|| err.to_string()),
    }
}

async fn find_customer<C: ConnectionTrait>(
    db: &C,
    is_individual: bool,
    company_id: Option<i64>,
    individual_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let mut query = CustomerEntity::find().filter(customer::Column::IsIndividual.eq(is_individual));
    if let Some(company_id) = company_id {
        query = query.filter(customer::Column::CompanyId.eq(company_id));
    }
    if let Some(individual_id) = individual_id {
        query = query.filter(customer::Column::IndividualId.eq(individual_id));
    }
    query
        .one(db)
        .await
        .map(|row| row.map(|row| row.id))
        .map_err(|err| err.to_string())
}
