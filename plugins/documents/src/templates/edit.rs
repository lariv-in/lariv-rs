use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, delete_confirmation, form, form_hx_post_selector, form_hx_post_url, modal,
    modal_keyed,
};
use lariv_core::html_form::CsrfToken;
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::keys::{DOCUMENT_DELETE_MODAL, DOCUMENT_EDIT_MODAL};
use crate::logic::TypeFields;
use crate::routes::{
    DocumentDeleteGetRouteTag, DocumentDeletePostRouteTag, DocumentEditPostRouteTag,
};

use super::create::document_form_inputs;

#[derive(Generic)]
pub struct DocumentEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub document_type: String,
    pub fields: TypeFields,
    pub error: String,
}

impl RenderTemplate for DocumentEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = DocumentDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            DOCUMENT_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit document" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(DOCUMENT_EDIT_MODAL, &modal_edit_post_url(
                        DocumentEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: document_form_inputs(&self.document_type, &self.fields, false, "", "", ""),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_documents.DocumentDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: DOCUMENT_DELETE_MODAL.id(),
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
            format!("#{}", DOCUMENT_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DOCUMENT_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DocumentDeletePostRouteTag::new(self.id).url();
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
