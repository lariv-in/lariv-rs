//! JSON sync protocol and database apply for one user's todos.
//!
//! The browser replaces a matching local row with the server row, then overlays
//! any pending edits the server has not acked yet.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::todo::{self, Entity as TodoEntity};
use crate::entities::todo_page::{self, Entity as TodoPageEntity};

/// One mutation from the browser. `op_id` is echoed on the resulting event.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientOp {
    Create {
        op_id: String,
        client_id: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        checked: bool,
    },
    /// Replace `delete` Unicode scalars at `index` with `insert`.
    /// Applied only when `rev` equals the row's `title_rev`.
    SpliceTitle {
        op_id: String,
        #[serde(default)]
        id: Option<i64>,
        #[serde(default)]
        client_id: Option<String>,
        rev: i64,
        index: u32,
        delete: u32,
        insert: String,
    },
    SetChecked {
        op_id: String,
        #[serde(default)]
        id: Option<i64>,
        #[serde(default)]
        client_id: Option<String>,
        checked: bool,
    },
    Delete {
        op_id: String,
        #[serde(default)]
        id: Option<i64>,
        #[serde(default)]
        client_id: Option<String>,
    },
    /// Move one row to `index` in the page's order. The server rewrites every row's order.
    Move {
        op_id: String,
        #[serde(default)]
        id: Option<i64>,
        #[serde(default)]
        client_id: Option<String>,
        index: u32,
    },
}

/// A row the server sends to every connected client for this user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoJson {
    pub id: i64,
    pub client_id: String,
    pub title: String,
    pub checked: bool,
    pub rev: i64,
    pub order: i64,
    pub updated_at: Option<DateTime<Utc>>,
}

/// A splice the server refused because `rev` was stale. Sent only to the socket that submitted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpliceReject {
    pub op_id: String,
    pub todo: TodoJson,
}

/// Broadcast events plus splice revisions the caller must report on the submitting socket.
#[derive(Debug)]
pub struct ApplyResult {
    pub events: Vec<ServerEvent>,
    pub rejects: Vec<SpliceReject>,
}

/// Live event broadcast after a committed write.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Upsert {
        #[serde(skip_serializing_if = "Option::is_none")]
        op_id: Option<String>,
        todo: TodoJson,
    },
    Delete {
        #[serde(skip_serializing_if = "Option::is_none")]
        op_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
    },
    /// `rev` is the row's title revision after this splice was stored.
    SpliceTitle {
        #[serde(skip_serializing_if = "Option::is_none")]
        op_id: Option<String>,
        id: i64,
        client_id: String,
        rev: i64,
        index: u32,
        delete: u32,
        insert: String,
    },
    /// `index` is where `client_id` sits after the move. Orders are dense from 0.
    Move {
        #[serde(skip_serializing_if = "Option::is_none")]
        op_id: Option<String>,
        id: i64,
        client_id: String,
        index: u32,
    },
}

/// First message on a connection, or a single live mutation.
#[derive(Debug)]
pub enum Incoming {
    Sync(Vec<ClientOp>),
    Op(ClientOp),
}

/// Parse a WebSocket text frame. `sync` carries a batch; anything else is one op.
pub fn parse_incoming(text: &str) -> Result<Incoming, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|err| format!("invalid json: {err}"))?;
    let kind = value
        .get("type")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    if kind == "sync" {
        let ops = value
            .get("ops")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let ops: Vec<ClientOp> =
            serde_json::from_value(ops).map_err(|err| format!("invalid sync ops: {err}"))?;
        return Ok(Incoming::Sync(ops));
    }
    let op: ClientOp = serde_json::from_value(value).map_err(|err| format!("invalid op: {err}"))?;
    Ok(Incoming::Op(op))
}

/// Apply `ops` in order for one page the user created.
/// Creates with the same `client_id` return the existing row.
pub async fn apply_ops<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
    ops: &[ClientOp],
) -> Result<ApplyResult, sea_orm::DbErr> {
    let mut events = Vec::with_capacity(ops.len());
    let mut rejects = Vec::new();
    for op in ops {
        match apply_one(db, user_id, page_id, op).await? {
            Step::Event(event) => events.push(event),
            Step::Reject(reject) => rejects.push(reject),
        }
    }
    Ok(ApplyResult { events, rejects })
}

