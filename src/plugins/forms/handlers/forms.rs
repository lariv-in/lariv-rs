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

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{
        entities::user::Entity as UserEntity,
        middleware::RequireAuth,
        state::AuthContext,
    },
    template::RenderAppPane,
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::forms::{
    entities::form::{self, Entity as FormEntity},
    forms::SurveyForm,
    handlers::ModalNameQuery,
    handlers::responses::{FormResponseListQuery, load_responses_page},
    keys::{
        FormCreateModalKey, FormDeleteModalKey, FormDetailResponsesTableKey, FormEditModalKey,
        FormSelectModalKey, FormSelectTableKey, FormTableKey,
    },
    logic::questions::{parse_questions_json, questions_editor_json, questions_to_json},
    routes::{FormDetailRouteTag, FormListRouteTag},
    state::FormsState,
    templates::{
        ConfirmDeletePage, FormCreateModalPage, FormDetailPage, FormEditModalPage, FormListPage,
        FormOption, FormRow, FormSelectPage,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct FormDetailQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "Email", alias = "email")]
    pub email: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, Deserialize, Default)]
pub struct FormListQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn forms_list_url() -> String {
    FormListRouteTag.url()
}

async fn author_display(db: &sea_orm::DatabaseConnection, user_id: i64) -> String {
    crate::web::opt_or_log(UserEntity::find_by_id(user_id).one(db).await, "find user by id")
        .map(|u| u.name)
        .unwrap_or_default()
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

async fn query_forms(
    db: &sea_orm::DatabaseConnection,
    q: &FormListQuery,
) -> (Vec<form::Model>, u32, u64) {
    let mut query = FormEntity::find();
    let title = q.title.clone().unwrap_or_default();
    if !title.is_empty() {
        query = query.filter(form::Column::Title.contains(&title));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Title DESC") => query.order_by_desc(form::Column::Title),
        s if s.eq_ignore_ascii_case("Title ASC") || s.eq_ignore_ascii_case("Title") => {
            query.order_by_asc(form::Column::Title)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt DESC") => {
            query.order_by_desc(form::Column::UpdatedAt)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt ASC") || s.eq_ignore_ascii_case("UpdatedAt") => {
            query.order_by_asc(form::Column::UpdatedAt)
        }
        _ => query.order_by_desc(form::Column::Id),
    };
    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn load_forms_page(
    db: &sea_orm::DatabaseConnection,
    q: &FormListQuery,
    tz: &str,
) -> ObjectList<FormRow> {
    let (models, page, total) = query_forms(db, q).await;
    let mut rows = Vec::with_capacity(models.len());
    for f in models {
        rows.push(FormRow {
            id: f.id,
            title: f.title,
            question_count: f.questions.len(),
            author: author_display(db, f.created_by_id).await,
            updated_at: format_updated_at(f.updated_at, tz),
        });
    }
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

async fn load_form_options_page(
    db: &sea_orm::DatabaseConnection,
    q: &FormListQuery,
) -> ObjectList<FormOption> {
    let (models, page, total) = query_forms(db, q).await;
    let rows = models
        .into_iter()
        .map(|f| FormOption {
            id: f.id,
            title: f.title,
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

pub(crate) async fn find_form(db: &sea_orm::DatabaseConnection, id: i64) -> Option<form::Model> {
    crate::web::opt_or_log(FormEntity::find_by_id(id).one(db).await, "find form by id")
}

fn survey_modal_error(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    id: Option<i64>,
    form: &SurveyForm,
    author_display: &str,
    error: &str,
) -> Response {
    let questions_json = form.questions_json.clone();
    if let Some(id) = id {
        let page = FormEditModalPage {
            id,
            form_name: q.form_name(),
            title: form.title.clone(),
            questions_json,
            created_by_id: form.created_by_id,
            author_display: author_display.to_string(),
            error: error.to_string(),
        };
        return html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response();
    }
    let page = FormCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: form.title.clone(),
        questions_json,
        created_by_id: form.created_by_id,
        author_display: author_display.to_string(),
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn list(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<FormListQuery>,
) -> maud::Markup {
    let forms = load_forms_page(&state.db, &q, &ctx.timezone).await;
    let page = FormListPage {
        forms,
        filter_title: q.title.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    if htmx.targets::<FormTableKey>() {
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

pub async fn select(
    Cap(state): Cap<FormsState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<FormListQuery>,
) -> maud::Markup {
    let forms = load_form_options_page(&state.db, &q).await;
    let page = FormSelectPage {
        forms,
        filter_title: q.title.clone().unwrap_or_default(),
        target_input: q.target_input.clone().unwrap_or_else(|| "form".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<FormSelectTableKey, FormSelectModalKey, _>(&htmx, &page)
}

pub async fn detail(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<FormDetailQuery>,
) -> Response {
    let Some(form) = find_form(&state.db, id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let response_q = FormResponseListQuery {
        form_id: Some(id.to_string()),
        name: q.name.clone(),
        email: q.email.clone(),
        sort: q.sort.clone(),
        page: q.page,
        page_size: q.page_size,
    };
    let responses = load_responses_page(&state.db, &response_q, &ctx.timezone).await;
    let page = FormDetailPage {
        id: form.id,
        title: form.title,
        author: author_display(&state.db, form.created_by_id).await,
        created_at: format_updated_at(form.created_at, &ctx.timezone),
        updated_at: format_updated_at(form.updated_at, &ctx.timezone),
        questions: form.questions.clone(),
        responses,
        filter_name: q.name.clone().unwrap_or_default(),
        filter_email: q.email.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    if htmx.targets::<FormDetailResponsesTableKey>() {
        return page.render_responses_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = FormCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        title: String::new(),
        questions_json: questions_to_json(&Default::default()),
        created_by_id: ctx.user.id,
        author_display: ctx.user.name.clone(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<SurveyForm>,
) -> Response {
    let created_by_id = if form.created_by_id == 0 {
        ctx.user.id
    } else {
        form.created_by_id
    };
    let questions = match parse_questions_json(&form.questions_json) {
        Ok(q) => q,
        Err(error) => {
            return survey_modal_error(
                &chrome,
                &ctx,
                &q,
                None,
                &form,
                &author_display(&state.db, created_by_id).await,
                &error,
            );
        }
    };
    let now = Utc::now();
    let model = form::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(form.title.clone()),
        questions: Set(questions),
        created_by_id: Set(created_by_id),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<FormCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &FormDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => survey_modal_error(
            &chrome,
            &ctx,
            &q,
            None,
            &form,
            &author_display(&state.db, created_by_id).await,
            &e.to_string(),
        ),
    }
}

pub async fn edit_get(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(form) = find_form(&state.db, id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let page = FormEditModalPage {
        id: form.id,
        form_name: q.form_name(),
        title: form.title,
        questions_json: questions_editor_json(&form.questions),
        created_by_id: form.created_by_id,
        author_display: author_display(&state.db, form.created_by_id).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<SurveyForm>,
) -> Response {
    let Some(existing) = find_form(&state.db, id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let created_by_id = if form.created_by_id == 0 {
        existing.created_by_id
    } else {
        form.created_by_id
    };
    let questions = match parse_questions_json(&form.questions_json) {
        Ok(q) => q,
        Err(error) => {
            return survey_modal_error(
                &chrome,
                &ctx,
                &q,
                Some(id),
                &form,
                &author_display(&state.db, created_by_id).await,
                &error,
            );
        }
    };
    let mut am: form::ActiveModel = existing.into();
    am.title = Set(form.title.clone());
    am.questions = Set(questions);
    am.created_by_id = Set(created_by_id);
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<FormEditModalKey>(
            &htmx,
            &FormDetailRouteTag::new(id).url(),
        ),
        Err(e) => survey_modal_error(
            &chrome,
            &ctx,
            &q,
            Some(id),
            &form,
            &author_display(&state.db, created_by_id).await,
            &e.to_string(),
        ),
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: FormDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this form and all its responses?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_forms.FormDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match FormEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&forms_list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete form");
            let page = ConfirmDeletePage {
                modal_uid: FormDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this form and all its responses?".into(),
                form_name: "p_forms.FormDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
