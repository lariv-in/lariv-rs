use crate::apps::define_register_apps;

define_register_apps! {
    plugin: HrTag;
    key: "p_hr";
    name: "HR";
    href: crate::plugins::hr::routes::ApplicantHubRouteTag.url();
    icon: "user-group";
    roles: [
        crate::plugins::hr::roles::Applicant,
        crate::plugins::hr::roles::Probation,
        crate::plugins::hr::roles::Employee,
        crate::plugins::hr::roles::ExEmployee
    ];
}
