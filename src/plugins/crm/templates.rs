use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldText,
        FieldTitle, FormOpts, LayoutMain, LayoutSidebar, ManyToManyItem, ObjectList,
        PaginationPage, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SlotCapability,
        SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow,
        button_clear, button_modal_form, button_post_route, button_submit, column_sort_url,
        container_column, container_row, data_table_list_refresh, delete_confirmation, detail,
        detail_header, field_text, field_title, form, form_hx_get_picker_route, form_hx_get_route,
        form_hx_post_route, form_hx_post_selector, form_hx_post_url, label, layout_main,
        layout_sidebar, modal, modal_keyed, pagination_pages, row_attr_navigate,
        row_attr_navigate_route, row_attr_select, row_attr_select_multi_extra, shell_scaffold,
        sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_filter,
        table_create_button, table_pagination, table_pagination_picker,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::{RenderPickerSelect, picker_create_button},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_query, modal_create_post_url, modal_edit_post_url},
};

use super::crumbs::{
    companies_list_crumbs, company_crumbs, completed_task_crumbs, contact_crumbs,
    contacts_list_crumbs, converted_lead_crumbs, failed_lead_crumbs, lead_crumbs, lead_tag_crumbs,
    lead_tags_list_crumbs, lead_update_crumbs, leads_list_crumbs, task_crumbs, tasks_list_crumbs,
};
use super::detail_menu::{
    company_detail_menu, completed_task_detail_menu, contact_detail_menu,
    converted_lead_detail_menu, failed_lead_detail_menu, lead_detail_menu, lead_tag_detail_menu,
    task_detail_menu,
};
use super::forms::{
    CompanyFilterForm, CompanyFilterFormField, CompanyForm, CompanyFormField, ContactFilterForm,
    ContactFilterFormField, ContactForm, ContactFormField, ConvertLeadForm, FailLeadForm,
    FailLeadFormField, LeadFilterForm, LeadFilterFormField, LeadForm, LeadFormField,
    LeadTagFilterForm, LeadTagFilterFormField, LeadTagForm, LeadTagFormField, LeadUpdateForm,
    LeadUpdateFormField, LeadUpdateQuickForm, TaskFilterForm, TaskFilterFormField, TaskForm,
    TaskFormField,
};
use super::keys::{
    CompanyCreateModalKey, CompanyDeleteModalKey, CompanyEditModalKey, CompanySelectModalKey,
    CompanySelectTableKey, CompanyTableKey, ContactCreateModalKey, ContactDeleteModalKey,
    ContactEditModalKey, ContactSelectModalKey, ContactSelectTableKey, ContactTableKey,
    LEAD_UPDATE_SAVED_EVENT, LeadConvertModalKey, LeadCreateModalKey, LeadDeleteModalKey,
    LeadEditModalKey, LeadFailModalKey, LeadHubTableKey, LeadTagCreateModalKey,
    LeadTagDeleteModalKey, LeadTagEditModalKey, LeadTagLeadsTableKey, LeadTagSelectModalKey,
    LeadTagSelectTableKey, LeadTagTableKey, LeadUpdateDeleteModalKey, LeadUpdateEditModalKey,
    LeadUpdatesKey, TaskCreateModalKey, TaskDeleteModalKey, TaskEditModalKey, TaskTableKey,
};
use super::routes::{
    CompanyCreatePostRouteTag, CompanyDefaultRouteTag, CompanyDeleteGetRouteTag,
    CompanyDeletePostRouteTag, CompanyDetailRouteTag, CompanyEditGetRouteTag,
    CompanyEditPostRouteTag, CompanyFkSelectRouteTag, ContactCreatePostRouteTag,
    ContactDefaultRouteTag, ContactDeleteGetRouteTag, ContactDeletePostRouteTag,
    ContactDetailRouteTag, ContactEditGetRouteTag, ContactEditPostRouteTag,
    ContactFkSelectRouteTag, ConvertedLeadReactivatePostRouteTag, FailedLeadReactivatePostRouteTag,
    LeadConvertGetRouteTag, LeadConvertPostRouteTag, LeadCreateGetRouteTag, LeadCreatePostRouteTag,
    LeadDefaultRouteTag, LeadDeleteGetRouteTag, LeadDeletePostRouteTag, LeadDetailRouteTag,
    LeadEditGetRouteTag, LeadEditPostRouteTag, LeadFailGetRouteTag, LeadFailPostRouteTag,
    LeadTagCreatePostRouteTag, LeadTagDefaultRouteTag, LeadTagDeleteGetRouteTag,
    LeadTagDeletePostRouteTag, LeadTagDetailRouteTag, LeadTagEditGetRouteTag,
    LeadTagEditPostRouteTag, LeadTagSelectRouteTag, LeadUpdateAddPostRouteTag,
    LeadUpdateDeleteGetRouteTag, LeadUpdateDeletePostRouteTag, LeadUpdateEditGetRouteTag,
    LeadUpdateEditPostRouteTag, TaskCompletePostRouteTag, TaskCreatePostRouteTag,
    TaskDefaultRouteTag, TaskDeleteGetRouteTag, TaskDeletePostRouteTag, TaskEditGetRouteTag,
    TaskEditPostRouteTag,
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

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn parse_hex_rgb(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
    let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
    let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
    Some((r, g, b))
}

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let s = f64::from(channel) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// White or near-black text so the label stays readable on `color`.
fn contrasting_text_on(color: &str) -> &'static str {
    let Some((r, g, b)) = parse_hex_rgb(color) else {
        return "#ffffff";
    };
    let luminance = 0.2126 * srgb_channel_to_linear(r)
        + 0.7152 * srgb_channel_to_linear(g)
        + 0.0722 * srgb_channel_to_linear(b);
    if luminance > 0.179 {
        "#111827"
    } else {
        "#ffffff"
    }
}

