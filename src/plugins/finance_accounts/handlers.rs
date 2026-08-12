pub mod accounts;
pub mod currencies;
pub mod journal_entries;
pub mod journals;
pub mod preferences;
pub mod source_docs;

mod util;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use crate::web::ModalFormQuery as ModalNameQuery;
