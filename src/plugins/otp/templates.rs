//! Maud templates for OTP recovery and admin preferences views.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonLink, ButtonSubmit, Crumb, FieldText, FieldTitle, FormOpts, LayoutMain,
        LayoutSidebar, ShellAuth, ShellChrome, ShellScaffold, SidebarMenu, SidebarNavLink,
        SlotCapability, SlotRegistrar, breadcrumbs, button_link, button_submit, container_column,
        container_row, field_text, field_title, form, form_hx_post_main, form_hx_post_main_url,
        layout_main, layout_sidebar, shell_auth, shell_scaffold, sidebar_menu,
        sidebar_nav_items_pane,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    plugins::signup::routes::SignupGetRouteTag,
    plugins::users::{
        forms::LoginForm,
        routes::{UsersLoginGetRouteTag, UsersLoginPostRouteTag},
        templates::UsersLoginPageTag,
    },
    tag::Tagged,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    traits::{get::IndexOfTemplateTag, replace::MapByTag},
};

use super::forms::{
    EmailIdentifierForm, EmailIdentifierFormField, PhoneIdentifierForm, PhoneIdentifierFormField,
    PreferencesForm, PreferencesFormField, VerifyForm, VerifyFormField,
};
use super::routes::{
    OtpEmailGetRouteTag, OtpEmailPostRouteTag, OtpForgotGetRouteTag, OtpPhoneGetRouteTag,
    OtpPhonePostRouteTag, OtpPrefsGetRouteTag, OtpPrefsPostRouteTag, OtpVerifyPostRouteTag,
};

pub struct OtpForgotPasswordPageTag;
pub struct OtpPhoneRequestPageTag;
pub struct OtpEmailRequestPageTag;
pub struct OtpVerifyPageTag;
pub struct OtpPreferencesPageTag;

/// Users login page with OTP “Forgot password?” link.
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
                        attrs: form_hx_post_main(UsersLoginPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: LoginForm::render_inputs(&FormCtx::form::<LoginForm>()),
                        actions: html! {
                            (container_column(
                                "w-full gap-2",
                                html! {
                                    (button_submit(ButtonSubmit {
                                        label: "Login",
                                        classes: "w-full",
                                        ..Default::default()
                                    }))
                                    (button_link(ButtonLink {
                                        label: "Forgot password?",
                                        href: &OtpForgotGetRouteTag.url(),
                                        classes: "w-full",
                                        ..Default::default()
                                    }))
                                    (button_link(ButtonLink {
                                        label: "Don't have an account? Sign up",
                                        href: &SignupGetRouteTag.url(),
                                        classes: "w-full",
                                        ..Default::default()
                                    }))
                                },
                            ))
                        },
                        ..Default::default()
                    }))
                },
            ))
        }
    }
}

impl RenderAppPane for LoginPageWithForgot {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
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

#[derive(Copy, Clone)]
pub struct Hook<LoginIdx>(std::marker::PhantomData<LoginIdx>);

impl<LoginIdx> Default for Hook<LoginIdx> {
    fn default() -> Self {
        Hook(std::marker::PhantomData)
    }
}

impl<T, LoginIdx> TemplateRegistrar<T> for Hook<LoginIdx>
where
    T: frunk::hlist::HList + Clone + ProvideRequestCaps + Send + Sync,
    T: IndexOfTemplateTag<UsersLoginPageTag, LoginIdx>,
    T: MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>,
    <T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>>::Output:
        frunk::hlist::HList,
{
    type Output = frunk::HList![
        Tagged<OtpPreferencesPageTag, TemplateOf<OtpPreferencesPage>>,
        Tagged<OtpVerifyPageTag, TemplateOf<OtpVerifyPage>>,
        Tagged<OtpEmailRequestPageTag, TemplateOf<EmailOtpRequestPage>>,
        Tagged<OtpPhoneRequestPageTag, TemplateOf<PhoneOtpRequestPage>>,
        Tagged<OtpForgotPasswordPageTag, TemplateOf<ForgotPasswordPage>>,
        ...<T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>>::Output
    ];

    fn register_templates(self, cap: TemplateCapability<T>) -> TemplateCapability<Self::Output> {
        cap.replace_template_tag::<UsersLoginPageTag, TemplateOf<LoginPageWithForgot>, LoginIdx>(
            |_| TemplateOf::new(),
        )
        .add::<OtpForgotPasswordPageTag, ForgotPasswordPage>()
        .add::<OtpPhoneRequestPageTag, PhoneOtpRequestPage>()
        .add::<OtpEmailRequestPageTag, EmailOtpRequestPage>()
        .add::<OtpVerifyPageTag, OtpVerifyPage>()
        .add::<OtpPreferencesPageTag, OtpPreferencesPage>()
    }
}

fn auth_pane(body: Markup) -> crate::components::AppLayoutHtml {
    use crate::components::app_layout_pane;
    app_layout_pane(body)
}

fn auth_main(body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: Markup::default(),
        content: body,
    })
}

