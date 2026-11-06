use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ManyToManyItem, ShellChrome, button_submit, form, form_hx_post_url,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, fk_value};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query, modal_create_post_url};

use crate::forms::{LeadForm, LeadFormField, LeadTagForm, LeadTagFormField};
use crate::keys::{CUSTOMER_CREATE_MODAL, LEAD_CREATE_MODAL, LEAD_TAG_CREATE_MODAL};
use crate::routes::{
    CustomerCreateGetRouteTag, CustomerCreatePostRouteTag, LeadCreateGetRouteTag,
    LeadCreatePostRouteTag, LeadTagCreateGetRouteTag, LeadTagCreatePostRouteTag,
};

pub const LEAD_CREATE: CreateModal<LeadCreateGetRouteTag, LeadCreatePostRouteTag> = CreateModal {
    key: LEAD_CREATE_MODAL,
    form_name: "p_crm.LeadCreateForm",
    get: LeadCreateGetRouteTag,
    post: LeadCreatePostRouteTag,
};

pub const LEAD_TAG_CREATE: CreateModal<LeadTagCreateGetRouteTag, LeadTagCreatePostRouteTag> =
    CreateModal {
        key: LEAD_TAG_CREATE_MODAL,
        form_name: "p_crm.LeadTagCreateForm",
        get: LeadTagCreateGetRouteTag,
        post: LeadTagCreatePostRouteTag,
    };

pub const CUSTOMER_CREATE: CreateModal<CustomerCreateGetRouteTag, CustomerCreatePostRouteTag> =
    CreateModal {
        key: CUSTOMER_CREATE_MODAL,
        form_name: "p_crm.CustomerCreateForm",
        get: CustomerCreateGetRouteTag,
        post: CustomerCreatePostRouteTag,
    };

pub(crate) fn lead_form_inputs(
    contact_id: i64,
    contact_display: &str,
    source: &str,
    notes: &str,
    tags: &[ManyToManyItem],
    assigned_to_id: i64,
    assigned_to_display: &str,
    order_expected_date: &str,
) -> Markup {
    let contact_id_s = fk_value(contact_id);
    let assigned_to_id_s = fk_value(assigned_to_id);
    LeadForm::render_inputs(
        &FormCtx::form::<LeadForm>(CsrfToken::current())
            .value(LeadFormField::ContactId, contact_id_s.as_str())
            .display(LeadFormField::ContactId, contact_display)
            .value(LeadFormField::Source, source)
            .value(LeadFormField::Notes, notes)
            .m2m(LeadFormField::Tags, tags)
            .choices(LeadFormField::Source, LeadForm::source_choices())
            .value(LeadFormField::AssignedToId, assigned_to_id_s.as_str())
            .display(LeadFormField::AssignedToId, assigned_to_display)
            .value(LeadFormField::OrderExpectedDate, order_expected_date),
    )
}

#[derive(Generic)]
pub struct LeadCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub contact_id: i64,
    pub contact_display: String,
    pub source: String,
    pub notes: String,
    pub tags: Vec<ManyToManyItem>,
    pub assigned_to_id: i64,
    pub assigned_to_display: String,
    pub order_expected_date: String,
    pub error: String,
}

impl RenderTemplate for LeadCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            LEAD_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New lead" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_CREATE_MODAL, &modal_create_post_url(
                        LeadCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: lead_form_inputs(
                        self.contact_id,
                        &self.contact_display,
                        &self.source,
                        &self.notes,
                        &self.tags,
                        self.assigned_to_id,
                        &self.assigned_to_display,
                        &self.order_expected_date,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create lead", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
#[derive(Generic)]
pub struct LeadTagCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub color: String,
    pub error: String,
}

impl RenderTemplate for LeadTagCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            LEAD_TAG_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New lead tag" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(LEAD_TAG_CREATE_MODAL, &modal_create_post_query(
                        LeadTagCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadTagForm::render_inputs(
                        &FormCtx::form::<LeadTagForm>(CsrfToken::current())
                            .value(LeadTagFormField::Name, &self.name)
                            .value(LeadTagFormField::Color, &self.color),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create tag", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
