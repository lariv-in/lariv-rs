//! Axum extractor for urlencoded POST bodies (duplicate field names supported).

use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{StatusCode, header},
};
use serde::de::DeserializeOwned;

use super::{FormError, UrlencodedFields};

/// POST body extractor for `#[html_form]` types and flat `*Body` structs.
///
/// Use this instead of [`axum::Form`] whenever the form includes many-to-many fields
/// (`Vec<i64>` / `Vec<String>`), which submit repeated keys (`TaxIds=1&TaxIds=2`).
#[derive(Debug, Clone)]
pub struct HtmlFormBody<T>(pub T);

impl<T> HtmlFormBody<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for HtmlFormBody<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S, T> FromRequest<S> for HtmlFormBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("application/x-www-form-urlencoded") {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected `application/x-www-form-urlencoded` request body".into(),
            ));
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

        UrlencodedFields::parse(&bytes)
            .and_then(|fields| fields.deserialize())
            .map(HtmlFormBody)
            .map_err(form_rejection)
    }
}

fn form_rejection(err: FormError) -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to deserialize form body: {err}"),
    )
}
