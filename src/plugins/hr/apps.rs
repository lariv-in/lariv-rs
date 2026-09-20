use crate::apps::define_register_apps;

define_register_apps! {
    plugin: HrTag;
    key: "p_hr";
    name: "HR";
    href: "/hr/applicants";
    icon: "user-group";
    roles: ["superuser", "applicant", "probation", "employee", "ex-employee"];
}
