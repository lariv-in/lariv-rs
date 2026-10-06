use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: HrTag;
    key: "p_hr";
    name: "HR";
    href: crate::routes::ApplicantHubRouteTag.url();
    icon: "user-group";
    roles: [];
}
