use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonLink, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, DetailHeader,
        FieldText, FormOpts, HtmlAttrs, LayoutMain, LayoutSidebar, ObjectList, PaginationPage,
        ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SlotCapability, SlotRegistrar,
        SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, breadcrumbs,
        button_link, button_modal_form, button_submit, column_sort_url, container_column,
        data_table_list_refresh, delete_confirmation, detail, detail_header, field_text, form,
        form_hx_get_picker_route, form_hx_get_route, form_hx_post_selector, form_hx_post_url,
        label, layout_main, layout_sidebar, modal, modal_keyed, pagination_pages,
        row_attr_navigate_route, row_attr_select, shell_scaffold, sidebar_menu,
        sidebar_menu_item_pane, sort_indicator, table_button_filter, table_create_button,
        table_pagination, table_pagination_picker, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::{RenderPickerSelect, picker_create_button},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_query, modal_edit_post_url},
};

use crate::plugins::crm::routes::CompanyDetailRouteTag;

use super::forms::{ContactFilterForm, ContactFilterFormField, ContactForm, ContactFormField};
use super::keys::{
    ContactCreateModalKey, ContactDeleteModalKey, ContactEditModalKey, ContactSelectModalKey,
    ContactSelectTableKey, ContactTableKey,
};
use super::routes::{
    ContactCreatePostRouteTag, ContactDefaultRouteTag, ContactDeleteGetRouteTag,
    ContactDeletePostRouteTag, ContactDetailRouteTag, ContactEditGetRouteTag,
    ContactEditPostRouteTag, ContactFkSelectRouteTag,
};

fn app_scaffold(
    title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    shell_scaffold(ShellScaffold {
        title,
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn contacts_menu() -> Markup {
    let list_url = ContactDefaultRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: "Contacts",
        children: sidebar_menu_item_pane(SidebarMenuItem {
            title: "All Contacts",
            url: &list_url,
            active: true,
            ..Default::default()
        }),
    })
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

fn contacts_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Contacts",
        href: None,
    }])
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

fn e164_phone(phone: &str) -> Option<String> {
    let phone = phone.trim();
    if phone.is_empty() {
        return None;
    }
    let parsed = phonenumber::parse(Some(phonenumber::country::IN), phone).ok()?;
    if !parsed.is_valid() {
        return None;
    }
    Some(parsed.format().mode(phonenumber::Mode::E164).to_string())
}

fn tel_href(phone: &str) -> Option<String> {
    e164_phone(phone).map(|e164| format!("tel:{e164}"))
}

fn whatsapp_href(phone: &str) -> Option<String> {
    let e164 = e164_phone(phone)?;
    let digits: String = e164.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    Some(format!("https://wa.me/{digits}"))
}

fn mailto_href(email: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') {
        return None;
    }
    Some(format!("mailto:{email}"))
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

fn render_picker_pagination<M: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, false);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination_picker(TablePagination {
        pages: &pages,
        hx_target: M::SELECTOR,
    })
}

