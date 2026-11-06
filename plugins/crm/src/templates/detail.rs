use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DetailHeader, FieldText, FieldTitle, FormOpts, ObjectList,
    ShellChrome, button_modal_form, button_post_route, button_submit, container_column,
    container_row, detail, detail_header, field_text, field_title, form, form_hx_post_route, label,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};

use crate::crumbs::{
    converted_lead_crumbs, failed_lead_crumbs, lead_crumbs, lead_tag_crumbs, lead_update_crumbs,
};
use crate::detail_menu::{
    converted_lead_detail_menu, failed_lead_detail_menu, lead_detail_menu, lead_tag_detail_menu,
};
use crate::forms::LeadUpdateQuickForm;
use crate::keys::{
    LEAD_CONVERT_MODAL, LEAD_EDIT_MODAL, LEAD_FAIL_MODAL, LEAD_TAG_EDIT_MODAL,
    LEAD_TAG_LEADS_TABLE, LEAD_UPDATE_DELETE_MODAL, LEAD_UPDATE_EDIT_MODAL,
    LEAD_UPDATE_SAVED_EVENT, LEAD_UPDATES,
};
use crate::routes::{
    ConvertedLeadReactivatePostRouteTag, FailedLeadReactivatePostRouteTag, LeadConvertGetRouteTag,
    LeadDetailRouteTag, LeadEditGetRouteTag, LeadEditPostRouteTag, LeadFailGetRouteTag,
    LeadTagEditGetRouteTag, LeadTagEditPostRouteTag, LeadUpdateAddPostRouteTag,
    LeadUpdateDeleteGetRouteTag, LeadUpdateEditGetRouteTag, LeadUpdateEditPostRouteTag,
};

use super::list::{
    LeadRow, LeadTagChip, render_lead_tags, render_leads_data_table, tab_nav_link,
    tag_leads_tab_href,
};
use super::{app_scaffold, scaffold_main, scaffold_pane};

fn lead_assignment_fields(assigned_to: &str, order_expected_date: &str) -> Markup {
    html! {
        (label("Salesperson", field_text(FieldText { value: assigned_to, classes: "" })))
        (label("Order expected date", field_text(FieldText { value: order_expected_date, classes: "" })))
    }
}

#[derive(Generic)]
pub struct LeadDetailPage {
    pub id: i64,
    pub display_name: String,
    pub contact_id: i64,
    pub contact_display: String,
    pub company_id: i64,
    pub company: String,
    pub email: String,
    pub source: String,
    pub notes: String,
    pub assigned_to: String,
    pub order_expected_date: String,
    pub tags: Vec<LeadTagChip>,
    pub updates: LeadUpdatesPanel,
}

impl LeadDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>(),
        ) {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadConvertForm",
                    href: &LeadConvertGetRouteTag::new(self.id).url(),
                    form_post_url: &LeadConvertGetRouteTag::new(self.id).path(),
                    modal_uid: LEAD_CONVERT_MODAL.id(),
                    label: "Convert",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadFailForm",
                    href: &LeadFailGetRouteTag::new(self.id).url(),
                    form_post_url: &LeadFailGetRouteTag::new(self.id).path(),
                    modal_uid: LEAD_FAIL_MODAL.id(),
                    label: "Mark failed",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadEditForm",
                    href: &LeadEditGetRouteTag::new(self.id).url(),
                    form_post_url: &LeadEditPostRouteTag::new(self.id).path(),
                    modal_uid: LEAD_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label("Contact", html! {
                        a class="link" href=(ContactDetailRouteTag::new(self.contact_id).url()) {
                            (self.contact_display)
                        }
                    }))
                    (label("Company", html! {
                        @if self.company_id > 0 {
                            a class="link" href=(CompanyDetailRouteTag::new(self.company_id).url()) {
                                (self.company)
                            }
                        } @else {
                            (field_text(FieldText { value: &self.company, classes: "" }))
                        }
                    }))
                    (label("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label("Source", field_text(FieldText { value: &self.source, classes: "" })))
                    (lead_assignment_fields(&self.assigned_to, &self.order_expected_date))
                    (label("Notes", field_text(FieldText { value: &self.notes, classes: "" })))
                    (render_lead_tags(&self.tags))
                    div class="mt-6" {
                        (self.updates.render())
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = lead_crumbs(&self.display_name, self.id, None);
        scaffold_pane(
            lead_detail_menu(&self.display_name, self.id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(lead_crumbs(&self.display_name, self.id, None), self.body())
    }
}

impl RenderTemplate for LeadDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Lead — Lariv",
            chrome,
            lead_detail_menu(&self.display_name, self.id, "detail"),
            lead_crumbs(&self.display_name, self.id, None),
            self.body(),
        )
    }
}
#[derive(Generic)]
pub struct LeadConvertDetailPage {
    pub converted_id: i64,
    pub lead_id: i64,
    pub display_name: String,
    pub converted_at: String,
    pub company_id: i64,
    pub contact_id: i64,
    pub company: String,
    pub contact_display: String,
    pub email: String,
    pub source: String,
    pub notes: String,
    pub assigned_to: String,
    pub order_expected_date: String,
    pub tags: Vec<LeadTagChip>,
    pub updates: LeadUpdatesPanel,
}

