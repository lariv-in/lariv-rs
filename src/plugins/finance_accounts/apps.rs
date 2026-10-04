pub const ACCOUNTING_APP_KEY: &str = "p_finance_accounts";

crate::define_register_apps! {
    plugin: FinanceAccountsTag;
    key: ACCOUNTING_APP_KEY;
    name: "Accounting";
    href: crate::plugins::finance_accounts::routes::FinanceDefaultRouteTag.url();
    icon: "building-library";
    roles: [];
}
