use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, Crumb, FieldText, FieldTitle, ShellChrome, SidebarMenu, SidebarMenuItem,
    breadcrumbs, button_modal_form, container_column, container_row, detail, field_text,
    field_title, label, sidebar_menu, sidebar_menu_item_pane,
};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::keys::BANK_ACCOUNT_EDIT_MODAL;
use crate::routes::{
    BankAccountDefaultRouteTag, BankAccountDetailRouteTag, BankAccountEditGetRouteTag,
    BankAccountEditPostRouteTag, BankAccountsMutate,
};

use super::{app_scaffold, scaffold_main, scaffold_pane};

fn account_crumbs(name: &str) -> Markup {
    let list_url = BankAccountDefaultRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Bank Accounts",
            href: Some(&list_url),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

fn account_detail_menu(name: &str, id: i64) -> Markup {
    let title = format!("Bank Account: {name}");
    let detail_url = BankAccountDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Account Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
        },
    })
}

#[derive(Generic)]
pub struct BankAccountDetailPage {
    pub id: i64,
    pub name: String,
    pub account_number: String,
    pub ifsc_code: String,
    pub bank_name: String,
    pub branch_name: String,
    pub micr_code: String,
}

impl BankAccountDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label("Account Number", field_text(FieldText { value: &self.account_number, classes: "" })))
                    (label("IFSC Code", field_text(FieldText { value: &self.ifsc_code, classes: "" })))
                    (label("Bank", field_text(FieldText { value: &self.bank_name, classes: "" })))
                    (label("Branch", field_text(FieldText { value: &self.branch_name, classes: "" })))
                    (label("MICR Code", field_text(FieldText { value: &self.micr_code, classes: "" })))
                    @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<BankAccountsMutate>()) {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_bank_accounts.BankAccountEditForm",
                                href: &BankAccountEditGetRouteTag::new(self.id).url(),
                                form_post_url: &BankAccountEditPostRouteTag::new(self.id).path(),
                                modal_uid: BANK_ACCOUNT_EDIT_MODAL.id(),
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

impl RenderAppPane for BankAccountDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = account_crumbs(&self.name);
        scaffold_pane(
            account_detail_menu(&self.name, self.id),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(account_crumbs(&self.name), self.body())
    }
}

impl RenderTemplate for BankAccountDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Bank Account — Lariv",
            chrome,
            account_detail_menu(&self.name, self.id),
            account_crumbs(&self.name),
            self.body(),
        )
    }
}
