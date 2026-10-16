//! HTTP integration: a to-do page is created, renamed, and deleted from the title the user submits.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use lariv_rs::app::{App, MountedApp};
use lariv_rs::components::{SharedChromeFolder, SlotTag};
use lariv_rs::http::{HttpCapability, HttpTag, MountRoutes, ProvideRequestCaps, into_axum_router};
use lariv_rs::plugins::todo as todo_plugin;
use lariv_rs::plugins::todo::entities::todo::{self as todo_row, Entity as TodoEntity};
use lariv_rs::plugins::todo::entities::todo_page::Entity as TodoPageEntity;
use lariv_rs::plugins::todo::routes::{
    TodoListRouteTag, TodoPageCreateGetRouteTag, TodoPageCreatePostRouteTag,
    TodoPageDeleteGetRouteTag, TodoPageDeletePostRouteTag, TodoPageDetailRouteTag,
    TodoPageEditGetRouteTag, TodoPageEditPostRouteTag,
};
use lariv_rs::plugins::users::{
    self, UsersTag, auth,
    entities::user::{self, Entity as UserEntity},
    null_text::NullText,
    phone::Phone,
    roles::Unassigned,
};
use lariv_rs::traits::get::GetByTag;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectOptions, ConnectionTrait, Database,
    DatabaseBackend, EntityTrait, QueryFilter, Schema,
};
use std::sync::Arc;
use tower::ServiceExt;

fn temp_paths() -> (PathBuf, PathBuf) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir();
    let db = dir.join(format!("lariv-todo-pages-{nanos}.db"));
    let cfg = dir.join(format!("lariv-todo-pages-{nanos}.toml"));
    (db, cfg)
}

fn form_body(csrf: &str, title: Option<&str>) -> String {
    match title {
        Some(title) => format!("csrf_token={csrf}&Title={}", urlencoding::encode(title)),
        None => format!("csrf_token={csrf}"),
    }
}

