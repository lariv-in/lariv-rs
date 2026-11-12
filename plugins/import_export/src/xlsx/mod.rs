//! Read and write catalog-aligned XLSX workbooks.

mod build;
mod parse;

pub use build::build_workbook;
pub use parse::{ParsedSheet, ParsedWorkbook, import_order, parse_workbook};
