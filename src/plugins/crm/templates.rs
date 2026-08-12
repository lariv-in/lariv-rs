use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonDeletePost, ButtonSubmit, DetailHeader, FieldText, FieldTitle, FormOpts, LayoutMain,
        LayoutSidebar, ObjectList, PaginationPage, ShellChrome, ShellScaffold, SidebarMenu,
        SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter,
        TableColumnHeader, TablePagination, TableRow, ButtonModalForm, button_clear,
        button_delete, button_delete_post_route, button_modal_form, button_post_route, button_submit,
        container_column, container_row,
        data_table_list_refresh, detail, detail_header, field_text, field_title, form, form_hx_get_picker_route,
        form_hx_get_route, form_hx_post_main, form_hx_post_url, label_inline, layout_main, layout_sidebar,
        modal_keyed, pagination_pages, row_attr_navigate, row_attr_navigate_route,
        row_attr_select, shell_scaffold, sidebar_menu, sidebar_menu_item_pane, table_button_filter,
        table_create_button, table_pagination, table_pagination_picker,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::{RenderPickerSelect, picker_create_button},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{CreateModal, modal_create_post_url},
};

use super::crumbs::{
    companies_list_crumbs, company_crumbs, contact_crumbs, contacts_list_crumbs,
    converted_lead_crumbs, deal_crumbs, deals_list_crumbs, failed_lead_crumbs,
    lead_crumbs, lead_edit_crumbs, leads_list_crumbs,
};
use super::detail_menu::{
    company_detail_menu, contact_detail_menu, converted_lead_detail_menu,
    deal_detail_menu, failed_lead_detail_menu, lead_detail_menu, lead_edit_menu,
};
use super::forms::{
    CompanyFilterForm, CompanyFilterFormField, CompanyForm, CompanyFormField, ContactFilterForm,
    ContactFilterFormField, ContactForm, ContactFormField, ConvertLeadDealSource,
    ConvertLeadDealSourceField, ConvertLeadForm, ConvertLeadFormField, DealFilterForm, DealFilterFormField, DealForm, DealFormField, FailLeadForm,
    FailLeadFormField, LeadFilterForm, LeadFilterFormField, LeadForm, LeadFormField,
};
use super::keys::{
    CompanyCreateModalKey, CompanySelectModalKey, CompanySelectTableKey, CompanyTableKey,
    ContactCreateModalKey, ContactSelectModalKey, ContactSelectTableKey, ContactTableKey,
    DealCreateModalKey, DealTableKey, LeadConvertModalKey, LeadCreateModalKey, LeadFailModalKey,
    LeadHubTableKey,
};
use super::routes::{
    CompanyCreatePostRouteTag, CompanyDefaultRouteTag,
    CompanyDeletePostRouteTag, CompanyDetailRouteTag, CompanyEditGetRouteTag,
    CompanyEditPostRouteTag, CompanyFkSelectRouteTag, ContactCreateGetRouteTag,
    ContactCreatePostRouteTag, ContactDefaultRouteTag, ContactDeletePostRouteTag,
    ContactDetailRouteTag, ContactEditGetRouteTag, ContactEditPostRouteTag,
    ContactFkSelectRouteTag, DealCreateGetRouteTag,
    DealCreatePostRouteTag, DealDefaultRouteTag, DealDeletePostRouteTag, DealDetailRouteTag,
    DealEditGetRouteTag, DealEditPostRouteTag, LeadConvertGetRouteTag,
    LeadConvertPostRouteTag, LeadCreateGetRouteTag, LeadCreatePostRouteTag, LeadDefaultRouteTag,
    LeadDeletePostRouteTag, LeadEditGetRouteTag, LeadEditPostRouteTag,
    FailedLeadReactivatePostRouteTag, LeadFailGetRouteTag, LeadFailPostRouteTag,
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

fn scaffold_pane(sidebar: Markup, crumbs: Markup, body: Markup) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
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

fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn crm_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "CRM",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Leads",
                url: &LeadDefaultRouteTag.url(),
                active: active == "leads",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Companies",
                url: &CompanyDefaultRouteTag.url(),
                active: active == "companies",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Contacts",
                url: &ContactDefaultRouteTag.url(),
                active: active == "contacts",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Deals",
                url: &DealDefaultRouteTag.url(),
                active: active == "deals",
                ..Default::default()
            }))
        },
    })
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

fn render_picker_pagination<M: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
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

fn tab_href(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(LeadDefaultRouteTag)
        .query("tab", tab)
        .build()
}

