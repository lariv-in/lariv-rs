lariv_core::define_register_apps! {
    plugin: HrAttendanceTag;
    key: "p_hr_attendance";
    name: "HR attendance";
    href: crate::routes::AttendanceListRouteTag.url();
    icon: "clock";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
