//! Typed [`CreateModal`] wiring for account swap keys.

use lariv_core::web::CreateModal;

use crate::keys::ACCOUNT_CREATE_MODAL;
use crate::routes::{AccountCreateGetRouteTag, AccountCreatePostRouteTag};

pub const ACCOUNT_CREATE: CreateModal<AccountCreateGetRouteTag, AccountCreatePostRouteTag> =
    CreateModal {
        key: ACCOUNT_CREATE_MODAL,
        form_name: "p_finance_accounts.AccountCreateForm",
        get: AccountCreateGetRouteTag,
        post: AccountCreatePostRouteTag,
    };

#[cfg(test)]
mod tests {
    use super::ACCOUNT_CREATE;
    use crate::keys::{ACCOUNT_CREATE_MODAL, ACCOUNT_TABLE};
    use lariv_core::web::{CreateModal, modal_create_href_for_picker, modal_create_href_for_table};

    #[test]
    fn account_picker_create_href_keeps_parent() {
        let href = modal_create_href_for_picker(
            "/finance/accounts/create/?ParentID=7",
            ACCOUNT_CREATE.form_name,
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
        let href = modal_create_href_for_table(
            "/finance/accounts/create/?ParentID=7",
            ACCOUNT_CREATE.form_name,
            ACCOUNT_TABLE,
        );
        assert!(href.contains("ParentID=7"), "{href}");
        assert!(
            href.contains("name=p_finance_accounts.AccountCreateForm"),
            "{href}"
        );
        assert!(href.contains("refresh=finance-accounts-table"), "{href}");
    }
}
