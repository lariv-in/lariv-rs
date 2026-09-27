crate::define_register_apps! {
    plugin: FinanceCreditnotesTag;
    key: "p_finance_creditnotes";
    name: "Finance credit notes";
    href: crate::plugins::finance_creditnotes::routes::CreditNoteDefaultRouteTag.url();
    icon: "document-minus";
    plugin_type: crate::apps::PluginType::Addon;
    roles: ["superuser"];
}
