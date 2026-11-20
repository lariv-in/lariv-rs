//! Reusable customers table and create-modal markup.

use crate::create_modals::CUSTOMER_CREATE;

use maud::{Markup, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::components::{
    ButtonSubmit, DataTable, DataTableDisplay, FieldText, FilterClauseView, FormOpts,
    ListFilterBuilder, ObjectList, ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader,
    TableListContent, TableRow, button_submit, column_sort_url, data_table, field_text,
    filter_error_banner, form, form_hx_get_picker_route, form_hx_get_route, form_hx_post_url,
    list_filter_builder, modal_keyed, render_pagination, render_picker_pagination,
    row_attr_navigate, row_attr_select, sort_indicator, table_button_filter, table_create_button,
    table_list_content,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::{FragmentGet, ProvideRequestCaps, RouteUrl};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_core::web::modal_create_post_query;

use super::forms::{CustomerForm, CustomerFormField};
use super::keys::{
    CUSTOMER_CREATE_MODAL, CUSTOMER_SELECT_MODAL, CUSTOMER_SELECT_TABLE, CUSTOMER_TABLE,
};
use super::routes::{CustomerCreatePostRouteTag, CustomerFkSelectRouteTag};

define_register_items! {
    plugin: CustomerTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        CustomerCreateModalIdx: CustomerCreateModalPageTag => CustomerCreateModalPage,
        CustomerSelectIdx: CustomerSelectPageTag => CustomerSelectPage,
    ]
}

fn filtered_data_table(
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
    data_table(DataTable {
        uid: key.id(),
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
pub struct CustomerRow {
    pub id: i64,
    pub name: String,
    pub kind: &'static str,
    pub href: String,
}

pub struct CustomerListPage {
    pub customers: ObjectList<CustomerRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl CustomerListPage {
    /// Render the customers data table. `R` is the host list route (filter HTMX target).
    pub fn render_table<R>(&self) -> Markup
    where
        R: RouteUrl + FragmentGet + Default,
    {
        self.render_customers(CustomerTableMode::List {
            filter_attrs: form_hx_get_route(R::default()),
        })
    }

    /// Same columns as [`Self::render_table`], with rows that fill a foreign-key input.
    pub fn render_picker_table(&self, target_input: &str) -> Markup {
        self.render_customers(CustomerTableMode::Picker { target_input })
    }

    fn render_customers(&self, mode: CustomerTableMode<'_>) -> Markup {
        let picker = matches!(mode, CustomerTableMode::Picker { .. });
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: !picker,
            },
            TableColumnHeader {
                key: "Kind",
                label: "Kind",
                sort_url: None,
                push_url: !picker,
            },
        ];
        let rows: Vec<TableRow> = self
            .customers
            .items
            .iter()
            .map(|row| TableRow {
                attrs: match mode {
                    CustomerTableMode::Picker { target_input } => {
                        row_attr_select(target_input, &row.id.to_string(), &row.name)
                    }
                    CustomerTableMode::List { .. } if row.href.is_empty() => {
                        lariv_core::components::HtmlAttrs::new()
                    }
                    CustomerTableMode::List { .. } => row_attr_navigate(&row.href),
                },
                cells: vec![
                    field_text(FieldText {
                        value: &row.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: row.kind,
                        classes: "",
                    }),
                ],
                key: row.id.to_string(),
                children: Vec::new(),
            })
            .collect();
        let (key, title, filter_attrs, filter_extra, pagination, create) = match mode {
            CustomerTableMode::List { filter_attrs } => {
                let create = if lariv_core::components::role_permitted(
                    &lariv_plugin_users::role_authorization::roles_for::<
                        super::routes::CustomerMutate,
                    >(),
                ) {
                    table_create_button(
                        CUSTOMER_TABLE,
                        CUSTOMER_CREATE,
                        Some("plus"),
                        "btn-square btn-outline btn-sm",
                    )
                } else {
                    html! {}
                };
                (
                    CUSTOMER_TABLE,
                    "Customers",
                    filter_attrs,
                    html! {},
                    render_pagination(
                        CUSTOMER_TABLE,
                        &self.path_and_query,
                        self.customers.number,
                        self.customers.num_pages,
                    ),
                    create,
                )
            }
            CustomerTableMode::Picker { target_input } => {
                let create = if lariv_core::components::role_permitted(
                    &lariv_plugin_users::role_authorization::roles_for::<
                        super::routes::CustomerMutate,
                    >(),
                ) {
                    picker_create_button(
                        CUSTOMER_CREATE,
                        target_input,
                        Some("plus"),
                        "btn-square btn-outline btn-sm",
                    )
                } else {
                    html! {}
                };
                (
                    CUSTOMER_SELECT_TABLE,
                    "Select customer",
                    form_hx_get_picker_route(CustomerFkSelectRouteTag).set("hx-push-url", "false"),
                    html! {
                        input type="hidden" name="target_input" value=(target_input) {}
                    },
                    render_picker_pagination(
                        CUSTOMER_SELECT_MODAL,
                        &self.path_and_query,
                        self.customers.number,
                        self.customers.num_pages,
                    ),
                    create,
                )
            }
        };
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: filter_attrs,
                    apply_label: "Apply",
                    extra: filter_extra,
                }),
                ..Default::default()
            }))
            (create)
        };
        filtered_data_table(
            key,
            title,
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

enum CustomerTableMode<'a> {
    List {
        filter_attrs: lariv_core::components::HtmlAttrs,
    },
    Picker {
        target_input: &'a str,
    },
}

