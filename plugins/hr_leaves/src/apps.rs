lariv_core::define_register_apps! {
    plugin: HrLeavesTag;
    key: "p_hr_leaves";
    name: "HR leaves";
    href: crate::routes::LeaveListRouteTag.url();
    icon: "calendar";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
