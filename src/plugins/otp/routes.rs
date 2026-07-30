//! OTP HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{
    OtpTag, handlers,
    templates::{
        EmailOtpRequestPage, ForgotPasswordPage, OtpEmailRequestPageTag, OtpForgotPasswordPageTag,
        OtpPhoneRequestPageTag, OtpPreferencesPage, OtpPreferencesPageTag, OtpVerifyPage,
        OtpVerifyPageTag, PhoneOtpRequestPage,
    },
};

define_plugin_routes! {
    plugin: OtpTag;
    proof: OtpRoutesProof;
    pages: [
        pane ForgotIdx, ForgotP => OtpForgotPasswordPageTag, ForgotPasswordPage;
        pane PhoneIdx, PhoneP => OtpPhoneRequestPageTag, PhoneOtpRequestPage;
        pane EmailIdx, EmailP => OtpEmailRequestPageTag, EmailOtpRequestPage;
        pane VerifyIdx, VerifyP => OtpVerifyPageTag, OtpVerifyPage;
        pane PrefsIdx, PrefsP => OtpPreferencesPageTag, OtpPreferencesPage;
    ];
    routes: [
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
