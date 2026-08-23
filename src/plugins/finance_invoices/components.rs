//! Invoice UI fragments.

mod input_invoice_lines_draft;
mod input_payment_batch_allocations;
mod input_payment_term_lines_draft;

pub use input_invoice_lines_draft::{
    InputInvoiceLinesDraft, field_invoice_lines, input_invoice_lines_draft,
};
pub use input_payment_batch_allocations::{
    InputPaymentBatchAllocations, input_payment_batch_allocations,
};
pub use input_payment_term_lines_draft::{
    INVOICE_PAYMENT_TERM_DATE_KINDS, InputPaymentTermLinesDraft, PaymentTermDateKindOption,
    field_payment_term_lines, input_payment_term_lines_draft,
};
