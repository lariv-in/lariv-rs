//! Persist round-trip tests (sqlite memory).

use sea_orm::{ConnectionTrait, Database, Schema, Statement};

use crate::plugins::llm_assistant::{
    content::{
        load_session_contents,
        sanitize::{ZWSP, sanitize_content_parts_for_genai_chat},
        save_content,
    },
    entities::{part_text, session, session_message, session_message_part, video_metadata},
    genai::{Content, Part, Role},
};

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("sqlite memory");
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    // Minimal users table for session FK (SQLite does not enforce unless enabled).
    db.execute(Statement::from_string(
        backend,
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)".to_string(),
    ))
    .await
    .expect("users");
    db.execute(Statement::from_string(
        backend,
        "INSERT INTO users (id) VALUES (1)".to_string(),
    ))
    .await
    .expect("user row");

    for stmt in [
        schema.create_table_from_entity(session::Entity),
        schema.create_table_from_entity(session_message::Entity),
        schema.create_table_from_entity(video_metadata::Entity),
        schema.create_table_from_entity(session_message_part::Entity),
        schema.create_table_from_entity(part_text::Entity),
    ] {
        db.execute(backend.build(&stmt))
            .await
            .expect("create table");
    }
    db
}

async fn create_session(db: &sea_orm::DatabaseConnection) -> i64 {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};
    let now = Utc::now();
    session::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(String::new()),
        user_id: Set(1),
        reply_email: Set(None),
        email_message_id: Set(None),
        email_references: Set(None),
    }
    .insert(db)
    .await
    .expect("session")
    .id
}

#[tokio::test]
async fn save_load_text_round_trip() {
    let db = setup_db().await;
    let sid = create_session(&db).await;

    save_content(&db, sid, &Content::text(Role::User, "hello"))
        .await
        .expect("save user");
    save_content(&db, sid, &Content::text(Role::Model, "world"))
        .await
        .expect("save model");

    let contents = load_session_contents(&db, sid).await.expect("load");
    assert_eq!(contents.len(), 2);
    assert_eq!(contents[0].role, Role::User);
    assert_eq!(contents[0].parts[0].text.as_deref(), Some("hello"));
    assert_eq!(contents[1].role, Role::Model);
    assert_eq!(contents[1].parts[0].text.as_deref(), Some("world"));
}

#[tokio::test]
async fn thought_only_reloads_with_zwsp() {
    let db = setup_db().await;
    let sid = create_session(&db).await;

    let content = Content {
        role: Role::Model,
        parts: vec![Part {
            thought: true,
            ..Default::default()
        }],
    };
    save_content(&db, sid, &content).await.expect("save");

    let mut loaded = load_session_contents(&db, sid).await.expect("load");
    assert_eq!(loaded.len(), 1);
    // load_session_contents already sanitizes
    assert_eq!(loaded[0].parts[0].text.as_deref(), Some(ZWSP));

    // also verify sanitize helper alone
    loaded[0].parts[0].text = None;
    sanitize_content_parts_for_genai_chat(&mut loaded[0]);
    assert_eq!(loaded[0].parts[0].text.as_deref(), Some(ZWSP));
}
