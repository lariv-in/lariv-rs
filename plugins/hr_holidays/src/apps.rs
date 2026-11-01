lariv_core::define_register_apps! {
    plugin: HrHolidaysTag;
    key: "p_hr_holidays";
    name: "HR holidays";
    href: crate::routes::HolidayListRouteTag.url();
    icon: "calendar-days";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
