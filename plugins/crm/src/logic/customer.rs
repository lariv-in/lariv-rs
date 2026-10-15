//! Find or create a customer for one company or one contact.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, FromQueryResult, QueryFilter, Statement,
};

use crate::entities::customer::{self, Entity as CustomerEntity};
use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};

/// `customer_id` must already point at `crm_customers`.
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
    if row.is_individual {
        row.individual_id
            .filter(|id| *id > 0)
            .map(|id| lariv_plugin_contacts::routes::ContactDetailRouteTag::new(id).url())
            .unwrap_or_default()
    } else {
        row.company_id
            .filter(|id| *id > 0)
            .map(|id| lariv_plugin_contacts::routes::CompanyDetailRouteTag::new(id).url())
            .unwrap_or_default()
    }
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

#[derive(Debug, Clone, FromQueryResult)]
pub struct NamedCustomer {
    pub id: i64,
    pub name: String,
}

const NAME_EXPR: &str = "CASE WHEN c.is_individual \
     THEN COALESCE(NULLIF(ct.name, ''), '#' || c.id::text) \
     ELSE COALESCE(NULLIF(co.name, ''), '#' || c.id::text) END";

pub async fn search(
    db: &DatabaseConnection,
    needle: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<NamedCustomer>, u64), String> {
    let needle = needle.trim().to_string();
    let like = format!("%{}%", needle.to_lowercase().replace('%', "\\%"));
    let listed = NamedCustomer::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        format!(
            "SELECT c.id, {NAME_EXPR} AS name \
             FROM crm_customers AS c \
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

async fn count_matching(db: &DatabaseConnection, needle: &str, like: &str) -> Result<u64, String> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!(
                "SELECT COUNT(*) AS total \
                 FROM crm_customers AS c \
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
