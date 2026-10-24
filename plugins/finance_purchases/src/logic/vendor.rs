//! Find or create a vendor for one company or one contact.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, FromQueryResult, QueryFilter, Statement,
};

use crate::entities::vendor::{self, Entity as VendorEntity};
use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};

/// `vendor_id` must already point at `purchase_vendors`.
pub async fn require_existing<C: ConnectionTrait>(db: &C, vendor_id: i64) -> Result<i64, String> {
    if vendor_id <= 0 {
        return Err("select a vendor".to_string());
    }
    match VendorEntity::find_by_id(vendor_id).one(db).await {
        Ok(Some(_)) => Ok(vendor_id),
        Ok(None) => Err("select a vendor".to_string()),
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

pub async fn display_name(db: &DatabaseConnection, vendor_id: i64) -> String {
    if vendor_id <= 0 {
        return String::new();
    }
    let Ok(Some(row)) = VendorEntity::find_by_id(vendor_id).one(db).await else {
        return String::new();
    };
    party_name(db, &row).await
}

async fn party_name(db: &DatabaseConnection, row: &vendor::Model) -> String {
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
pub struct NamedVendor {
    pub id: i64,
    pub name: String,
}

const NAME_EXPR: &str = "CASE WHEN v.is_individual \
     THEN COALESCE(NULLIF(ct.name, ''), '#' || v.id::text) \
     ELSE COALESCE(NULLIF(co.name, ''), '#' || v.id::text) END";

pub async fn search(
    db: &DatabaseConnection,
    needle: &str,
    limit: u64,
    offset: u64,
) -> Result<(Vec<NamedVendor>, u64), String> {
    let needle = needle.trim().to_string();
    let like = format!("%{}%", needle.to_lowercase().replace('%', "\\%"));
    let listed = NamedVendor::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        format!(
            "SELECT v.id, {NAME_EXPR} AS name \
             FROM purchase_vendors AS v \
             LEFT JOIN crm_contacts AS ct ON ct.id = v.individual_id \
             LEFT JOIN crm_companies AS co ON co.id = v.company_id \
             WHERE ($1 = '' OR lower({NAME_EXPR}) LIKE $2) \
             ORDER BY 2, v.id \
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
                 FROM purchase_vendors AS v \
                 LEFT JOIN crm_contacts AS ct ON ct.id = v.individual_id \
                 LEFT JOIN crm_companies AS co ON co.id = v.company_id \
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
    if let Some(existing) = find_vendor(db, is_individual, company_id, individual_id).await? {
        return Ok(existing);
    }
    let now = chrono::Utc::now();
    let inserted = vendor::ActiveModel {
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
        Err(err) => find_vendor(db, is_individual, company_id, individual_id)
            .await?
            .ok_or_else(|| err.to_string()),
    }
}

async fn find_vendor<C: ConnectionTrait>(
    db: &C,
    is_individual: bool,
    company_id: Option<i64>,
    individual_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let mut query = VendorEntity::find().filter(vendor::Column::IsIndividual.eq(is_individual));
    if let Some(company_id) = company_id {
        query = query.filter(vendor::Column::CompanyId.eq(company_id));
    }
    if let Some(individual_id) = individual_id {
        query = query.filter(vendor::Column::IndividualId.eq(individual_id));
    }
    query
        .one(db)
        .await
        .map(|row| row.map(|row| row.id))
        .map_err(|err| err.to_string())
}
