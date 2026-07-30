use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        AppLayoutKey, ButtonLink, ButtonSubmit, FieldText, FieldTitle, FormOpts, InputEmail,
        InputPassword, InputPhone, InputText, LayoutSidebar, RegisterSlots, ShellAuth, ShellChrome,
        ShellScaffold, SidebarMenu, SidebarMenuBack, SidebarMenuItem, SlotCapability, SwapKey,
        button_link, button_submit, container_column, container_error, container_row, field_text,
        field_title, form, form_hx_post_main, input_email, input_password, input_phone, input_text,
        layout_sidebar, shell_auth, shell_scaffold, sidebar_menu, sidebar_menu_item,
    },
    http::ProvideRequestCaps,
    plugins::users::templates::UsersLoginPageTag,
    tag::Tagged,
    template::{
        RegisterTemplates, RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf,
    },
    traits::replace::MapByTag,
};

use super::OtpTag;

pub struct OtpForgotPasswordPageTag;
pub struct OtpPhoneRequestPageTag;
pub struct OtpEmailRequestPageTag;
pub struct OtpVerifyPageTag;
pub struct OtpPreferencesPageTag;

/// Users login page with OTP “Forgot password?” link (Go `p_otp` login patch).
#[derive(Generic)]
pub struct LoginPageWithForgot {
    pub error: String,
}

impl LoginPageWithForgot {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: "Login",
                        classes: "",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main("/users/login/"),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: html! {
                            (container_error(None, input_email(InputEmail {
                                label: "Email",
                                name: "Email",
                                required: true,
                                ..Default::default()
                            })))
                            (container_error(None, input_password(InputPassword {
                                label: "Password",
                                name: "Password",
                                required: true,
                                ..Default::default()
                            })))
                        },
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Login",
                                classes: "w-full mb-4",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Forgot password?",
                                href: "/otp/forgot-password/",
                                classes: "w-full",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Don't have an account? Sign up",
                                href: "/users/signup/",
                                classes: "w-full",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    }))
                },
            ))
        }
    }
}

impl RenderAppPane for LoginPageWithForgot {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for LoginPageWithForgot {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

impl<T, LoginIdx> RegisterTemplates<OtpTag, LoginIdx> for TemplateCapability<T>
where
    T: frunk::hlist::HList + Clone + ProvideRequestCaps + Send + Sync,
    T: MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>,
    <T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>>::Output:
        frunk::hlist::HList + Clone + ProvideRequestCaps + Send + Sync,
{
    #[allow(
        clippy::type_complexity,
        reason = "OTP pages prepended onto users templates after login replace"
    )]
    type Output = TemplateCapability<
        frunk::HList![
            Tagged<OtpPreferencesPageTag, TemplateOf<OtpPreferencesPage>>,
            Tagged<OtpVerifyPageTag, TemplateOf<OtpVerifyPage>>,
            Tagged<OtpEmailRequestPageTag, TemplateOf<EmailOtpRequestPage>>,
            Tagged<OtpPhoneRequestPageTag, TemplateOf<PhoneOtpRequestPage>>,
            Tagged<OtpForgotPasswordPageTag, TemplateOf<ForgotPasswordPage>>,
            ...<T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>>::Output
        ],
    >;

    fn register_templates(self) -> Self::Output {
        // Go `p_otp` patches `p_users.LoginPage` to insert "Forgot password?".
        self.replace_template::<UsersLoginPageTag, LoginIdx, TemplateOf<LoginPageWithForgot>>(
            |_| TemplateOf::new(),
        )
        .add::<OtpForgotPasswordPageTag, ForgotPasswordPage>()
        .add::<OtpPhoneRequestPageTag, PhoneOtpRequestPage>()
        .add::<OtpEmailRequestPageTag, EmailOtpRequestPage>()
        .add::<OtpVerifyPageTag, OtpVerifyPage>()
        .add::<OtpPreferencesPageTag, OtpPreferencesPage>()
    }
}