/// This page's todos, in `order`. Missing `client_id` values are filled in.
/// A page the user did not create yields an empty list.
pub async fn load_snapshot<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
) -> Result<Vec<TodoJson>, sea_orm::DbErr> {
    let rows = owned_todos(user_id, page_id)
        .order_by_asc(todo::Column::Order)
        .order_by_asc(todo::Column::Id)
        .all(db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let row = ensure_client_id(db, row).await?;
        out.push(TodoJson::from_model(&row));
    }
    Ok(out)
}

async fn apply_one<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
    op: &ClientOp,
) -> Result<Step, sea_orm::DbErr> {
    match op {
        ClientOp::Create {
            op_id,
            client_id,
            title,
            checked,
        } => {
            if client_id.is_empty() {
                return Ok(Step::Event(delete_event(op_id, None, None)));
            }
            if find_visible_page(db, user_id, page_id).await?.is_none() {
                return Ok(Step::Event(delete_event(
                    op_id,
                    None,
                    Some(client_id.clone()),
                )));
            }
            if let Some(existing) = find_by_client_id(db, user_id, page_id, client_id).await? {
                return Ok(Step::Event(upsert_event(op_id, existing)));
            }
            let now = Utc::now();
            let order = next_order(db, user_id, page_id).await?;
            let model = todo::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                user_id: Set(user_id),
                checked: Set(*checked),
                title: Set(title.clone()),
                title_rev: Set(0),
                client_id: Set(Some(client_id.clone())),
                todo_page_id: Set(page_id),
                order: Set(order),
            };
            match model.insert(db).await {
                Ok(row) => Ok(Step::Event(upsert_event(op_id, row))),
                Err(err) => {
                    if let Some(existing) =
                        find_by_client_id(db, user_id, page_id, client_id).await?
                    {
                        Ok(Step::Event(upsert_event(op_id, existing)))
                    } else {
                        Err(err)
                    }
                }
            }
        }
        ClientOp::SpliceTitle {
            op_id,
            id,
            client_id,
            rev,
            index,
            delete,
            insert,
        } => {
            let Some(row) = find_owned(db, user_id, page_id, *id, client_id.as_deref()).await?
            else {
                return Ok(Step::Event(delete_event(op_id, *id, client_id.clone())));
            };
            if row.title_rev != *rev {
                return Ok(Step::Reject(SpliceReject {
                    op_id: op_id.clone(),
                    todo: TodoJson::from_model(&row),
                }));
            }
            let title = apply_splice(&row.title, *index, *delete, insert);
            let mut active: todo::ActiveModel = row.into();
            active.title = Set(title);
            active.title_rev = Set(*rev + 1);
            active.updated_at = Set(Some(Utc::now()));
            let row = active.update(db).await?;
            Ok(Step::Event(ServerEvent::SpliceTitle {
                op_id: Some(op_id.clone()),
                id: row.id,
                client_id: row.client_id.clone().unwrap_or_default(),
                rev: row.title_rev,
                index: *index,
                delete: *delete,
                insert: insert.clone(),
            }))
        }
        ClientOp::SetChecked {
            op_id,
            id,
            client_id,
            checked,
        } => {
            let Some(row) = find_owned(db, user_id, page_id, *id, client_id.as_deref()).await?
            else {
                return Ok(Step::Event(delete_event(op_id, *id, client_id.clone())));
            };
            let mut active: todo::ActiveModel = row.into();
            active.checked = Set(*checked);
            active.updated_at = Set(Some(Utc::now()));
            let row = active.update(db).await?;
            Ok(Step::Event(upsert_event(op_id, row)))
        }
        ClientOp::Delete {
            op_id,
            id,
            client_id,
        } => {
            let Some(row) = find_owned(db, user_id, page_id, *id, client_id.as_deref()).await?
            else {
                return Ok(Step::Event(delete_event(op_id, *id, client_id.clone())));
            };
            let event = delete_event(op_id, Some(row.id), row.client_id.clone());
            TodoEntity::delete_many()
                .filter(todo::Column::Id.eq(row.id))
                .filter(todo::Column::UserId.eq(user_id))
                .exec(db)
                .await?;
            Ok(Step::Event(event))
        }
        ClientOp::Move {
            op_id,
            id,
            client_id,
            index,
        } => {
            let Some(target) = find_owned(db, user_id, page_id, *id, client_id.as_deref()).await?
            else {
                return Ok(Step::Event(delete_event(op_id, *id, client_id.clone())));
            };
            let mut rows = owned_todos(user_id, page_id)
                .order_by_asc(todo::Column::Order)
                .order_by_asc(todo::Column::Id)
                .all(db)
                .await?;
            let Some(from) = rows.iter().position(|row| row.id == target.id) else {
                return Ok(Step::Event(delete_event(
                    op_id,
                    Some(target.id),
                    target.client_id,
                )));
            };
            let row = rows.remove(from);
            let dest = (*index as usize).min(rows.len());
            rows.insert(dest, row);
            let now = Utc::now();
            for (ord, row) in rows.iter().enumerate() {
                let ord = i64::try_from(ord).unwrap_or(i64::MAX);
                if row.order == ord {
                    continue;
                }
                let mut active: todo::ActiveModel = row.clone().into();
                active.order = Set(ord);
                active.updated_at = Set(Some(now));
                active.update(db).await?;
            }
            Ok(Step::Event(ServerEvent::Move {
                op_id: Some(op_id.clone()),
                id: target.id,
                client_id: target.client_id.unwrap_or_default(),
                index: u32::try_from(dest).unwrap_or(u32::MAX),
            }))
        }
    }
}

