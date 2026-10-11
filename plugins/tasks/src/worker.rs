//! Poll periodic tasks and materialize a task when the interval has elapsed.
//!
//! A schedule is due when `last_added_at + frequency` is at or before now.
//! Each due row creates one task from the template fields, then advances
//! `last_added_at` and `due_datetime` by `frequency`. The created task keeps
//! the due datetime from before that advance, so the next task is due one
//! interval later.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait, sea_query::Expr,
};

use crate::entities::{
    periodic_task::{self, Entity as PeriodicTaskEntity},
    task,
};
use crate::push::{self, PushSettings};

const LOG_TARGET: &str = "tasks::periodic";
const POLL_INTERVAL: Duration = Duration::from_secs(30);

static STARTED: AtomicBool = AtomicBool::new(false);

/// Spawn the polling loop. Later calls are ignored.
pub fn spawn(db: DatabaseConnection, push: PushSettings) {
    if STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    push::warn_if_disabled(&push);
    let client = reqwest::Client::new();
    tokio::spawn(async move {
        tracing::info!(target: LOG_TARGET, "periodic task worker started");
        loop {
            if let Err(e) = poll_once(&db, &push, &client).await {
                tracing::error!(target: LOG_TARGET, "periodic task poll failed: {e}");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

/// Previous timestamp plus `frequency`. `None` when `frequency` is not positive
/// or the result does not fit in a datetime.
pub fn add_frequency(at: DateTime<Utc>, frequency_ns: i64) -> Option<DateTime<Utc>> {
    if frequency_ns <= 0 {
        return None;
    }
    at.checked_add_signed(chrono::Duration::nanoseconds(frequency_ns))
}

/// `true` when `last_added_at + frequency` is at or before `now`.
pub fn is_due(last_added_at: DateTime<Utc>, frequency_ns: i64, now: DateTime<Utc>) -> bool {
    add_frequency(last_added_at, frequency_ns).is_some_and(|due_at| due_at <= now)
}

/// Load every periodic task and create a task for each one that is due.
pub async fn poll_once(
    db: &DatabaseConnection,
    push: &PushSettings,
    client: &reqwest::Client,
) -> Result<(), String> {
    let rows = PeriodicTaskEntity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    for row in rows {
        let Some(next_last_added_at) = add_frequency(row.last_added_at, row.frequency) else {
            continue;
        };
        if next_last_added_at > now {
            continue;
        }
        let Some(next_due) = add_frequency(row.due_datetime, row.frequency) else {
            tracing::error!(
                target: LOG_TARGET,
                periodic_task_id = row.id,
                "frequency does not fit on the due datetime"
            );
            continue;
        };
        if let Err(e) = materialize(db, push, client, &row, now, next_last_added_at, next_due).await
        {
            tracing::error!(
                target: LOG_TARGET,
                periodic_task_id = row.id,
                "failed to create task: {e}"
            );
        }
    }
    Ok(())
}

async fn materialize(
    db: &DatabaseConnection,
    push: &PushSettings,
    client: &reqwest::Client,
    row: &periodic_task::Model,
    now: DateTime<Utc>,
    next_last_added_at: DateTime<Utc>,
    next_due: DateTime<Utc>,
) -> Result<(), String> {
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let claimed = PeriodicTaskEntity::update_many()
        .col_expr(
            periodic_task::Column::LastAddedAt,
            Expr::value(next_last_added_at),
        )
        .col_expr(periodic_task::Column::DueDatetime, Expr::value(next_due))
        .col_expr(periodic_task::Column::UpdatedAt, Expr::value(now))
        .filter(periodic_task::Column::Id.eq(row.id))
        .filter(periodic_task::Column::LastAddedAt.eq(row.last_added_at))
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    if claimed.rows_affected == 0 {
        txn.rollback().await.map_err(|e| e.to_string())?;
        return Ok(());
    }
    let created = task::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(row.title.clone()),
        description: Set(row.description.clone()),
        assigned_to_id: Set(row.assigned_to_id),
        status_id: Set(row.status_id),
        parent_id: Set(row.parent_id),
        priority: Set(row.priority),
        due_datetime: Set(row.due_datetime),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    tracing::info!(
        target: LOG_TARGET,
        periodic_task_id = row.id,
        task_id = created.id,
        "created task from periodic task"
    );
    push::notify_created(db, push, client, &created).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .expect("rfc3339")
            .with_timezone(&Utc)
    }

    #[test]
    fn due_when_interval_has_elapsed() {
        let last = ts("2026-10-11T10:00:00Z");
        let hour = chrono::Duration::hours(1).num_nanoseconds().expect("ns");
        assert!(is_due(last, hour, ts("2026-10-11T11:00:00Z")));
        assert!(is_due(last, hour, ts("2026-10-11T12:00:00Z")));
        assert!(!is_due(last, hour, ts("2026-10-11T10:59:59Z")));
    }

    #[test]
    fn advance_adds_the_frequency() {
        let last = ts("2026-10-11T10:00:00Z");
        let hour = chrono::Duration::hours(1).num_nanoseconds().expect("ns");
        assert_eq!(add_frequency(last, hour), Some(ts("2026-10-11T11:00:00Z")));
        assert_eq!(add_frequency(last, 0), None);
    }

    #[test]
    fn due_datetime_advances_by_the_frequency() {
        let due = ts("2026-10-11T18:00:00Z");
        let day = chrono::Duration::days(1).num_nanoseconds().expect("ns");
        assert_eq!(add_frequency(due, day), Some(ts("2026-10-12T18:00:00Z")));
    }

    #[test]
    fn non_positive_frequency_is_never_due() {
        let last = ts("2026-10-11T10:00:00Z");
        let now = ts("2026-10-11T12:00:00Z");
        assert!(!is_due(last, 0, now));
        assert!(!is_due(last, -1, now));
    }
}
