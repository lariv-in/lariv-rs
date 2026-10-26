//! Line tax junctions for quotations.

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

pub async fn set_goods_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(
        db,
        "quotation_goods_line_taxes",
        "line_id",
        line_id,
        tax_ids,
    )
    .await
}

pub async fn set_service_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    set_line_taxes(
        db,
        "quotation_service_line_taxes",
        "line_id",
        line_id,
        tax_ids,
    )
    .await
}

pub async fn load_goods_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, "quotation_goods_line_taxes", line_id).await
}

pub async fn load_service_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    load_line_tax_ids(db, "quotation_service_line_taxes", line_id).await
}

async fn set_line_taxes<C: ConnectionTrait>(
    db: &C,
    table: &str,
    column: &str,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        format!("DELETE FROM {table} WHERE {column} = $1"),
        [line_id.into()],
    ))
    .await?;
    for tax_id in tax_ids {
        if *tax_id <= 0 {
            continue;
        }
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("INSERT INTO {table} ({column}, tax_id) VALUES ($1, $2)"),
            [line_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

async fn load_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    table: &str,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT tax_id FROM {table} WHERE line_id = $1 ORDER BY tax_id"),
            [line_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.try_get::<i64>("", "tax_id").ok())
        .collect())
}
