//! OTP HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.
//!
//! Login paths duplicate users routes so this plugin's handlers win when
//! installed later (see [`crate::http::MountRoutes`]).

use crate::define_plugin_routes;

use super::handlers;

define_plugin_routes! {
    plugin: OtpTag;
    routes: [
        get OtpLoginGetRouteTag, "/users/login", handlers::auth::login_get;
        post OtpLoginPostRouteTag, "/users/login", handlers::auth::login_post;
        get OtpForgotGetRouteTag, "/otp/forgot-password", handlers::auth::forgot_get;
        get OtpPhoneGetRouteTag, "/otp/login/sms", handlers::auth::phone_get;
        post OtpPhonePostRouteTag, "/otp/login/sms", handlers::auth::phone_post;
        get OtpEmailGetRouteTag, "/otp/login/email", handlers::auth::email_get;
        post OtpEmailPostRouteTag, "/otp/login/email", handlers::auth::email_post;
        get OtpVerifyGetRouteTag, "/otp/verify", handlers::auth::verify_get;
        post OtpVerifyPostRouteTag, "/otp/verify", handlers::auth::verify_post;
        get OtpPrefsGetRouteTag, "/otp/preferences", handlers::preferences::get;
        post OtpPrefsPostRouteTag, "/otp/preferences", handlers::preferences::post;
    ]
}