fn render_lead_tag_chip(tag: &LeadTagChip) -> Markup {
    use crate::components::attrs::escape_attr;
    use crate::components::hx_nav_app_layout_for_url;
    use maud::PreEscaped;

    let href = LeadTagDetailRouteTag::new(tag.id).url();
    let text = contrasting_text_on(&tag.color);
    html! {
        (PreEscaped(format!(
            r#"<a href="{href}" class="badge border-0 font-medium"{attrs} style="background-color: {bg}; color: {fg};">"#,
            href = escape_attr(&href),
            attrs = hx_nav_app_layout_for_url(&href).as_string(),
            bg = escape_attr(&tag.color),
            fg = escape_attr(text),
        )))
        (tag.name)
        (PreEscaped("</a>"))
    }
}

fn render_lead_tags(tags: &[LeadTagChip]) -> Markup {
    label(
        "Tags",
        html! {
            div class="flex flex-wrap gap-2" {
                @for tag in tags {
                    (render_lead_tag_chip(tag))
                }
            }
        },
    )
}

fn lead_form_inputs(
    contact_id: i64,
    contact_display: &str,
    source: &str,
    notes: &str,
    tags: &[ManyToManyItem],
) -> Markup {
    let choices = LeadForm::source_choices();
    let contact_id_s = fk_value(contact_id);
    let choice_pairs: Vec<(String, String)> = choices
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    LeadForm::render_inputs(
        &FormCtx::form::<LeadForm>()
            .value(LeadFormField::ContactId, contact_id_s.as_str())
            .display(LeadFormField::ContactId, contact_display)
            .value(LeadFormField::Source, source)
            .value(LeadFormField::Notes, notes)
            .m2m(LeadFormField::Tags, tags)
            .choices(LeadFormField::Source, &choice_pairs),
    )
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
                title: "Tags",
                url: &LeadTagDefaultRouteTag.url(),
                active: active == "tags",
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
                title: "Tasks",
                url: &TaskDefaultRouteTag.url(),
                active: active == "tasks",
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

fn tab_href(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(LeadDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn tag_leads_tab_href(tag_id: i64, tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(LeadTagDetailRouteTag::new(tag_id))
        .query("tab", tab)
        .build()
}

fn tab_nav_link(href: &str, active: bool, label: &str) -> Markup {
    use crate::components::attrs::escape_attr;
    use maud::PreEscaped;

    let cls = if active { "tab tab-active" } else { "tab" };
    let nav = crate::components::nav_content_attrs(href);
    html! {
        (PreEscaped(format!(
            r#"<a class="{cls}" href="{href}"{attrs}>"#,
            cls = escape_attr(cls),
            href = escape_attr(href),
            attrs = nav.as_string(),
        )))
        (label)
        (PreEscaped("</a>"))
    }
}

fn render_leads_data_table<K: SwapKey>(
    title: &str,
    leads: &ObjectList<LeadRow>,
    sort: &str,
    path_and_query: &str,
    actions: Markup,
) -> Markup {
    let name_sort = column_sort_url(path_and_query, "Name", sort);
    let company_sort = column_sort_url(path_and_query, "Company", sort);
    let email_sort = column_sort_url(path_and_query, "Email", sort);
    let source_sort = column_sort_url(path_and_query, "Source", sort);
    let name_label = format!("Name{}", sort_indicator(sort, "Name"));
    let company_label = format!("Company{}", sort_indicator(sort, "Company"));
    let email_label = format!("Email{}", sort_indicator(sort, "Email"));
    let source_label = format!("Source{}", sort_indicator(sort, "Source"));
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
        TableColumnHeader {
            key: "Source",
            label: &source_label,
            sort_url: Some(&source_sort),
            push_url: true,
        },
        TableColumnHeader {
            key: "Status",
            label: "Status",
            sort_url: None,
            push_url: true,
        },
    ];
    let rows: Vec<TableRow> = leads
        .items
        .iter()
        .map(|r| TableRow {
            attrs: row_attr_navigate(&r.detail_href),
            cells: vec![
                field_text(FieldText {
                    value: &r.name,
                    classes: "",
                }),
                field_text(FieldText {
                    value: &r.company,
                    classes: "",
                }),
                field_text(FieldText {
                    value: &r.email,
                    classes: "",
                }),
                field_text(FieldText {
                    value: &r.source,
                    classes: "",
                }),
                field_text(FieldText {
                    value: &r.status,
                    classes: "",
                }),
            ],
        })
        .collect();
    let pagination = render_pagination::<K>(path_and_query, leads.number, leads.num_pages);
    data_table_list_refresh::<K>(title, actions, &headers, &rows, pagination, path_and_query)
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
        LeadEditModalIdx: LeadEditModalPageTag => LeadEditModalPage,
        LeadCreateModalIdx: LeadCreateModalPageTag => LeadCreateModalPage,
        LeadTagCreateModalIdx: LeadTagCreateModalPageTag => LeadTagCreateModalPage,
        LeadTagSelectIdx: LeadTagSelectPageTag => LeadTagSelectPage,
        LeadTagListIdx: LeadTagListPageTag => LeadTagListPage,
        LeadTagDetailIdx: LeadTagDetailPageTag => LeadTagDetailPage,
        LeadTagEditModalIdx: LeadTagEditModalPageTag => LeadTagEditModalPage,
        ConvertLeadModalIdx: ConvertLeadModalPageTag => ConvertLeadModalPage,
        FailLeadModalIdx: FailLeadModalPageTag => FailLeadModalPage,
        LeadConvertDetailIdx: LeadConvertDetailPageTag => LeadConvertDetailPage,
        LeadFailDetailIdx: LeadFailDetailPageTag => LeadFailDetailPage,
        CompanyListIdx: CompanyListPageTag => CompanyListPage,
        CompanyDetailIdx: CompanyDetailPageTag => CompanyDetailPage,
        CompanyEditModalIdx: CompanyEditModalPageTag => CompanyEditModalPage,
        CompanyCreateModalIdx: CompanyCreateModalPageTag => CompanyCreateModalPage,
        CompanySelectIdx: CompanySelectPageTag => CompanySelectPage,
        ContactListIdx: ContactListPageTag => ContactListPage,
        ContactDetailIdx: ContactDetailPageTag => ContactDetailPage,
        ContactEditModalIdx: ContactEditModalPageTag => ContactEditModalPage,
        ContactCreateModalIdx: ContactCreateModalPageTag => ContactCreateModalPage,
        ContactSelectIdx: ContactSelectPageTag => ContactSelectPage,
        TaskListIdx: TaskListPageTag => TaskListPage,
        TaskDetailIdx: TaskDetailPageTag => TaskDetailPage,
        CompletedTaskDetailIdx: CompletedTaskDetailPageTag => CompletedTaskDetailPage,
        TaskEditModalIdx: TaskEditModalPageTag => TaskEditModalPage,
        TaskCreateModalIdx: TaskCreateModalPageTag => TaskCreateModalPage,
        LeadUpdateDetailIdx: LeadUpdateDetailPageTag => LeadUpdateDetailPage,
        LeadUpdateEditModalIdx: LeadUpdateEditModalPageTag => LeadUpdateEditModalPage,
        ConfirmDeleteIdx: CrmConfirmDeletePageTag => ConfirmDeletePage,
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
    pub filter_company_id: i64,
    pub filter_company_display: String,
    pub filter_contact: String,
    pub filter_tags: Vec<ManyToManyItem>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl LeadHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        tab_nav_link(&tab_href(tab), self.tab == tab, label)
    }

    pub fn render_table(&self) -> Markup {
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<LeadHubTableKey, LeadDefaultRouteTag>(
                        LeadDefaultRouteTag,
                    ),
                    inputs: LeadFilterForm::render_inputs(
                        &FormCtx::form::<LeadFilterForm>()
                            .value(
                                LeadFilterFormField::CompanyId,
                                &fk_value(self.filter_company_id),
                            )
                            .display(
                                LeadFilterFormField::CompanyId,
                                &self.filter_company_display,
                            )
                            .value(LeadFilterFormField::Contact, &self.filter_contact)
                            .m2m(LeadFilterFormField::Tags, &self.filter_tags),
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
        render_leads_data_table::<LeadHubTableKey>(
            "Leads",
            &self.leads,
            &self.sort,
            &self.path_and_query,
            actions,
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
    pub contact_id: i64,
    pub contact_display: String,
    pub company_id: i64,
    pub company: String,
    pub email: String,
    pub source: String,
    pub notes: String,
    pub tags: Vec<LeadTagChip>,
    pub can_edit: bool,
    pub updates: LeadUpdatesPanel,
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
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadEditForm",
                    href: &LeadEditGetRouteTag::new(self.id).url(),
                    form_post_url: &LeadEditPostRouteTag::new(self.id).path(),
                    modal_uid: LeadEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = lead_crumbs(&self.display_name, self.id, None);
        scaffold_pane(
            lead_detail_menu(&self.display_name, self.id, "detail"),
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
            lead_detail_menu(&self.display_name, self.id, "detail"),
            lead_crumbs(&self.display_name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct LeadEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub contact_id: i64,
    pub contact_display: String,
    pub source: String,
    pub notes: String,
    pub tags: Vec<ManyToManyItem>,
    pub reason: String,
    pub show_reason: bool,
    pub error: String,
}

impl RenderTemplate for LeadEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = LeadDeleteGetRouteTag::new(self.id).url();
        let mut inputs = lead_form_inputs(
            self.contact_id,
            &self.contact_display,
            &self.source,
            &self.notes,
            &self.tags,
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
        modal_keyed::<LeadEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit lead" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadEditModalKey>(&modal_edit_post_url(
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
                            modal_uid: LeadDeleteModalKey::ID,
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
pub struct LeadCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub contact_id: i64,
    pub contact_display: String,
    pub source: String,
    pub notes: String,
    pub tags: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for LeadCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
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
                    inputs: lead_form_inputs(
                        self.contact_id,
                        &self.contact_display,
                        &self.source,
                        &self.notes,
                        &self.tags,
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
    pub error: String,
}

impl RenderTemplate for ConvertLeadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<LeadConvertModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Convert lead" }
                p class="mb-4 text-sm opacity-80" {
                    "Convert this lead using its existing company and contact."
                }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadConvertModalKey>(&modal_create_post_url(
                        LeadConvertPostRouteTag::new(self.lead_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ConvertLeadForm::render_inputs(&FormCtx::form::<ConvertLeadForm>()),
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
    pub company: String,
    pub contact_display: String,
    pub email: String,
    pub source: String,
    pub notes: String,
    pub tags: Vec<LeadTagChip>,
    pub can_edit: bool,
    pub updates: LeadUpdatesPanel,
}

impl LeadConvertDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
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
                    modal_uid: LeadFailModalKey::ID,
                    label: "Mark failed",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadEditForm",
                    href: &LeadEditGetRouteTag::new(self.lead_id).url(),
                    form_post_url: &LeadEditPostRouteTag::new(self.lead_id).path(),
                    modal_uid: LeadEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = converted_lead_crumbs(&self.display_name, self.converted_id, None);
        scaffold_pane(
            converted_lead_detail_menu(&self.display_name, self.converted_id, "detail"),
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
    pub tags: Vec<LeadTagChip>,
    pub can_edit: bool,
    pub updates: LeadUpdatesPanel,
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
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadEditForm",
                    href: &LeadEditGetRouteTag::new(self.lead_id).url(),
                    form_post_url: &LeadEditPostRouteTag::new(self.lead_id).path(),
                    modal_uid: LeadEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = failed_lead_crumbs(&self.display_name, self.failed_id, None);
        scaffold_pane(
            failed_lead_detail_menu(&self.display_name, self.failed_id, "detail"),
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
            failed_lead_detail_menu(&self.display_name, self.failed_id, "detail"),
            failed_lead_crumbs(&self.display_name, self.failed_id, None),
            self.body(),
        )
    }
}

// --- Lead tags ---

#[derive(Clone)]
pub struct LeadTagOption {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Clone)]
pub struct LeadTagChip {
    pub id: i64,
    pub name: String,
    pub color: String,
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
        modal_keyed::<LeadTagCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New lead tag" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadTagCreateModalKey>(&modal_create_post_query(
                        LeadTagCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadTagForm::render_inputs(
                        &FormCtx::form::<LeadTagForm>()
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

#[derive(Generic)]
pub struct LeadTagSelectPage {
    pub tags: ObjectList<LeadTagOption>,
    pub filter_name: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl RenderPickerSelect<LeadTagSelectTableKey, LeadTagSelectModalKey> for LeadTagSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "Tags"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .tags
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_select_multi_extra(
                    target,
                    &t.id.to_string(),
                    &t.name,
                    &[("color", t.color.as_str())],
                ),
                cells: vec![html! {
                    span class="inline-flex items-center gap-2" {
                        span class="w-3 h-3 rounded-full shrink-0" style=(format!("background-color: {}", t.color)) {}
                        (t.name)
                    }
                }],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        LeadTagSelectTableKey,
                        LeadTagSelectModalKey,
                        LeadTagSelectRouteTag,
                    >(LeadTagSelectRouteTag)
                    .set("hx-push-url", "false"),
                    inputs: html! {
                        (LeadTagFilterForm::render_inputs(
                            &FormCtx::form::<LeadTagFilterForm>()
                                .value(LeadTagFilterFormField::Name, &self.filter_name),
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
                (picker_create_button::<LeadTagCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<LeadTagSelectTableKey>(
            "Select tags",
            actions,
            &headers,
            &rows,
            render_picker_pagination::<LeadTagSelectModalKey>(
                &self.path_and_query,
                self.tags.number,
                self.tags.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for LeadTagSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Clone)]
pub struct LeadTagRow {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Generic)]
pub struct LeadTagListPage {
    pub tags: ObjectList<LeadTagRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl LeadTagListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .tags
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_navigate_route(LeadTagDetailRouteTag::new(t.id)),
                cells: vec![html! {
                    span class="inline-flex items-center gap-2" {
                        span class="w-3 h-3 rounded-full shrink-0" style=(format!("background-color: {}", t.color)) {}
                        (t.name)
                    }
                }],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<LeadTagTableKey, LeadTagDefaultRouteTag>(
                        LeadTagDefaultRouteTag,
                    ),
                    inputs: LeadTagFilterForm::render_inputs(
                        &FormCtx::form::<LeadTagFilterForm>()
                            .value(LeadTagFilterFormField::Name, &self.filter_name),
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
                (table_create_button::<LeadTagTableKey, LeadTagCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination::<LeadTagTableKey>(
            &self.path_and_query,
            self.tags.number,
            self.tags.num_pages,
        );
        data_table_list_refresh::<LeadTagTableKey>(
            "Tags",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for LeadTagListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            crm_menu("tags"),
            lead_tags_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(lead_tags_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for LeadTagListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "CRM Tags — Lariv",
            chrome,
            crm_menu("tags"),
            lead_tags_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct LeadTagDetailPage {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub can_edit: bool,
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
        render_leads_data_table::<LeadTagLeadsTableKey>(
            "Leads",
            &self.leads,
            &self.sort,
            &self.path_and_query,
            html! {},
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
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.LeadTagEditForm",
                                href: &LeadTagEditGetRouteTag::new(self.id).url(),
                                form_post_url: &LeadTagEditPostRouteTag::new(self.id).path(),
                                modal_uid: LeadTagEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = lead_tag_crumbs(&self.name, self.id, None);
        scaffold_pane(
            lead_tag_detail_menu(&self.name, self.id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
        modal_keyed::<LeadTagEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit tag" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadTagEditModalKey>(&modal_edit_post_url(
                        LeadTagEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadTagForm::render_inputs(
                        &FormCtx::form::<LeadTagForm>()
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
                            modal_uid: LeadTagDeleteModalKey::ID,
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

// --- Companies ---

#[derive(Clone)]
pub struct CompanyRow {
    pub id: i64,
    pub name: String,
    pub website: String,
}

#[derive(Generic)]
pub struct CompanyListPage {
    pub companies: ObjectList<CompanyRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl CompanyListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .companies
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_navigate_route(CompanyDetailRouteTag::new(a.id)),
                cells: vec![field_text(FieldText {
                    value: &a.name,
                    classes: "",
                })],
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
    pub can_edit: bool,
}

impl CompanyDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label("Address line 1", field_text(FieldText { value: &self.address_line_1, classes: "" })))
                    (label("City", field_text(FieldText { value: &self.city, classes: "" })))
                    (label("Website", field_text(FieldText { value: &self.website, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.CompanyEditForm",
                                href: &CompanyEditGetRouteTag::new(self.id).url(),
                                form_post_url: &CompanyEditPostRouteTag::new(self.id).path(),
                                modal_uid: CompanyEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = company_crumbs(&self.name, self.id, None);
        scaffold_pane(
            company_detail_menu(&self.name, self.id, "detail"),
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
            company_detail_menu(&self.name, self.id, "detail"),
            company_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct CompanyEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub website: String,
    pub error: String,
}

impl RenderTemplate for CompanyEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = CompanyDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<CompanyEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit company" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<CompanyEditModalKey>(&modal_edit_post_url(
                        CompanyEditPostRouteTag::new(self.id),
                        &self.form_name,
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
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_crm.CompanyDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: CompanyDeleteModalKey::ID,
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
    pub error: String,
}

impl RenderTemplate for CompanyCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<CompanyCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New company" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<CompanyCreateModalKey>(&modal_create_post_query(
                        CompanyCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
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
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl RenderPickerSelect<CompanySelectTableKey, CompanySelectModalKey> for CompanySelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .companies
            .items
            .iter()
            .map(|a| TableRow {
                attrs: row_attr_select(&self.target_input, &a.id.to_string(), &a.name),
                cells: vec![field_text(FieldText {
                    value: &a.name,
                    classes: "",
                })],
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
                (picker_create_button::<CompanyCreateModalKey>(
                    &self.target_input,
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
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
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
                        value: &c.company_id.to_string(),
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
    pub email: String,
    pub phone: String,
    pub is_primary: bool,
    pub can_edit: bool,
}

impl ContactDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.display_name, classes: "" }))
                    (label("Company", field_text(FieldText { value: &self.company_id.to_string(), classes: "" })))
                    (label("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label("Primary", field_text(FieldText { value: if self.is_primary { "Yes" } else { "No" }, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.ContactEditForm",
                                href: &ContactEditGetRouteTag::new(self.id).url(),
                                form_post_url: &ContactEditPostRouteTag::new(self.id).path(),
                                modal_uid: ContactEditModalKey::ID,
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

impl RenderAppPane for ContactDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = contact_crumbs(&self.display_name, self.id, None);
        scaffold_pane(
            contact_detail_menu(&self.display_name, self.id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            contact_crumbs(&self.display_name, self.id, None),
            self.body(),
        )
    }
}

impl RenderTemplate for ContactDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Contact — Lariv",
            chrome,
            contact_detail_menu(&self.display_name, self.id, "detail"),
            contact_crumbs(&self.display_name, self.id, None),
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
                (form(FormOpts {
                    attrs: form_hx_post_url::<ContactEditModalKey>(&modal_edit_post_url(
                        ContactEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ContactForm::render_inputs(
                        &FormCtx::form::<ContactForm>()
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
                            name: "p_crm.ContactDeleteForm",
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
                (form(FormOpts {
                    attrs: form_hx_post_url::<ContactCreateModalKey>(&modal_create_post_query(
                        ContactCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ContactForm::render_inputs(
                        &FormCtx::form::<ContactForm>()
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
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        ContactSelectTableKey,
                        ContactSelectModalKey,
                        ContactFkSelectRouteTag,
                    >(ContactFkSelectRouteTag)
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

// --- Tasks ---

fn task_tab_href(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(TaskDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn task_filter_clear_button(assigned_to_id: &str, assigned_to_display: &str) -> Markup {
    use crate::components::attrs::escape_attr;
    use maud::PreEscaped;
    let onclick = "const f=this.closest('form');f.querySelectorAll('input[name=Title]').forEach(el=>el.value='');window.dispatchEvent(new CustomEvent('fk-select',{detail:{name:'AssignedToId',value:this.dataset.defaultAssignedToId,display:this.dataset.defaultAssignedToDisplay}}));";
    html! {
        (PreEscaped(format!(
            r#"<button type="button" class="btn btn-ghost" data-default-assigned-to-id="{id}" data-default-assigned-to-display="{display}" onclick="{onclick}">"#,
            id = escape_attr(assigned_to_id),
            display = escape_attr(assigned_to_display),
            onclick = escape_attr(onclick),
        )))
        "Clear"
        (PreEscaped("</button>"))
    }
}

#[derive(Clone)]
pub struct TaskRow {
    pub id: i64,
    pub title: String,
    pub assigned_to: String,
    pub assigned_to_id: i64,
    pub due_date: String,
    pub status: String,
    pub completed_at: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct TaskListPage {
    pub tasks: ObjectList<TaskRow>,
    pub tab: String,
    pub filter_title: String,
    pub filter_assigned_to_id: String,
    pub filter_assigned_to_display: String,
    pub default_assigned_to_id: String,
    pub default_assigned_to_display: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl TaskListPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        use crate::components::attrs::escape_attr;
        use maud::PreEscaped;

        let active = self.tab == tab;
        let cls = if active { "tab tab-active" } else { "tab" };
        let href = task_tab_href(tab);
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
        let completed = self.tab == "completed";
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let assigned_sort = column_sort_url(&self.path_and_query, "AssignedTo", &self.sort);
        let due_sort = column_sort_url(&self.path_and_query, "DueDate", &self.sort);
        let status_sort = column_sort_url(&self.path_and_query, "Status", &self.sort);
        let completed_sort = column_sort_url(&self.path_and_query, "CompletedAt", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let assigned_label = format!("Assigned To{}", sort_indicator(&self.sort, "AssignedTo"));
        let due_label = format!("Due Date{}", sort_indicator(&self.sort, "DueDate"));
        let status_label = format!("Status{}", sort_indicator(&self.sort, "Status"));
        let completed_label = format!("Completed At{}", sort_indicator(&self.sort, "CompletedAt"));
        let headers = if completed {
            vec![
                TableColumnHeader {
                    key: "Title",
                    label: &title_label,
                    sort_url: Some(&title_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "AssignedTo",
                    label: &assigned_label,
                    sort_url: Some(&assigned_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "DueDate",
                    label: &due_label,
                    sort_url: Some(&due_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "CompletedAt",
                    label: &completed_label,
                    sort_url: Some(&completed_sort),
                    push_url: true,
                },
            ]
        } else {
            vec![
                TableColumnHeader {
                    key: "Title",
                    label: &title_label,
                    sort_url: Some(&title_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "AssignedTo",
                    label: &assigned_label,
                    sort_url: Some(&assigned_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "DueDate",
                    label: &due_label,
                    sort_url: Some(&due_sort),
                    push_url: true,
                },
                TableColumnHeader {
                    key: "Status",
                    label: &status_label,
                    sort_url: Some(&status_sort),
                    push_url: true,
                },
            ]
        };
        let rows: Vec<TableRow> = self
            .tasks
            .items
            .iter()
            .map(|t| {
                let fourth = if completed {
                    t.completed_at.as_str()
                } else {
                    t.status.as_str()
                };
                TableRow {
                    attrs: row_attr_navigate(&t.detail_href),
                    cells: vec![
                        field_text(FieldText {
                            value: &t.title,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &t.assigned_to,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &t.due_date,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: fourth,
                            classes: "",
                        }),
                    ],
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<TaskTableKey, TaskDefaultRouteTag>(
                        TaskDefaultRouteTag,
                    ),
                    inputs: html! {
                        input type="hidden" name="tab" value=(self.tab) {}
                        (TaskFilterForm::render_inputs(
                            &FormCtx::form::<TaskFilterForm>()
                                .value(TaskFilterFormField::Title, &self.filter_title)
                                .value(
                                    TaskFilterFormField::AssignedToId,
                                    &self.filter_assigned_to_id,
                                )
                                .display(
                                    TaskFilterFormField::AssignedToId,
                                    &self.filter_assigned_to_display,
                                ),
                        ))
                    },
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                            (task_filter_clear_button(
                                &self.default_assigned_to_id,
                                &self.default_assigned_to_display,
                            ))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit && !completed {
            actions = html! {
                (actions)
                (table_create_button::<TaskTableKey, TaskCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<TaskTableKey>(
            "Tasks",
            actions,
            &headers,
            &rows,
            render_pagination::<TaskTableKey>(
                &self.path_and_query,
                self.tasks.number,
                self.tasks.num_pages,
            ),
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        html! {
            div class="tabs tabs-boxed mb-4" {
                (self.tab_link("uncompleted", "Uncompleted"))
                (self.tab_link("completed", "Completed"))
            }
            (self.render_table())
        }
    }
}

impl RenderAppPane for TaskListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(crm_menu("tasks"), tasks_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(tasks_list_crumbs(), self.body())
    }
}

impl RenderTemplate for TaskListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "CRM Tasks — Lariv",
            chrome,
            crm_menu("tasks"),
            tasks_list_crumbs(),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct TaskDetailPage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub assigned_to: String,
    pub due_date: String,
    pub status: String,
    pub can_edit: bool,
}

impl TaskDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.title, classes: "" }))
                    (label("Assigned To", field_text(FieldText { value: &self.assigned_to, classes: "" })))
                    (label("Due Date", field_text(FieldText { value: &self.due_date, classes: "" })))
                    (label("Status", field_text(FieldText { value: &self.status, classes: "" })))
                    (label("Description", field_text(FieldText { value: &self.description, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_post_route(
                                TaskCompletePostRouteTag::new(self.id),
                                "Mark as completed",
                                "btn-primary",
                            ))
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.TaskEditForm",
                                href: &TaskEditGetRouteTag::new(self.id).url(),
                                form_post_url: &TaskEditPostRouteTag::new(self.id).path(),
                                modal_uid: TaskEditModalKey::ID,
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

impl RenderAppPane for TaskDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = task_crumbs(&self.title, self.id, None);
        scaffold_pane(
            task_detail_menu(&self.title, self.id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(task_crumbs(&self.title, self.id, None), self.body())
    }
}

impl RenderTemplate for TaskDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Task — Lariv",
            chrome,
            task_detail_menu(&self.title, self.id, "detail"),
            task_crumbs(&self.title, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct CompletedTaskDetailPage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub assigned_to: String,
    pub due_date: String,
    pub completed_at: String,
}

impl CompletedTaskDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.title, classes: "" }))
                    (label("Assigned To", field_text(FieldText { value: &self.assigned_to, classes: "" })))
                    (label("Due Date", field_text(FieldText { value: &self.due_date, classes: "" })))
                    (label("Completed at", field_text(FieldText { value: &self.completed_at, classes: "" })))
                    (label("Description", field_text(FieldText { value: &self.description, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for CompletedTaskDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = completed_task_crumbs(&self.title, self.id, None);
        scaffold_pane(
            completed_task_detail_menu(&self.title, self.id, "detail"),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            completed_task_crumbs(&self.title, self.id, None),
            self.body(),
        )
    }
}

impl RenderTemplate for CompletedTaskDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Completed task — Lariv",
            chrome,
            completed_task_detail_menu(&self.title, self.id, "detail"),
            completed_task_crumbs(&self.title, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct TaskEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub description: String,
    pub assigned_to_id: i64,
    pub assigned_to_display: String,
    pub due_date: String,
    pub error: String,
}

impl RenderTemplate for TaskEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TaskDeleteGetRouteTag::new(self.id).url();
        let assigned_to_id_s = fk_value(self.assigned_to_id);
        modal_keyed::<TaskEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit task" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<TaskEditModalKey>(&modal_edit_post_url(
                        TaskEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TaskForm::render_inputs(
                        &FormCtx::form::<TaskForm>()
                            .value(TaskFormField::Title, &self.title)
                            .value(TaskFormField::Description, &self.description)
                            .value(TaskFormField::AssignedToId, assigned_to_id_s.as_str())
                            .display(TaskFormField::AssignedToId, &self.assigned_to_display)
                            .value(TaskFormField::DueDate, &self.due_date),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_crm.TaskDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: TaskDeleteModalKey::ID,
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
pub struct TaskCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub description: String,
    pub assigned_to_id: i64,
    pub assigned_to_display: String,
    pub due_date: String,
    pub error: String,
}

impl RenderTemplate for TaskCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let assigned_to_id_s = fk_value(self.assigned_to_id);
        modal_keyed::<TaskCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New task" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<TaskCreateModalKey>(&modal_create_post_url(
                        TaskCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TaskForm::render_inputs(
                        &FormCtx::form::<TaskForm>()
                            .value(TaskFormField::Title, &self.title)
                            .value(TaskFormField::Description, &self.description)
                            .value(TaskFormField::AssignedToId, assigned_to_id_s.as_str())
                            .display(TaskFormField::AssignedToId, &self.assigned_to_display)
                            .value(TaskFormField::DueDate, &self.due_date),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create task", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

// --- Lead updates ---

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
    pub can_edit: bool,
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
        let attrs =
            form_hx_post_route::<LeadUpdatesKey, _>(LeadUpdateAddPostRouteTag::new(self.lead_id))
                .set("x-data", x_data)
                .set(format!("@{LEAD_UPDATE_SAVED_EVENT}"), "clearDraft()");
        form(FormOpts {
            attrs,
            inputs: LeadUpdateQuickForm::render_inputs(&FormCtx::form::<LeadUpdateQuickForm>()),
            actions: html! {
                (button_submit(ButtonSubmit {
                    label: "Add Update",
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }

    pub fn render_list(&self) -> Markup {
        html! {
            div id=(LeadUpdatesKey::ID) class="max-h-96 overflow-y-auto flex flex-col divide-y divide-base-300 border border-base-300 rounded-box" {
                @if self.items.is_empty() {
                    div class="text-sm opacity-60 px-3 py-4 text-center" { "No updates" }
                } @else {
                    @for item in &self.items {
                        div class="px-3 py-2 text-sm flex gap-2 items-start" {
                            div class="min-w-0 flex-1" {
                                div class="text-xs opacity-60 whitespace-nowrap" { (item.datetime) }
                                div class="whitespace-pre-wrap break-words" { (item.description) }
                            }
                            @if self.can_edit {
                                div class="flex gap-1 shrink-0" {
                                    (button_modal_form(ButtonModalForm {
                                        name: "p_crm.LeadUpdateEditForm",
                                        href: &LeadUpdateEditGetRouteTag::new(item.id).url(),
                                        form_post_url: &LeadUpdateEditPostRouteTag::new(item.id).path(),
                                        modal_uid: LeadUpdateEditModalKey::ID,
                                        label: "Edit",
                                        classes: "btn-ghost btn-xs",
                                        ..Default::default()
                                    }))
                                    (button_modal_form(ButtonModalForm {
                                        name: "p_crm.LeadUpdateDeleteForm",
                                        href: &LeadUpdateDeleteGetRouteTag::new(item.id).url(),
                                        form_post_url: &LeadUpdateDeleteGetRouteTag::new(item.id).url(),
                                        modal_uid: LeadUpdateDeleteModalKey::ID,
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
                @if self.can_edit {
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
    pub can_edit: bool,
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
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_crm.LeadUpdateEditForm",
                                href: &LeadUpdateEditGetRouteTag::new(self.id).url(),
                                form_post_url: &LeadUpdateEditPostRouteTag::new(self.id).path(),
                                modal_uid: LeadUpdateEditModalKey::ID,
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = lead_update_crumbs(&self.display_name, self.lead_id, &self.datetime);
        scaffold_pane(
            lead_detail_menu(&self.display_name, self.lead_id, ""),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
        modal_keyed::<LeadUpdateEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit update" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<LeadUpdateEditModalKey>(&modal_edit_post_url(
                        LeadUpdateEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: LeadUpdateForm::render_inputs(
                        &FormCtx::form::<LeadUpdateForm>()
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
                            modal_uid: LeadUpdateDeleteModalKey::ID,
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
            format!("#{}", LeadDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            LeadDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.modal_uid == CompanyDeleteModalKey::ID {
            CompanyDeletePostRouteTag::new(self.id).url()
        } else if self.modal_uid == ContactDeleteModalKey::ID {
            ContactDeletePostRouteTag::new(self.id).url()
        } else if self.modal_uid == TaskDeleteModalKey::ID {
            TaskDeletePostRouteTag::new(self.id).url()
        } else if self.modal_uid == LeadTagDeleteModalKey::ID {
            LeadTagDeletePostRouteTag::new(self.id).url()
        } else if self.modal_uid == LeadUpdateDeleteModalKey::ID {
            LeadUpdateDeletePostRouteTag::new(self.id).url()
        } else {
            LeadDeletePostRouteTag::new(self.id).url()
        };
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
