//! OTP app catalog tile.

use crate::apps::define_register_apps;

define_register_apps! {
    plugin: OtpTag;
    key: "p_otp";
    name: "OTP Preferences";
    href: crate::plugins::otp::routes::OtpPrefsGetRouteTag.url();
    icon: "key";
    roles: [];
}
