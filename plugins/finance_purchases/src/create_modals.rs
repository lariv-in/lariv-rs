//! Typed [`PickerModal`] wiring for purchase swap keys.

use super::keys::{DraftPurchaseSelectModalKey, DraftPurchaseSelectTableKey};

lariv_core::impl_picker_modal!(DraftPurchaseSelectModalKey, DraftPurchaseSelectTableKey);
