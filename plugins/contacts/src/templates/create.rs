use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, fk_value};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query};

use crate::forms::{ContactForm, ContactFormField};
use crate::keys::CONTACT_CREATE_MODAL;
use crate::routes::{ContactCreateGetRouteTag, ContactCreatePostRouteTag};

pub const CONTACT_CREATE: CreateModal<ContactCreateGetRouteTag, ContactCreatePostRouteTag> =
    CreateModal {
        key: CONTACT_CREATE_MODAL,
        form_name: "p_contacts.ContactCreateForm",
        get: ContactCreateGetRouteTag,
        post: ContactCreatePostRouteTag,
    };

#[derive(Generic)]
pub struct ContactCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub company_id: i64,
    pub company_display: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub is_primary: String,
    pub error: String,
}

impl RenderTemplate for ContactCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let company_id_s = fk_value(self.company_id);
        modal_keyed(
            CONTACT_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New contact" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(CONTACT_CREATE_MODAL, &modal_create_post_query(
                        ContactCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ContactForm::render_inputs(
                        &FormCtx::form::<ContactForm>(CsrfToken::current())
                            .value(ContactFormField::CompanyId, company_id_s.as_str())
                            .display(ContactFormField::CompanyId, &self.company_display)
                            .value(ContactFormField::Name, &self.name)
                            .value(ContactFormField::Email, &self.email)
                            .value(ContactFormField::Phone, &self.phone)
                            .value(ContactFormField::IsPrimary, &self.is_primary),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create contact", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
