use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::forms::{
    entities::{
        form::{self, Entity as FormEntity},
        form_response::{self, Entity as FormResponseEntity},
    },
    forms::FormResponseForm,
    handlers::{forms::find_form, ModalNameQuery},
    keys::{
        FormResponseCreateModalKey, FormResponseDeleteModalKey, FormResponseEditModalKey,
    },
    logic::{
        answers::{answers_to_json, normalize_answers_json, parse_answers_json},
        questions::questions_to_json,
    },
    routes::{FormDetailRouteTag, FormListRouteTag, FormResponseDetailRouteTag},
    state::FormsState,
    templates::{
        AnswerDisplayRow, ConfirmDeletePage, FormResponseCreateModalPage,
        FormResponseDetailPage, FormResponseEditModalPage, FormResponseRow,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct FormResponseListQuery {
    #[serde(default, rename = "FormId", alias = "form_id")]
    pub form_id: Option<String>,
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

fn forms_list_url() -> String {
    FormListRouteTag.url()
}

#[derive(Debug, Deserialize, Default)]
pub struct FormResponseCreateQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default, rename = "FormId", alias = "form_id")]
    pub form_id: Option<i64>,
}

fn parse_form_id(raw: &str) -> Option<i64> {
    raw.trim().parse().ok().filter(|id| *id > 0)
}

async fn form_title(db: &sea_orm::DatabaseConnection, form_id: i64) -> String {
    find_form(db, form_id)
        .await
        .map(|f| f.title)
        .unwrap_or_default()
}

