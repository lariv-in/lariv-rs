use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, container_row, form, form_hx_post_url,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, fk_value};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query};

use crate::forms::{DocumentForm, DocumentFormField, DocumentFormFlag};
use crate::keys::DOCUMENT_CREATE_MODAL;
use crate::logic::TypeFields;
use crate::routes::{DocumentCreateGetRouteTag, DocumentCreatePostRouteTag};

pub const DOCUMENT_CREATE: CreateModal<DocumentCreateGetRouteTag, DocumentCreatePostRouteTag> =
    CreateModal {
        key: DOCUMENT_CREATE_MODAL,
        form_name: "p_documents.DocumentCreateForm",
        get: DocumentCreateGetRouteTag,
        post: DocumentCreatePostRouteTag,
    };

pub(crate) fn document_form_inputs(
    document_type: &str,
    fields: &TypeFields,
    reload_type: bool,
    form_name: &str,
    refresh_table: &str,
    target_input: &str,
) -> Markup {
    let vnode_id = fk_value(fields.vnode_id);
    let kind = crate::document_type::DocumentType::parse(document_type).unwrap_or_default();
    let is_aadhar = kind == crate::document_type::DocumentType::AadharCard;
    let is_pan = kind == crate::document_type::DocumentType::Pan;
    let is_passport = kind == crate::document_type::DocumentType::Passport;
    html! {
        (document_type_select(
            document_type,
            DocumentForm::document_type_choices(),
            reload_type,
            form_name,
            refresh_table,
            target_input,
        ))
        (DocumentForm::render_inputs(
            &FormCtx::form::<DocumentForm>(CsrfToken::current())
                .value(DocumentFormField::DocumentType, document_type)
                .value(DocumentFormField::VnodeId, vnode_id.as_str())
                .display(DocumentFormField::VnodeId, &fields.vnode_name)
                .value(DocumentFormField::AadharNumber, &fields.aadhar_number)
                .value(DocumentFormField::PanNumber, &fields.pan_number)
                .value(DocumentFormField::PassportNumber, &fields.passport_number)
                .value(DocumentFormField::Name, &fields.name)
                .value(DocumentFormField::Gender, &fields.gender)
                .value(DocumentFormField::DateOfBirth, &fields.date_of_birth)
                .value(DocumentFormField::Address, &fields.address)
                .value(DocumentFormField::Nationality, &fields.nationality)
                .value(DocumentFormField::ExpiryDate, &fields.expiry_date)
                .choices(DocumentFormField::DocumentType, DocumentForm::document_type_choices())
                .choices(DocumentFormField::Gender, DocumentForm::gender_choices())
                .flag(DocumentFormFlag::IsAadhar, is_aadhar)
                .flag(DocumentFormFlag::IsPan, is_pan)
                .flag(DocumentFormFlag::IsPassport, is_passport)
                .flag(DocumentFormFlag::HasGender, is_aadhar || is_passport),
        ))
    }
}

fn document_type_select(
    document_type: &str,
    choices: &[(&str, &str)],
    reload_type: bool,
    form_name: &str,
    refresh_table: &str,
    target_input: &str,
) -> Markup {
    let reload = lariv_core::http::RouteQueryBuilder::new(DocumentCreateGetRouteTag)
        .query("name", form_name)
        .query("refresh", refresh_table)
        .query("target_input", target_input)
        .build();
    let options = html! {
        @for &(value, label) in choices {
            option value=(value) selected[value == document_type] { (label) }
        }
    };
    if reload_type {
        html! {
            label class="form-control w-full" {
                span class="label-text" { "Document type" }
                select
                    class="select select-bordered w-full"
                    name="document_type"
                    required
                    hx-get=(reload)
                    hx-trigger="change"
                    hx-target=(DOCUMENT_CREATE_MODAL.selector())
                    hx-swap="outerHTML"
                    hx-include="closest form"
                {
                    (options)
                }
            }
        }
    } else {
        html! {
            label class="form-control w-full" {
                span class="label-text" { "Document type" }
                select class="select select-bordered w-full" name="document_type" required {
                    (options)
                }
            }
        }
    }
}

#[derive(Generic)]
pub struct DocumentCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub document_type: String,
    pub fields: TypeFields,
    pub error: String,
}

impl RenderTemplate for DocumentCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_documents.DocumentCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            DOCUMENT_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Add document",
                    subtitle: "Save an identity document",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        DOCUMENT_CREATE_MODAL,
                        &modal_create_post_query(
                            DocumentCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                            &self.target_input,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: document_form_inputs(
                        &self.document_type,
                        &self.fields,
                        true,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    ),
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save document",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
