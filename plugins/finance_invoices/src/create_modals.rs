//! Typed [`PickerModal`] wiring for invoice swap keys.

use super::keys::{DraftInvoiceSelectModalKey, DraftInvoiceSelectTableKey};

lariv_core::impl_picker_modal!(DraftInvoiceSelectModalKey, DraftInvoiceSelectTableKey);
