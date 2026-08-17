use crate::define_register_export;

define_register_export! {
    plugin: super::OtpTag;
    table: "otp_preferences";
    model: "OtpPreferences";
    columns: [
        "id",
        "created_at",
        "updated_at",
        "sms_otp_template_id",
        "otp_template_id",
        "msg91_auth_key",
        "sms_otp_field_name",
        "sms_otp_extra_fields",
        "email_otp_template_string",
        "smtp_host",
        "smtp_port",
        "smtp_username",
        "smtp_password",
        "smtp_from",
    ];
}
