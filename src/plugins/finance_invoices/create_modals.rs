//! Typed [`PickerModal`] wiring for invoice swap keys.

use super::keys::{DraftInvoiceSelectModalKey, DraftInvoiceSelectTableKey};

crate::impl_picker_modal!(DraftInvoiceSelectModalKey, DraftInvoiceSelectTableKey);