pub struct CustomerSelectPage {
    pub list: CustomerListPage,
    pub target_input: String,
}

impl RenderPickerSelect for CustomerSelectPage {
    fn render_table(&self) -> Markup {
        self.list.render_picker_table(&self.target_input)
    }
}

impl RenderTemplate for CustomerSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(CUSTOMER_SELECT_MODAL).into_inner()
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
            "p_customer.CustomerForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            CUSTOMER_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create customer",
                    subtitle: "A customer is one company or one contact.",
                    attrs: form_hx_post_url(
                        CUSTOMER_CREATE_MODAL,
                        &modal_create_post_query(
                            CustomerCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                            &self.target_input,
                        ),
                    ),
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

#[cfg(test)]
mod tests {
    use super::{CustomerListPage, CustomerRow};
    use lariv_core::components::ObjectList;
    use lariv_core::http::{FragmentGet, RouteTag, RouteUrl};

    #[derive(Default)]
    struct TestListRoute;
    impl RouteTag for TestListRoute {
        const PATH: &'static str = "/finance/customers";
    }
    impl RouteUrl for TestListRoute {
        fn path(self) -> String {
            Self::PATH.to_string()
        }
    }
    impl FragmentGet for TestListRoute {
        const TARGET: lariv_core::components::SwapKey = super::super::keys::CUSTOMER_TABLE;
    }

    #[test]
    fn list_shows_each_customer() {
        let page = CustomerListPage {
            customers: ObjectList::from_page(
                vec![
                    CustomerRow {
                        id: 1,
                        name: "Acme".into(),
                        kind: "Company",
                        href: "/dashboard/companies/4/".into(),
                    },
                    CustomerRow {
                        id: 2,
                        name: "Ada Lovelace".into(),
                        kind: "Contact",
                        href: "/dashboard/contacts/9/".into(),
                    },
                ],
                1,
                20,
                2,
            ),
            fields: Vec::new(),
            filters: Vec::new(),
            filter_errors: Vec::new(),
            sort: "Name ASC".into(),
            path_and_query: "/finance/customers/".into(),
            page_size: 20,
        };
        let html = page.render_table::<TestListRoute>().into_string();
        assert!(html.contains("Acme"), "{html}");
        assert!(html.contains("Ada Lovelace"), "{html}");
        assert!(html.contains("Company"), "{html}");
        assert!(html.contains("Contact"), "{html}");
        assert!(html.contains("/dashboard/companies/4/"), "{html}");
        assert!(html.contains("Customers"), "{html}");
    }
}
