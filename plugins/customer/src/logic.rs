//! List `crm_customers` for the dashboard page.

use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, Statement};

use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};

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
             FROM crm_customers AS c \
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

async fn count_matching(db: &DatabaseConnection, needle: &str, like: &str) -> Result<u64, String> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
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
