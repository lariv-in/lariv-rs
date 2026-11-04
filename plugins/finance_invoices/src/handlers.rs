pub mod cancelled;
pub mod drafts;
pub mod hub;
pub mod invoice_pdf_preview;
pub mod pay;
pub mod pdf;
pub mod posted;
pub mod preferences;
pub mod quotations;
pub mod settlements;
pub mod xlsx;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;
