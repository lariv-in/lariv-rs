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

#[cfg(test)]
mod tests {
    use crate::plugins::finance_accounts::keys::{AccountCreateModalKey, AccountTableKey};
    use crate::web::{CreateModal, modal_create_href_for_picker, modal_create_href_for_table};

    #[test]
    fn account_picker_create_href_keeps_parent() {
        let href = modal_create_href_for_picker(
            "/finance/accounts/create/?ParentID=7",
            AccountCreateModalKey::FORM_NAME,
            "ChildIDs",
        );
        assert!(href.contains("ParentID=7"), "{href}");
        assert!(
            href.contains("name=p_finance_accounts.AccountCreateForm"),
            "{href}"
        );
        assert!(href.contains("target_input=ChildIDs"), "{href}");
    }

    #[test]
    fn account_table_create_href_keeps_parent() {
        let href = modal_create_href_for_table::<AccountTableKey>(
            "/finance/accounts/create/?ParentID=7",
            AccountCreateModalKey::FORM_NAME,
        );
        assert!(href.contains("ParentID=7"), "{href}");
        assert!(
            href.contains("name=p_finance_accounts.AccountCreateForm"),
            "{href}"
        );
        assert!(href.contains("refresh=finance-accounts-table"), "{href}");
    }
}
