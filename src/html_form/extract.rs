//! Axum extractor for urlencoded POST bodies (duplicate field names supported).

use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{StatusCode, header},
};
use serde::de::DeserializeOwned;

use super::csrf::{csrf_rejection, verify_form_csrf};
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

        let headers = req.headers().clone();
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

        UrlencodedFields::parse(&bytes)
            .and_then(|fields| {
                verify_form_csrf(&headers, &fields)?;
                fields.deserialize()
            })
            .map(HtmlFormBody)
            .map_err(form_rejection)
    }
}

fn form_rejection(err: FormError) -> (StatusCode, String) {
    if let Some(rej) = csrf_rejection(&err) {
        return rej;
    }
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to deserialize form body: {err}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html_form::{CSRF_COOKIE, CSRF_FIELD, generate_csrf_token};
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use serde::Deserialize;
    use tower::ServiceExt;

    #[derive(Debug, Deserialize)]
    struct NameForm {
        #[serde(rename = "Name")]
        name: String,
    }

    async fn echo(HtmlFormBody(form): HtmlFormBody<NameForm>) -> String {
        form.name
    }

    fn app() -> Router {
        Router::new().route("/", post(echo))
    }

    #[tokio::test]
    async fn accepts_matching_csrf_cookie_and_field() {
        let token = generate_csrf_token();
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .header(header::COOKIE, format!("{CSRF_COOKIE}={token}"))
                    .body(Body::from(format!("{CSRF_FIELD}={token}&Name=Ada")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Ada");
    }

    #[tokio::test]
    async fn rejects_missing_csrf() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("Name=Ada"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rejects_mismatched_csrf() {
        let cookie = generate_csrf_token();
        let field = generate_csrf_token();
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .header(header::COOKIE, format!("{CSRF_COOKIE}={cookie}"))
                    .body(Body::from(format!("{CSRF_FIELD}={field}&Name=Ada")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
