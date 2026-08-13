pub mod cancelled;
pub mod drafts;
pub mod hub;
pub mod invoice_pdf_preview;
pub mod payment_batches;
pub mod payments;
pub mod pdf;
pub mod posted;
pub mod preferences;
pub mod settlements;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use crate::web::ModalFormQuery as ModalNameQuery;
