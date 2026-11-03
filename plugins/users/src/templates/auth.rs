use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonLink, ButtonSubmit, FieldSubtitle, FieldTitle, FormOpts, LayoutMain, ShellAuth,
    ShellChrome, button_link, button_submit, container_column, field_subtitle, field_title, form,
    form_hx_post_main, layout_main, shell_auth,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;

use crate::forms::LoginForm;
use crate::routes::{UsersLoginGetRouteTag, UsersLoginPostRouteTag};

#[derive(Generic)]
pub struct LoginPage {
    pub error: String,
}

impl LoginPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: "Login",
                        classes: "",
                    }))
                    (form(&CsrfToken::current(), FormOpts {
                        attrs: form_hx_post_main(UsersLoginPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: LoginForm::render_inputs(&FormCtx::form::<LoginForm>(CsrfToken::current())),
                        actions: html! {
                            (container_column(
                                "w-full gap-2",
                                html! {
                                    (button_submit(ButtonSubmit {
                                        label: "Login",
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

/// Auth card body wrapped as `#app-layout` for HTMX swaps.
fn auth_pane(body: Markup) -> lariv_core::components::AppLayoutHtml {
    use lariv_core::components::app_layout_pane;
    app_layout_pane(body)
}

fn auth_main(body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: Markup::default(),
        content: body,
    })
}

impl lariv_core::template::RenderAppPane for LoginPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for LoginPage {
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
pub struct UnauthenticatedPage {}

impl UnauthenticatedPage {
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

impl lariv_core::template::RenderAppPane for UnauthenticatedPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for UnauthenticatedPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}
