use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query};

use crate::forms::{BankAccountForm, BankAccountFormField};
use crate::keys::BANK_ACCOUNT_CREATE_MODAL;
use crate::routes::{BankAccountCreateGetRouteTag, BankAccountCreatePostRouteTag};

pub const BANK_ACCOUNT_CREATE: CreateModal<
    BankAccountCreateGetRouteTag,
    BankAccountCreatePostRouteTag,
> = CreateModal {
    key: BANK_ACCOUNT_CREATE_MODAL,
    form_name: "p_bank_accounts.BankAccountCreateForm",
    get: BankAccountCreateGetRouteTag,
    post: BankAccountCreatePostRouteTag,
};

#[derive(Generic)]
pub struct BankAccountCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub ifsc_display: String,
    pub micr_code: String,
    pub error: String,
}

impl RenderTemplate for BankAccountCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            BANK_ACCOUNT_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New bank account" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(BANK_ACCOUNT_CREATE_MODAL, &modal_create_post_query(
                        BankAccountCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
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
                        (button_submit(ButtonSubmit { label: "Create bank account", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
