use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: BankAccountsTag;
    key: "p_bank_accounts";
    name: "Bank Accounts";
    href: crate::routes::BankAccountDefaultRouteTag.url();
    icon: "building-library";
    roles: [];
}
