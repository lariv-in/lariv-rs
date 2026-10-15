//! Customer picker and the create modal that fills it.

use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, DataTable, DataTableDisplay, FilterClauseView, FormOpts, ListFilterBuilder,
    ObjectList, PaginationPage, ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader,
    TableListContent, TablePagination, TableRow, button_submit, column_sort_url, data_table,
    filter_error_banner, form, form_hx_get_picker_route, form_hx_post_url, list_filter_builder,
    modal_keyed, pagination_pages, row_attr_select, sort_indicator, table_button_filter,
    table_list_content, table_pagination_picker,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_create_post_query;

use crate::forms::{CustomerForm, CustomerFormField};
use crate::keys::{CustomerCreateModalKey, CustomerSelectModalKey, CustomerSelectTableKey};
use crate::routes::CustomerFkSelectRouteTag;

pub struct CustomerRow {
    pub id: i64,
    pub name: String,
}

pub struct CustomerSelectPage {
    pub customers: ObjectList<CustomerRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<CustomerSelectTableKey, CustomerSelectModalKey> for CustomerSelectPage {
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
            .customers
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_select(&self.target_input, &row.id.to_string(), &row.name),
                cells: vec![lariv_core::components::field_text(
                    lariv_core::components::FieldText {
                        value: &row.name,
                        classes: "",
                    },
                )],
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
                    attrs: form_hx_get_picker_route::<
                        CustomerSelectTableKey,
                        CustomerSelectModalKey,
                        CustomerFkSelectRouteTag,
                    >(CustomerFkSelectRouteTag)
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
            &lariv_plugin_users::role_authorization::roles_for::<super::routes::CrmMutate>(),
        ) {
            actions = html! {
                (actions)
                (picker_create_button::<CustomerCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        filtered_data_table::<CustomerSelectTableKey>(
            "Select customer",
            actions,
            &headers,
            &rows,
            render_picker_pagination::<CustomerSelectModalKey>(
                &self.path_and_query,
                self.customers.number,
                self.customers.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for CustomerSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

pub struct CustomerCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub company_id: i64,
    pub company_display: String,
    pub individual_id: i64,
    pub individual_display: String,
    pub error: String,
}

impl RenderTemplate for CustomerCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_crm.CustomerCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<CustomerCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create customer",
                    subtitle: "A customer is one company or one contact.",
                    attrs: form_hx_post_url::<CustomerCreateModalKey>(&modal_create_post_query(
                        crate::routes::CustomerCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    inputs: CustomerForm::render_inputs(
                        &FormCtx::form::<CustomerForm>(CsrfToken::current())
                            .value(CustomerFormField::CompanyId, self.company_id.to_string())
                            .display(CustomerFormField::CompanyId, &self.company_display)
                            .value(
                                CustomerFormField::IndividualId,
                                self.individual_id.to_string(),
                            )
                            .display(CustomerFormField::IndividualId, &self.individual_display),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create customer", ..Default::default() }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

fn filtered_data_table<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: K::SELECTOR,
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
    data_table(DataTable {
        uid: K::ID,
        title,
        subtitle: "",
        classes: "w-full",
        actions,
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
