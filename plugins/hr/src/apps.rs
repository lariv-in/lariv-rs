use lariv_core::apps::define_register_apps;

pub const HR_APP_KEY: &str = "p_hr";

define_register_apps! {
    plugin: HrTag;
    key: HR_APP_KEY;
    name: "HR";
    href: crate::routes::HrHomeRouteTag.url();
    icon: "user-group";
    roles: [];
}
