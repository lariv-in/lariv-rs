pub mod movements;
pub mod pdf;
pub mod pdf_preview;
pub mod preferences;
pub mod stocks;

pub use lariv_core::web::ModalFormQuery as ModalNameQuery;

use axum::http::Uri;

pub fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}
