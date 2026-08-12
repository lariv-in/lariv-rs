//! Default Typst + Minijinja invoice PDF template shipped with the invoices plugin.

pub const DEFAULT_INVOICE_PDF_TEMPLATE: &str =
    include_str!("templates/example_invoice_pdf_template.typ.tmpl");
