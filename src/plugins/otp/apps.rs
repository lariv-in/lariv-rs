//! OTP app catalog tile (Go `p_otp` `PluginTypeApp` registration).

use crate::apps::define_register_apps;

use super::OtpTag;

define_register_apps! {
    plugin: OtpTag;
    key: "p_otp";
    name: "OTP Preferences";
    href: "/otp/preferences";
    icon: "key";
    roles: [];
}
