//! Replace users auth pages so signup links are gone.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonLink, ButtonSubmit, FieldSubtitle, FieldTitle, FormOpts, ShellAuth,
        ShellChrome, button_link, button_submit, container_column, field_subtitle,
        field_title, form, form_hx_post_main, shell_auth,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    plugins::users::{
        forms::LoginForm,
        routes::{UsersLoginGetRouteTag, UsersLoginPostRouteTag},
        templates::{UsersLoginPageTag, UsersUnauthenticatedPageTag},
    },
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    traits::{
        get::IndexOfTemplateTag,
        replace::MapByTag,
    },
};

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
                        attrs: form_hx_post_main(UsersLoginPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: LoginForm::render_inputs(&FormCtx::form::<LoginForm>()),
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

fn auth_pane(body: Markup) -> crate::components::AppLayoutHtml {
    use crate::components::app_layout_pane;
    app_layout_pane(body)
}

impl RenderAppPane for LoginPageNoSignup {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        use crate::components::layout::layout_main;
        layout_main(self.body())
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
                                href: &UsersLoginGetRouteTag.url(),
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        use crate::components::layout::layout_main;
        layout_main(self.body())
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

#[derive(Copy, Clone)]
pub struct Hook<LoginIdx, UnauthIdx>(std::marker::PhantomData<(LoginIdx, UnauthIdx)>);

impl<LoginIdx, UnauthIdx> Default for Hook<LoginIdx, UnauthIdx> {
    fn default() -> Self {
        Hook(std::marker::PhantomData)
    }
}

type LoginReplaced<T, LoginIdx> =
    <T as MapByTag<UsersLoginPageTag, TemplateOf<LoginPageNoSignup>, LoginIdx>>::Output;

impl<T, LoginIdx, UnauthIdx> TemplateRegistrar<T> for Hook<LoginIdx, UnauthIdx>
where
    T: frunk::hlist::HList + Clone + ProvideRequestCaps + Send + Sync,
    T: IndexOfTemplateTag<UsersLoginPageTag, LoginIdx>,
    T: MapByTag<UsersLoginPageTag, TemplateOf<LoginPageNoSignup>, LoginIdx>,
    LoginReplaced<T, LoginIdx>: IndexOfTemplateTag<UsersUnauthenticatedPageTag, UnauthIdx>,
    LoginReplaced<T, LoginIdx>:
        MapByTag<UsersUnauthenticatedPageTag, TemplateOf<UnauthenticatedPageNoSignup>, UnauthIdx>,
{
    type Output = <LoginReplaced<T, LoginIdx> as MapByTag<
        UsersUnauthenticatedPageTag,
        TemplateOf<UnauthenticatedPageNoSignup>,
        UnauthIdx,
    >>::Output;

    fn register_templates(
        self,
        cap: TemplateCapability<T>,
    ) -> TemplateCapability<Self::Output> {
        cap.replace_template_tag::<UsersLoginPageTag, TemplateOf<LoginPageNoSignup>, LoginIdx>(|_| {
            TemplateOf::new()
        })
        .replace_template_tag::<
            UsersUnauthenticatedPageTag,
            TemplateOf<UnauthenticatedPageNoSignup>,
            UnauthIdx,
        >(|_| TemplateOf::new())
    }
}
