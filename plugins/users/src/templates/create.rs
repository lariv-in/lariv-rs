use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, container_row, form, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_create_post_query;

use crate::forms::{UserForm, UserFormField};
use crate::keys::USER_CREATE_MODAL;
use crate::routes::UsersCreatePostRouteTag;

#[derive(Generic)]
pub struct UserCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    pub error: String,
}

impl RenderTemplate for UserCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_users.UserCreateForm"
        } else {
            self.form_name.as_str()
        };
        let ctx = FormCtx::form::<UserForm>(CsrfToken::current())
            .value(UserFormField::Name, self.name.as_str())
            .value(UserFormField::Email, self.email.as_str())
            .value(UserFormField::Phone, self.phone.as_str())
            .value(UserFormField::Timezone, self.timezone.as_str())
            .choices(
                UserFormField::Timezone,
                lariv_core::datetime::timezone_choices(),
            )
            .value(UserFormField::Role, self.role.as_str());
        modal_keyed(
            USER_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create User",
                    subtitle: "Create a new user",
                    classes: "@container",
                    attrs: lariv_core::components::swap::form_hx_post_for_url(
                        USER_CREATE_MODAL,
                        &modal_create_post_query(
                            UsersCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                            &self.target_input,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: UserForm::render_inputs(&ctx),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save User",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
