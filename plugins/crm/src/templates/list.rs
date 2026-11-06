use maud::{Markup, html};

use lariv_core::components::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::components::{
    ButtonModalForm, DataTable, DataTableDisplay, FieldText, FilterClauseView, ListFilterBuilder,
    ObjectList, ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader, TableListContent,
    TableRow, button_modal_form, column_sort_url, contrasting_text_on, data_table, field_text,
    filter_error_banner, form_hx_get_picker_route, form_hx_get_route, label, list_filter_builder,
    render_pagination, render_picker_pagination, row_attr_navigate, row_attr_navigate_route,
    row_attr_select_multi_extra, sort_indicator, table_button_columns, table_button_filter,
    table_create_button, table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::crumbs::{lead_tags_list_crumbs, leads_list_crumbs};
use crate::keys::{
    LEAD_CREATE_MODAL, LEAD_HUB_TABLE, LEAD_TAG_SELECT_MODAL, LEAD_TAG_SELECT_TABLE, LEAD_TAG_TABLE,
};
use crate::routes::{
    LeadCreateGetRouteTag, LeadDefaultRouteTag, LeadTagDefaultRouteTag, LeadTagDetailRouteTag,
    LeadTagSelectRouteTag,
};

use super::create::LEAD_TAG_CREATE;
use super::{app_scaffold, crm_menu, scaffold_main, scaffold_pane};

pub(crate) fn filtered_data_table(
    key: SwapKey,
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let hx_target = key.selector();
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: &hx_target,
        }))
    };
    let grid = html! {
        (filter_error_banner(errors))
        (lariv_core::components::table::table_grid_content(headers, rows))
    };
    let column_keys: Vec<&str> = headers
        .iter()
        .map(|header| header.key)
        .filter(|key| !key.is_empty())
        .collect();
    let toolbar = if headers.len() > 1 {
        html! {
            (table_button_columns(headers))
            (actions)
        }
    } else {
        actions
    };
    data_table(DataTable {
        uid: key.id(),
        title,
        subtitle: "",
        classes: "w-full",
        actions: toolbar,
        displays: vec![
            DataTableDisplay {
                name: "Grid".into(),
                html: grid,
            },
            DataTableDisplay {
                name: "List".into(),
                html: list,
            },
        ],
        default_view: "List",
        pagination,
        oob: false,
        refresh_url,
        column_keys: &column_keys,
        instance_id: "",
        body_only: false,
    })
}
pub(crate) fn tab_href(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(LeadDefaultRouteTag)
        .query("tab", tab)
        .build()
}

pub(crate) fn tag_leads_tab_href(tag_id: i64, tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(LeadTagDetailRouteTag::new(tag_id))
        .query("tab", tab)
        .build()
}

pub(crate) fn tab_nav_link(href: &str, active: bool, label: &str) -> Markup {
    use lariv_core::components::attrs::escape_attr;
    use maud::PreEscaped;

    let cls = if active { "tab tab-active" } else { "tab" };
    let nav = lariv_core::components::nav_content_attrs(href);
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

pub(crate) fn render_leads_data_table(
    key: SwapKey,
    title: &str,
    leads: &ObjectList<LeadRow>,
    sort: &str,
    path_and_query: &str,
    actions: Markup,
    errors: &[FilterRowError],
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
            key: String::new(),
            children: Vec::new(),
        })
        .collect();
    let pagination = render_pagination(key, path_and_query, leads.number, leads.num_pages);
    filtered_data_table(
        key,
        title,
        actions,
        &headers,
        &rows,
        pagination,
        path_and_query,
        errors,
    )
}

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

pub(crate) fn render_lead_tag_chip(tag: &LeadTagChip) -> Markup {
    use lariv_core::components::attrs::escape_attr;
    use lariv_core::components::hx_nav_app_layout_for_url;
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

pub(crate) fn render_lead_tags(tags: &[LeadTagChip]) -> Markup {
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

pub struct LeadHubPage {
    pub leads: ObjectList<LeadRow>,
    pub tab: String,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl LeadHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        tab_nav_link(&tab_href(tab), self.tab == tab, label)
    }

    pub fn render_table(&self) -> Markup {
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        LeadDefaultRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="tab" value=(self.tab.as_str()) {}
                    },
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>(),
        ) && self.tab == "active"
        {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_crm.LeadCreateForm",
                    href: &LeadCreateGetRouteTag.url(),
                    form_post_url: &LeadCreateGetRouteTag.path(),
                    modal_uid: LEAD_CREATE_MODAL.id(),
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        render_leads_data_table(
            LEAD_HUB_TABLE,
            "Leads",
            &self.leads,
            &self.sort,
            &self.path_and_query,
            actions,
            &self.filter_errors,
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
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = leads_list_crumbs();
        scaffold_pane(crm_menu("leads"), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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

pub struct LeadTagSelectPage {
    pub tags: ObjectList<LeadTagOption>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect for LeadTagSelectPage {
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
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_picker_route(LeadTagSelectRouteTag)
                    .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input) {}
                    },
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>(),
        ) {
            actions = html! {
                (actions)
                (picker_create_button(LEAD_TAG_CREATE,
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        filtered_data_table(
            LEAD_TAG_SELECT_TABLE,
            "Select tags",
            actions,
            &headers,
            &rows,
            render_picker_pagination(
                LEAD_TAG_SELECT_MODAL,
                &self.path_and_query,
                self.tags.number,
                self.tags.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for LeadTagSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(LEAD_TAG_SELECT_MODAL).into_inner()
    }
}

#[derive(Clone)]
pub struct LeadTagRow {
    pub id: i64,
    pub name: String,
    pub color: String,
}

pub struct LeadTagListPage {
    pub tags: ObjectList<LeadTagRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
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
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        LeadTagDefaultRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::CrmMutate>(),
        ) {
            actions = html! {
                (actions)
                (table_create_button(LEAD_TAG_TABLE, LEAD_TAG_CREATE,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination(
            LEAD_TAG_TABLE,
            &self.path_and_query,
            self.tags.number,
            self.tags.num_pages,
        );
        filtered_data_table(
            LEAD_TAG_TABLE,
            "Tags",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderAppPane for LeadTagListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            crm_menu("tags"),
            lead_tags_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