fn auth_pane(body: Markup) -> Markup {
    html! {
        (PreEscaped(format!(r#"<div id="{}">"#, AppLayoutKey::ID)))
        (body)
        (PreEscaped("</div>"))
    }
}

fn app_scaffold(chrome: &ShellChrome, sidebar: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        sidebar,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(sidebar: Markup, body: Markup) -> Markup {
    layout_sidebar(LayoutSidebar {
        sidebar,
        content: body,
    })
}

fn scaffold_main(body: Markup) -> Markup {
    use crate::components::layout::layout_main;
    layout_main(body)
}

fn otp_prefs_menu() -> Markup {
    sidebar_menu(SidebarMenu {
        title: "OTP Preferences",
        back: Some(SidebarMenuBack {
            title: "Back to Home",
            url: "/dashboard/",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Preferences",
                url: "/otp/preferences/",
                ..Default::default()
            }))
        },
    })
}

#[derive(Generic)]
pub struct ForgotPasswordPage {}

impl ForgotPasswordPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "w-80",
                html! {
                    (container_row(
                        "items-center",
                        html! {
                            (button_link(ButtonLink {
                                icon_name: Some("arrow-left"),
                                href: "/users/login/",
                                classes: "btn-ghost btn-square",
                                ..Default::default()
                            }))
                            (field_title(FieldTitle {
                                value: "Forgot Password",
                                classes: "grow text-center",
                            }))
                            (button_link(ButtonLink {
                                icon_name: Some("arrow-left"),
                                classes: "btn-ghost btn-square invisible",
                                ..Default::default()
                            }))
                        },
                    ))
                    (container_column(
                        "gap-2 mt-3",
                        html! {
                            (button_link(ButtonLink {
                                label: "Reset password with email",
                                href: "/otp/login/email/",
                                classes: "w-full",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Reset password with phone number",
                                href: "/otp/login/sms/",
                                classes: "w-full",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl crate::template::RenderAppPane for ForgotPasswordPage {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for ForgotPasswordPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct PhoneOtpRequestPage {
    pub identifier: String,
    pub error: String,
}

impl PhoneOtpRequestPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: "Login via SMS",
                        classes: "",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main("/otp/login/sms/"),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: html! {
                            (container_error(
                                Some(self.error.as_str()).filter(|e| !e.is_empty()),
                                input_phone(InputPhone {
                                    label: "Phone Number",
                                    name: "Identifier",
                                    value: &self.identifier,
                                    required: true,
                                    ..Default::default()
                                }),
                            ))
                        },
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Send OTP",
                                classes: "w-full",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    }))
                    (container_row(
                        "text-center mt-4",
                        html! {
                            (button_link(ButtonLink {
                                label: "Back to Login",
                                href: "/users/login/",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl crate::template::RenderAppPane for PhoneOtpRequestPage {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for PhoneOtpRequestPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct EmailOtpRequestPage {
    pub identifier: String,
    pub error: String,
}

impl EmailOtpRequestPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "w-80",
                html! {
                    (field_title(FieldTitle {
                        value: "Login via Email",
                        classes: "",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main("/otp/login/email/"),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: html! {
                            (container_error(
                                Some(self.error.as_str()).filter(|e| !e.is_empty()),
                                input_email(InputEmail {
                                    label: "Email Address",
                                    name: "Identifier",
                                    value: &self.identifier,
                                    required: true,
                                    ..Default::default()
                                }),
                            ))
                        },
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Send OTP",
                                classes: "w-full",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    }))
                    (container_row(
                        "text-center mt-4",
                        html! {
                            (button_link(ButtonLink {
                                label: "Back to Login",
                                href: "/users/login/",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl crate::template::RenderAppPane for EmailOtpRequestPage {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for EmailOtpRequestPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct OtpVerifyPage {
    pub identifier: String,
    pub otp: String,
    pub otp_error: String,
    pub password_error: String,
    pub password2_error: String,
}

impl OtpVerifyPage {
    fn body(&self) -> Markup {
        let mut action = String::from("/otp/verify/?identifier=");
        for b in self.identifier.as_bytes() {
            match *b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    action.push(char::from(*b));
                }
                _ => action.push_str(&format!("%{b:02X}")),
            }
        }
        html! {
            (container_column(
                "w-80",
                html! {
                    (field_title(FieldTitle {
                        value: "Verify OTP",
                        classes: "",
                    }))
                    (field_text(FieldText {
                        value: "Enter the code we sent and choose a new password.",
                        classes: "text-sm text-gray-600 mb-2",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main(&action),
                        inputs: html! {
                            (container_error(
                                Some(self.otp_error.as_str()).filter(|e| !e.is_empty()),
                                input_text(InputText {
                                    label: "OTP",
                                    name: "Otp",
                                    value: &self.otp,
                                    required: true,
                                    ..Default::default()
                                }),
                            ))
                            (container_error(
                                Some(self.password_error.as_str()).filter(|e| !e.is_empty()),
                                input_password(InputPassword {
                                    label: "New password",
                                    name: "NewPassword",
                                    required: true,
                                    ..Default::default()
                                }),
                            ))
                            (container_error(
                                Some(self.password2_error.as_str()).filter(|e| !e.is_empty()),
                                input_password(InputPassword {
                                    label: "Confirm new password",
                                    name: "NewPassword2",
                                    required: true,
                                    ..Default::default()
                                }),
                            ))
                        },
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Verify & Login",
                                classes: "w-full",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    }))
                    (container_row(
                        "text-center mt-4",
                        html! {
                            (button_link(ButtonLink {
                                label: "Cancel",
                                href: "/users/login/",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl crate::template::RenderAppPane for OtpVerifyPage {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for OtpVerifyPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct OtpPreferencesPage {
    pub msg91_auth_key: String,
    pub sms_otp_template_id: String,
    pub otp_template_id: String,
    pub sms_otp_field_name: String,
    pub sms_otp_extra_fields: String,
    pub email_otp_template_string: String,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub error: String,
}

impl OtpPreferencesPage {
    fn body(&self) -> Markup {
        form(FormOpts {
            attrs: form_hx_post_main("/otp/preferences/"),
            title: "OTP Preferences",
            subtitle: "Configure OTP settings for SMS and Email",
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: html! {
                (field_text(FieldText {
                    value: "SMS OTP Settings",
                    classes: "text-lg font-semibold mt-4",
                }))
                (container_error(None, input_text(InputText {
                    label: "MSG91 Auth Key",
                    name: "Msg91AuthKey",
                    value: &self.msg91_auth_key,
                    ..Default::default()
                })))
                (container_row(
                    "grid grid-cols-1 gap-1 @md:grid-cols-2",
                    html! {
                        (container_error(None, input_text(InputText {
                            label: "SMS OTP Template ID",
                            name: "SmsOtpTemplateId",
                            value: &self.sms_otp_template_id,
                            ..Default::default()
                        })))
                        (container_error(None, input_text(InputText {
                            label: "General OTP Template ID (Fallback)",
                            name: "OtpTemplateId",
                            value: &self.otp_template_id,
                            ..Default::default()
                        })))
                    },
                ))
                (container_error(None, input_text(InputText {
                    label: "SMS OTP Field Name",
                    name: "SmsOtpFieldName",
                    value: &self.sms_otp_field_name,
                    ..Default::default()
                })))
                (container_error(None, input_text(InputText {
                    label: "SMS OTP Extra Fields (JSON)",
                    name: "SmsOtpExtraFields",
                    value: &self.sms_otp_extra_fields,
                    ..Default::default()
                })))
                (field_text(FieldText {
                    value: "Email OTP Settings",
                    classes: "text-lg font-semibold mt-4",
                }))
                (container_error(None, input_text(InputText {
                    label: "Email OTP Template String",
                    name: "EmailOtpTemplateString",
                    value: &self.email_otp_template_string,
                    ..Default::default()
                })))
                (container_row(
                    "grid grid-cols-1 gap-1 @md:grid-cols-2",
                    html! {
                        (container_error(None, input_text(InputText {
                            label: "SMTP Host",
                            name: "SmtpHost",
                            value: &self.smtp_host,
                            ..Default::default()
                        })))
                        (container_error(None, input_text(InputText {
                            label: "SMTP Port",
                            name: "SmtpPort",
                            value: &self.smtp_port,
                            ..Default::default()
                        })))
                    },
                ))
                (container_row(
                    "grid grid-cols-1 gap-1 @md:grid-cols-2",
                    html! {
                        (container_error(None, input_text(InputText {
                            label: "SMTP Username",
                            name: "SmtpUsername",
                            value: &self.smtp_username,
                            ..Default::default()
                        })))
                        (container_error(None, input_text(InputText {
                            label: "SMTP Password",
                            name: "SmtpPassword",
                            value: &self.smtp_password,
                            ..Default::default()
                        })))
                    },
                ))
                (container_error(None, input_text(InputText {
                    label: "SMTP From Address",
                    name: "SmtpFrom",
                    value: &self.smtp_from,
                    ..Default::default()
                })))
            },
            actions: html! {
                (button_submit(ButtonSubmit {
                    label: "Save Preferences",
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for OtpPreferencesPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(otp_prefs_menu(), self.body())
    }

    fn render_main(&self) -> Markup {
        scaffold_main(self.body())
    }
}

impl RenderTemplate for OtpPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(chrome, otp_prefs_menu(), self.body())
    }
}

define_register_items! {
    plugin: OtpTag;
    capability: SlotCapability;
    trait: RegisterSlots;
    method: register_slots;
    bounds: [];
    items: [];
}