crate::define_register_items! {
    plugin: CrmTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        LeadHubIdx: LeadHubPageTag => LeadHubPage,
        LeadDetailIdx: LeadDetailPageTag => LeadDetailPage,
        LeadFormIdx: LeadFormPageTag => LeadFormPage,
        LeadCreateModalIdx: LeadCreateModalPageTag => LeadCreateModalPage,
        ConvertLeadModalIdx: ConvertLeadModalPageTag => ConvertLeadModalPage,
        FailLeadModalIdx: FailLeadModalPageTag => FailLeadModalPage,
        LeadConvertDetailIdx: LeadConvertDetailPageTag => LeadConvertDetailPage,
        LeadFailDetailIdx: LeadFailDetailPageTag => LeadFailDetailPage,
        CompanyListIdx: CompanyListPageTag => CompanyListPage,
        CompanyDetailIdx: CompanyDetailPageTag => CompanyDetailPage,
        CompanyFormIdx: CompanyFormPageTag => CompanyFormPage,
        CompanyCreateModalIdx: CompanyCreateModalPageTag => CompanyCreateModalPage,
        CompanySelectIdx: CompanySelectPageTag => CompanySelectPage,
        ContactListIdx: ContactListPageTag => ContactListPage,
        ContactDetailIdx: ContactDetailPageTag => ContactDetailPage,
        ContactFormIdx: ContactFormPageTag => ContactFormPage,
        ContactCreateModalIdx: ContactCreateModalPageTag => ContactCreateModalPage,
        ContactSelectIdx: ContactSelectPageTag => ContactSelectPage,
        DealListIdx: DealListPageTag => DealListPage,
        DealDetailIdx: DealDetailPageTag => DealDetailPage,
        DealFormIdx: DealFormPageTag => DealFormPage,
        DealCreateModalIdx: DealCreateModalPageTag => DealCreateModalPage,
    ]
}

