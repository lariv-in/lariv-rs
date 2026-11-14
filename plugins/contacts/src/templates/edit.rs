use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, delete_confirmation, form, form_hx_post_selector, form_hx_post_url, modal,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, fk_value};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::forms::{CompanyForm, CompanyFormField, ContactForm, ContactFormField};
use crate::keys::{
    COMPANY_DELETE_MODAL, COMPANY_EDIT_MODAL, CONTACT_DELETE_MODAL, CONTACT_EDIT_MODAL,
};
use crate::routes::{
    CompanyDeleteGetRouteTag, CompanyDeletePostRouteTag, CompanyEditPostRouteTag,
    ContactDeleteGetRouteTag, ContactDeletePostRouteTag, ContactEditPostRouteTag,
};

#[derive(Generic)]
pub struct ContactEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub company_id: i64,
    pub company_display: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub is_primary: String,
    pub error: String,
}

impl RenderTemplate for ContactEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = ContactDeleteGetRouteTag::new(self.id).url();
        let company_id_s = fk_value(self.company_id);
        modal_keyed(
            CONTACT_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit contact" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(CONTACT_EDIT_MODAL, &modal_edit_post_url(
                        ContactEditPostRouteTag::new(self.id),
                        &self.form_name,
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
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_contacts.ContactDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: CONTACT_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct CompanyEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub website: String,
    pub gstin: String,
    pub cin: String,
    pub pan: String,
    pub phone: String,
    pub email: String,
    pub error: String,
}

impl RenderTemplate for CompanyEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = CompanyDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            COMPANY_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit company" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(COMPANY_EDIT_MODAL, &modal_edit_post_url(
                        CompanyEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CompanyForm::render_inputs(
                        &FormCtx::form::<CompanyForm>(CsrfToken::current())
                            .value(CompanyFormField::Name, &self.name)
                            .value(CompanyFormField::AddressLine1, &self.address_line_1)
                            .value(CompanyFormField::AddressLine2, &self.address_line_2)
                            .value(CompanyFormField::City, &self.city)
                            .value(CompanyFormField::Pincode, &self.pincode)
                            .value(CompanyFormField::State, &self.state)
                            .choices(CompanyFormField::State, CompanyForm::state_choices())
                            .value(CompanyFormField::Website, &self.website)
                            .value(CompanyFormField::Gstin, &self.gstin)
                            .value(CompanyFormField::Cin, &self.cin)
                            .value(CompanyFormField::Pan, &self.pan)
                            .value(CompanyFormField::Phone, &self.phone)
                            .value(CompanyFormField::Email, &self.email),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_contacts.CompanyDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: COMPANY_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", CONTACT_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            CONTACT_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.modal_uid == COMPANY_DELETE_MODAL.id() {
            CompanyDeletePostRouteTag::new(self.id).url()
        } else {
            ContactDeletePostRouteTag::new(self.id).url()
        };
        modal(lariv_core::components::Modal {
            uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
