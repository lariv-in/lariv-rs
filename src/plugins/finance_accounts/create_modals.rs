//! Typed [`CreateModal`] / [`PickerModal`] wiring for account swap keys.

use super::keys::{AccountCreateModalKey, AccountSelectModalKey, AccountSelectTableKey};
use super::routes::{AccountCreateGetRouteTag, AccountCreatePostRouteTag};

crate::impl_create_modal!(
    AccountCreateModalKey,
    AccountCreateGetRouteTag,
    AccountCreatePostRouteTag,
    "p_finance_accounts.AccountCreateForm"
);
crate::impl_picker_modal!(AccountSelectModalKey, AccountSelectTableKey);
