lariv_core::define_register_apps! {
    plugin: FinanceCreditnotesTag;
    key: "p_finance_creditnotes";
    name: "Finance credit notes";
    href: crate::routes::CreditNoteDefaultRouteTag.url();
    icon: "document-minus";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
