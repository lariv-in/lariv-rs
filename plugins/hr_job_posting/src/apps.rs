lariv_core::define_register_apps! {
    plugin: HrJobPostingTag;
    key: "p_hr_job_posting";
    name: "HR job posting";
    href: crate::routes::JobFormListRouteTag.url();
    icon: "briefcase";
    plugin_type: lariv_core::apps::PluginType::Addon;
    roles: [];
}
