pub mod hub;
pub mod drafts;
pub mod posted;
pub mod cancelled;
pub mod payments;
pub mod payment_batches;
pub mod settlements;
pub mod preferences;
pub mod pdf;
pub mod invoice_pdf_preview;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use crate::web::ModalFormQuery as ModalNameQuery;