impl LeadConvertDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>(),
        ) {
            html! {
                (button_post_route(
                    ConvertedLeadReactivatePostRouteTag::new(self.converted_id),
                    "Make active again",
                    "btn-primary",
                ))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadFailForm",
                    href: &LeadFailGetRouteTag::new(self.lead_id).url(),
                    form_post_url: &LeadFailGetRouteTag::new(self.lead_id).path(),
                    modal_uid: LEAD_FAIL_MODAL.id(),
                    label: "Mark failed",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadEditForm",
                    href: &LeadEditGetRouteTag::new(self.lead_id).url(),
                    form_post_url: &LeadEditPostRouteTag::new(self.lead_id).path(),
                    modal_uid: LEAD_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label("Converted at", field_text(FieldText { value: &self.converted_at, classes: "" })))
                    (label("Contact", html! {
                        a class="link" href=(ContactDetailRouteTag::new(self.contact_id).url()) {
                            (self.contact_display)
                        }
                    }))
                    (label("Company", html! {
                        a class="link" href=(CompanyDetailRouteTag::new(self.company_id).url()) {
                            (self.company)
                        }
                    }))
                    (label("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label("Source", field_text(FieldText { value: &self.source, classes: "" })))
                    (lead_assignment_fields(&self.assigned_to, &self.order_expected_date))
                    (label("Notes", field_text(FieldText { value: &self.notes, classes: "" })))
                    (render_lead_tags(&self.tags))
                    div class="mt-6" {
                        (self.updates.render())
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadConvertDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = converted_lead_crumbs(&self.display_name, self.converted_id, None);
        scaffold_pane(
            converted_lead_detail_menu(&self.display_name, self.converted_id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            converted_lead_crumbs(&self.display_name, self.converted_id, None),
            self.body(),
        )
    }
}

impl RenderTemplate for LeadConvertDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Converted lead — Lariv",
            chrome,
            converted_lead_detail_menu(&self.display_name, self.converted_id, "detail"),
            converted_lead_crumbs(&self.display_name, self.converted_id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct LeadFailDetailPage {
    pub failed_id: i64,
    pub lead_id: i64,
    pub display_name: String,
    pub failed_at: String,
    pub reason: String,
    pub contact_id: i64,
    pub contact_display: String,
    pub company_id: i64,
    pub company: String,
    pub email: String,
    pub source: String,
    pub notes: String,
    pub assigned_to: String,
    pub order_expected_date: String,
    pub tags: Vec<LeadTagChip>,
    pub updates: LeadUpdatesPanel,
}

impl LeadFailDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>(),
        ) {
            html! {
                (button_post_route(
                    FailedLeadReactivatePostRouteTag::new(self.failed_id),
                    "Make active again",
                    "btn-primary",
                ))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadEditForm",
                    href: &LeadEditGetRouteTag::new(self.lead_id).url(),
                    form_post_url: &LeadEditPostRouteTag::new(self.lead_id).path(),
                    modal_uid: LEAD_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label("Failed at", field_text(FieldText { value: &self.failed_at, classes: "" })))
                    (label("Reason", field_text(FieldText { value: &self.reason, classes: "" })))
                    (label("Contact", html! {
                        @if self.contact_id > 0 {
                            a class="link" href=(ContactDetailRouteTag::new(self.contact_id).url()) {
                                (self.contact_display)
                            }
                        } @else {
                            (field_text(FieldText { value: &self.contact_display, classes: "" }))
                        }
                    }))
                    (label("Company", html! {
                        @if self.company_id > 0 {
                            a class="link" href=(CompanyDetailRouteTag::new(self.company_id).url()) {
                                (self.company)
                            }
                        } @else {
                            (field_text(FieldText { value: &self.company, classes: "" }))
                        }
                    }))
                    (label("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label("Source", field_text(FieldText { value: &self.source, classes: "" })))
                    (lead_assignment_fields(&self.assigned_to, &self.order_expected_date))
                    (label("Notes", field_text(FieldText { value: &self.notes, classes: "" })))
                    (render_lead_tags(&self.tags))
                    div class="mt-6" {
                        (self.updates.render())
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadFailDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = failed_lead_crumbs(&self.display_name, self.failed_id, None);
        scaffold_pane(
            failed_lead_detail_menu(&self.display_name, self.failed_id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            failed_lead_crumbs(&self.display_name, self.failed_id, None),
            self.body(),
        )
    }
}

impl RenderTemplate for LeadFailDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Failed lead — Lariv",
            chrome,
            failed_lead_detail_menu(&self.display_name, self.failed_id, "detail"),
            failed_lead_crumbs(&self.display_name, self.failed_id, None),
            self.body(),
        )
    }
}
#[derive(Generic)]
pub struct LeadTagDetailPage {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub tab: String,
    pub leads: ObjectList<LeadRow>,
    pub sort: String,
    pub path_and_query: String,
}

impl LeadTagDetailPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        tab_nav_link(&tag_leads_tab_href(self.id, tab), self.tab == tab, label)
    }

    pub fn render_leads_table(&self) -> Markup {
        render_leads_data_table(
            LEAD_TAG_LEADS_TABLE,
            "Leads",
            &self.leads,
            &self.sort,
            &self.path_and_query,
            html! {},
            &[],
        )
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label("Color", html! {
                        span class="inline-flex items-center gap-2" {
                            span class="w-4 h-4 rounded-full shrink-0 border border-base-300" style=(format!("background-color: {}", self.color)) {}
                            span class="font-mono text-sm" { (self.color) }
                        }
                    }))
                    @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>()) {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.LeadTagEditForm",
                                href: &LeadTagEditGetRouteTag::new(self.id).url(),
                                form_post_url: &LeadTagEditPostRouteTag::new(self.id).path(),
                                modal_uid: LEAD_TAG_EDIT_MODAL.id(),
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        }))
                    }
                }))
            }))
            div class="tabs tabs-boxed mb-4 mt-6" {
                (self.tab_link("active", "Active"))
                (self.tab_link("converted", "Converted"))
                (self.tab_link("failed", "Failed"))
            }
            (self.render_leads_table())
        }
    }
}

