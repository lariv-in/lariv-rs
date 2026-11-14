//! HTTP handlers for assistant test-case CRUD.

use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::list_filters::{FilterField, apply_filters, parse_list_filters, text_column};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::{
    entities::user::{Column as UserColumn, Entity as UserEntity},
    middleware::RequireAuth,
};

use crate::{
    entities::test_case::{self, Entity as TestCaseEntity},
    forms::TestCaseForm,
    keys::{TEST_CASE_CREATE_MODAL, TEST_CASE_EDIT_MODAL, TEST_CASE_TABLE},
    routes::{TestCaseDetailRouteTag, TestCaseListRouteTag},
    state::LlmAssistantTesterState,
    templates::{
        ConfirmDeletePage, TestCaseCreateModalPage, TestCaseDetailPage, TestCaseEditModalPage,
        TestCaseListPage, TestCaseRow,
    },
};

/// Modal opener query (`?name=…&refresh=table-id`).
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;

#[derive(Debug, Deserialize, Default)]
pub struct TestCaseListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn filter_fields() -> Vec<FilterField> {
    vec![text_column(
        "name",
        "Test Case Name",
        test_case::Column::Name,
    )]
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    lariv_core::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

async fn author_display(db: &sea_orm::DatabaseConnection, user_id: i64) -> String {
    lariv_core::web::opt_or_log(
        UserEntity::find_by_id(user_id).one(db).await,
        "find user by id",
    )
    .map(|u| u.name)
    .unwrap_or_default()
}

async fn author_names(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> std::collections::HashMap<i64, String> {
    if ids.is_empty() {
        return std::collections::HashMap::new();
    }
    match UserEntity::find()
        .filter(UserColumn::Id.is_in(ids.to_vec()))
        .all(db)
        .await
    {
        Ok(users) => users.into_iter().map(|user| (user.id, user.name)).collect(),
        Err(e) => {
            tracing::error!(error = %e, "find test case authors");
            std::collections::HashMap::new()
        }
    }
}

fn name_error(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        Some("Test case name is required.".into())
    } else {
        None
    }
}

async fn load_page(
    db: &sea_orm::DatabaseConnection,
    q: &TestCaseListQuery,
    condition: sea_orm::Condition,
    tz: &str,
) -> ObjectList<TestCaseRow> {
    let query = TestCaseEntity::find().filter(condition);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(test_case::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(test_case::Column::Name)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt DESC") => {
            query.order_by_desc(test_case::Column::UpdatedAt)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt ASC") || s.eq_ignore_ascii_case("UpdatedAt") => {
            query.order_by_asc(test_case::Column::UpdatedAt)
        }
        _ => query.order_by_desc(test_case::Column::Id),
    };

    let page = q.page.get();
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let author_ids: Vec<i64> = models.iter().map(|case| case.created_by_id).collect();
    let authors = author_names(db, &author_ids).await;
    let rows = models
        .into_iter()
        .map(|case| TestCaseRow {
            author_name: authors
                .get(&case.created_by_id)
                .cloned()
                .unwrap_or_default(),
            id: case.id,
            name: case.name,
            updated_at: format_updated_at(case.updated_at, tz),
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

/// HTTP handler: list test cases.
pub async fn list(
    Cap(state): Cap<LlmAssistantTesterState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TestCaseListQuery>,
) -> maud::Markup {
    let fields = filter_fields();
    let clauses = parse_list_filters(uri.query().unwrap_or(""));
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    let cases = load_page(&state.db, &q, applied.condition, &ctx.timezone).await;
    let page = TestCaseListPage {
        cases,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    if htmx.targets(TEST_CASE_TABLE) {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: test case detail.
pub async fn detail(
    Cap(state): Cap<LlmAssistantTesterState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(case) = lariv_core::web::opt_or_log(
        TestCaseEntity::find_by_id(id).one(&state.db).await,
        "find test case by id",
    ) else {
        return Redirect::to(&TestCaseListRouteTag.url()).into_response();
    };
    let author_name = author_display(&state.db, case.created_by_id).await;
    let page = TestCaseDetailPage {
        id: case.id,
        name: case.name,
        definition: case.definition,
        solution: case.solution,
        author_name,
        updated_at: format_updated_at(case.updated_at, &ctx.timezone),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: create-modal GET.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = TestCaseCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        definition: String::new(),
        solution: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: create test case.
pub async fn create_post(
    Cap(state): Cap<LlmAssistantTesterState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TestCaseForm>,
) -> Response {
    let name = form.name.trim().to_string();
    if let Some(error) = name_error(&name) {
        let page = TestCaseCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            name,
            definition: form.definition,
            solution: form.solution,
            error,
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let model = test_case::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        definition: Set(form.definition.clone()),
        solution: Set(form.solution.clone()),
        created_by_id: Set(ctx.user.id),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done(
            &htmx,
            TEST_CASE_CREATE_MODAL,
            &q.refresh_table(),
            &TestCaseDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = TestCaseCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                definition: form.definition,
                solution: form.solution,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: edit-modal GET.
pub async fn edit_get(
    Cap(state): Cap<LlmAssistantTesterState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(case) = lariv_core::web::opt_or_log(
        TestCaseEntity::find_by_id(id).one(&state.db).await,
        "find test case by id",
    ) else {
        return Redirect::to(&TestCaseListRouteTag.url()).into_response();
    };
    let page = TestCaseEditModalPage {
        id: case.id,
        form_name: q.form_name(),
        name: case.name,
        definition: case.definition,
        solution: case.solution,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: update test case.
pub async fn edit_post(
    Cap(state): Cap<LlmAssistantTesterState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TestCaseForm>,
) -> Response {
    let Some(case) = lariv_core::web::opt_or_log(
        TestCaseEntity::find_by_id(id).one(&state.db).await,
        "find test case by id",
    ) else {
        return Redirect::to(&TestCaseListRouteTag.url()).into_response();
    };
    let name = form.name.trim().to_string();
    if let Some(error) = name_error(&name) {
        let page = TestCaseEditModalPage {
            id,
            form_name: q.form_name(),
            name,
            definition: form.definition,
            solution: form.solution,
            error,
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let mut am: test_case::ActiveModel = case.into();
    am.name = Set(name);
    am.definition = Set(form.definition.clone());
    am.solution = Set(form.solution.clone());
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done(
            &htmx,
            TEST_CASE_EDIT_MODAL,
            &TestCaseDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = TestCaseEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                definition: form.definition,
                solution: form.solution,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

/// HTTP handler: delete-modal GET.
pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: delete test case.
pub async fn delete_post(
    Cap(state): Cap<LlmAssistantTesterState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match TestCaseEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&TestCaseListRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete test case");
            let page = ConfirmDeletePage {
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
