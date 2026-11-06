//! Load and save type-specific rows behind a document.
//!
//! Callers only pass a [`document::Model`]. `document_type` selects the table
//! and `document_type_id` is that table's primary key.

pub mod aadhar;
pub mod document;
pub mod pan;
pub mod passport;
pub mod types;

pub use aadhar::normalize_aadhar_number;
pub use document::{
    create_document, delete_document, load_type_fields, load_type_summaries, update_document,
    validate_document_form,
};
pub use types::{TypeFields, TypeSummary};
