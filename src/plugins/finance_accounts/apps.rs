pub const ACCOUNTING_APP_KEY: &str = "p_finance_accounts";

crate::define_register_apps! {
    plugin: FinanceAccountsTag;
    key: ACCOUNTING_APP_KEY;
    name: "Accounting";
    href: "/finance/";
    icon: "building-library";
    roles: ["superuser"];
}
