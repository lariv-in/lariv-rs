//! Request form structs for OTP.

use serde::Deserialize;

use crate::html_form::{
    html_form,
    widgets::{Email, Password, Phone, Section, Text},
};

#[derive(Debug, Deserialize)]
pub struct IdentifierForm {
    #[serde(rename = "Identifier", alias = "identifier")]
    pub identifier: String,
}

#[html_form]
pub struct PhoneIdentifierForm {
    #[form(label = "Phone Number", widget = Phone, required)]
    pub identifier: String,
}

#[html_form]
pub struct EmailIdentifierForm {
    #[form(label = "Email Address", widget = Email, required)]
    pub identifier: String,
}

#[html_form]
pub struct VerifyForm {
    #[form(label = "OTP", required, widget = Text)]
    pub otp: String,

    #[form(label = "New password", widget = Password, required)]
    pub new_password: String,

    #[form(label = "Confirm new password", widget = Password, required)]
    pub new_password2: String,
}

#[html_form(default)]
pub struct PreferencesForm {
    #[form(widget = Section, label = "SMS OTP Settings")]
    _section_sms: (),

    #[form(label = "MSG91 Auth Key", widget = Text)]
    pub msg91_auth_key: String,

    #[form(label = "SMS OTP Template ID", widget = Text, row = "sms_tpl")]
    pub sms_otp_template_id: String,

    #[form(label = "General OTP Template ID (Fallback)", widget = Text, row = "sms_tpl")]
    pub otp_template_id: String,

    #[form(label = "SMS OTP Field Name", widget = Text)]
    pub sms_otp_field_name: String,

    #[form(label = "SMS OTP Extra Fields (JSON)", widget = Text)]
    pub sms_otp_extra_fields: String,

    #[form(widget = Section, label = "Email OTP Settings")]
    _section_email: (),

    #[form(label = "Email OTP Template String", widget = Text)]
    pub email_otp_template_string: String,

    #[form(label = "SMTP Host", widget = Text, row = "smtp_host")]
    pub smtp_host: String,

    #[form(label = "SMTP Port", widget = Text, row = "smtp_host")]
    pub smtp_port: String,

    #[form(label = "SMTP Username", widget = Text, row = "smtp_user")]
    pub smtp_username: String,

    #[form(label = "SMTP Password", widget = Text, row = "smtp_user")]
    pub smtp_password: String,

    #[form(label = "SMTP From Address", widget = Text)]
    pub smtp_from: String,
}
