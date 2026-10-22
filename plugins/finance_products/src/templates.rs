use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::table::table_grid_content;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DataTable, DataTableDisplay, DeleteConfirmation,
    FieldText, FieldTitle, FilterClauseView, FormOpts, ListFilterBuilder, ManyToManyItem,
    ObjectList, PaginationPage, ShellChrome, SlotCapability, SlotRegistrar, SwapKey,
    TableButtonFilter, TableColumnHeader, TableListContent, TablePagination, TableRow, breadcrumbs,
    button_modal_form, button_submit, column_sort_url, container_column, container_row, data_table,
    delete_confirmation, detail, field_text, field_title, filter_error_banner, form,
    form_hx_get_route, form_hx_post_selector, form_hx_post_url, label, list_filter_builder, modal,
    modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select_extra, sort_indicator,
    table_button_columns, table_button_filter, table_create_button, table_list_content,
    table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::{modal_create_post_query, modal_edit_post_url};

use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use super::forms::{ProductForm, ProductFormField};

fn data_table_with_filter_errors<K: SwapKey>(
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
        (table_grid_content(headers, rows))
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
        uid: K::ID,
        title,
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
        pagination,
        refresh_url,
        column_keys: &column_keys,
        ..Default::default()
    })
}
use super::keys::{
    ProductCreateModalKey, ProductDeleteModalKey, ProductEditModalKey, ProductSelectModalKey,
    ProductSelectTableKey, ProductTableKey,
};
use super::routes::{
    ProductCreatePostRouteTag, ProductDefaultRouteTag, ProductDeleteGetRouteTag,
    ProductDeletePostRouteTag, ProductDetailRouteTag, ProductEditGetRouteTag,
    ProductEditPostRouteTag, ProductFkSelectRouteTag,
};

fn products_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Products",
        href: None,
    }])
}

fn product_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = ProductDefaultRouteTag.url();
    let detail_url = ProductDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Products",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Products",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

fn product_detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Product: {name}");
    let detail_url = ProductDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Product Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(menu_title, &nav, None, html! {})
}

lariv_core::define_register_items! {
    plugin: FinanceProductsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ProductListIdx: ProductListPageTag => ProductListPage,
        ProductDetailIdx: ProductDetailPageTag => ProductDetailPage,
        ProductEditModalIdx: ProductEditModalPageTag => ProductEditModalPage,
        ProductCreateModalIdx: ProductCreateModalPageTag => ProductCreateModalPage,
        ProductSelectIdx: ProductSelectPageTag => ProductSelectPage,
        ConfirmDeleteIdx: ProductConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

lariv_core::define_register_items! {
    plugin: FinanceProductsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn product_list_filter(page: &ProductListPage) -> Markup {
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: &page.sort,
        attrs: form_hx_get_route::<ProductTableKey, ProductDefaultRouteTag>(ProductDefaultRouteTag),
        apply_label: "Apply Filters",
        extra: html! {},
    })
}

