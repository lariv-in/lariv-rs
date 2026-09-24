use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::plugins::tasks::entities::{
    task::{self, Entity as TaskEntity},
    task_log,
};
use crate::plugins::tasks::scope::{find_task_scoped, status_display_label, user_display_label};
use crate::plugins::users::state::AuthContext;

pub struct TaskFields {
    pub title: String,
    pub description: String,
    pub assigned_to_id: i64,
    pub status_id: i64,
    pub priority: i32,
    pub due_datetime: DateTime<Utc>,
}

pub struct TaskChangeLabels {
    pub assigned_to_before: String,
    pub assigned_to_after: String,
    pub status_before: String,
    pub status_after: String,
    pub due_before: String,
    pub due_after: String,
}

const DESCRIPTION_PREVIEW: usize = 80;

pub async fn update_task(
    db: &DatabaseConnection,
    existing: task::Model,
    fields: TaskFields,
    auth: &AuthContext,
) -> Result<task::Model, String> {
    let now = Utc::now();
    let mut am: task::ActiveModel = existing.clone().into();
    am.updated_at = Set(Some(now));
    am.title = Set(fields.title);
    am.description = Set(fields.description);
    am.assigned_to_id = Set(fields.assigned_to_id);
    am.status_id = Set(fields.status_id);
    am.priority = Set(fields.priority);
    am.due_datetime = Set(fields.due_datetime);
    let saved = am.update(db).await.map_err(|e| e.to_string())?;
    crate::web::log_err(
        log_task_update(db, &existing, &saved, auth).await,
        "append task change log",
    );
    Ok(saved)
}

pub async fn delete_task(
    db: &DatabaseConnection,
    task_id: i64,
    auth: &AuthContext,
) -> Result<(), String> {
    let existing = find_task_scoped(db, task_id, auth)
        .await
        .ok_or_else(|| "task not found".to_string())?;
    TaskEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn log_task_update(
    db: &DatabaseConnection,
    before: &task::Model,
    after: &task::Model,
    auth: &AuthContext,
) -> Result<(), String> {
    let labels = TaskChangeLabels {
        assigned_to_before: label_or_id(
            user_display_label(db, before.assigned_to_id).await,
            before.assigned_to_id,
        ),
        assigned_to_after: label_or_id(
            user_display_label(db, after.assigned_to_id).await,
            after.assigned_to_id,
        ),
        status_before: label_or_id(
            status_display_label(db, before.status_id).await,
            before.status_id,
        ),
        status_after: label_or_id(
            status_display_label(db, after.status_id).await,
            after.status_id,
        ),
        due_before: auth.format_datetime(before.due_datetime).into_string(),
        due_after: auth.format_datetime(after.due_datetime).into_string(),
    };
    let Some(summary) = summarize_task_changes(before, after, &labels) else {
        return Ok(());
    };
    append_task_log(db, after.id, summary, Utc::now())
        .await
        .map(|_| ())
}

pub async fn append_task_log(
    db: &DatabaseConnection,
    task_id: i64,
    description: String,
    datetime: DateTime<Utc>,
) -> Result<task_log::Model, String> {
    let now = Utc::now();
    task_log::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        task_id: Set(task_id),
        description: Set(description),
        datetime: Set(datetime),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub fn summarize_task_changes(
    before: &task::Model,
    after: &task::Model,
    labels: &TaskChangeLabels,
) -> Option<String> {
    let mut lines = Vec::new();
    if before.title != after.title {
        lines.push(format!(
            "Title changed from {} to {}.",
            quote(&before.title),
            quote(&after.title)
        ));
    }
    if before.description != after.description {
        lines.push(format!(
            "Description changed from {} to {}.",
            quote(&preview_text(&before.description)),
            quote(&preview_text(&after.description))
        ));
    }
    if before.assigned_to_id != after.assigned_to_id {
        lines.push(format!(
            "Assigned to changed from {} to {}.",
            labels.assigned_to_before, labels.assigned_to_after
        ));
    }
    if before.status_id != after.status_id {
        lines.push(format!(
            "Status changed from {} to {}.",
            labels.status_before, labels.status_after
        ));
    }
    if before.priority != after.priority {
        lines.push(format!(
            "Priority changed from {} to {}.",
            before.priority, after.priority
        ));
    }
    if before.due_datetime != after.due_datetime && labels.due_before != labels.due_after {
        lines.push(format!(
            "Due date changed from {} to {}.",
            labels.due_before, labels.due_after
        ));
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn label_or_id(label: String, id: i64) -> String {
    if label.trim().is_empty() {
        format!("#{id}")
    } else {
        label
    }
}

fn quote(value: &str) -> String {
    format!("\"{value}\"")
}

fn preview_text(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "(empty)".into();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() > DESCRIPTION_PREVIEW {
        format!(
            "{}…",
            chars[..DESCRIPTION_PREVIEW].iter().collect::<String>()
        )
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> task::Model {
        task::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            title: "Write docs".into(),
            description: "First draft".into(),
            assigned_to_id: 1,
            status_id: 1,
            priority: 0,
            due_datetime: DateTime::parse_from_rfc3339("2026-09-23T10:00:00Z")
                .expect("rfc3339")
                .with_timezone(&Utc),
        }
    }

    fn sample_labels() -> TaskChangeLabels {
        TaskChangeLabels {
            assigned_to_before: "Ada".into(),
            assigned_to_after: "Ada".into(),
            status_before: "Open".into(),
            status_after: "Open".into(),
            due_before: "23/09/2026 15:30:00".into(),
            due_after: "23/09/2026 15:30:00".into(),
        }
    }

    #[test]
    fn unchanged_task_has_no_summary() {
        let task = sample_task();
        assert_eq!(summarize_task_changes(&task, &task, &sample_labels()), None);
    }

    #[test]
    fn title_change_is_quoted() {
        let before = sample_task();
        let mut after = before.clone();
        after.title = "Ship docs".into();
        assert_eq!(
            summarize_task_changes(&before, &after, &sample_labels()).as_deref(),
            Some("Title changed from \"Write docs\" to \"Ship docs\".")
        );
    }

    #[test]
    fn multiple_field_changes_are_joined() {
        let before = sample_task();
        let mut after = before.clone();
        after.assigned_to_id = 2;
        after.status_id = 3;
        after.priority = 2;
        let mut labels = sample_labels();
        labels.assigned_to_after = "Bob".into();
        labels.status_after = "Done".into();
        assert_eq!(
            summarize_task_changes(&before, &after, &labels).as_deref(),
            Some(
                "Assigned to changed from Ada to Bob.\nStatus changed from Open to Done.\nPriority changed from 0 to 2."
            )
        );
    }

    #[test]
    fn empty_description_uses_placeholder() {
        let before = sample_task();
        let mut after = before.clone();
        after.description.clear();
        assert_eq!(
            summarize_task_changes(&before, &after, &sample_labels()).as_deref(),
            Some("Description changed from \"First draft\" to \"(empty)\".")
        );
    }

    #[test]
    fn due_date_skipped_when_labels_match() {
        let before = sample_task();
        let mut after = before.clone();
        after.due_datetime = before.due_datetime + chrono::Duration::seconds(30);
        assert_eq!(
            summarize_task_changes(&before, &after, &sample_labels()),
            None
        );
    }
}