enum Step {
    Event(ServerEvent),
    Reject(SpliceReject),
}

/// One title edit: delete `delete` Unicode scalars at `index`, then insert `insert`.
/// Indexes past the end land at the end. Deletes that run past the end stop at the end.
pub fn apply_splice(title: &str, index: u32, delete: u32, insert: &str) -> String {
    let mut chars: Vec<char> = title.chars().collect();
    let start = (index as usize).min(chars.len());
    let remove = (delete as usize).min(chars.len() - start);
    let replacement: Vec<char> = insert.chars().collect();
    chars.splice(start..start + remove, replacement);
    chars.into_iter().collect()
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TextSplice {
    index: u32,
    delete: u32,
    insert: String,
}

/// Retarget `op` so it still describes the same edit after `already` has been applied.
/// When both are inserts at the same index, `insert_after` places `op` after `already`.
/// The browser copy in `todo_client.js` follows these rules.
#[cfg(test)]
fn transform_splice(op: TextSplice, already: &TextSplice, insert_after: bool) -> TextSplice {
    let start = transform_pos(op.index, already, insert_after);
    let end = transform_pos(op.index.saturating_add(op.delete), already, insert_after);
    TextSplice {
        index: start,
        delete: end.saturating_sub(start),
        insert: op.insert,
    }
}

#[cfg(test)]
fn transform_pos(pos: u32, already: &TextSplice, insert_after: bool) -> u32 {
    let start = already.index;
    let end = already.index.saturating_add(already.delete);
    let inserted = u32::try_from(already.insert.chars().count()).unwrap_or(u32::MAX);
    if already.delete == 0 && pos == start {
        return if insert_after {
            pos.saturating_add(inserted)
        } else {
            pos
        };
    }
    if pos <= start {
        return pos;
    }
    if pos >= end {
        let shifted = i64::from(pos) + i64::from(inserted) - i64::from(already.delete);
        return u32::try_from(shifted).unwrap_or(0);
    }
    start
}

async fn ensure_client_id<C: ConnectionTrait>(
    db: &C,
    row: todo::Model,
) -> Result<todo::Model, sea_orm::DbErr> {
    if row
        .client_id
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Ok(row);
    }
    let mut active: todo::ActiveModel = row.into();
    active.client_id = Set(Some(Uuid::new_v4().to_string()));
    active.update(db).await
}

fn upsert_event(op_id: &str, row: todo::Model) -> ServerEvent {
    ServerEvent::Upsert {
        op_id: Some(op_id.to_owned()),
        todo: TodoJson::from_model(&row),
    }
}

fn delete_event(op_id: &str, id: Option<i64>, client_id: Option<String>) -> ServerEvent {
    ServerEvent::Delete {
        op_id: Some(op_id.to_owned()),
        id,
        client_id: client_id.filter(|value| !value.is_empty()),
    }
}