fn location_of(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn id_from_location(location: &str) -> i64 {
    location
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .parse()
        .expect("page id in redirect")
}

async fn send<M, HttpIdx, Routes, SlotIdx>(
    app: &MountedApp<M>,
    method: &str,
    uri: &str,
    token: &str,
    csrf: &str,
    body: Option<String>,
) -> axum::response::Response
where
    M: GetByTag<HttpTag, HttpIdx, Value = Arc<HttpCapability<Routes>>>,
    M: GetByTag<SlotTag, SlotIdx, Value = SharedChromeFolder>,
    M: ProvideRequestCaps + Clone + Send + Sync + 'static,
    Routes: MountRoutes + Clone,
{
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", format!("auth-token={token}; csrf_token={csrf}"));
    if body.is_some() {
        builder = builder.header("content-type", "application/x-www-form-urlencoded");
    }
    into_axum_router(app)
        .oneshot(builder.body(Body::from(body.unwrap_or_default())).unwrap())
        .await
        .expect("response")
}

#[tokio::test]
async fn create_edit_and_delete_a_page_by_title() {
    let (db_path, cfg_path) = temp_paths();
    let database_url = format!("sqlite://{}?mode=rwc", db_path.display());
    std::fs::write(
        &cfg_path,
        format!(
            r#"
database_url = "{database_url}"
[users]
adminEmail = "admin@test.local"
adminPassword = "adminadmin"
signingKey = "dGVzdC1zaWduaW5nLWtleS1wYWRkZWQtdG8tNjQtYnl0ZXMhISEhISEhISEhISE="
jwtIssuer = "dW5pcXVpdHktdGVzdC1pc3N1ZXItcGFkZGVkLXRvLTY0LWJ5dGVzIQ=="
"#
        ),
    )
    .expect("write temp config");

    // Schema changes on a pooled SQLite connection can see a stale `todos` name.
    // Apply them on one connection, then open the app pool against the same file.
    let mut opt = ConnectOptions::new(&database_url);
    opt.max_connections(1).sqlx_logging(false);
    let setup = Database::connect(opt).await.expect("setup connection");
    let users_table = Schema::new(DatabaseBackend::Sqlite).create_table_from_entity(UserEntity);
    setup.execute(&users_table).await.expect("users table");
    todo_plugin::migrations::migrate(&setup)
        .await
        .expect("todo migrations");
    drop(setup);

    let app = App::new_web_app();
    let app = users::install(app);
    let app = todo_plugin::install(app);
    let app = app.load_config(&cfg_path).await.expect("load_config");
    let _ = std::fs::remove_file(&cfg_path);
    let app = app.mount();

    let users_state = app.get_capability_output::<UsersTag, _>();
    let db = users_state.db.clone();

    user::ActiveModel {
        name: Set("Admin".into()),
        email: Set(NullText::from("admin@test.local")),
        phone: Set(Phone::from("5550100100")),
        role: Set(lariv_rs::plugins::users::roles::Superuser::NAME.into()),
        timezone: Set(NullText::from("UTC")),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("admin user");
    let admin = UserEntity::find()
        .filter(user::Column::Email.eq("admin@test.local"))
        .one(&db)
        .await
        .expect("query admin")
        .expect("admin user");
    let admin_token = auth::login_token(&admin, &users_state.signing_key, &users_state.jwt_issuer)
        .expect("admin jwt");

    let other = user::ActiveModel {
        name: Set("Other".into()),
        email: Set(NullText::from("other@test.local")),
        phone: Set(Phone::from("5550100199")),
        role: Set(Unassigned::NAME.into()),
        timezone: Set(NullText::from("UTC")),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("other user");
    let other_token = auth::login_token(&other, &users_state.signing_key, &users_state.jwt_issuer)
        .expect("other jwt");
    let csrf = lariv_rs::html_form::generate_csrf_token();

    let blank = send(
        &app,
        "POST",
        &TodoPageCreatePostRouteTag.path(),
        &admin_token,
        &csrf,
        Some(form_body(&csrf, Some("   "))),
    )
    .await;
    assert_eq!(blank.status(), StatusCode::OK);
    let blank_body = axum::body::to_bytes(blank.into_body(), 64 * 1024)
        .await
        .expect("blank body");
    let blank_html = String::from_utf8_lossy(&blank_body);
    assert!(blank_html.contains("Title is required."));
    assert!(
        TodoPageEntity::find()
            .all(&db)
            .await
            .expect("pages")
            .is_empty()
    );

    let created = send(
        &app,
        "POST",
        &TodoPageCreatePostRouteTag.path(),
        &admin_token,
        &csrf,
        Some(form_body(&csrf, Some("Groceries"))),
    )
    .await;
    assert_eq!(created.status(), StatusCode::SEE_OTHER);
    let created_at = location_of(&created);
    let page_id = id_from_location(&created_at);
    let page = TodoPageEntity::find_by_id(page_id)
        .one(&db)
        .await
        .expect("query page")
        .expect("created page");
    assert_eq!(page.title, "Groceries");
    assert_eq!(page.created_by_id, admin.id);

    let form = send(
        &app,
        "GET",
        &TodoPageCreateGetRouteTag.path(),
        &admin_token,
        &csrf,
        None,
    )
    .await;
    assert_eq!(form.status(), StatusCode::OK);
    let form_bytes = axum::body::to_bytes(form.into_body(), 64 * 1024)
        .await
        .expect("create form");
    let form_html = String::from_utf8_lossy(&form_bytes);
    assert!(form_html.contains("New page"));
    assert!(form_html.contains("name=\"Title\""));

    let detail = send(
        &app,
        "GET",
        &TodoPageDetailRouteTag::new(page_id).path(),
        &admin_token,
        &csrf,
        None,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_bytes = axum::body::to_bytes(detail.into_body(), 64 * 1024)
        .await
        .expect("detail");
    let detail_html = String::from_utf8_lossy(&detail_bytes);
    assert!(detail_html.contains("mx-auto"));
    assert!(detail_html.contains("max-w-2xl"));
    assert!(detail_html.contains("p_todo.TodoPageEditForm"));
    assert!(!detail_html.contains("p_todo.TodoPageDeleteForm"));
    let list_at = detail_html.find("id=\"todo-list\"").expect("todo list");
    let add_at = detail_html.find("id=\"todo-add\"").expect("add button");
    let status_at = detail_html.find("id=\"todo-status\"").expect("status");
    assert!(list_at < add_at && add_at < status_at);
    assert!(detail_html[add_at..status_at].contains("w-full"));

    let edit = send(
        &app,
        "GET",
        &TodoPageEditGetRouteTag::new(page_id).path(),
        &admin_token,
        &csrf,
        None,
    )
    .await;
    assert_eq!(edit.status(), StatusCode::OK);
    let edit_bytes = axum::body::to_bytes(edit.into_body(), 64 * 1024)
        .await
        .expect("edit form");
    let edit_html = String::from_utf8_lossy(&edit_bytes);
    assert!(edit_html.contains("p_todo.TodoPageDeleteForm"));

    let renamed = send(
        &app,
        "POST",
        &TodoPageEditPostRouteTag::new(page_id).path(),
        &admin_token,
        &csrf,
        Some(form_body(&csrf, Some("Groceries and milk"))),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::SEE_OTHER);
    assert!(location_of(&renamed).contains(&page_id.to_string()));
    let page = TodoPageEntity::find_by_id(page_id)
        .one(&db)
        .await
        .expect("query page")
        .expect("renamed page");
    assert_eq!(page.title, "Groceries and milk");

    let stolen = send(
        &app,
        "POST",
        &TodoPageEditPostRouteTag::new(page_id).path(),
        &other_token,
        &csrf,
        Some(form_body(&csrf, Some("Stolen"))),
    )
    .await;
    assert_eq!(stolen.status(), StatusCode::SEE_OTHER);
    assert_eq!(location_of(&stolen), TodoListRouteTag.url());
    let page = TodoPageEntity::find_by_id(page_id)
        .one(&db)
        .await
        .expect("query page")
        .expect("page still owned");
    assert_eq!(page.title, "Groceries and milk");

    let refused_delete = send(
        &app,
        "POST",
        &TodoPageDeletePostRouteTag::new(page_id).path(),
        &other_token,
        &csrf,
        Some(form_body(&csrf, None)),
    )
    .await;
    assert_eq!(refused_delete.status(), StatusCode::SEE_OTHER);
    assert!(
        TodoPageEntity::find_by_id(page_id)
            .one(&db)
            .await
            .expect("query page")
            .is_some()
    );

    todo_row::ActiveModel {
        user_id: Set(admin.id),
        todo_page_id: Set(page_id),
        title: Set("Milk".into()),
        checked: Set(false),
        title_rev: Set(0),
        client_id: Set(Some("client-milk".into())),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("todo");

    let confirm = send(
        &app,
        "GET",
        &TodoPageDeleteGetRouteTag::new(page_id).path(),
        &admin_token,
        &csrf,
        None,
    )
    .await;
    assert_eq!(confirm.status(), StatusCode::OK);
    let confirm_bytes = axum::body::to_bytes(confirm.into_body(), 64 * 1024)
        .await
        .expect("delete form");
    let confirm_html = String::from_utf8_lossy(&confirm_bytes);
    assert!(confirm_html.contains("Delete page"));

    let deleted = send(
        &app,
        "POST",
        &TodoPageDeletePostRouteTag::new(page_id).path(),
        &admin_token,
        &csrf,
        Some(form_body(&csrf, None)),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::SEE_OTHER);
    assert_eq!(location_of(&deleted), TodoListRouteTag.url());
    assert!(
        TodoPageEntity::find_by_id(page_id)
            .one(&db)
            .await
            .expect("query page")
            .is_none()
    );
    assert!(
        TodoEntity::find()
            .filter(todo_row::Column::TodoPageId.eq(page_id))
            .all(&db)
            .await
            .expect("todos")
            .is_empty()
    );

    let _ = std::fs::remove_file(&db_path);
}
