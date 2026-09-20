//! SQLite-memory tests for read/create/edit VNode tools.

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, Schema, Statement};
use serde_json::json;

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::{
            entities::filesystem_node,
            node::{self, NodeFile},
            storage::{DynFilestore, LocalFilestore},
        },
        llm_assistant::entities::{session, session_vnode_read},
    },
    rune_env::RuneEnvCapability,
};

use super::{
    create_vnode::CreateVnodeTool,
    download_file::{DownloadFileTool, store_at_path},
    edit_vnode::EditVnodeTool,
    http_fetch::Fetched,
    read_vnode::ReadVnodeTool,
};

struct Harness {
    db: sea_orm::DatabaseConnection,
    store: Arc<DynFilestore>,
    rune_env: RuneEnvCapability,
    session_id: i64,
}

impl Harness {
    fn ctx(&self) -> ToolCtx<'_> {
        ToolCtx {
            db: &self.db,
            store: Arc::clone(&self.store),
            cse_api_key: "",
            cse_cx: "",
            rune_env: &self.rune_env,
            hitl: None,
            hitl_gate: None,
            session_id: Some(self.session_id),
            genai: None,
            subagents: None,
        }
    }
}

async fn setup() -> Harness {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("sqlite memory");
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    db.execute_raw(Statement::from_string(
        backend,
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)".to_string(),
    ))
    .await
    .expect("users");
    db.execute_raw(Statement::from_string(
        backend,
        "INSERT INTO users (id) VALUES (1)".to_string(),
    ))
    .await
    .expect("user row");

    for stmt in [
        schema.create_table_from_entity(session::Entity),
        schema.create_table_from_entity(filesystem_node::Entity),
        schema.create_table_from_entity(session_vnode_read::Entity),
    ] {
        db.execute(&stmt)
            .await
            .expect("create table");
    }

    let now = Utc::now();
    let session_id = session::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(String::new()),
        user_id: Set(1),
        reply_email: Set(None),
        email_message_id: Set(None),
        email_references: Set(None),
        context_tokens: Set(0),
        is_subagent: Set(false),
    }
    .insert(&db)
    .await
    .expect("session")
    .id;

    let dir = std::env::temp_dir().join(format!("lariv-vnode-tools-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let store: Arc<DynFilestore> = Arc::new(LocalFilestore::new(dir.to_string_lossy()));

    Harness {
        db,
        store,
        rune_env: RuneEnvCapability::new(),
        session_id,
    }
}

async fn seed_file(h: &Harness, name: &str, data: Vec<u8>) -> filesystem_node::Model {
    node::create(
        &h.db,
        h.store.as_ref(),
        name.into(),
        false,
        Some(NodeFile::Bytes {
            filename: name.into(),
            data,
        }),
        None,
    )
    .await
    .expect("seed file")
}

#[tokio::test]
async fn create_refuses_overwrite() {
    let h = setup().await;
    let ctx = h.ctx();
    CreateVnodeTool
        .run(&ctx, json!({ "path": "/notes/todo.md", "content": "one" }))
        .await
        .expect("create");
    let err = CreateVnodeTool
        .run(&ctx, json!({ "path": "/notes/todo.md", "content": "two" }))
        .await
        .unwrap_err();
    assert!(err.contains("already exists"), "{err}");
    assert!(err.contains("edit_vnode"), "{err}");
}

#[tokio::test]
async fn edit_refuses_unread() {
    let h = setup().await;
    seed_file(&h, "notes.txt", b"hello".to_vec()).await;
    let err = EditVnodeTool
        .run(
            &h.ctx(),
            json!({ "path": "/notes.txt", "content": "changed" }),
        )
        .await
        .unwrap_err();
    assert!(err.contains("has not yet been read"), "{err}");
}

#[tokio::test]
async fn edit_refuses_stale() {
    let h = setup().await;
    seed_file(&h, "notes.txt", b"hello".to_vec()).await;
    ReadVnodeTool
        .run(&h.ctx(), json!({ "path": "/notes.txt" }))
        .await
        .expect("read");

    let vnode = node::get_by_path(&h.db, "/notes.txt")
        .await
        .expect("lookup")
        .0
        .expect("exists");
    node::update(
        &h.db,
        h.store.as_ref(),
        vnode,
        "notes.txt".into(),
        Some(NodeFile::Bytes {
            filename: "notes.txt".into(),
            data: b"outside edit".to_vec(),
        }),
    )
    .await
    .expect("external update");

    let err = EditVnodeTool
        .run(
            &h.ctx(),
            json!({ "path": "/notes.txt", "content": "from llm" }),
        )
        .await
        .unwrap_err();
    assert!(err.contains("edited since the last read_vnode"), "{err}");
}

#[tokio::test]
async fn read_and_edit_refuse_binary() {
    let h = setup().await;
    seed_file(&h, "blob.bin", vec![0xff, 0xfe, 0x00]).await;

    let read_err = ReadVnodeTool
        .run(&h.ctx(), json!({ "path": "/blob.bin" }))
        .await
        .unwrap_err();
    assert!(read_err.contains("not a text file"), "{read_err}");

    let edit_err = EditVnodeTool
        .run(&h.ctx(), json!({ "path": "/blob.bin", "content": "text" }))
        .await
        .unwrap_err();
    assert!(edit_err.contains("not a text file"), "{edit_err}");
}

#[tokio::test]
async fn read_unblocks_edit() {
    let h = setup().await;
    seed_file(&h, "notes.txt", b"hello".to_vec()).await;
    let read = ReadVnodeTool
        .run(&h.ctx(), json!({ "path": "/notes.txt" }))
        .await
        .expect("read");
    assert_eq!(read["content"], "hello");

    let edited = EditVnodeTool
        .run(
            &h.ctx(),
            json!({ "path": "/notes.txt", "content": "hello world" }),
        )
        .await
        .expect("edit");
    assert_eq!(edited["bytes"], 11);

    let reread = ReadVnodeTool
        .run(&h.ctx(), json!({ "path": "/notes.txt" }))
        .await
        .expect("reread");
    assert_eq!(reread["content"], "hello world");
}

#[tokio::test]
async fn create_then_edit_without_extra_read() {
    let h = setup().await;
    CreateVnodeTool
        .run(
            &h.ctx(),
            json!({ "path": "/notes/todo.md", "content": "one" }),
        )
        .await
        .expect("create");
    let edited = EditVnodeTool
        .run(
            &h.ctx(),
            json!({ "path": "/notes/todo.md", "content": "two" }),
        )
        .await
        .expect("edit after create");
    assert_eq!(edited["path"], "/notes/todo.md");
    assert_eq!(edited["bytes"], 3);

    let reread = ReadVnodeTool
        .run(&h.ctx(), json!({ "path": "/notes/todo.md" }))
        .await
        .expect("reread");
    assert_eq!(reread["content"], "two");
}

#[tokio::test]
async fn create_refuses_directory_path() {
    let h = setup().await;
    node::create(&h.db, h.store.as_ref(), "notes".into(), true, None, None)
        .await
        .expect("dir");
    let err = CreateVnodeTool
        .run(&h.ctx(), json!({ "path": "/notes", "content": "nope" }))
        .await
        .unwrap_err();
    assert!(err.contains("directory"), "{err}");
}

#[tokio::test]
async fn download_saves_bytes_at_path() {
    let h = setup().await;
    let saved = store_at_path(
        &h.ctx(),
        "/downloads/photo.png",
        Fetched {
            url: "https://example.com/photo.png".into(),
            content_type: "image/png".into(),
            body: b"\x89PNG\r\n\x1a\n".to_vec(),
        },
    )
    .await
    .expect("download");
    assert_eq!(saved["path"], "/downloads/photo.png");
    assert_eq!(saved["name"], "photo.png");
    assert_eq!(saved["bytes"], 8);
    assert_eq!(saved["content_type"], "image/png");
    assert_eq!(saved["url"], "https://example.com/photo.png");
    assert!(
        saved["download_url"]
            .as_str()
            .expect("download_url")
            .contains("/filesystem/")
    );

    let err = store_at_path(
        &h.ctx(),
        "/downloads/photo.png",
        Fetched {
            url: "https://example.com/photo.png".into(),
            content_type: "image/png".into(),
            body: b"\x89PNG\r\n\x1a\n".to_vec(),
        },
    )
    .await
    .unwrap_err();
    assert!(err.contains("already exists"), "{err}");
}

#[tokio::test]
async fn download_refuses_directory_path() {
    let h = setup().await;
    node::create(
        &h.db,
        h.store.as_ref(),
        "downloads".into(),
        true,
        None,
        None,
    )
    .await
    .expect("dir");
    let err = store_at_path(
        &h.ctx(),
        "/downloads",
        Fetched {
            url: "https://example.com/a.bin".into(),
            content_type: "application/octet-stream".into(),
            body: b"data".to_vec(),
        },
    )
    .await
    .unwrap_err();
    assert!(err.contains("directory"), "{err}");
}

#[tokio::test]
async fn download_refuses_empty_body() {
    let h = setup().await;
    let err = store_at_path(
        &h.ctx(),
        "/downloads/empty.bin",
        Fetched {
            url: "https://example.com/empty.bin".into(),
            content_type: String::new(),
            body: Vec::new(),
        },
    )
    .await
    .unwrap_err();
    assert!(err.contains("empty"), "{err}");
}

#[test]
fn declarations_name_the_tools() {
    assert_eq!(ReadVnodeTool.declaration().name, "read_vnode");
    assert_eq!(CreateVnodeTool.declaration().name, "create_vnode");
    assert_eq!(DownloadFileTool.declaration().name, "download_file");
    assert_eq!(EditVnodeTool.declaration().name, "edit_vnode");
}
