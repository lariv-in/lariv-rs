use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ManyToManyItem, ShellChrome,
    button_modal_form, button_submit, delete_confirmation, form, form_hx_post_selector,
    form_hx_post_url, modal, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, fk_value};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use crate::forms::{
    ConvertLeadForm, FailLeadForm, FailLeadFormField, LeadTagForm, LeadTagFormField,
    LeadUpdateForm, LeadUpdateFormField,
};
use crate::keys::{
    LEAD_CONVERT_MODAL, LEAD_DELETE_MODAL, LEAD_EDIT_MODAL, LEAD_FAIL_MODAL, LEAD_TAG_DELETE_MODAL,
    LEAD_TAG_EDIT_MODAL, LEAD_UPDATE_DELETE_MODAL, LEAD_UPDATE_EDIT_MODAL,
};
use crate::routes::{
    LeadConvertPostRouteTag, LeadDeleteGetRouteTag, LeadDeletePostRouteTag, LeadEditPostRouteTag,
    LeadFailPostRouteTag, LeadTagDeleteGetRouteTag, LeadTagDeletePostRouteTag,
    LeadTagEditPostRouteTag, LeadUpdateDeleteGetRouteTag, LeadUpdateDeletePostRouteTag,
    LeadUpdateEditPostRouteTag,
};

use super::create::lead_form_inputs;

#[derive(Generic)]
pub struct LeadEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub customer_id: i64,
    pub customer_display: String,
    pub source: String,
    pub notes: String,
    pub tags: Vec<ManyToManyItem>,
    pub assigned_to_id: i64,
    pub assigned_to_display: String,
    pub order_expected_date: String,
    pub reason: String,
    pub show_reason: bool,
    pub error: String,
}

impl RenderTemplate for LeadEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = LeadDeleteGetRouteTag::new(self.id).url();
        let mut inputs = lead_form_inputs(
            self.customer_id,
            &self.customer_display,
            &self.source,
            &self.notes,
            &self.tags,
            self.assigned_to_id,
            &self.assigned_to_display,
            &self.order_expected_date,
        );
        if self.show_reason {
            inputs = html! {
                (inputs)
                (FailLeadForm::render_inputs(
                    &FormCtx::form::<FailLeadForm>(CsrfToken::current())
                        .value(FailLeadFormField::Reason, &self.reason),
                ))
            };
        }
        modal_keyed(
            LEAD_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit lead" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_EDIT_MODAL, &modal_edit_post_url(
                        LeadEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs,
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_crm.LeadDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: LEAD_DELETE_MODAL.id(),
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
pub struct ConvertLeadModalPage {
    pub lead_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
}

impl RenderTemplate for ConvertLeadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            LEAD_CONVERT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Convert lead" }
                p class="mb-4 text-sm opacity-80" {
                    "Convert this lead using its customer."
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_CONVERT_MODAL, &modal_create_post_url(
                        LeadConvertPostRouteTag::new(self.lead_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ConvertLeadForm::render_inputs(&FormCtx::form::<ConvertLeadForm>(CsrfToken::current())),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Convert", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct FailLeadModalPage {
    pub lead_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub reason: String,
    pub error: String,
}

impl RenderTemplate for FailLeadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            LEAD_FAIL_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Mark lead failed" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_FAIL_MODAL, &modal_create_post_url(
                        LeadFailPostRouteTag::new(self.lead_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: FailLeadForm::render_inputs(
                        &FormCtx::form::<FailLeadForm>(CsrfToken::current())
                            .value(FailLeadFormField::Reason, &self.reason),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Mark failed", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
#[derive(Generic)]
pub struct LeadTagEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub color: String,
    pub error: String,
}

impl RenderTemplate for LeadTagEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = LeadTagDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            LEAD_TAG_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit tag" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_TAG_EDIT_MODAL, &modal_edit_post_url(
                        LeadTagEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadTagForm::render_inputs(
                        &FormCtx::form::<LeadTagForm>(CsrfToken::current())
                            .value(LeadTagFormField::Name, &self.name)
                            .value(LeadTagFormField::Color, &self.color),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_crm.LeadTagDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: LEAD_TAG_DELETE_MODAL.id(),
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
pub struct LeadUpdateEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub created_by_id: i64,
    pub created_by_display: String,
    pub datetime: String,
    pub description: String,
    pub error: String,
}

impl RenderTemplate for LeadUpdateEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = LeadUpdateDeleteGetRouteTag::new(self.id).url();
        let created_by_id_s = fk_value(self.created_by_id);
        modal_keyed(
            LEAD_UPDATE_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit update" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_UPDATE_EDIT_MODAL, &modal_edit_post_url(
                        LeadUpdateEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadUpdateForm::render_inputs(
                        &FormCtx::form::<LeadUpdateForm>(CsrfToken::current())
                            .value(LeadUpdateFormField::CreatedById, created_by_id_s.as_str())
                            .display(LeadUpdateFormField::CreatedById, &self.created_by_display)
                            .value(LeadUpdateFormField::Datetime, &self.datetime)
                            .value(LeadUpdateFormField::Description, &self.description),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_crm.LeadUpdateDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: LEAD_UPDATE_DELETE_MODAL.id(),
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
            format!("#{}", LEAD_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            LEAD_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.modal_uid == LEAD_TAG_DELETE_MODAL.id() {
            LeadTagDeletePostRouteTag::new(self.id).url()
        } else if self.modal_uid == LEAD_UPDATE_DELETE_MODAL.id() {
            LeadUpdateDeletePostRouteTag::new(self.id).url()
        } else {
            LeadDeletePostRouteTag::new(self.id).url()
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
