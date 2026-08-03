//! Error types for Gemini API calls.

use thiserror::Error;

/// Errors from [`crate::genai::GenaiClient`] HTTP and JSON handling.
#[derive(Debug, Error)]
pub enum GenaiError {
    /// No API key configured (set plugin `apiKey` or `GOOGLE_API_KEY` / `GEMINI_API_KEY`).
    #[error("Gemini API key is not configured")]
    MissingApiKey,
    /// Underlying reqwest transport failure.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// Non-success HTTP status with response body.
    #[error("Gemini API error: status {status}, body: {body}")]
    Api { status: u16, body: String },
    /// Success HTTP status but JSON `error` field in the response envelope.
    #[error("Gemini API returned error: {message}")]
    ApiMessage { message: String },
    /// Response parsed but contained no candidate content.
    #[error("empty model response (no candidates)")]
    EmptyResponse,
    /// Response body was not valid JSON for the expected schema.
    #[error("failed to parse response JSON: {0}")]
    Json(String),
}