async fn build_forms_catalog(db: &sea_orm::DatabaseConnection) -> String {
    let forms = FormEntity::find()
        .order_by_asc(form::Column::Title)
        .all(db)
        .await
        .unwrap_or_default();
    let map: HashMap<i64, _> = forms
        .into_iter()
        .map(|f| (f.id, f.questions.0.clone()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

pub(crate) async fn query_responses(
    db: &sea_orm::DatabaseConnection,
    q: &FormResponseListQuery,
) -> (Vec<form_response::Model>, u32, u64) {
    let mut query = FormResponseEntity::find();
    if let Some(form_id) = q.form_id.as_deref().and_then(parse_form_id) {
        query = query.filter(form_response::Column::FormId.eq(form_id));
    }
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(form_response::Column::Name.contains(&name));
    }
    let email = q.email.clone().unwrap_or_default();
    if !email.is_empty() {
        query = query.filter(form_response::Column::Email.contains(&email));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("SubmittedAt DESC") => {
            query.order_by_desc(form_response::Column::SubmittedAt)
        }
        s if s.eq_ignore_ascii_case("SubmittedAt ASC")
            || s.eq_ignore_ascii_case("SubmittedAt") =>
        {
            query.order_by_asc(form_response::Column::SubmittedAt)
        }
        s if s.eq_ignore_ascii_case("Name DESC") => {
            query.order_by_desc(form_response::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(form_response::Column::Name)
        }
        _ => query.order_by_desc(form_response::Column::Id),
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

pub(crate) async fn load_responses_page(
    db: &sea_orm::DatabaseConnection,
    q: &FormResponseListQuery,
    tz: &str,
) -> ObjectList<FormResponseRow> {
    let (models, page, total) = query_responses(db, q).await;
    let mut rows = Vec::with_capacity(models.len());
    for r in models {
        rows.push(FormResponseRow {
            id: r.id,
            form_id: r.form_id,
            form_title: form_title(db, r.form_id).await,
            name: r.name.unwrap_or_default(),
            email: r.email.unwrap_or_default(),
            submitted_at: crate::datetime::DatetimeLabel::short(r.submitted_at, tz).into_string(),
        });
    }
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

async fn find_response(db: &sea_orm::DatabaseConnection, id: i64) -> Option<form_response::Model> {
    crate::web::opt_or_log(FormResponseEntity::find_by_id(id).one(db).await, "find response")
}

fn response_modal_error(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    id: Option<i64>,
    form: &FormResponseForm,
    form_display: &str,
    questions_json: &str,
    forms_catalog_json: &str,
    error: &str,
) -> Response {
    if let Some(id) = id {
        let page = FormResponseEditModalPage {
            id,
            form_name: q.form_name(),
            form_id: form.form_id,
            form_display: form_display.to_string(),
            name: form.name.clone(),
            email: form.email.clone(),
            submitted_at: form.submitted_at.clone(),
            answers_json: normalize_answers_json(&form.answers_json),
            questions_json: questions_json.to_string(),
            forms_catalog_json: forms_catalog_json.to_string(),
            error: error.to_string(),
        };
        return html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response();
    }
    let page = FormResponseCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form_id: form.form_id,
        form_display: form_display.to_string(),
        name: form.name.clone(),
        email: form.email.clone(),
        submitted_at: form.submitted_at.clone(),
        answers_json: normalize_answers_json(&form.answers_json),
        questions_json: questions_json.to_string(),
        forms_catalog_json: forms_catalog_json.to_string(),
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn questions_for_form_id(db: &sea_orm::DatabaseConnection, form_id: i64) -> String {
    find_form(db, form_id)
        .await
        .map(|f| questions_to_json(&f.questions))
        .unwrap_or_else(|| "[]".into())
}

pub async fn detail(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(response) = find_response(&state.db, id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let Some(parent) = find_form(&state.db, response.form_id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let answers: Vec<AnswerDisplayRow> = parent
        .questions
        .iter()
        .filter_map(|q| {
            response
                .answers
                .get(&q.form_question_id)
                .map(|a| AnswerDisplayRow {
                    question: q.display_text.clone(),
                    answer: crate::plugins::forms::logic::answers::format_answer(a),
                })
        })
        .collect();
    let page = FormResponseDetailPage {
        id: response.id,
        form_id: response.form_id,
        form_title: parent.title,
        name: response.name.unwrap_or_default(),
        email: response.email.unwrap_or_default(),
        submitted_at: ctx.format_datetime(response.submitted_at).into_string(),
        answers,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<FormsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<FormResponseCreateQuery>,
) -> maud::Markup {
    let catalog = build_forms_catalog(&state.db).await;
    let form_id = q.form_id.filter(|id| *id > 0).unwrap_or(0);
    let form_display = if form_id > 0 {
        form_title(&state.db, form_id).await
    } else {
        String::new()
    };
    let questions_json = if form_id > 0 {
        questions_for_form_id(&state.db, form_id).await
    } else {
        "[]".into()
    };
    let page = FormResponseCreateModalPage {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        form_id,
        form_display,
        name: String::new(),
        email: String::new(),
        submitted_at: ctx.datetime_local_input(Utc::now()).into_string(),
        answers_json: answers_to_json(&Default::default()),
        questions_json,
        forms_catalog_json: catalog,
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
    HtmlFormBody(form): HtmlFormBody<FormResponseForm>,
) -> Response {
    let catalog = build_forms_catalog(&state.db).await;
    let form_display = form_title(&state.db, form.form_id).await;
    let Some(parent) = find_form(&state.db, form.form_id).await else {
        return response_modal_error(
            &chrome,
            &ctx,
            &q,
            None,
            &form,
            &form_display,
            "[]",
            &catalog,
            "Form not found",
        );
    };
    let Some(submitted_at) = ctx.parse_datetime_local_input(&form.submitted_at) else {
        return response_modal_error(
            &chrome,
            &ctx,
            &q,
            None,
            &form,
            &form_display,
            &questions_to_json(&parent.questions),
            &catalog,
            "Invalid submitted date/time",
        );
    };
    let answers = match parse_answers_json(&form.answers_json, &parent.questions) {
        Ok(a) => a,
        Err(error) => {
            return response_modal_error(
                &chrome,
                &ctx,
                &q,
                None,
                &form,
                &form_display,
                &questions_to_json(&parent.questions),
                &catalog,
                &error,
            );
        }
    };
    let model = form_response::ActiveModel {
        id: Default::default(),
        form_id: Set(form.form_id),
        answers: Set(answers),
        submitted_at: Set(submitted_at),
        name: Set(opt_string(form.name.clone())),
        email: Set(opt_string(form.email.clone())),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<FormResponseCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &FormResponseDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => response_modal_error(
            &chrome,
            &ctx,
            &q,
            None,
            &form,
            &form_display,
            &questions_to_json(&parent.questions),
            &catalog,
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
    let Some(response) = find_response(&state.db, id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let catalog = build_forms_catalog(&state.db).await;
    let questions_json = questions_for_form_id(&state.db, response.form_id).await;
    let page = FormResponseEditModalPage {
        id: response.id,
        form_name: q.form_name(),
        form_id: response.form_id,
        form_display: form_title(&state.db, response.form_id).await,
        name: response.name.unwrap_or_default(),
        email: response.email.unwrap_or_default(),
        submitted_at: ctx
            .datetime_local_input(response.submitted_at)
            .into_string(),
        answers_json: answers_to_json(&response.answers),
        questions_json,
        forms_catalog_json: catalog,
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
    HtmlFormBody(form): HtmlFormBody<FormResponseForm>,
) -> Response {
    let Some(existing) = find_response(&state.db, id).await else {
        return Redirect::to(&forms_list_url()).into_response();
    };
    let catalog = build_forms_catalog(&state.db).await;
    let form_display = form_title(&state.db, form.form_id).await;
    let Some(parent) = find_form(&state.db, form.form_id).await else {
        return response_modal_error(
            &chrome,
            &ctx,
            &q,
            Some(id),
            &form,
            &form_display,
            "[]",
            &catalog,
            "Form not found",
        );
    };
    let Some(submitted_at) = ctx.parse_datetime_local_input(&form.submitted_at) else {
        return response_modal_error(
            &chrome,
            &ctx,
            &q,
            Some(id),
            &form,
            &form_display,
            &questions_to_json(&parent.questions),
            &catalog,
            "Invalid submitted date/time",
        );
    };
    let answers = match parse_answers_json(&form.answers_json, &parent.questions) {
        Ok(a) => a,
        Err(error) => {
            return response_modal_error(
                &chrome,
                &ctx,
                &q,
                Some(id),
                &form,
                &form_display,
                &questions_to_json(&parent.questions),
                &catalog,
                &error,
            );
        }
    };
    let mut am: form_response::ActiveModel = existing.into();
    am.form_id = Set(form.form_id);
    am.answers = Set(answers);
    am.submitted_at = Set(submitted_at);
    am.name = Set(opt_string(form.name.clone()));
    am.email = Set(opt_string(form.email.clone()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<FormResponseEditModalKey>(
            &htmx,
            &FormResponseDetailRouteTag::new(id).url(),
        ),
        Err(e) => response_modal_error(
            &chrome,
            &ctx,
            &q,
            Some(id),
            &form,
            &form_display,
            &questions_to_json(&parent.questions),
            &catalog,
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
        modal_uid: FormResponseDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this response?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_forms.FormResponseDeleteForm".into()),
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
    let redirect_url = find_response(&state.db, id)
        .await
        .map(|response| FormDetailRouteTag::new(response.form_id).url())
        .unwrap_or_else(|| forms_list_url());
    match FormResponseEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&redirect_url),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete form response");
            let page = ConfirmDeletePage {
                modal_uid: FormResponseDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this response?".into(),
                form_name: "p_forms.FormResponseDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}
