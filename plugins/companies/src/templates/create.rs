use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query};

use crate::forms::{CompanyForm, CompanyFormField};
use crate::keys::COMPANY_CREATE_MODAL;
use crate::routes::{CompanyCreateGetRouteTag, CompanyCreatePostRouteTag};

pub const COMPANY_CREATE: CreateModal<CompanyCreateGetRouteTag, CompanyCreatePostRouteTag> =
    CreateModal {
        key: COMPANY_CREATE_MODAL,
        form_name: "p_companies.CompanyCreateForm",
        get: CompanyCreateGetRouteTag,
        post: CompanyCreatePostRouteTag,
    };

#[derive(Generic)]
pub struct CompanyCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
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

impl RenderTemplate for CompanyCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            COMPANY_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New company" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(COMPANY_CREATE_MODAL, &modal_create_post_query(
                        CompanyCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
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
                        (button_submit(ButtonSubmit { label: "Create company", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
