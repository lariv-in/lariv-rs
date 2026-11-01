pub mod document;
pub mod editor;
pub mod mail;
pub mod number;
pub mod pdf;
pub mod prefs;
pub mod tax;

pub use document::{
    LineInput, copy_quotation, create_quotation, delete_quotation, parse_lines, update_quotation,
};
pub use number::resolve_quotation_number;
