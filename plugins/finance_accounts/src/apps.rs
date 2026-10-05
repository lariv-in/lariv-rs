pub const ACCOUNTING_APP_KEY: &str = "p_finance_accounts";

lariv_core::define_register_apps! {
    plugin: FinanceAccountsTag;
    key: ACCOUNTING_APP_KEY;
    name: "Accounting";
    href: crate::routes::FinanceDefaultRouteTag.url();
    icon: "building-library";
    roles: [];
}