impl RenderAppPane for LeadTagDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = lead_tag_crumbs(&self.name, self.id, None);
        scaffold_pane(
            lead_tag_detail_menu(&self.name, self.id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(lead_tag_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for LeadTagDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Tag — Lariv",
            chrome,
            lead_tag_detail_menu(&self.name, self.id, "detail"),
            lead_tag_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}
#[derive(Clone)]
pub struct LeadUpdateItem {
    pub id: i64,
    pub datetime: String,
    pub description: String,
}

#[derive(Clone)]
pub struct LeadUpdatesPanel {
    pub lead_id: i64,
    pub items: Vec<LeadUpdateItem>,
    pub default_datetime: String,
}

impl LeadUpdatesPanel {
    fn escape_js_str(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\'', "\\'")
    }

    fn render_add_form(&self) -> Markup {
        let default_dt = Self::escape_js_str(&self.default_datetime);
        let x_data = format!(
            "{{ datetime: $persist('{default_dt}').as('crm-lead-update-draft-datetime-{id}'), description: $persist('').as('crm-lead-update-draft-description-{id}'), clearDraft() {{ this.description = ''; this.datetime = '{default_dt}'; }} }}",
            id = self.lead_id,
            default_dt = default_dt,
        );
        let attrs = form_hx_post_route(LeadUpdateAddPostRouteTag::new(self.lead_id))
            .set("x-data", x_data)
            .set(format!("@{LEAD_UPDATE_SAVED_EVENT}"), "clearDraft()");
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs,
                inputs: LeadUpdateQuickForm::render_inputs(&FormCtx::form::<LeadUpdateQuickForm>(
                    CsrfToken::current(),
                )),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Add Update",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }

    pub fn render_list(&self) -> Markup {
        html! {
            div id=(LEAD_UPDATES.id()) class="max-h-96 overflow-y-auto flex flex-col divide-y divide-base-300 border border-base-300 rounded-box" {
                @if self.items.is_empty() {
                    div class="text-sm opacity-60 px-3 py-4 text-center" { "No updates" }
                } @else {
                    @for item in &self.items {
                        div class="px-3 py-2 text-sm flex gap-2 items-start" {
                            div class="min-w-0 flex-1" {
                                div class="text-xs opacity-60 whitespace-nowrap" { (item.datetime) }
                                div class="whitespace-pre-wrap break-words" { (item.description) }
                            }
                            @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>()) {
                                div class="flex gap-1 shrink-0" {
                                    (button_modal_form(ButtonModalForm {
                                        name: "p_crm.LeadUpdateEditForm",
                                        href: &LeadUpdateEditGetRouteTag::new(item.id).url(),
                                        form_post_url: &LeadUpdateEditPostRouteTag::new(item.id).path(),
                                        modal_uid: LEAD_UPDATE_EDIT_MODAL.id(),
                                        label: "Edit",
                                        classes: "btn-ghost btn-xs",
                                        ..Default::default()
                                    }))
                                    (button_modal_form(ButtonModalForm {
                                        name: "p_crm.LeadUpdateDeleteForm",
                                        href: &LeadUpdateDeleteGetRouteTag::new(item.id).url(),
                                        form_post_url: &LeadUpdateDeleteGetRouteTag::new(item.id).url(),
                                        modal_uid: LEAD_UPDATE_DELETE_MODAL.id(),
                                        label: "Delete",
                                        classes: "btn-ghost btn-xs text-error",
                                        ..Default::default()
                                    }))
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn render(&self) -> Markup {
        html! {
            div class="flex flex-col gap-3" {
                div class="text-lg font-semibold" { "Updates" }
                @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>()) {
                    (self.render_add_form())
                }
                (self.render_list())
            }
        }
    }
}

#[derive(Generic)]
pub struct LeadUpdateDetailPage {
    pub id: i64,
    pub lead_id: i64,
    pub display_name: String,
    pub created_by: String,
    pub datetime: String,
    pub description: String,
}

impl LeadUpdateDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.datetime, classes: "" }))
                    (label("Lead", html! {
                        a class="link" href=(LeadDetailRouteTag::new(self.lead_id).url()) {
                            (self.display_name)
                        }
                    }))
                    (label("Created by", field_text(FieldText { value: &self.created_by, classes: "" })))
                    (label("Description", field_text(FieldText { value: &self.description, classes: "" })))
                    @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>()) {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.LeadUpdateEditForm",
                                href: &LeadUpdateEditGetRouteTag::new(self.id).url(),
                                form_post_url: &LeadUpdateEditPostRouteTag::new(self.id).path(),
                                modal_uid: LEAD_UPDATE_EDIT_MODAL.id(),
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadUpdateDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = lead_update_crumbs(&self.display_name, self.lead_id, &self.datetime);
        scaffold_pane(
            lead_detail_menu(&self.display_name, self.lead_id, ""),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            lead_update_crumbs(&self.display_name, self.lead_id, &self.datetime),
            self.body(),
        )
    }
}

impl RenderTemplate for LeadUpdateDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Lead update — Lariv",
            chrome,
            lead_detail_menu(&self.display_name, self.lead_id, ""),
            lead_update_crumbs(&self.display_name, self.lead_id, &self.datetime),
            self.body(),
        )
    }
}