crate::define_register_items! {
    plugin: CrmTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

// --- Leads ---

#[derive(Clone)]
pub struct LeadRow {
    pub id: i64,
    pub name: String,
    pub company: String,
    pub email: String,
    pub source: String,
    pub status: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct LeadHubPage {
    pub leads: ObjectList<LeadRow>,
    pub tab: String,
    pub filter_company: String,
    pub filter_email: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl LeadHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        use crate::components::attrs::escape_attr;
        use maud::PreEscaped;

        let active = self.tab == tab;
        let cls = if active { "tab tab-active" } else { "tab" };
        let href = tab_href(tab);
        let nav = crate::components::nav_content_attrs(&href);
        html! {
            (PreEscaped(format!(
                r#"<a class="{cls}" href="{href}"{attrs}>"#,
                cls = escape_attr(cls),
                href = escape_attr(&href),
                attrs = nav.as_string(),
            )))
            (label)
            (PreEscaped("</a>"))
        }
    }

    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: true },
            TableColumnHeader { key: "Company", label: "Company", sort_url: None, push_url: true },
            TableColumnHeader { key: "Email", label: "Email", sort_url: None, push_url: true },
            TableColumnHeader { key: "Source", label: "Source", sort_url: None, push_url: true },
            TableColumnHeader { key: "Status", label: "Status", sort_url: None, push_url: true },
        ];
        let rows: Vec<TableRow> = self
            .leads
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_navigate(&r.detail_href),
                cells: vec![
                    field_text(FieldText { value: &r.name, classes: "" }),
                    field_text(FieldText { value: &r.company, classes: "" }),
                    field_text(FieldText { value: &r.email, classes: "" }),
                    field_text(FieldText { value: &r.source, classes: "" }),
                    field_text(FieldText { value: &r.status, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<LeadHubTableKey, LeadDefaultRouteTag>(
                        LeadDefaultRouteTag,
                    ),
                    inputs: LeadFilterForm::render_inputs(
                        &FormCtx::form::<LeadFilterForm>()
                            .value(LeadFilterFormField::Company, &self.filter_company)
                            .value(LeadFilterFormField::Email, &self.filter_email),
                    ),
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit && self.tab == "active" {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadCreateForm",
                    href: &LeadCreateGetRouteTag.url(),
                    form_post_url: &LeadCreateGetRouteTag.path(),
                    modal_uid: LeadCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<LeadHubTableKey>(
            &self.path_and_query,
            self.leads.number,
            self.leads.num_pages,
        );
        data_table_list_refresh::<LeadHubTableKey>(
            "Leads",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        html! {
            div class="tabs tabs-boxed mb-4" {
                (self.tab_link("active", "Active"))
                (self.tab_link("converted", "Converted"))
                (self.tab_link("failed", "Failed"))
            }
            (self.render_table())
        }
    }
}

impl RenderAppPane for LeadHubPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = leads_list_crumbs();
        scaffold_pane(crm_menu("leads"), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(leads_list_crumbs(), self.body())
    }
}

impl RenderTemplate for LeadHubPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "CRM Leads — Lariv",
            chrome,
            crm_menu("leads"),
            leads_list_crumbs(),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct LeadDetailPage {
    pub id: i64,
    pub display_name: String,
    pub company_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub source: String,
    pub notes: String,
    pub can_edit: bool,
}

impl LeadDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadConvertForm",
                    href: &LeadConvertGetRouteTag::new(self.id).url(),
                    form_post_url: &LeadConvertGetRouteTag::new(self.id).path(),
                    modal_uid: LeadConvertModalKey::ID,
                    label: "Convert",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadFailForm",
                    href: &LeadFailGetRouteTag::new(self.id).url(),
                    form_post_url: &LeadFailGetRouteTag::new(self.id).path(),
                    modal_uid: LeadFailModalKey::ID,
                    label: "Mark failed",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                a class="btn btn-outline" href=(LeadEditGetRouteTag::new(self.id).url()) { "Edit" }
                (button_delete_post_route(
                    LeadDeletePostRouteTag::new(self.id),
                    ButtonDeletePost {
                        label: "Delete",
                        confirm: "Permanently delete this lead?",
                        classes: "btn-error",
                    },
                ))
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
                    (label_inline("Company", field_text(FieldText { value: &self.company_name, classes: "" })))
                    (label_inline("First name", field_text(FieldText { value: &self.first_name, classes: "" })))
                    (label_inline("Last name", field_text(FieldText { value: &self.last_name, classes: "" })))
                    (label_inline("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label_inline("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label_inline("Source", field_text(FieldText { value: &self.source, classes: "" })))
                    (label_inline("Notes", field_text(FieldText { value: &self.notes, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = lead_crumbs(&self.display_name, self.id, None);
        scaffold_pane(
            lead_detail_menu(&self.display_name, self.id, "detail", self.can_edit),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(lead_crumbs(&self.display_name, self.id, None), self.body())
    }
}

impl RenderTemplate for LeadDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Lead — Lariv",
            chrome,
            lead_detail_menu(&self.display_name, self.id, "detail", self.can_edit),
            lead_crumbs(&self.display_name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct LeadFormPage {
    pub id: i64,
    pub company_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub source: String,
    pub notes: String,
    pub reason: String,
    pub show_reason: bool,
    pub menu_title: String,
    pub detail_url: String,
    pub display_name: String,
    pub list_tab: String,
    pub can_edit: bool,
}

impl LeadFormPage {
    fn body(&self) -> Markup {
        let choices = LeadForm::source_choices();
        let mut inputs = LeadForm::render_inputs(
            &FormCtx::form::<LeadForm>()
                .value(LeadFormField::CompanyName, &self.company_name)
                .value(LeadFormField::FirstName, &self.first_name)
                .value(LeadFormField::LastName, &self.last_name)
                .value(LeadFormField::Email, &self.email)
                .value(LeadFormField::Phone, &self.phone)
                .value(LeadFormField::Source, &self.source)
                .value(LeadFormField::Notes, &self.notes)
                .choices(
                    LeadFormField::Source,
                    &choices
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<_>>(),
                ),
        );
        if self.show_reason {
            inputs = html! {
                (inputs)
                (FailLeadForm::render_inputs(
                    &FormCtx::form::<FailLeadForm>()
                        .value(FailLeadFormField::Reason, &self.reason),
                ))
            };
        }
        form(FormOpts {
            attrs: form_hx_post_main(LeadEditPostRouteTag::new(self.id)),
            inputs,
            actions: html! {
                (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
            },
            ..Default::default()
        })
    }
}

impl RenderAppPane for LeadFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = lead_edit_crumbs(&self.list_tab, &self.display_name, &self.detail_url);
        scaffold_pane(
            lead_edit_menu(
                self.menu_title.clone(),
                self.detail_url.clone(),
                self.id,
                self.can_edit,
            ),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            lead_edit_crumbs(&self.list_tab, &self.display_name, &self.detail_url),
            self.body(),
        )
    }
}

impl RenderTemplate for LeadFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit lead — Lariv",
            chrome,
            lead_edit_menu(
                self.menu_title.clone(),
                self.detail_url.clone(),
                self.id,
                self.can_edit,
            ),
            lead_edit_crumbs(&self.list_tab, &self.display_name, &self.detail_url),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct LeadCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub company_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub source: String,
    pub notes: String,
    pub error: String,
}

impl RenderTemplate for LeadCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let choices = LeadForm::source_choices();
        modal_keyed::<LeadCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New lead" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadCreateModalKey>(&modal_create_post_url(
                        LeadCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadForm::render_inputs(
                        &FormCtx::form::<LeadForm>()
                            .value(LeadFormField::CompanyName, &self.company_name)
                            .value(LeadFormField::FirstName, &self.first_name)
                            .value(LeadFormField::LastName, &self.last_name)
                            .value(LeadFormField::Email, &self.email)
                            .value(LeadFormField::Phone, &self.phone)
                            .value(LeadFormField::Source, &self.source)
                            .value(LeadFormField::Notes, &self.notes)
                            .choices(
                                LeadFormField::Source,
                                &choices
                                    .iter()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect::<Vec<_>>(),
                            ),
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
pub struct ConvertLeadModalPage {
    pub lead_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub company_id: i64,
    pub company_display: String,
    pub deal_kind: String,
    pub deal_name: String,
    pub deal_amount: String,
    pub deal_stage: String,
    pub error: String,
}

impl RenderTemplate for ConvertLeadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let choices = ConvertLeadDealSource::deal_stage_choices();
        let company_id_s = fk_value(self.company_id);
        modal_keyed::<LeadConvertModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Convert lead" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadConvertModalKey>(&modal_create_post_url(
                        LeadConvertPostRouteTag::new(self.lead_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ConvertLeadForm::render_inputs(
                        &FormCtx::form::<ConvertLeadForm>()
                            .value(
                                ConvertLeadFormField::CompanyId,
                                company_id_s.as_str(),
                            )
                            .display(
                                ConvertLeadFormField::CompanyId,
                                self.company_display.as_str(),
                            )
                            .kind::<ConvertLeadDealSource>(&self.deal_kind)
                            .value(ConvertLeadDealSourceField::DealName, self.deal_name.as_str())
                            .value(
                                ConvertLeadDealSourceField::DealAmount,
                                self.deal_amount.as_str(),
                            )
                            .value(ConvertLeadDealSourceField::DealStage, self.deal_stage.as_str())
                            .choices(
                                ConvertLeadDealSourceField::DealStage,
                                &choices
                                    .iter()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect::<Vec<_>>(),
                            ),
                    ),
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
        modal_keyed::<LeadFailModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Mark lead failed" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadFailModalKey>(&modal_create_post_url(
                        LeadFailPostRouteTag::new(self.lead_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: FailLeadForm::render_inputs(
                        &FormCtx::form::<FailLeadForm>()
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
pub struct LeadConvertDetailPage {
    pub converted_id: i64,
    pub lead_id: i64,
    pub display_name: String,
    pub converted_at: String,
    pub company_id: i64,
    pub contact_id: i64,
    pub customer_id: i64,
    pub deal_id: Option<i64>,
    pub company_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub source: String,
    pub notes: String,
    pub can_edit: bool,
}

impl LeadConvertDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                a class="btn btn-outline" href=(LeadEditGetRouteTag::new(self.lead_id).url()) { "Edit" }
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
                    (label_inline("Converted at", field_text(FieldText { value: &self.converted_at, classes: "" })))
                    (label_inline("Company", field_text(FieldText { value: &self.company_name, classes: "" })))
                    (label_inline("First name", field_text(FieldText { value: &self.first_name, classes: "" })))
                    (label_inline("Last name", field_text(FieldText { value: &self.last_name, classes: "" })))
                    (label_inline("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label_inline("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label_inline("Source", field_text(FieldText { value: &self.source, classes: "" })))
                    (label_inline("Notes", field_text(FieldText { value: &self.notes, classes: "" })))
                    p class="mt-4" {
                        a class="link" href=(CompanyDetailRouteTag::new(self.company_id).url()) { "Company #" (self.company_id) }
                        " · "
                        a class="link" href=(ContactDetailRouteTag::new(self.contact_id).url()) { "Contact #" (self.contact_id) }
                        " · "
                        a class="link" href={ "/customers/c/" (self.customer_id) "/" } { "Customer #" (self.customer_id) }
                        @if let Some(did) = self.deal_id {
                            " · "
                            a class="link" href=(DealDetailRouteTag::new(did).url()) { "Deal #" (did) }
                        }
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadConvertDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = converted_lead_crumbs(&self.display_name, self.converted_id, None);
        scaffold_pane(
            converted_lead_detail_menu(
                &self.display_name,
                self.converted_id,
                self.lead_id,
                "detail",
                self.can_edit,
            ),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
            converted_lead_detail_menu(
                &self.display_name,
                self.converted_id,
                self.lead_id,
                "detail",
                self.can_edit,
            ),
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
    pub company_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub source: String,
    pub notes: String,
    pub can_edit: bool,
}

impl LeadFailDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_post_route(
                    FailedLeadReactivatePostRouteTag::new(self.failed_id),
                    "Make active again",
                    "btn-primary",
                ))
                a class="btn btn-outline" href=(LeadEditGetRouteTag::new(self.lead_id).url()) { "Edit" }
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
                    (label_inline("Failed at", field_text(FieldText { value: &self.failed_at, classes: "" })))
                    (label_inline("Reason", field_text(FieldText { value: &self.reason, classes: "" })))
                    (label_inline("Company", field_text(FieldText { value: &self.company_name, classes: "" })))
                    (label_inline("First name", field_text(FieldText { value: &self.first_name, classes: "" })))
                    (label_inline("Last name", field_text(FieldText { value: &self.last_name, classes: "" })))
                    (label_inline("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label_inline("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label_inline("Source", field_text(FieldText { value: &self.source, classes: "" })))
                    (label_inline("Notes", field_text(FieldText { value: &self.notes, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for LeadFailDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = failed_lead_crumbs(&self.display_name, self.failed_id, None);
        scaffold_pane(
            failed_lead_detail_menu(
                &self.display_name,
                self.failed_id,
                self.lead_id,
                "detail",
                self.can_edit,
            ),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
            failed_lead_detail_menu(
                &self.display_name,
                self.failed_id,
                self.lead_id,
                "detail",
                self.can_edit,
            ),
            failed_lead_crumbs(&self.display_name, self.failed_id, None),
            self.body(),
        )
    }
}

// --- Companies ---

#[derive(Clone)]
pub struct CompanyRow {
    pub id: i64,
    pub name: String,
    pub website: String,
    pub customer_id: String,
}

#[derive(Generic)]
pub struct CompanyListPage {
    pub companies: ObjectList<CompanyRow>,
    pub filter_name: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl CompanyListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [TableColumnHeader {
            key: "Name",
            label: "Name",
            sort_url: None,
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .companies
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_navigate_route(CompanyDetailRouteTag::new(a.id)),
                cells: vec![field_text(FieldText { value: &a.name, classes: "" })],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<CompanyTableKey, CompanyDefaultRouteTag>(
                        CompanyDefaultRouteTag,
                    ),
                    inputs: CompanyFilterForm::render_inputs(
                        &FormCtx::form::<CompanyFilterForm>()
                            .value(CompanyFilterFormField::Name, &self.filter_name),
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
                (table_create_button::<CompanyTableKey, CompanyCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination::<CompanyTableKey>(
            &self.path_and_query,
            self.companies.number,
            self.companies.num_pages,
        );
        data_table_list_refresh::<CompanyTableKey>(
            "Companies",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for CompanyListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            crm_menu("companies"),
            companies_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(companies_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for CompanyListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "CRM Companies — Lariv",
            chrome,
            crm_menu("companies"),
            companies_list_crumbs(),
            self.render_table(),
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
    pub customer_id: Option<i64>,
    pub can_edit: bool,
}

impl CompanyDetailPage {
    fn body(&self) -> Markup {
        let customer_link = self.customer_id.map(|cid| {
            html! {
                a class="link" href={ "/customers/c/" (cid) "/" } { "Customer #" (cid) }
            }
        });
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label_inline("Address line 1", field_text(FieldText { value: &self.address_line_1, classes: "" })))
                    (label_inline("City", field_text(FieldText { value: &self.city, classes: "" })))
                    (label_inline("Website", field_text(FieldText { value: &self.website, classes: "" })))
                    @if let Some(link) = customer_link {
                        p { (link) }
                    }
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            a class="btn btn-outline btn-sm" href=(CompanyEditGetRouteTag::new(self.id).url()) { "Edit" }
                            (button_delete(
                                CompanyDeletePostRouteTag::new(self.id),
                                "Delete company",
                                "Permanently delete this company?",
                            ))
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for CompanyDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = company_crumbs(&self.name, self.id, None);
        scaffold_pane(
            company_detail_menu(&self.name, self.id, "detail", self.can_edit),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(company_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for CompanyDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Company — Lariv",
            chrome,
            company_detail_menu(&self.name, self.id, "detail", self.can_edit),
            company_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct CompanyFormPage {
    pub id: i64,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub website: String,
}

impl CompanyFormPage {
    fn body(&self) -> Markup {
        form(FormOpts {
            attrs: form_hx_post_main(CompanyEditPostRouteTag::new(self.id)),
            inputs: CompanyForm::render_inputs(
                &FormCtx::form::<CompanyForm>()
                    .value(CompanyFormField::Name, &self.name)
                    .value(CompanyFormField::AddressLine1, &self.address_line_1)
                    .value(CompanyFormField::AddressLine2, &self.address_line_2)
                    .value(CompanyFormField::City, &self.city)
                    .value(CompanyFormField::Pincode, &self.pincode)
                    .value(CompanyFormField::State, &self.state)
                    .value(CompanyFormField::Website, &self.website),
            ),
            actions: html! { (button_submit(ButtonSubmit { label: "Save", ..Default::default() })) },
            ..Default::default()
        })
    }
}

impl RenderAppPane for CompanyFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = company_crumbs(&self.name, self.id, Some("Edit"));
        scaffold_pane(
            company_detail_menu(&self.name, self.id, "edit", true),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(company_crumbs(&self.name, self.id, Some("Edit")), self.body())
    }
}

impl RenderTemplate for CompanyFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit company — Lariv",
            chrome,
            company_detail_menu(&self.name, self.id, "edit", true),
            company_crumbs(&self.name, self.id, Some("Edit")),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct CompanyCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub website: String,
    pub error: String,
}

impl RenderTemplate for CompanyCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<CompanyCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New company" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<CompanyCreateModalKey>(&modal_create_post_url(
                        CompanyCreatePostRouteTag,
                        CompanyCreateModalKey::FORM_NAME,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CompanyForm::render_inputs(
                        &FormCtx::form::<CompanyForm>()
                            .value(CompanyFormField::Name, &self.name)
                            .value(CompanyFormField::AddressLine1, &self.address_line_1)
                            .value(CompanyFormField::AddressLine2, &self.address_line_2)
                            .value(CompanyFormField::City, &self.city)
                            .value(CompanyFormField::Pincode, &self.pincode)
                            .value(CompanyFormField::State, &self.state)
                            .value(CompanyFormField::Website, &self.website),
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

#[derive(Generic)]
pub struct CompanySelectPage {
    pub companies: ObjectList<CompanyRow>,
    pub filter_name: String,
    pub target_input: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl RenderPickerSelect<CompanySelectTableKey, CompanySelectModalKey> for CompanySelectPage {
    fn render_table(&self) -> Markup {
        let headers = [TableColumnHeader {
            key: "Name",
            label: "Name",
            sort_url: None,
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .companies
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_select(&self.target_input, &a.id.to_string(), &a.name),
                cells: vec![field_text(FieldText { value: &a.name, classes: "" })],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        CompanySelectTableKey,
                        CompanySelectModalKey,
                        CompanyFkSelectRouteTag,
                    >(CompanyFkSelectRouteTag)
                    .set("hx-push-url", "false"),
                    inputs: html! {
                        (CompanyFilterForm::render_inputs(
                            &FormCtx::form::<CompanyFilterForm>()
                                .value(CompanyFilterFormField::Name, &self.filter_name),
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
                (picker_create_button::<CompanyCreateModalKey, CompanySelectModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<CompanySelectTableKey>(
            "Select company",
            actions,
            &headers,
            &rows,
            render_picker_pagination::<CompanySelectModalKey>(
                &self.path_and_query,
                self.companies.number,
                self.companies.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for CompanySelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

// --- Contacts ---

#[derive(Clone)]
pub struct ContactRow {
    pub id: i64,
    pub company_id: i64,
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
    pub path_and_query: String,
    pub can_edit: bool,
}

impl ContactListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: true },
            TableColumnHeader { key: "Company", label: "Company", sort_url: None, push_url: true },
            TableColumnHeader { key: "Email", label: "Email", sort_url: None, push_url: true },
        ];
        let rows: Vec<TableRow> = self
            .contacts
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_navigate_route(ContactDetailRouteTag::new(c.id)),
                cells: vec![
                    field_text(FieldText { value: &c.name, classes: "" }),
                    field_text(FieldText { value: &c.company_id.to_string(), classes: "" }),
                    field_text(FieldText { value: &c.email, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<ContactTableKey, ContactDefaultRouteTag>(
                        ContactDefaultRouteTag,
                    ),
                    inputs: ContactFilterForm::render_inputs(
                        &FormCtx::form::<ContactFilterForm>()
                            .value(ContactFilterFormField::CompanyId, &self.filter_company_id)
                            .display(
                                ContactFilterFormField::CompanyId,
                                &self.filter_company_display,
                            )
                            .value(ContactFilterFormField::Name, &self.filter_name),
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
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.ContactCreateForm",
                    href: &ContactCreateGetRouteTag.url(),
                    form_post_url: &ContactCreateGetRouteTag.path(),
                    modal_uid: ContactCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
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
        scaffold_pane(
            crm_menu("contacts"),
            contacts_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(contacts_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for ContactListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "CRM Contacts — Lariv",
            chrome,
            crm_menu("contacts"),
            contacts_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct ContactDetailPage {
    pub id: i64,
    pub company_id: i64,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub title: String,
    pub is_primary: bool,
    pub can_edit: bool,
}

impl ContactDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.display_name, classes: "" }))
                    (label_inline("Company", field_text(FieldText { value: &self.company_id.to_string(), classes: "" })))
                    (label_inline("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label_inline("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label_inline("Title", field_text(FieldText { value: &self.title, classes: "" })))
                    (label_inline("Primary", field_text(FieldText { value: if self.is_primary { "Yes" } else { "No" }, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            a class="btn btn-outline btn-sm" href=(ContactEditGetRouteTag::new(self.id).url()) { "Edit" }
                            (button_delete(
                                ContactDeletePostRouteTag::new(self.id),
                                "Delete contact",
                                "Permanently delete this contact?",
                            ))
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for ContactDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = contact_crumbs(&self.display_name, self.id, None);
        scaffold_pane(
            contact_detail_menu(&self.display_name, self.id, "detail", self.can_edit),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(contact_crumbs(&self.display_name, self.id, None), self.body())
    }
}

impl RenderTemplate for ContactDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Contact — Lariv",
            chrome,
            contact_detail_menu(&self.display_name, self.id, "detail", self.can_edit),
            contact_crumbs(&self.display_name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ContactFormPage {
    pub id: i64,
    pub company_id: i64,
    pub company_display: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub title: String,
    pub is_primary: String,
}

impl ContactFormPage {
    fn display_name(&self) -> String {
        let name = format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string();
        if name.is_empty() {
            format!("Contact #{}", self.id)
        } else {
            name
        }
    }

    fn body(&self) -> Markup {
        let company_id_s = fk_value(self.company_id);
        form(FormOpts {
            attrs: form_hx_post_main(ContactEditPostRouteTag::new(self.id)),
            inputs: ContactForm::render_inputs(
                &FormCtx::form::<ContactForm>()
                    .value(ContactFormField::CompanyId, company_id_s.as_str())
                    .display(ContactFormField::CompanyId, &self.company_display)
                    .value(ContactFormField::FirstName, &self.first_name)
                    .value(ContactFormField::LastName, &self.last_name)
                    .value(ContactFormField::Email, &self.email)
                    .value(ContactFormField::Phone, &self.phone)
                    .value(ContactFormField::Title, &self.title)
                    .value(ContactFormField::IsPrimary, &self.is_primary),
            ),
            actions: html! { (button_submit(ButtonSubmit { label: "Save", ..Default::default() })) },
            ..Default::default()
        })
    }
}

impl RenderAppPane for ContactFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = contact_crumbs(&self.display_name(), self.id, Some("Edit"));
        scaffold_pane(
            contact_detail_menu(&self.display_name(), self.id, "edit", true),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            contact_crumbs(&self.display_name(), self.id, Some("Edit")),
            self.body(),
        )
    }
}

impl RenderTemplate for ContactFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit contact — Lariv",
            chrome,
            contact_detail_menu(&self.display_name(), self.id, "edit", true),
            contact_crumbs(&self.display_name(), self.id, Some("Edit")),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ContactCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub company_id: i64,
    pub company_display: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub title: String,
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
                (form(FormOpts {
                    attrs: form_hx_post_url::<ContactCreateModalKey>(&modal_create_post_url(
                        ContactCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ContactForm::render_inputs(
                        &FormCtx::form::<ContactForm>()
                            .value(ContactFormField::CompanyId, company_id_s.as_str())
                            .display(ContactFormField::CompanyId, &self.company_display)
                            .value(ContactFormField::FirstName, &self.first_name)
                            .value(ContactFormField::LastName, &self.last_name)
                            .value(ContactFormField::Email, &self.email)
                            .value(ContactFormField::Phone, &self.phone)
                            .value(ContactFormField::Title, &self.title)
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
    pub path_and_query: String,
}

impl RenderPickerSelect<ContactSelectTableKey, ContactSelectModalKey> for ContactSelectPage {
    fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: false },
            TableColumnHeader { key: "Email", label: "Email", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self
            .contacts
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_select(&self.target_input, &c.id.to_string(), &c.name),
                cells: vec![
                    field_text(FieldText { value: &c.name, classes: "" }),
                    field_text(FieldText { value: &c.email, classes: "" }),
                ],
            })
            .collect();
        data_table_list_refresh::<ContactSelectTableKey>(
            "Select contact",
            table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<ContactSelectTableKey, ContactFkSelectRouteTag>(
                        ContactFkSelectRouteTag,
                    )
                    .set("hx-push-url", "false"),
                    inputs: html! {
                        (ContactFilterForm::render_inputs(
                            &FormCtx::form::<ContactFilterForm>()
                                .value(ContactFilterFormField::CompanyId, &self.filter_company_id)
                                .display(
                                    ContactFilterFormField::CompanyId,
                                    &self.filter_company_display,
                                )
                                .value(ContactFilterFormField::Name, &self.filter_name),
                        ))
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }),
            &headers,
            &rows,
            render_pagination::<ContactSelectTableKey>(
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

// --- Deals ---

#[derive(Clone)]
pub struct DealRow {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub stage: String,
    pub amount: String,
}

#[derive(Generic)]
pub struct DealListPage {
    pub deals: ObjectList<DealRow>,
    pub filter_company_id: String,
    pub filter_company_display: String,
    pub filter_name: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl DealListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [
            TableColumnHeader { key: "Name", label: "Name", sort_url: None, push_url: true },
            TableColumnHeader { key: "Stage", label: "Stage", sort_url: None, push_url: true },
            TableColumnHeader { key: "Amount", label: "Amount", sort_url: None, push_url: true },
        ];
        let rows: Vec<TableRow> = self
            .deals
            .items
            .iter()
            .map(|d| TableRow {
                attrs: row_attr_navigate_route(DealDetailRouteTag::new(d.id)),
                cells: vec![
                    field_text(FieldText { value: &d.name, classes: "" }),
                    field_text(FieldText { value: &d.stage, classes: "" }),
                    field_text(FieldText { value: &d.amount, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<DealTableKey, DealDefaultRouteTag>(DealDefaultRouteTag),
                    inputs: DealFilterForm::render_inputs(
                        &FormCtx::form::<DealFilterForm>()
                            .value(DealFilterFormField::CompanyId, &self.filter_company_id)
                            .display(
                                DealFilterFormField::CompanyId,
                                &self.filter_company_display,
                            )
                            .value(DealFilterFormField::Name, &self.filter_name),
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
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.DealCreateForm",
                    href: &DealCreateGetRouteTag.url(),
                    form_post_url: &DealCreateGetRouteTag.path(),
                    modal_uid: DealCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<DealTableKey>(
            "Deals",
            actions,
            &headers,
            &rows,
            render_pagination::<DealTableKey>(
                &self.path_and_query,
                self.deals.number,
                self.deals.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for DealListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(crm_menu("deals"), deals_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(deals_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for DealListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "CRM Deals — Lariv",
            chrome,
            crm_menu("deals"),
            deals_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct DealDetailPage {
    pub id: i64,
    pub company_id: i64,
    pub primary_contact_id: i64,
    pub name: String,
    pub amount: String,
    pub stage: String,
    pub expected_close_date: String,
    pub can_edit: bool,
}

impl DealDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label_inline("Company", field_text(FieldText { value: &self.company_id.to_string(), classes: "" })))
                    (label_inline("Primary contact", field_text(FieldText { value: &self.primary_contact_id.to_string(), classes: "" })))
                    (label_inline("Stage", field_text(FieldText { value: &self.stage, classes: "" })))
                    (label_inline("Amount", field_text(FieldText { value: &self.amount, classes: "" })))
                    (label_inline("Expected close", field_text(FieldText { value: &self.expected_close_date, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            a class="btn btn-outline btn-sm" href=(DealEditGetRouteTag::new(self.id).url()) { "Edit" }
                            (button_delete(
                                DealDeletePostRouteTag::new(self.id),
                                "Delete deal",
                                "Permanently delete this deal?",
                            ))
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for DealDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = deal_crumbs(&self.name, self.id, None);
        scaffold_pane(
            deal_detail_menu(&self.name, self.id, "detail", self.can_edit),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(deal_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for DealDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Deal — Lariv",
            chrome,
            deal_detail_menu(&self.name, self.id, "detail", self.can_edit),
            deal_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct DealFormPage {
    pub id: i64,
    pub company_id: i64,
    pub company_display: String,
    pub primary_contact_id: i64,
    pub primary_contact_display: String,
    pub name: String,
    pub amount: String,
    pub stage: String,
    pub expected_close_date: String,
}

impl DealFormPage {
    fn body(&self) -> Markup {
        let choices = DealForm::stage_choices();
        let company_id_s = fk_value(self.company_id);
        let primary_contact_id_s = fk_value(self.primary_contact_id);
        form(FormOpts {
            attrs: form_hx_post_main(DealEditPostRouteTag::new(self.id)),
            inputs: DealForm::render_inputs(
                &FormCtx::form::<DealForm>()
                    .value(DealFormField::CompanyId, company_id_s.as_str())
                    .display(DealFormField::CompanyId, &self.company_display)
                    .value(DealFormField::PrimaryContactId, primary_contact_id_s.as_str())
                    .display(
                        DealFormField::PrimaryContactId,
                        &self.primary_contact_display,
                    )
                    .value(DealFormField::Name, &self.name)
                    .value(DealFormField::Amount, &self.amount)
                    .value(DealFormField::Stage, &self.stage)
                    .value(DealFormField::ExpectedCloseDate, &self.expected_close_date)
                    .choices(
                        DealFormField::Stage,
                        &choices
                            .iter()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect::<Vec<_>>(),
                    ),
            ),
            actions: html! { (button_submit(ButtonSubmit { label: "Save", ..Default::default() })) },
            ..Default::default()
        })
    }
}

impl RenderAppPane for DealFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = deal_crumbs(&self.name, self.id, Some("Edit"));
        scaffold_pane(
            deal_detail_menu(&self.name, self.id, "edit", true),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(deal_crumbs(&self.name, self.id, Some("Edit")), self.body())
    }
}

impl RenderTemplate for DealFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit deal — Lariv",
            chrome,
            deal_detail_menu(&self.name, self.id, "edit", true),
            deal_crumbs(&self.name, self.id, Some("Edit")),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct DealCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub company_id: i64,
    pub company_display: String,
    pub primary_contact_id: i64,
    pub primary_contact_display: String,
    pub name: String,
    pub amount: String,
    pub stage: String,
    pub expected_close_date: String,
    pub error: String,
}

impl RenderTemplate for DealCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let choices = DealForm::stage_choices();
        let company_id_s = fk_value(self.company_id);
        let primary_contact_id_s = fk_value(self.primary_contact_id);
        modal_keyed::<DealCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New deal" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<DealCreateModalKey>(&modal_create_post_url(
                        DealCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: DealForm::render_inputs(
                        &FormCtx::form::<DealForm>()
                            .value(DealFormField::CompanyId, company_id_s.as_str())
                            .display(DealFormField::CompanyId, &self.company_display)
                            .value(DealFormField::PrimaryContactId, primary_contact_id_s.as_str())
                            .display(
                                DealFormField::PrimaryContactId,
                                &self.primary_contact_display,
                            )
                            .value(DealFormField::Name, &self.name)
                            .value(DealFormField::Amount, &self.amount)
                            .value(DealFormField::Stage, &self.stage)
                            .value(DealFormField::ExpectedCloseDate, &self.expected_close_date)
                            .choices(
                                DealFormField::Stage,
                                &choices
                                    .iter()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect::<Vec<_>>(),
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create deal", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}