impl TodoJson {
    fn from_model(row: &todo::Model) -> Self {
        Self {
            id: row.id,
            client_id: row.client_id.clone().unwrap_or_default(),
            title: row.title.clone(),
            checked: row.checked,
            rev: row.title_rev,
            order: row.order,
            updated_at: row.updated_at,
        }
    }
}

/// A page is visible only to the user who created it.
pub async fn find_visible_page<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
) -> Result<Option<todo_page::Model>, sea_orm::DbErr> {
    TodoPageEntity::find()
        .filter(todo_page::Column::Id.eq(page_id))
        .filter(todo_page::Column::CreatedById.eq(user_id))
        .one(db)
        .await
}

/// Items on a page the user created.
fn owned_todos(user_id: i64, page_id: i64) -> sea_orm::Select<TodoEntity> {
    TodoEntity::find()
        .inner_join(todo_page::Entity)
        .filter(todo::Column::UserId.eq(user_id))
        .filter(todo::Column::TodoPageId.eq(page_id))
        .filter(todo_page::Column::CreatedById.eq(user_id))
}

async fn next_order<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
) -> Result<i64, sea_orm::DbErr> {
    let last = owned_todos(user_id, page_id)
        .order_by_desc(todo::Column::Order)
        .one(db)
        .await?;
    Ok(last.map(|row| row.order.saturating_add(1)).unwrap_or(0))
}

async fn find_by_client_id<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
    client_id: &str,
) -> Result<Option<todo::Model>, sea_orm::DbErr> {
    owned_todos(user_id, page_id)
        .filter(todo::Column::ClientId.eq(client_id))
        .one(db)
        .await
}

