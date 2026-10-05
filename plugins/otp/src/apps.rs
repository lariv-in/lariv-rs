//! OTP app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: OtpTag;
    key: "p_otp";
    name: "OTP Preferences";
    href: crate::routes::OtpPrefsGetRouteTag.url();
    icon: "key";
    roles: [];
}