fn app_scaffold(chrome: &ShellChrome, sidebar: Markup, crumbs: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn otp_prefs_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "OTP Preferences",
        href: None,
    }])
}

fn otp_prefs_menu(current_path: &str) -> Markup {
    let prefs_url = OtpPrefsGetRouteTag.url();
    let links = [SidebarNavLink {
        key: "preferences",
        title: "Preferences",
        url: &prefs_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "OTP Preferences",
        children: sidebar_nav_items_pane(&links, current_path),
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
                                href: &UsersLoginGetRouteTag.url(),
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
                                href: &OtpEmailGetRouteTag.url(),
                                classes: "w-full",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Reset password with phone number",
                                href: &OtpPhoneGetRouteTag.url(),
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
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
                        attrs: form_hx_post_main(OtpPhonePostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: PhoneIdentifierForm::render_inputs(
                            &FormCtx::form::<PhoneIdentifierForm>()
                                .value(
                                    PhoneIdentifierFormField::Identifier,
                                    self.identifier.as_str(),
                                )
                                .error(
                                    PhoneIdentifierFormField::Identifier,
                                    Some(self.error.as_str()).filter(|e| !e.is_empty()),
                                ),
                        ),
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
                                href: &UsersLoginGetRouteTag.url(),
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
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
                        attrs: form_hx_post_main(OtpEmailPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: EmailIdentifierForm::render_inputs(
                            &FormCtx::form::<EmailIdentifierForm>()
                                .value(
                                    EmailIdentifierFormField::Identifier,
                                    self.identifier.as_str(),
                                )
                                .error(
                                    EmailIdentifierFormField::Identifier,
                                    Some(self.error.as_str()).filter(|e| !e.is_empty()),
                                ),
                        ),
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
                                href: &UsersLoginGetRouteTag.url(),
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
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
        html! {
            (container_column(
                "w-80",
                html! {
                    (field_title(FieldTitle {
                        value: "Verify OTP",
                        classes: "",
                    }))
                    (field_text(FieldText {
                        value: "Enter the code we sent to log in. Set a password only when resetting credentials.",
                        classes: "text-sm text-gray-600 mb-2",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main_url(&OtpVerifyPostRouteTag.with_query().query("identifier", &self.identifier).build_with_query()),
                        inputs: VerifyForm::render_inputs(
                            &FormCtx::form::<VerifyForm>()
                                .value(VerifyFormField::Otp, self.otp.as_str())
                                .error(
                                    VerifyFormField::Otp,
                                    Some(self.otp_error.as_str()).filter(|e| !e.is_empty()),
                                )
                                .error(
                                    VerifyFormField::NewPassword,
                                    Some(self.password_error.as_str()).filter(|e| !e.is_empty()),
                                )
                                .error(
                                    VerifyFormField::NewPassword2,
                                    Some(self.password2_error.as_str()).filter(|e| !e.is_empty()),
                                ),
                        ),
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
                                href: &UsersLoginGetRouteTag.url(),
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
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
            attrs: form_hx_post_main(OtpPrefsPostRouteTag),
            title: "OTP Preferences",
            subtitle: "Configure OTP settings for SMS and Email",
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: PreferencesForm::render_inputs(
                &FormCtx::form::<PreferencesForm>()
                    .value(
                        PreferencesFormField::Msg91AuthKey,
                        self.msg91_auth_key.as_str(),
                    )
                    .value(
                        PreferencesFormField::SmsOtpTemplateId,
                        self.sms_otp_template_id.as_str(),
                    )
                    .value(
                        PreferencesFormField::OtpTemplateId,
                        self.otp_template_id.as_str(),
                    )
                    .value(
                        PreferencesFormField::SmsOtpFieldName,
                        self.sms_otp_field_name.as_str(),
                    )
                    .value(
                        PreferencesFormField::SmsOtpExtraFields,
                        self.sms_otp_extra_fields.as_str(),
                    )
                    .value(
                        PreferencesFormField::EmailOtpTemplateString,
                        self.email_otp_template_string.as_str(),
                    )
                    .value(PreferencesFormField::SmtpHost, self.smtp_host.as_str())
                    .value(PreferencesFormField::SmtpPort, self.smtp_port.as_str())
                    .value(
                        PreferencesFormField::SmtpUsername,
                        self.smtp_username.as_str(),
                    )
                    .value(
                        PreferencesFormField::SmtpPassword,
                        self.smtp_password.as_str(),
                    )
                    .value(PreferencesFormField::SmtpFrom, self.smtp_from.as_str()),
            ),
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = otp_prefs_crumbs();
        scaffold_pane(
            otp_prefs_menu(&OtpPrefsGetRouteTag.url()),
            crumbs,
            self.body(),
        )
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(otp_prefs_crumbs(), self.body())
    }
}

impl RenderTemplate for OtpPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            otp_prefs_menu(&OtpPrefsGetRouteTag.url()),
            otp_prefs_crumbs(),
            self.body(),
        )
    }
}

define_register_items! {
    plugin: OtpTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
