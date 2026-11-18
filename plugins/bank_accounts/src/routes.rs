use super::{
    handlers,
    keys::{
        BANK_ACCOUNT_DELETE_MODAL, BANK_ACCOUNT_TABLE, BANK_SELECT_MODAL, BANK_SELECT_TABLE,
        MICR_SELECT_MODAL, MICR_SELECT_TABLE,
    },
};

pub struct BankAccountsView;

pub struct BankAccountsMutate;

lariv_core::define_plugin_routes! {
    plugin: BankAccountsTag;
    prefix: "/dashboard";
    routes: [
        get BankAccountDefaultRouteTag, "/bank-accounts", handlers::bank_accounts::list, fragment(BANK_ACCOUNT_TABLE), authorize(BankAccountsView, []);
        get BankAccountCreateGetRouteTag, "/bank-accounts/create", handlers::bank_accounts::create_get, modal, authorize(BankAccountsMutate, []);
        post BankAccountCreatePostRouteTag, "/bank-accounts/create", handlers::bank_accounts::create_post, authorize(BankAccountsMutate, []);
        get BankAccountDetailRouteTag, "/bank-accounts/{id}", handlers::bank_accounts::detail, authorize(BankAccountsView, []);
        get BankAccountEditGetRouteTag, "/bank-accounts/{id}/edit", handlers::bank_accounts::edit_get, modal, authorize(BankAccountsMutate, []);
        post BankAccountEditPostRouteTag, "/bank-accounts/{id}/edit", handlers::bank_accounts::edit_post, authorize(BankAccountsMutate, []);
        get BankAccountDeleteGetRouteTag, "/bank-accounts/{id}/delete", handlers::bank_accounts::delete_get, modal, authorize(BankAccountsMutate, []);
        post BankAccountDeletePostRouteTag, "/bank-accounts/{id}/delete", bare handlers::bank_accounts::delete_post, fragment(BANK_ACCOUNT_DELETE_MODAL), authorize(BankAccountsMutate, []);
        get BankFkSelectRouteTag, "/banks/pick", handlers::banks::select, fk_select(BANK_SELECT_TABLE, BANK_SELECT_MODAL), authorize(BankAccountsView, []);
        get BankMicrSelectRouteTag, "/banks/micr/pick", handlers::micr::select, fk_select(MICR_SELECT_TABLE, MICR_SELECT_MODAL), authorize(BankAccountsView, []);
    ]
}