crate::define_register_items! {
    plugin: ContactsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ContactListIdx: ContactListPageTag => ContactListPage,
        ContactDetailIdx: ContactDetailPageTag => ContactDetailPage,
        ContactEditModalIdx: ContactEditModalPageTag => ContactEditModalPage,
        ContactCreateModalIdx: ContactCreateModalPageTag => ContactCreateModalPage,
        ContactSelectIdx: ContactSelectPageTag => ContactSelectPage,
        ConfirmDeleteIdx: ContactConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

crate::define_register_items! {
    plugin: ContactsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

#[derive(Clone)]
pub struct ContactRow {
    pub id: i64,
    pub company_id: i64,
    pub company: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub is_primary: bool,
}

#[derive(Generic)]
pub struct ContactListPage {
    pub contacts: ObjectList<ContactRow>,
    pub filter_company_id: String,
    pub filter_company_display: String,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl ContactListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let company_sort = column_sort_url(&self.path_and_query, "Company", &self.sort);
        let email_sort = column_sort_url(&self.path_and_query, "Email", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let company_label = format!("Company{}", sort_indicator(&self.sort, "Company"));
        let email_label = format!("Email{}", sort_indicator(&self.sort, "Email"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Company",
                label: &company_label,
                sort_url: Some(&company_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Email",
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .contacts
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_navigate_route(ContactDetailRouteTag::new(c.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &c.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.company,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.email,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<ContactTableKey, ContactDefaultRouteTag>(
                        ContactDefaultRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        ContactFilterForm::render_inputs(
                            &FormCtx::form::<ContactFilterForm>(CsrfToken::current())
                                .value(ContactFilterFormField::CompanyId, &self.filter_company_id)
                                .display(
                                    ContactFilterFormField::CompanyId,
                                    &self.filter_company_display,
                                )
                                .value(ContactFilterFormField::Name, &self.filter_name),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (table_create_button::<ContactTableKey, ContactCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<ContactTableKey>(
            "Contacts",
            actions,
            &headers,
            &rows,
            render_pagination::<ContactTableKey>(
                &self.path_and_query,
                self.contacts.number,
                self.contacts.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for ContactListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(contacts_menu(), contacts_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(contacts_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for ContactListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Contacts — Lariv",
            chrome,
            contacts_menu(),
            contacts_list_crumbs(),
            self.render_table(),
        )
    }
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
    pub can_edit: bool,
}

impl ContactDetailPage {
    fn actions(&self) -> Markup {
        let tel = tel_href(&self.phone);
        let whatsapp = whatsapp_href(&self.phone);
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
            @if self.can_edit {
                (button_modal_form(ButtonModalForm {
                    name: "p_contacts.ContactEditForm",
                    href: &edit_get,
                    form_post_url: &edit_post,
                    modal_uid: ContactEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = contact_crumbs(&self.display_name, self.id);
        scaffold_pane(
            contact_detail_menu(&self.display_name, self.id),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
pub struct ContactEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub company_id: i64,
    pub company_display: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub is_primary: String,
    pub error: String,
}

impl RenderTemplate for ContactEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = ContactDeleteGetRouteTag::new(self.id).url();
        let company_id_s = fk_value(self.company_id);
        modal_keyed::<ContactEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit contact" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ContactEditModalKey>(&modal_edit_post_url(
                        ContactEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ContactForm::render_inputs(
                        &FormCtx::form::<ContactForm>(CsrfToken::current())
                            .value(ContactFormField::CompanyId, company_id_s.as_str())
                            .display(ContactFormField::CompanyId, &self.company_display)
                            .value(ContactFormField::Name, &self.name)
                            .value(ContactFormField::Email, &self.email)
                            .value(ContactFormField::Phone, &self.phone)
                            .value(ContactFormField::IsPrimary, &self.is_primary),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_contacts.ContactDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: ContactDeleteModalKey::ID,
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
pub struct ContactCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub company_id: i64,
    pub company_display: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub is_primary: String,
    pub error: String,
}

impl RenderTemplate for ContactCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let company_id_s = fk_value(self.company_id);
        modal_keyed::<ContactCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New contact" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ContactCreateModalKey>(&modal_create_post_query(
                        ContactCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ContactForm::render_inputs(
                        &FormCtx::form::<ContactForm>(CsrfToken::current())
                            .value(ContactFormField::CompanyId, company_id_s.as_str())
                            .display(ContactFormField::CompanyId, &self.company_display)
                            .value(ContactFormField::Name, &self.name)
                            .value(ContactFormField::Email, &self.email)
                            .value(ContactFormField::Phone, &self.phone)
                            .value(ContactFormField::IsPrimary, &self.is_primary),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create contact", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct ContactSelectPage {
    pub contacts: ObjectList<ContactRow>,
    pub filter_company_id: String,
    pub filter_company_display: String,
    pub filter_name: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl RenderPickerSelect<ContactSelectTableKey, ContactSelectModalKey> for ContactSelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let email_sort = column_sort_url(&self.path_and_query, "Email", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let email_label = format!("Email{}", sort_indicator(&self.sort, "Email"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Email",
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .contacts
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_select(&self.target_input, &c.id.to_string(), &c.name),
                cells: vec![
                    field_text(FieldText {
                        value: &c.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.email,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_picker_route::<
                        ContactSelectTableKey,
                        ContactSelectModalKey,
                        ContactFkSelectRouteTag,
                    >(ContactFkSelectRouteTag)
                    .set("hx-push-url", "false"),
                    inputs: html! {
                        (with_list_filter_common(
                            ContactFilterForm::render_inputs(
                                &FormCtx::form::<ContactFilterForm>(CsrfToken::current())
                                    .value(ContactFilterFormField::CompanyId, &self.filter_company_id)
                                    .display(
                                        ContactFilterFormField::CompanyId,
                                        &self.filter_company_display,
                                    )
                                    .value(ContactFilterFormField::Name, &self.filter_name),
                            ),
                            self.page_size,
                        ))
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (picker_create_button::<ContactCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<ContactSelectTableKey>(
            "Select contact",
            actions,
            &headers,
            &rows,
            render_picker_pagination::<ContactSelectModalKey>(
                &self.path_and_query,
                self.contacts.number,
                self.contacts.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for ContactSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
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
            format!("#{}", ContactDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            ContactDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = ContactDeletePostRouteTag::new(self.id).url();
        modal(crate::components::Modal {
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

#[cfg(test)]
mod tests {
    use super::{mailto_href, tel_href, whatsapp_href};

    #[test]
    fn tel_href_formats_indian_mobile() {
        assert_eq!(tel_href("9876543210").as_deref(), Some("tel:+919876543210"));
    }

    #[test]
    fn whatsapp_href_uses_digits_only() {
        assert_eq!(
            whatsapp_href("9876543210").as_deref(),
            Some("https://wa.me/919876543210")
        );
    }

    #[test]
    fn mailto_href_trims_and_validates() {
        assert_eq!(mailto_href("  a@b.com ").as_deref(), Some("mailto:a@b.com"));
        assert!(mailto_href("").is_none());
        assert!(mailto_href("not-an-email").is_none());
    }

    #[test]
    fn empty_phone_has_no_call_or_whatsapp() {
        assert!(tel_href("").is_none());
        assert!(whatsapp_href("   ").is_none());
    }

    #[test]
    fn contact_detail_header_renders_reach_actions() {
        let page = super::ContactDetailPage {
            id: 1,
            company_id: 0,
            company: String::new(),
            display_name: "Ada".into(),
            email: "ada@example.com".into(),
            phone: "9876543210".into(),
            is_primary: false,
            can_edit: false,
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
}
