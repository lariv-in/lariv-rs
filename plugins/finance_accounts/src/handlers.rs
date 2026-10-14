pub mod accounts;
pub mod currencies;
pub mod custom_source_docs;
pub mod journal_entries;
pub mod journals;
pub mod preferences;
pub mod source_docs;

mod util;

/// Modal opener query (`?name=…&refresh=table-id`). Case-sensitive vs filter `Name`.
pub use lariv_core::web::ModalFormQuery as ModalNameQuery;
