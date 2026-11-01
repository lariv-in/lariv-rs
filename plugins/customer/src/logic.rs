//! Find, create, and list `customers` rows.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, QueryFilter, Statement,
};

use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};
use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};

use crate::entities::customer::{self, Entity as CustomerEntity};

const NAME_EXPR: &str = "CASE WHEN c.is_individual \
     THEN COALESCE(NULLIF(ct.name, ''), '#' || CAST(c.id AS TEXT)) \
     ELSE COALESCE(NULLIF(co.name, ''), '#' || CAST(c.id AS TEXT)) END";

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
    let direction = if sort.to_ascii_uppercase().contains("DESC") {
        "DESC"
    } else {
        "ASC"
    };
    let backend = db.get_database_backend();
    let listed = CustomerSqlRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        format!(
            "SELECT c.id, {NAME_EXPR} AS name, c.is_individual, c.company_id, c.individual_id \
             FROM customers AS c \
             LEFT JOIN crm_contacts AS ct ON ct.id = c.individual_id \
             LEFT JOIN crm_companies AS co ON co.id = c.company_id \
             WHERE ($1 = '' OR lower({NAME_EXPR}) LIKE $2) \
             ORDER BY 2 {direction}, c.id \
             LIMIT $3 OFFSET $4"
        ),
        [
            needle.clone().into(),
            like.clone().into(),
            (limit as i64).into(),
            (offset as i64).into(),
        ],
    ))
    .all(db)
    .await
    .map_err(|err| err.to_string())?;
    let total = count_matching(db, &needle, &like).await?;
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
    let listed = NamedCustomer::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        format!(
            "SELECT c.id, {NAME_EXPR} AS name \
             FROM customers AS c \
             LEFT JOIN crm_contacts AS ct ON ct.id = c.individual_id \
             LEFT JOIN crm_companies AS co ON co.id = c.company_id \
             WHERE ($1 = '' OR lower({NAME_EXPR}) LIKE $2) \
             ORDER BY 2, c.id \
             LIMIT $3 OFFSET $4"
        ),
        [
            needle.clone().into(),
            like.clone().into(),
            (limit as i64).into(),
            (offset as i64).into(),
        ],
    ))
    .all(db)
    .await
    .map_err(|err| err.to_string())?;
    let total = count_matching(db, &needle, &like).await?;
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
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT COUNT(*) AS total \
                 FROM customers AS c \
                 LEFT JOIN crm_contacts AS ct ON ct.id = c.individual_id \
                 LEFT JOIN crm_companies AS co ON co.id = c.company_id \
                 WHERE ($1 = '' OR lower({NAME_EXPR}) LIKE $2)"
            ),
            [needle.to_string().into(), like.to_string().into()],
        ))
        .await
        .map_err(|err| err.to_string())?;
    let Some(row) = row else {
        return Ok(0);
    };
    let total: i64 = row.try_get("", "total").map_err(|err| err.to_string())?;
    Ok(u64::try_from(total.max(0)).unwrap_or(0))
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