async fn find_owned<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    page_id: i64,
    id: Option<i64>,
    client_id: Option<&str>,
) -> Result<Option<todo::Model>, sea_orm::DbErr> {
    if let Some(client_id) = client_id.filter(|value| !value.is_empty())
        && let Some(row) = find_by_client_id(db, user_id, page_id, client_id).await?
    {
        return Ok(Some(row));
    }
    let Some(id) = id.filter(|id| *id > 0) else {
        return Ok(None);
    };
    owned_todos(user_id, page_id)
        .filter(todo::Column::Id.eq(id))
        .one(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DatabaseBackend, Schema};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ClientRow {
        id: Option<i64>,
        client_id: String,
        title: String,
        checked: bool,
    }

    /// Server version replaces the matching client row (by `client_id`, else by `id`).
    fn replace_with_server_row(rows: &mut Vec<ClientRow>, server: ClientRow) {
        let matched = rows.iter_mut().find(|row| {
            if !server.client_id.is_empty() && row.client_id == server.client_id {
                return true;
            }
            matches!((row.id, server.id), (Some(left), Some(right)) if left == right)
        });
        if let Some(row) = matched {
            *row = server;
        } else {
            rows.push(server);
        }
    }

    fn upsert(applied: &ApplyResult) -> &TodoJson {
        applied
            .events
            .first()
            .and_then(|event| match event {
                ServerEvent::Upsert { todo, .. } => Some(todo),
                ServerEvent::Delete { .. }
                | ServerEvent::SpliceTitle { .. }
                | ServerEvent::Move { .. } => None,
            })
            .expect("upsert")
    }

    fn splice(index: u32, delete: u32, insert: &str) -> TextSplice {
        TextSplice {
            index,
            delete,
            insert: insert.into(),
        }
    }

    #[test]
    fn splice_edits_scalars_and_clamps() {
        assert_eq!(apply_splice("hi", 1, 0, "X"), "hXi");
        assert_eq!(apply_splice("hi", 1, 1, ""), "h");
        assert_eq!(apply_splice("hi", 2, 1, ""), "hi");
        assert_eq!(apply_splice("cat", 0, 0, "a "), "a cat");
        assert_eq!(apply_splice("hi", 50, 0, "!"), "hi!");
        assert_eq!(apply_splice("hi", 1, 10, ""), "h");
        assert_eq!(apply_splice("a👍b", 1, 1, "X"), "aXb");
    }

    #[test]
    fn transform_shifts_pending_around_a_remote_splice() {
        let pending = splice(5, 0, "!");
        let before = splice(0, 0, "X");
        assert_eq!(transform_splice(pending.clone(), &before, true).index, 6);

        let after = splice(8, 0, "Z");
        assert_eq!(transform_splice(pending, &after, true).index, 5);

        let same = splice(2, 0, "c");
        let remote = splice(2, 0, "X");
        let shifted = transform_splice(same.clone(), &remote, true);
        assert_eq!(shifted.index, 3);
        assert_eq!(shifted.insert, "c");

        let base = "ab";
        let server = apply_splice(base, remote.index, remote.delete, &remote.insert);
        let converged = apply_splice(&server, shifted.index, shifted.delete, &shifted.insert);
        let display = apply_splice(base, same.index, same.delete, &same.insert);
        let remote_in_display = transform_splice(remote, &same, false);
        let display_after = apply_splice(
            &display,
            remote_in_display.index,
            remote_in_display.delete,
            &remote_in_display.insert,
        );
        assert_eq!(converged, "abXc");
        assert_eq!(display_after, converged);

        let overlap = transform_splice(splice(0, 5, ""), &splice(2, 2, ""), true);
        assert_eq!(overlap.index, 0);
        assert_eq!(overlap.delete, 3);
        let deleted = apply_splice("0123456789", 2, 2, "");
        assert_eq!(
            apply_splice(&deleted, overlap.index, overlap.delete, ""),
            "56789"
        );
    }

    #[test]
    fn server_row_replaces_client_row() {
        let mut rows = vec![ClientRow {
            id: None,
            client_id: "c1".into(),
            title: "local".into(),
            checked: false,
        }];
        replace_with_server_row(
            &mut rows,
            ClientRow {
                id: Some(5),
                client_id: "c1".into(),
                title: "server".into(),
                checked: true,
            },
        );
        assert_eq!(rows.len(), 1);
        let row = rows.first().expect("replaced row");
        assert_eq!(row.id, Some(5));
        assert_eq!(row.title, "server");
        assert!(row.checked);

        replace_with_server_row(
            &mut rows,
            ClientRow {
                id: Some(9),
                client_id: "c2".into(),
                title: "other".into(),
                checked: false,
            },
        );
        assert_eq!(rows.len(), 2);
        let other = rows.get(1).expect("appended row");
        assert_eq!(other.title, "other");
    }

    #[tokio::test]
    async fn create_is_idempotent_and_server_title_replaces_client() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        db.execute_unprepared("PRAGMA foreign_keys = OFF")
            .await
            .expect("foreign keys");
        let schema = Schema::new(DatabaseBackend::Sqlite);
        let users = schema.create_table_from_entity(lariv_plugin_users::entities::user::Entity);
        db.execute(&users).await.expect("create users");
        let pages = schema.create_table_from_entity(TodoPageEntity);
        db.execute(&pages).await.expect("create todo pages");
        let todos = schema.create_table_from_entity(TodoEntity);
        db.execute(&todos).await.expect("create todos");
        let now = Utc::now();
        let page = todo_page::ActiveModel {
            title: Set("Untitled".to_owned()),
            created_by_id: Set(7),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("page");
        let page_id = page.id;

        let created = apply_ops(
            &db,
            7,
            page_id,
            &[ClientOp::Create {
                op_id: "1".into(),
                client_id: "c1".into(),
                title: "local".into(),
                checked: false,
            }],
        )
        .await
        .expect("create");
        let todo = upsert(&created);
        let id = todo.id;
        assert_eq!(todo.title, "local");
        assert_eq!(todo.rev, 0);

        let again = apply_ops(
            &db,
            7,
            page_id,
            &[ClientOp::Create {
                op_id: "2".into(),
                client_id: "c1".into(),
                title: "ignored".into(),
                checked: true,
            }],
        )
        .await
        .expect("create again");
        let todo = upsert(&again);
        assert_eq!(todo.id, id);
        assert_eq!(todo.title, "local");
        assert!(!todo.checked);

        let typed = apply_ops(
            &db,
            7,
            page_id,
            &[
                ClientOp::SpliceTitle {
                    op_id: "3".into(),
                    id: Some(id),
                    client_id: Some("c1".into()),
                    rev: 0,
                    index: 5,
                    delete: 0,
                    insert: "!".into(),
                },
                ClientOp::SpliceTitle {
                    op_id: "4".into(),
                    id: Some(id),
                    client_id: Some("c1".into()),
                    rev: 1,
                    index: 0,
                    delete: 0,
                    insert: "X".into(),
                },
            ],
        )
        .await
        .expect("splice");
        assert!(typed.rejects.is_empty());
        assert_eq!(typed.events.len(), 2);

        let stale = apply_ops(
            &db,
            7,
            page_id,
            &[ClientOp::SpliceTitle {
                op_id: "5".into(),
                id: Some(id),
                client_id: Some("c1".into()),
                rev: 0,
                index: 0,
                delete: 1,
                insert: String::new(),
            }],
        )
        .await
        .expect("stale splice");
        assert!(stale.events.is_empty());
        let rejected = stale.rejects.first().expect("reject");
        assert_eq!(rejected.op_id, "5");
        assert_eq!(rejected.todo.title, "Xlocal!");
        assert_eq!(rejected.todo.rev, 2);

        let snapshot = load_snapshot(&db, 7, page_id).await.expect("snapshot");
        assert_eq!(snapshot.len(), 1);
        let saved = snapshot.first().expect("saved row");
        assert_eq!(saved.id, id);
        assert_eq!(saved.client_id, "c1");
        assert_eq!(saved.title, "Xlocal!");
        assert_eq!(saved.rev, 2);
        assert_eq!(saved.order, 0);

        let other = load_snapshot(&db, 8, page_id).await.expect("other user");
        assert!(other.is_empty());

        let pages = TodoPageEntity::find().all(&db).await.expect("pages");
        assert_eq!(pages.len(), 1);
        let page = pages.first().expect("untitled page");
        assert_eq!(page.title, "Untitled");
        assert_eq!(page.created_by_id, 7);
        let stored = TodoEntity::find_by_id(id).one(&db).await.expect("row");
        let stored = stored.expect("saved model");
        assert_eq!(stored.todo_page_id, page.id);
        let hidden = TodoPageEntity::find()
            .filter(todo_page::Column::CreatedById.eq(8))
            .all(&db)
            .await
            .expect("other pages");
        assert!(hidden.is_empty());
    }

    #[tokio::test]
    async fn move_rewrites_order_and_snapshot_follows_it() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        db.execute_unprepared("PRAGMA foreign_keys = OFF")
            .await
            .expect("foreign keys");
        let schema = Schema::new(DatabaseBackend::Sqlite);
        db.execute(&schema.create_table_from_entity(lariv_plugin_users::entities::user::Entity))
            .await
            .expect("users");
        db.execute(&schema.create_table_from_entity(TodoPageEntity))
            .await
            .expect("pages");
        db.execute(&schema.create_table_from_entity(TodoEntity))
            .await
            .expect("todos");
        let now = Utc::now();
        let page = todo_page::ActiveModel {
            title: Set("Errands".into()),
            created_by_id: Set(7),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("page");

        for (op_id, client_id, title) in [("1", "c1", "milk"), ("2", "c2", "bread")] {
            apply_ops(
                &db,
                7,
                page.id,
                &[ClientOp::Create {
                    op_id: op_id.into(),
                    client_id: client_id.into(),
                    title: title.into(),
                    checked: false,
                }],
            )
            .await
            .expect("create");
        }

        let moved = apply_ops(
            &db,
            7,
            page.id,
            &[ClientOp::Move {
                op_id: "3".into(),
                id: None,
                client_id: Some("c2".into()),
                index: 0,
            }],
        )
        .await
        .expect("move");
        let event = moved.events.first().expect("move event");
        match event {
            ServerEvent::Move {
                client_id, index, ..
            } => {
                assert_eq!(client_id, "c2");
                assert_eq!(*index, 0);
            }
            _ => panic!("expected move"),
        }

        let snapshot = load_snapshot(&db, 7, page.id).await.expect("snapshot");
        let titles: Vec<_> = snapshot.iter().map(|row| row.title.as_str()).collect();
        assert_eq!(titles, ["bread", "milk"]);
        assert_eq!(snapshot[0].order, 0);
        assert_eq!(snapshot[1].order, 1);
    }
}
