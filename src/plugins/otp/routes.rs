//! OTP HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.
//!
//! Login paths duplicate users routes so this plugin's handlers win when
//! installed later (see [`crate::http::MountRoutes`]).

use crate::define_plugin_routes;

use super::handlers;

/// OTP preferences. Empty allowlist: superuser only until another plugin patches it.
pub struct OtpPrefsAdmin;

define_plugin_routes! {
    plugin: OtpTag;
    prefix: "/dashboard";
    routes: [
        get OtpLoginGetRouteTag, "/users/login", root handlers::auth::login_get;
        post OtpLoginPostRouteTag, "/users/login", root handlers::auth::login_post;
        get OtpForgotGetRouteTag, "/otp/forgot-password", root handlers::auth::forgot_get;
        get OtpPhoneGetRouteTag, "/otp/login/sms", root handlers::auth::phone_get;
        post OtpPhonePostRouteTag, "/otp/login/sms", root handlers::auth::phone_post;
        get OtpEmailGetRouteTag, "/otp/login/email", root handlers::auth::email_get;
        post OtpEmailPostRouteTag, "/otp/login/email", root handlers::auth::email_post;
        get OtpVerifyGetRouteTag, "/otp/verify", root handlers::auth::verify_get;
        post OtpVerifyPostRouteTag, "/otp/verify", root handlers::auth::verify_post;
        get OtpPrefsGetRouteTag, "/otp/preferences", handlers::preferences::get, authorize(OtpPrefsAdmin, []);
        post OtpPrefsPostRouteTag, "/otp/preferences", handlers::preferences::post, authorize(OtpPrefsAdmin, []);
    ]
}
