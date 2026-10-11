pub mod cancelled;
pub mod drafts;
pub mod hub;
pub mod pdf;
pub mod posted;
pub mod preferences;
pub mod purchase_pdf_preview;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;
