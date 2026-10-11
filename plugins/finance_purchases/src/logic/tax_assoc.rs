//! M2M tax association helpers for purchase tables.

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

pub async fn set_draft_purchase_taxes<C: ConnectionTrait>(
    db: &C,
    draft_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM draft_purchase_taxes WHERE draft_purchase_id = $1",
        [draft_id.into()],
    ))
    .await?;
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO draft_purchase_taxes (draft_purchase_id, tax_id) VALUES ($1, $2)",
            [draft_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn load_draft_purchase_tax_ids<C: ConnectionTrait>(
    db: &C,
    draft_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM draft_purchase_taxes WHERE draft_purchase_id = $1",
            [draft_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

pub async fn set_draft_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM draft_purchase_line_taxes WHERE draft_purchase_line_id = $1",
        [line_id.into()],
    ))
    .await?;
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO draft_purchase_line_taxes (draft_purchase_line_id, tax_id) VALUES ($1, $2)",
            [line_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn load_draft_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM draft_purchase_line_taxes WHERE draft_purchase_line_id = $1",
            [line_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

pub async fn set_posted_purchase_taxes<C: ConnectionTrait>(
    db: &C,
    posted_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO posted_purchase_taxes (posted_purchase_id, tax_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [posted_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn set_posted_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO posted_purchase_line_taxes (posted_purchase_line_id, tax_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [line_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

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

pub async fn load_posted_purchase_tax_ids<C: ConnectionTrait>(
    db: &C,
    posted_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM posted_purchase_taxes WHERE posted_purchase_id = $1",
            [posted_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

pub async fn load_posted_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM posted_purchase_line_taxes WHERE posted_purchase_line_id = $1",
            [line_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

pub async fn load_cancelled_purchase_tax_ids<C: ConnectionTrait>(
    db: &C,
    cancelled_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM cancelled_purchase_taxes WHERE cancelled_purchase_id = $1",
            [cancelled_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

pub async fn load_cancelled_line_tax_ids<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT tax_id FROM cancelled_purchase_line_taxes WHERE cancelled_purchase_line_id = $1",
            [line_id.into()],
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<i64>("", "tax_id").ok())
        .collect())
}

pub async fn set_cancelled_purchase_taxes<C: ConnectionTrait>(
    db: &C,
    cancelled_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO cancelled_purchase_taxes (cancelled_purchase_id, tax_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [cancelled_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}

pub async fn set_cancelled_line_taxes<C: ConnectionTrait>(
    db: &C,
    line_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    for tax_id in tax_ids {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO cancelled_purchase_line_taxes (cancelled_purchase_line_id, tax_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            [line_id.into(), (*tax_id).into()],
        ))
        .await?;
    }
    Ok(())
}
