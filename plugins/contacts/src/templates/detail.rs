use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonLink, ButtonModalForm, Crumb, DetailHeader, FieldText, FieldTitle, HtmlAttrs,
    ShellChrome, SidebarMenu, SidebarMenuItem, breadcrumbs, button_link, button_modal_form,
    container_column, container_row, detail, detail_header, field_text, field_title, label,
    sidebar_menu, sidebar_menu_item_pane,
};
use lariv_core::db::phonenumber::{parse_phone, tel_href, whatsapp_href};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::gst_state::state_with_code;
use crate::keys::{COMPANY_EDIT_MODAL, CONTACT_EDIT_MODAL};
use crate::routes::{
    CompanyDefaultRouteTag, CompanyDetailRouteTag, CompanyEditGetRouteTag, CompanyEditPostRouteTag,
    ContactDefaultRouteTag, ContactDetailRouteTag, ContactEditGetRouteTag, ContactEditPostRouteTag,
    ContactsMutate,
};

use super::{app_scaffold, scaffold_main, scaffold_pane};

pub(crate) fn mailto_href(email: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') {
        return None;
    }
    Some(format!("mailto:{email}"))
}

fn contact_detail_menu(display_name: &str, id: i64) -> Markup {
    let title = format!("Contact: {display_name}");
    let detail_url = ContactDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Contact Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
        },
    })
}

fn contact_crumbs(name: &str, _id: i64) -> Markup {
    let list_url = ContactDefaultRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Contacts",
            href: Some(&list_url),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

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

#[derive(Generic)]
pub struct ContactDetailPage {
    pub id: i64,
    pub company_id: i64,
    pub company: String,
    pub display_name: String,
    pub email: String,
    pub phone: String,
    pub is_primary: bool,
}

impl ContactDetailPage {
    fn actions(&self) -> Markup {
        let phone = parse_phone(&self.phone);
        let tel = phone.as_ref().map(tel_href);
        let whatsapp = phone.as_ref().map(whatsapp_href);
        let email = mailto_href(&self.email);
        let edit_get = ContactEditGetRouteTag::new(self.id).url();
        let edit_post = ContactEditPostRouteTag::new(self.id).path();
        html! {
            @if let Some(href) = tel.as_deref() {
                (button_link(ButtonLink {
                    label: "Call",
                    href,
                    icon_name: Some("phone"),
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
            @if let Some(href) = whatsapp.as_deref() {
                (button_link(ButtonLink {
                    label: "WhatsApp",
                    href,
                    icon_name: Some("chat-bubble-left-ellipsis"),
                    classes: "btn-outline",
                    attrs: HtmlAttrs::new()
                        .set("target", "_blank")
                        .set("rel", "noopener noreferrer"),
                }))
            }
            @if let Some(href) = email.as_deref() {
                (button_link(ButtonLink {
                    label: "Email",
                    href,
                    icon_name: Some("envelope"),
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
            @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<ContactsMutate>()) {
                (button_modal_form(ButtonModalForm {
                    name: "p_contacts.ContactEditForm",
                    href: &edit_get,
                    form_post_url: &edit_post,
                    modal_uid: CONTACT_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        }
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions: self.actions(),
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
                    (label("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label("Primary", field_text(FieldText { value: if self.is_primary { "Yes" } else { "No" }, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for ContactDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = contact_crumbs(&self.display_name, self.id);
        scaffold_pane(
            contact_detail_menu(&self.display_name, self.id),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(contact_crumbs(&self.display_name, self.id), self.body())
    }
}

impl RenderTemplate for ContactDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Contact — Lariv",
            chrome,
            contact_detail_menu(&self.display_name, self.id),
            contact_crumbs(&self.display_name, self.id),
            self.body(),
        )
    }
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
}

impl CompanyDetailPage {
    fn body(&self) -> Markup {
        let state = state_with_code(&self.state, &self.gstin);
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
                    @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<ContactsMutate>()) {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_contacts.CompanyEditForm",
                                href: &CompanyEditGetRouteTag::new(self.id).url(),
                                form_post_url: &CompanyEditPostRouteTag::new(self.id).path(),
                                modal_uid: COMPANY_EDIT_MODAL.id(),
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
    use super::{CompanyDetailPage, ContactDetailPage, mailto_href};

    #[test]
    fn mailto_href_trims_and_validates() {
        assert_eq!(mailto_href("  a@b.com ").as_deref(), Some("mailto:a@b.com"));
        assert!(mailto_href("").is_none());
        assert!(mailto_href("not-an-email").is_none());
    }

    #[test]
    fn contact_detail_header_renders_reach_actions() {
        let page = ContactDetailPage {
            id: 1,
            company_id: 0,
            company: String::new(),
            display_name: "Ada".into(),
            email: "ada@example.com".into(),
            phone: "9876543210".into(),
            is_primary: false,
        };
        let html = page.body().into_string();
        assert!(html.contains("tel:+919876543210"));
        assert!(html.contains("https://wa.me/919876543210"));
        assert!(html.contains("mailto:ada@example.com"));
        assert!(html.contains("Call"));
        assert!(html.contains("WhatsApp"));
        assert!(html.contains("Email"));
        assert!(html.contains("target=\"_blank\""));
        assert!(!html.contains(">Edit<"));
    }

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
        };
        let html = page.body().into_string();
        assert!(html.contains(">State<"));
        assert!(html.contains("29 — KARNATAKA"));
    }
}
