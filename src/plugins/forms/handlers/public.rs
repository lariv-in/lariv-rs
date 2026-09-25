use axum::{
    body::Body,
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::{html_form::HtmlFormBody, http::Cap};

use crate::plugins::filesystem::state::FilesystemState;
use crate::plugins::forms::{
    access_status::AccessStatus,
    color::{contrast_content_hex, u24_to_hex},
    entities::form::Model as Form,
    entities::form_response,
    forms::PublicFormResponseForm,
    handlers::forms::find_form_by_uid,
    logic::{
        answers::{answers_to_json, parse_answers_json},
        questions::questions_to_json,
    },
    routes::FormPublicBackgroundRouteTag,
    state::FormsState,
    templates::{PublicFormClosedPage, PublicFormLook, PublicFormPage, PublicFormThanksPage},
};

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Form not found").into_response()
}

fn parse_uid(raw: &str) -> Option<Uuid> {
    Uuid::parse_str(raw.trim()).ok()
}

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn public_look(form: &Form) -> PublicFormLook {
    PublicFormLook {
        accent_color_hex: u24_to_hex(form.accent_color),
        accent_content_hex: contrast_content_hex(form.accent_color).to_string(),
        background_url: form
            .background_vnode_id
            .filter(|id| *id > 0)
            .map(|_| FormPublicBackgroundRouteTag::new(form.uid.to_string()).url()),
    }
}

fn closed_page(form: &Form) -> Response {
    PublicFormClosedPage {
        title: form.title.clone(),
        look: public_look(form),
    }
    .render()
    .into_response()
}

fn fill_page(
    form: &Form,
    questions_json: String,
    body: &PublicFormResponseForm,
    error: String,
) -> Response {
    PublicFormPage {
        uid: form.uid.to_string(),
        title: form.title.clone(),
        description: form.description.clone(),
        look: public_look(form),
        name: body.name.clone(),
        email: body.email.clone(),
        answers_json: body.answers_json.clone(),
        questions_json,
        error,
    }
    .render()
    .into_response()
}

pub async fn get(Cap(state): Cap<FormsState>, Path(uid): Path<String>) -> Response {
    let Some(uid) = parse_uid(&uid) else {
        return not_found();
    };
    let Some(form) = find_form_by_uid(&state.db, uid).await else {
        return not_found();
    };
    if !form.access_status.accepts_submissions() {
        return closed_page(&form);
    }
    PublicFormPage {
        uid: form.uid.to_string(),
        title: form.title.clone(),
        description: form.description.clone(),
        look: public_look(&form),
        name: String::new(),
        email: String::new(),
        answers_json: answers_to_json(&Default::default()),
        questions_json: questions_to_json(&form.questions),
        error: String::new(),
    }
    .render()
    .into_response()
}

pub async fn post(
    Cap(state): Cap<FormsState>,
    Path(uid): Path<String>,
    HtmlFormBody(body): HtmlFormBody<PublicFormResponseForm>,
) -> Response {
    let Some(uid) = parse_uid(&uid) else {
        return not_found();
    };
    let Some(form) = find_form_by_uid(&state.db, uid).await else {
        return not_found();
    };
    if form.access_status != AccessStatus::AnyoneWithLink {
        return closed_page(&form);
    }
    let questions_json = questions_to_json(&form.questions);
    let answers = match parse_answers_json(&body.answers_json, &form.questions) {
        Ok(a) => a,
        Err(error) => {
            return fill_page(&form, questions_json, &body, error);
        }
    };
    let model = form_response::ActiveModel {
        id: Default::default(),
        form_id: Set(form.id),
        answers: Set(answers),
        submitted_at: Set(Utc::now()),
        name: Set(opt_string(body.name.clone())),
        email: Set(opt_string(body.email.clone())),
    };
    match model.insert(&state.db).await {
        Ok(_) => PublicFormThanksPage {
            title: form.title.clone(),
            look: public_look(&form),
        }
        .render()
        .into_response(),
        Err(e) => fill_page(&form, questions_json, &body, e.to_string()),
    }
}

/// Unauthenticated inline image for a public form's background VNode.
pub async fn background(
    Cap(state): Cap<FormsState>,
    Cap(fs): Cap<FilesystemState>,
    Path(uid): Path<String>,
) -> Response {
    let Some(uid) = parse_uid(&uid) else {
        return not_found();
    };
    let Some(form) = find_form_by_uid(&state.db, uid).await else {
        return not_found();
    };
    let Some(vnode_id) = form.background_vnode_id.filter(|id| *id > 0) else {
        return not_found();
    };
    let Some(node) = crate::web::opt_or_log(
        crate::plugins::filesystem::node::get_by_id(&fs.db, vnode_id).await,
        "get form background vnode",
    ) else {
        return not_found();
    };
    if node.is_directory {
        return not_found();
    }
    let Some(path) = node.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return not_found();
    };
    match fs.store.open(path, &node.name).await {
        Ok(mut download) => {
            let mut buf = Vec::new();
            if download.reader.read_to_end(&mut buf).await.is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            let mut response = Response::new(Body::from(buf));
            if let Ok(v) = HeaderValue::from_str(&download.content_type) {
                response.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            let filename = download.filename.replace('"', "");
            if let Ok(v) = HeaderValue::from_str(&format!("inline; filename=\"{filename}\"")) {
                response
                    .headers_mut()
                    .insert(header::CONTENT_DISPOSITION, v);
            }
            response
        }
        Err(_) => not_found(),
    }
}
