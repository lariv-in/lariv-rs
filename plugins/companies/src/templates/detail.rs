use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonPost, ButtonSubmit, Crumb, FieldText, FieldTitle, FormOpts, HtmlAttrs,
    ShellChrome, SidebarMenu, SidebarMenuItem, breadcrumbs, button_modal_form, button_post,
    button_submit, container_column, container_row, detail, field_text, field_title, form, label,
    sidebar_menu, sidebar_menu_item_pane,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::forms::{CompanyMemberForm, CompanyMemberFormField};
use crate::gst_state::state_with_code;
use crate::keys::COMPANY_EDIT_MODAL;
use crate::routes::{
    CompaniesMutate, CompanyDefaultRouteTag, CompanyDetailRouteTag, CompanyEditGetRouteTag,
    CompanyEditPostRouteTag, CompanyMemberAddPostRouteTag, CompanyMemberRemovePostRouteTag,
};

use super::{app_scaffold, scaffold_main, scaffold_pane};

fn company_crumbs(name: &str, _id: i64) -> Markup {
    let list_url = CompanyDefaultRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Companies",
            href: Some(&list_url),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

fn company_detail_menu(name: &str, id: i64) -> Markup {
    let title = format!("Company: {name}");
    let detail_url = CompanyDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Company Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
        },
    })
}

#[derive(Clone)]
pub struct CompanyMemberRow {
    pub membership_id: i64,
    pub label: String,
    pub owned_by: bool,
    pub is_self: bool,
}

#[derive(Generic)]
pub struct CompanyDetailPage {
    pub id: i64,
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
    pub is_owner: bool,
    pub members: Vec<CompanyMemberRow>,
    pub member_error: String,
    pub member_email: String,
}

impl CompanyDetailPage {
    fn body(&self) -> Markup {
        let state = state_with_code(&self.state, &self.gstin);
        let can_edit = self.is_owner
            && lariv_core::components::role_permitted(
                &lariv_plugin_users::role_authorization::roles_for::<CompaniesMutate>(),
            );
        let add_member_url = CompanyMemberAddPostRouteTag::new(self.id).url();
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label("Address line 1", field_text(FieldText { value: &self.address_line_1, classes: "" })))
                    (label("City", field_text(FieldText { value: &self.city, classes: "" })))
                    (label("State", field_text(FieldText { value: &state, classes: "" })))
                    (label("Website", field_text(FieldText { value: &self.website, classes: "" })))
                    (label("GSTIN", field_text(FieldText { value: &self.gstin, classes: "" })))
                    (label("CIN", field_text(FieldText { value: &self.cin, classes: "" })))
                    (label("PAN", field_text(FieldText { value: &self.pan, classes: "" })))
                    (label("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    @if can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_companies.CompanyEditForm",
                                href: &CompanyEditGetRouteTag::new(self.id).url(),
                                form_post_url: &CompanyEditPostRouteTag::new(self.id).path(),
                                modal_uid: COMPANY_EDIT_MODAL.id(),
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        }))
                    }
                    (container_column("mt-8 gap-3", html! {
                        h2 class="text-lg font-semibold" { "Members" }
                        @if !self.members.is_empty() {
                            ul class="flex flex-col gap-2" {
                                @for member in &self.members {
                                    li class="flex items-center justify-between gap-3 border-b border-base-300 pb-2" {
                                        div class="flex flex-col" {
                                            span { (member.label) }
                                            @if member.owned_by {
                                                span class="text-sm opacity-70" { "Owner" }
                                            }
                                        }
                                        @if can_edit && !member.is_self {
                                            (button_post(ButtonPost {
                                                label: "Remove",
                                                action: &CompanyMemberRemovePostRouteTag::new(
                                                    self.id,
                                                    member.membership_id,
                                                )
                                                .url(),
                                                classes: "btn btn-ghost btn-sm",
                                                ..Default::default()
                                            }))
                                        }
                                    }
                                }
                            }
                        }
                        @if can_edit {
                            (form(&CsrfToken::current(), FormOpts {
                                action: Some(add_member_url.as_str()),
                                classes: "mt-2",
                                form_error: Some(self.member_error.as_str()).filter(|e| !e.is_empty()),
                                inputs: CompanyMemberForm::render_inputs(
                                    &FormCtx::form::<CompanyMemberForm>(CsrfToken::current())
                                        .value(CompanyMemberFormField::Email, &self.member_email),
                                ),
                                actions: html! {
                                    (button_submit(ButtonSubmit {
                                        label: "Add member",
                                        classes: "btn btn-primary btn-sm w-fit",
                                        ..Default::default()
                                    }))
                                },
                                attrs: HtmlAttrs::new(),
                                ..Default::default()
                            }))
                        }
                    }))
                }))
            }))
        }
    }
}

impl RenderAppPane for CompanyDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = company_crumbs(&self.name, self.id);
        scaffold_pane(
            company_detail_menu(&self.name, self.id),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(company_crumbs(&self.name, self.id), self.body())
    }
}

impl RenderTemplate for CompanyDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Company — Lariv",
            chrome,
            company_detail_menu(&self.name, self.id),
            company_crumbs(&self.name, self.id),
            self.body(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::CompanyDetailPage;

    #[test]
    fn company_detail_shows_state_with_code() {
        let page = CompanyDetailPage {
            id: 1,
            name: "Acme".into(),
            address_line_1: String::new(),
            address_line_2: String::new(),
            city: "Bengaluru".into(),
            pincode: String::new(),
            state: "Karnataka".into(),
            website: String::new(),
            gstin: String::new(),
            cin: String::new(),
            pan: String::new(),
            phone: String::new(),
            email: String::new(),
            is_owner: false,
            members: Vec::new(),
            member_error: String::new(),
            member_email: String::new(),
        };
        let html = page.body().into_string();
        assert!(html.contains(">State<"));
        assert!(html.contains("29 — KARNATAKA"));
    }
}