fn product_select_filter(page: &ProductSelectPage) -> Markup {
    let target_input = page.target_input.clone();
    list_filter_builder(ListFilterBuilder {
        fields: &page.fields,
        rows: &page.filters,
        page_size: page.page_size,
        sort: &page.sort,
        attrs: form_hx_get_route::<ProductSelectTableKey, ProductFkSelectRouteTag>(
            ProductFkSelectRouteTag,
        )
        .set("hx-push-url", "false"),
        apply_label: "Apply",
        extra: html! {
            input type="hidden" name="target_input" value=(target_input) {}
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

#[derive(Clone)]
pub struct ProductRow {
    pub id: i64,
    pub stock_name: String,
    pub reference: String,
    pub name: String,
    pub base_price_formula: String,
    pub sales_price_formula: String,
    /// Plain unit sales price for picker → rate fill when the product has no variables.
    pub sales_price_value: String,
    pub hsn_code: String,
    pub has_formula: bool,
    /// JSON array of `{name, type, placeholder}` for the invoice line editor.
    pub variables_json: String,
    /// Copied onto an invoice line when this product is picked.
    pub remarks: String,
}

#[derive(Generic)]
pub struct ProductListPage {
    pub products: ObjectList<ProductRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl ProductListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let stock_sort = column_sort_url(&self.path_and_query, "Stock", &self.sort);
        let reference_sort = column_sort_url(&self.path_and_query, "Reference", &self.sort);
        let base_price_sort = column_sort_url(&self.path_and_query, "BasePriceFormula", &self.sort);
        let sales_price_sort =
            column_sort_url(&self.path_and_query, "SalesPriceFormula", &self.sort);
        let hsn_sort = column_sort_url(&self.path_and_query, "HSN", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let stock_label = format!("Stock{}", sort_indicator(&self.sort, "Stock"));
        let reference_label = format!("Reference{}", sort_indicator(&self.sort, "Reference"));
        let base_price_label = format!(
            "Base price of one product{}",
            sort_indicator(&self.sort, "BasePriceFormula")
        );
        let sales_price_label = format!(
            "Sales price of one product{}",
            sort_indicator(&self.sort, "SalesPriceFormula")
        );
        let hsn_label = format!("HSN{}", sort_indicator(&self.sort, "HSN"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Stock",
                label: &stock_label,
                sort_url: Some(&stock_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Reference",
                label: &reference_label,
                sort_url: Some(&reference_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "BasePriceFormula",
                label: &base_price_label,
                sort_url: Some(&base_price_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "SalesPriceFormula",
                label: &sales_price_label,
                sort_url: Some(&sales_price_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "HSN",
                label: &hsn_label,
                sort_url: Some(&hsn_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .products
            .items
            .iter()
            .map(|p| TableRow {
                attrs: row_attr_navigate_route(ProductDetailRouteTag::new(p.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &p.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.stock_name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.reference,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.base_price_formula,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.sales_price_formula,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &p.hsn_code,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: product_list_filter(self),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                super::routes::FinanceProductsMutate,
            >(),
        ) {
            actions = html! {
                (actions)
                (table_create_button::<ProductTableKey, ProductCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination::<ProductTableKey>(
            &self.path_and_query,
            self.products.number,
            self.products.num_pages,
        );
        data_table_with_filter_errors::<ProductTableKey>(
            "Products",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for ProductListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, products_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(products_list_crumbs(), self.body())
    }
}

impl RenderTemplate for ProductListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Products",
            chrome,
            products_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct ProductDetailPage {
    pub id: i64,
    pub name: String,
    pub stock_name: String,
    pub reference: String,
    pub remarks: String,
    pub hsn_code: String,
    pub taxes: String,
    pub variables: String,
    pub base_price_formula: String,
    pub sales_price_formula: String,
}

impl ProductDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label("Stock", field_text(FieldText { value: &self.stock_name, classes: "" })))
                    (label("Reference", field_text(FieldText { value: &self.reference, classes: "" })))
                    (label("Remarks", field_text(FieldText { value: &self.remarks, classes: "" })))
                    (label("Taxes", field_text(FieldText { value: &self.taxes, classes: "" })))
                    (label("Variables", field_text(FieldText {
                        value: if self.variables.trim().is_empty() { "—" } else { self.variables.as_str() },
                        classes: "",
                    })))
                    (label("Base price of one product", field_text(FieldText {
                        value: if self.base_price_formula.trim().is_empty() { "—" } else { self.base_price_formula.as_str() },
                        classes: "",
                    })))
                    (label("Sales price of one product", field_text(FieldText {
                        value: if self.sales_price_formula.trim().is_empty() { "—" } else { self.sales_price_formula.as_str() },
                        classes: "",
                    })))
                    (label("HSN code", field_text(FieldText { value: &self.hsn_code, classes: "" })))
                    @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<super::routes::FinanceProductsMutate>()) {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_finance_products.ProductEditForm",
                                href: &ProductEditGetRouteTag::new(self.id).url(),
                                form_post_url: &ProductEditPostRouteTag::new(self.id).path(),
                                modal_uid: ProductEditModalKey::ID,
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

    fn menu(&self) -> Markup {
        product_detail_menu(self.id, &self.name)
    }
}

impl RenderAppPane for ProductDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = product_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(product_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for ProductDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = product_crumbs(self.id, &self.name, None);
        app_scaffold_with_sidebar("Product", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct ProductEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub stock_id: i64,
    pub stock_display: String,
    pub reference: String,
    pub remarks: String,
    pub variables: Vec<String>,
    pub base_price_formula: String,
    pub sales_price_formula: String,
    pub hsn_code: i64,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for ProductEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = ProductDeleteGetRouteTag::new(self.id).url();
        let stock_value = if self.stock_id > 0 {
            self.stock_id.to_string()
        } else {
            String::new()
        };
        modal_keyed::<ProductEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit product" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ProductEditModalKey>(&modal_edit_post_url(
                        ProductEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ProductForm::render_inputs(
                        &FormCtx::form::<ProductForm>(CsrfToken::current())
                            .value(ProductFormField::Name, &self.name)
                            .value(ProductFormField::StockId, stock_value.as_str())
                            .display(ProductFormField::StockId, &self.stock_display)
                            .value(ProductFormField::Reference, &self.reference)
                            .value(ProductFormField::Remarks, &self.remarks)
                            .list(ProductFormField::Variables, &self.variables)
                            .value(ProductFormField::BasePriceFormula, &self.base_price_formula)
                            .value(
                                ProductFormField::SalesPriceFormula,
                                &self.sales_price_formula,
                            )
                            .value(ProductFormField::HsnCode, self.hsn_code.to_string())
                            .m2m(ProductFormField::TaxIds, &self.tax_items),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_finance_products.ProductDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: ProductDeleteModalKey::ID,
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
pub struct ProductCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub stock_id: i64,
    pub stock_display: String,
    pub reference: String,
    pub remarks: String,
    pub variables: Vec<String>,
    pub base_price_formula: String,
    pub sales_price_formula: String,
    pub hsn_code: i64,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for ProductCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_products.ProductCreateForm"
        } else {
            self.form_name.as_str()
        };
        let stock_value = if self.stock_id > 0 {
            self.stock_id.to_string()
        } else {
            String::new()
        };
        modal_keyed::<ProductCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Product",
                    subtitle: "Create a new product",
                    classes: "@container",
                    attrs: form_hx_post_url::<ProductCreateModalKey>(&modal_create_post_query(
                        ProductCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ProductForm::render_inputs(
                        &FormCtx::form::<ProductForm>(CsrfToken::current())
                            .value(ProductFormField::Name, &self.name)
                            .value(ProductFormField::StockId, stock_value.as_str())
                            .display(ProductFormField::StockId, &self.stock_display)
                            .value(ProductFormField::Reference, &self.reference)
                            .value(ProductFormField::Remarks, &self.remarks)
                            .list(ProductFormField::Variables, &self.variables)
                            .value(ProductFormField::BasePriceFormula, &self.base_price_formula)
                            .value(
                                ProductFormField::SalesPriceFormula,
                                &self.sales_price_formula,
                            )
                            .value(ProductFormField::HsnCode, self.hsn_code.to_string())
                            .m2m(ProductFormField::TaxIds, &self.tax_items),
                    ),
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Product",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

#[derive(Generic)]
pub struct ProductSelectPage {
    pub products: ObjectList<ProductRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<ProductSelectTableKey, ProductSelectModalKey> for ProductSelectPage {
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
            .products
            .items
            .iter()
            .map(|p| TableRow {
                attrs: row_attr_select_extra(
                    &self.target_input,
                    &p.id.to_string(),
                    &p.name,
                    &[
                        ("sales_price", p.sales_price_value.as_str()),
                        ("has_formula", if p.has_formula { "1" } else { "0" }),
                        ("variables", p.variables_json.as_str()),
                        ("remarks", p.remarks.as_str()),
                    ],
                ),
                cells: vec![field_text(FieldText {
                    value: &p.name,
                    classes: "",
                })],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: product_select_filter(self),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                super::routes::FinanceProductsMutate,
            >(),
        ) {
            actions = html! {
                (actions)
                (picker_create_button::<ProductCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination::<ProductSelectTableKey>(
            &self.path_and_query,
            self.products.number,
            self.products.num_pages,
        );
        data_table_with_filter_errors::<ProductSelectTableKey>(
            "Select Product",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for ProductSelectPage {
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
            format!("#{}", ProductDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            ProductDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = ProductDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
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
