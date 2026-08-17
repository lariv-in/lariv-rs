//! Signup page plus login / unauthenticated replacements that include signup CTAs.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonLink, ButtonSubmit, FieldSubtitle, FieldTitle, FormOpts, ShellAuth, ShellChrome,
        button_link, button_submit, container_column, field_subtitle, field_title, form,
        form_hx_post_main, shell_auth,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    plugins::{
        otp::routes::OtpForgotGetRouteTag,
        users::{
            forms::LoginForm,
            routes::{UsersLoginGetRouteTag, UsersLoginPostRouteTag},
            templates::{UsersLoginPageTag, UsersUnauthenticatedPageTag},
        },
    },
    tag::Tagged,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    traits::{get::IndexOfTemplateTag, replace::MapByTag},
};

use super::{
    forms::{SignupForm, SignupFormField},
    routes::{SignupGetRouteTag, SignupPostRouteTag},
};

/// Login page with a signup CTA (same fields as [`crate::plugins::users::templates::LoginPage`]).
#[derive(Generic)]
pub struct LoginPageWithSignup {
    pub error: String,
}

impl LoginPageWithSignup {
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

fn auth_pane(body: Markup) -> crate::components::AppLayoutHtml {
    use crate::components::app_layout_pane;
    app_layout_pane(body)
}

fn auth_main(body: Markup) -> crate::components::MainContentHtml {
    use crate::components::{LayoutMain, layout_main};
    layout_main(LayoutMain {
        breadcrumbs: Markup::default(),
        content: body,
    })
}

impl RenderAppPane for LoginPageWithSignup {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for LoginPageWithSignup {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

/// Unauthenticated landing with a Sign Up button.
#[derive(Generic)]
pub struct UnauthenticatedPageWithSignup {}

impl UnauthenticatedPageWithSignup {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "w-80 items-center text-center",
                html! {
                    (field_title(FieldTitle {
                        value: "Welcome",
                        classes: "",
                    }))
                    (field_subtitle(FieldSubtitle {
                        value: "Please log in or create an account to continue.",
                        classes: "",
                    }))
                    (container_column(
                        "w-full mt-4 gap-2",
                        html! {
                            (button_link(ButtonLink {
                                label: "Login",
                                href: &UsersLoginGetRouteTag.url(),
                                classes: "btn btn-primary text-white w-full",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Sign Up",
                                href: &SignupGetRouteTag.url(),
                                classes: "btn btn-outline w-full",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl RenderAppPane for UnauthenticatedPageWithSignup {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for UnauthenticatedPageWithSignup {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

pub struct SignupPageTag;

#[derive(Generic)]
pub struct SignupPage {
    pub error: String,
}

impl SignupPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: "Create an Account",
                        classes: "",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main(SignupPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: SignupForm::render_inputs(
                            &FormCtx::form::<SignupForm>()
                                .value(
                                    SignupFormField::Timezone,
                                    crate::datetime::DEFAULT_TIMEZONE,
                                )
                                .choices(
                                    SignupFormField::Timezone,
                                    crate::datetime::timezone_choices(),
                                ),
                        ),
                        actions: html! {
                            (container_column(
                                "w-full gap-2",
                                html! {
                                    (button_submit(ButtonSubmit {
                                        label: "Sign Up",
                                        classes: "w-full",
                                        ..Default::default()
                                    }))
                                    (button_link(ButtonLink {
                                        label: "Already have an account? Login",
                                        href: &UsersLoginGetRouteTag.url(),
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

impl RenderAppPane for SignupPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for SignupPage {
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
pub struct Hook<LoginIdx, UnauthIdx>(std::marker::PhantomData<(LoginIdx, UnauthIdx)>);

impl<LoginIdx, UnauthIdx> Default for Hook<LoginIdx, UnauthIdx> {
    fn default() -> Self {
        Hook(std::marker::PhantomData)
    }
}

type LoginReplaced<T, LoginIdx> =
    <T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithSignup>, LoginIdx>>::Output;

type BothReplaced<T, LoginIdx, UnauthIdx> = <LoginReplaced<T, LoginIdx> as MapByTag<
    UsersUnauthenticatedPageTag,
    TemplateOf<UnauthenticatedPageWithSignup>,
    UnauthIdx,
>>::Output;

impl<T, LoginIdx, UnauthIdx> TemplateRegistrar<T> for Hook<LoginIdx, UnauthIdx>
where
    T: frunk::hlist::HList + Clone + ProvideRequestCaps + Send + Sync,
    T: IndexOfTemplateTag<UsersLoginPageTag, LoginIdx>,
    T: MapByTag<UsersLoginPageTag, TemplateOf<LoginPageWithSignup>, LoginIdx>,
    LoginReplaced<T, LoginIdx>: IndexOfTemplateTag<UsersUnauthenticatedPageTag, UnauthIdx>,
    LoginReplaced<T, LoginIdx>:
        MapByTag<UsersUnauthenticatedPageTag, TemplateOf<UnauthenticatedPageWithSignup>, UnauthIdx>,
    BothReplaced<T, LoginIdx, UnauthIdx>: frunk::hlist::HList,
{
    type Output = frunk::HList![
        Tagged<SignupPageTag, TemplateOf<SignupPage>>,
        ...BothReplaced<T, LoginIdx, UnauthIdx>
    ];

    fn register_templates(self, cap: TemplateCapability<T>) -> TemplateCapability<Self::Output> {
        cap.replace_template_tag::<UsersLoginPageTag, TemplateOf<LoginPageWithSignup>, LoginIdx>(
            |_| TemplateOf::new(),
        )
        .replace_template_tag::<
            UsersUnauthenticatedPageTag,
            TemplateOf<UnauthenticatedPageWithSignup>,
            UnauthIdx,
        >(|_| TemplateOf::new())
        .add::<SignupPageTag, SignupPage>()
    }
}
