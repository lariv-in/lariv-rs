use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenaiError {
    #[error("Gemini API key is not configured")]
    MissingApiKey,
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Gemini API error: status {status}, body: {body}")]
    Api { status: u16, body: String },
    #[error("Gemini API returned error: {message}")]
    ApiMessage { message: String },
    #[error("empty model response (no candidates)")]
    EmptyResponse,
    #[error("failed to parse response JSON: {0}")]
    Json(String),
}
