use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, delete_confirmation, form, form_hx_post_selector, form_hx_post_url, modal,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::forms::{BankAccountForm, BankAccountFormField};
use crate::keys::{BANK_ACCOUNT_DELETE_MODAL, BANK_ACCOUNT_EDIT_MODAL};
use crate::routes::{
    BankAccountDeleteGetRouteTag, BankAccountDeletePostRouteTag, BankAccountEditPostRouteTag,
};

#[derive(Generic)]
pub struct BankAccountEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub ifsc_display: String,
    pub micr_code: String,
    pub error: String,
}

impl RenderTemplate for BankAccountEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = BankAccountDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            BANK_ACCOUNT_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit bank account" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(BANK_ACCOUNT_EDIT_MODAL, &modal_edit_post_url(
                        BankAccountEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: BankAccountForm::render_inputs(
                        &FormCtx::form::<BankAccountForm>(CsrfToken::current())
                            .value(BankAccountFormField::Name, &self.name)
                            .value(BankAccountFormField::AccountNumber, &self.account_number)
                            .value(BankAccountFormField::IfscCode, &self.ifsc_code)
                            .display(BankAccountFormField::IfscCode, &self.ifsc_display)
                            .value(BankAccountFormField::MicrCode, &self.micr_code),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_bank_accounts.BankAccountDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: BANK_ACCOUNT_DELETE_MODAL.id(),
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
            format!("#{}", BANK_ACCOUNT_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            BANK_ACCOUNT_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = BankAccountDeletePostRouteTag::new(self.id).url();
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
