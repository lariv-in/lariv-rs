use crate::apps::define_register_apps;

define_register_apps! {
    plugin: HrTag;
    key: "p_hr";
    name: "HR";
    href: crate::plugins::hr::routes::ApplicantHubRouteTag.url();
    icon: "user-group";
    roles: ["superuser", "applicant", "probation", "employee", "ex-employee"];
}
