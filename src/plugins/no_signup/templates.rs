//! Replace users auth pages so signup links are gone (Go `p_no_signup` page patch).

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    components::{
        AppLayoutKey, ButtonLink, ButtonSubmit, FieldSubtitle, FieldTitle, FormOpts, ShellAuth,
        ShellChrome, SwapKey, button_link, button_submit, container_column, field_subtitle,
        field_title, form, form_hx_post_main, shell_auth,
    },
    html_form::{FormCtx, HtmlForm},
    plugins::users::forms::LoginForm,
    plugins::users::templates::{UsersLoginPageTag, UsersUnauthenticatedPageTag},
    template::{
        RegisterTemplates, RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf,
    },
    traits::replace::MapByTag,
};

use super::NoSignupTag;

/// Login page without the signup CTA (same fields as [`crate::plugins::users::templates::LoginPage`]).
#[derive(Generic)]
pub struct LoginPageNoSignup {
    pub error: String,
}

impl LoginPageNoSignup {
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
                        inputs: LoginForm::render_inputs(&FormCtx::new()),
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Login",
                                classes: "w-full mb-4",
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

fn auth_pane(body: Markup) -> Markup {
    html! {
        (PreEscaped(format!(r#"<div id="{}">"#, AppLayoutKey::ID)))
        (body)
        (PreEscaped("</div>"))
    }
}

impl RenderAppPane for LoginPageNoSignup {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for LoginPageNoSignup {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

/// Unauthenticated landing without a Sign Up button.
#[derive(Generic)]
pub struct UnauthenticatedPageNoSignup {}

impl UnauthenticatedPageNoSignup {
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
                        value: "Please log in to continue.",
                        classes: "",
                    }))
                    (container_column(
                        "w-full mt-4 gap-2",
                        html! {
                            (button_link(ButtonLink {
                                label: "Login",
                                href: "/users/login/",
                                classes: "btn btn-primary text-white w-full",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl RenderAppPane for UnauthenticatedPageNoSignup {
    fn render_pane(&self) -> Markup {
        auth_pane(self.body())
    }
}

impl RenderTemplate for UnauthenticatedPageNoSignup {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

impl<T, LoginIdx, UnauthIdx> RegisterTemplates<NoSignupTag, (LoginIdx, UnauthIdx)>
    for TemplateCapability<T>
where
    T: MapByTag<UsersLoginPageTag, TemplateOf<LoginPageNoSignup>, LoginIdx>,
    <T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageNoSignup>, LoginIdx>>::Output:
        MapByTag<UsersUnauthenticatedPageTag, TemplateOf<UnauthenticatedPageNoSignup>, UnauthIdx>,
{
    type Output = TemplateCapability<
        <<T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageNoSignup>, LoginIdx>>::Output as MapByTag<
            UsersUnauthenticatedPageTag,
            TemplateOf<UnauthenticatedPageNoSignup>,
            UnauthIdx,
        >>::Output,
    >;

    fn register_templates(self) -> Self::Output {
        self.replace_template::<UsersLoginPageTag, LoginIdx, TemplateOf<LoginPageNoSignup>>(
            |_| TemplateOf::new(),
        )
        .replace_template::<UsersUnauthenticatedPageTag, UnauthIdx, TemplateOf<UnauthenticatedPageNoSignup>>(
            |_| TemplateOf::new(),
        )
    }
}
