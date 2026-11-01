lariv_core::define_register_apps! {
    plugin: HrOvertimeTag;
    key: "p_hr_overtime";
    name: "HR overtime";
    href: crate::routes::OvertimeListRouteTag.url();
    icon: "clock";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
