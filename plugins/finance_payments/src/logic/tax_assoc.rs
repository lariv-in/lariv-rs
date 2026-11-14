use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

pub async fn set_payment_taxes<C: ConnectionTrait>(
    db: &C,
    payment_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO payment_taxes (payment_id, tax_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [payment_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn set_payment_gst_taxes<C: ConnectionTrait>(
    db: &C,
    payment_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM payment_gst_taxes WHERE payment_id = $1",
        [payment_id.into()],
    ))
    .await?;
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO payment_gst_taxes (payment_id, tax_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [payment_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn replace_advance_adjustments<C: ConnectionTrait>(
    db: &C,
    payment_id: i64,
    adjustments: &[(chrono::NaiveDate, rust_decimal::Decimal)],
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM gst_advance_adjustments WHERE payment_id = $1",
        [payment_id.into()],
    ))
    .await?;
    for (date, amount) in adjustments {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO gst_advance_adjustments (payment_id, adjusted_on, gross_amount) VALUES ($1, $2, $3)",
            [payment_id.into(), (*date).into(), (*amount).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn load_payment_tax_ids<C: ConnectionTrait>(
    db: &C,
    payment_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM payment_taxes WHERE payment_id = $1",
            [payment_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}
