use sea_orm::EntityTrait;

use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        documents::{logic, routes::DocumentDefaultRouteTag, scope::find_document_scoped},
        filesystem::{
            entities::filesystem_node::Entity as VNodeEntity, state::FilesystemState,
            zip::read_file_bytes,
        },
        users::middleware::RequireAuth,
    },
};

use super::super::{
    cert::SignerIdentity,
    pdf::{is_pdf_bytes, is_pdf_name, sign_pdf, signed_filename},
    scope::find_own_signature,
    state::SigningState,
};
use super::signature::EmptyForm;

pub async fn sign(
    Cap(signing): Cap<SigningState>,
    Cap(files): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    HtmlFormBody(_form): HtmlFormBody<EmptyForm>,
) -> Response {
    let Some(doc) = find_document_scoped(&signing.db, id).await else {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    };
    let fields = match logic::load_type_fields(&signing.db, &doc).await {
        Ok(fields) => fields,
        Err(err) => return (StatusCode::BAD_REQUEST, err).into_response(),
    };
    if !is_pdf_name(&fields.vnode_name) {
        return (StatusCode::BAD_REQUEST, "Only PDF files can be signed").into_response();
    }
    let Some(signature) = find_own_signature(&signing.db, &ctx).await else {
        return (
            StatusCode::BAD_REQUEST,
            "Create a signature before signing this document",
        )
            .into_response();
    };
    let Some(node) = crate::web::opt_or_log(
        VNodeEntity::find_by_id(fields.vnode_id)
            .one(&files.db)
            .await,
        "find document file",
    ) else {
        return (StatusCode::BAD_REQUEST, "Document file was not found").into_response();
    };
    if node.is_directory {
        return (StatusCode::BAD_REQUEST, "Choose a file, not a folder").into_response();
    }
    let bytes = match read_file_bytes(files.store.as_ref(), &node).await {
        Ok(bytes) => bytes,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };
    if !is_pdf_bytes(&bytes) {
        return (StatusCode::BAD_REQUEST, "File is not a PDF").into_response();
    }
    let prefs = match crate::plugins::documents::preferences::load_preferences(&signing.db).await {
        Ok(prefs) => prefs,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };
    let identity = match SignerIdentity::from_preferences(
        &prefs.signing_authority_name,
        prefs.validity_duration,
    ) {
        Ok(identity) => identity,
        Err(err) => return (StatusCode::BAD_REQUEST, err).into_response(),
    };
    let signed = match sign_pdf(&bytes, signing.keys.as_ref(), &signature.key_ref, &identity).await
    {
        Ok(signed) => signed,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };
    let filename = signed_filename(&fields.vnode_name).replace('"', "");
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Body::from(signed),
    )
        .into_response()
}
