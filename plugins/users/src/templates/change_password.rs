use frunk::Generic;
use maud::Markup;

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_main,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;

use crate::forms::PasswordForm;
use crate::routes::{UsersChangePasswordPostRouteTag, UsersSelfChangePasswordPostRouteTag};

use super::detail::{user_crumbs, user_detail_menu};
use super::self_::user_self_menu;
use super::{app_scaffold, scaffold_main, scaffold_pane, self_crumbs};

#[derive(Generic)]
pub struct ChangePasswordPage {
    pub user_id: i64,
    pub user_name: String,
    pub action: String,
    pub error: String,
    pub is_self: bool,
}

impl ChangePasswordPage {
    fn sidebar_and_copy(&self) -> (Markup, &'static str, &'static str) {
        if self.is_self {
            (
                user_self_menu(&self.user_name, "password"),
                "Change Password",
                "Update your password",
            )
        } else {
            (
                user_detail_menu(self.user_id, &self.user_name, "password", true),
                "Change Password",
                "Update user password",
            )
        }
    }

    fn crumbs(&self) -> Markup {
        if self.is_self {
            self_crumbs(&self.user_name, Some("Change Password"))
        } else {
            user_crumbs(self.user_id, &self.user_name, Some("Change Password"))
        }
    }

    fn pane_body(&self, title: &str, subtitle: &str) -> Markup {
        form(
            &CsrfToken::current(),
            FormOpts {
                title,
                subtitle,
                attrs: if self.is_self {
                    form_hx_post_main(UsersSelfChangePasswordPostRouteTag)
                } else {
                    form_hx_post_main(UsersChangePasswordPostRouteTag::new(self.user_id))
                },
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: PasswordForm::render_inputs(&FormCtx::form::<PasswordForm>(
                    CsrfToken::current(),
                )),
                actions: button_submit(ButtonSubmit {
                    label: "Change Password",
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
    }
}

impl lariv_core::template::RenderAppPane for ChangePasswordPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let (sidebar, title, subtitle) = self.sidebar_and_copy();
        scaffold_pane(sidebar, self.crumbs(), self.pane_body(title, subtitle))
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let (_sidebar, title, subtitle) = self.sidebar_and_copy();
        scaffold_main(self.crumbs(), self.pane_body(title, subtitle))
    }
}

impl RenderTemplate for ChangePasswordPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let (sidebar, title, subtitle) = self.sidebar_and_copy();
        app_scaffold(
            &format!("{title} — Lariv"),
            chrome,
            sidebar,
            self.crumbs(),
            self.pane_body(title, subtitle),
        )
    }
}
