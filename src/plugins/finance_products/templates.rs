use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonDeletePost, ButtonSubmit, Crumb, FieldText, FieldTitle, FormOpts,
        ManyToManyItem, ObjectList, PaginationPage, ShellChrome, SlotCapability, SlotRegistrar,
        SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, ButtonModalForm,
        breadcrumbs, button_clear, button_delete_post_route, button_modal_form, button_submit,
        container_column, container_row, data_table_list_refresh, detail, field_text, field_title,
        form, form_hx_get_route, form_hx_post_url, label_inline, modal_keyed, pagination_pages,
        row_attr_navigate_route, row_attr_select_extra, column_sort_url, sort_indicator,
        table_button_filter, table_pagination,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    picker::RenderPickerSelect,
    web::{modal_create_post_url, modal_edit_post_url},
};

use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use crate::plugins::finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use super::forms::{
    ProductFilterForm, ProductFilterFormField, ProductForm, ProductFormField,
};
use super::keys::{
    ProductCreateModalKey, ProductEditModalKey, ProductSelectModalKey, ProductSelectTableKey,
    ProductTableKey,
};
use super::routes::{
    ProductCreateGetRouteTag, ProductCreatePostRouteTag, ProductDefaultRouteTag,
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

crate::define_register_items! {
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
    ]
}

crate::define_register_items! {
    plugin: FinanceProductsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn product_filter_form(name: &str, reference: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<ProductTableKey, ProductDefaultRouteTag>(
            ProductDefaultRouteTag,
        ),
        inputs: ProductFilterForm::render_inputs(
            &FormCtx::form::<ProductFilterForm>()
                .value(ProductFilterFormField::Name, name)
                .value(ProductFilterFormField::Reference, reference),
        ),
        actions: html! {
            (container_row("flex gap-2", html! {
                (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
            }))
        },
        ..Default::default()
    })
}

fn product_select_filter_form(name: &str, reference: &str, target_input: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<ProductSelectTableKey, ProductFkSelectRouteTag>(
            ProductFkSelectRouteTag,
        )
        .set("hx-push-url", "false"),
        inputs: html! {
            (ProductFilterForm::render_inputs(
                &FormCtx::form::<ProductFilterForm>()
                    .value(ProductFilterFormField::Name, name)
                    .value(ProductFilterFormField::Reference, reference),
            ))
            input type="hidden" name="target_input" value=(target_input) {}
        },
        actions: html! {
            (container_row("flex gap-2", html! {
                (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
            }))
        },
        ..Default::default()
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
    pub product_type: String,
    pub reference: String,
    pub name: String,
    pub base_cost: String,
    pub sales_price: String,
    /// Plain numeric sales price for picker → rate fill (no currency symbol).
    pub sales_price_value: String,
    pub hsn_code: String,
}

#[derive(Generic)]
pub struct ProductListPage {
    pub products: ObjectList<ProductRow>,
    pub filter_name: String,
    pub filter_reference: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl ProductListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let reference_sort = column_sort_url(&self.path_and_query, "Reference", &self.sort);
        let base_cost_sort = column_sort_url(&self.path_and_query, "BaseCost", &self.sort);
        let sales_price_sort = column_sort_url(&self.path_and_query, "SalesPrice", &self.sort);
        let hsn_sort = column_sort_url(&self.path_and_query, "HSN", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let reference_label = format!("Reference{}", sort_indicator(&self.sort, "Reference"));
        let base_cost_label = format!("Base cost{}", sort_indicator(&self.sort, "BaseCost"));
        let sales_price_label = format!("Sales price{}", sort_indicator(&self.sort, "SalesPrice"));
        let hsn_label = format!("HSN{}", sort_indicator(&self.sort, "HSN"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Reference",
                label: &reference_label,
                sort_url: Some(&reference_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "BaseCost",
                label: &base_cost_label,
                sort_url: Some(&base_cost_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "SalesPrice",
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
                    field_text(FieldText { value: &p.name, classes: "" }),
                    field_text(FieldText { value: &p.product_type, classes: "" }),
                    field_text(FieldText { value: &p.reference, classes: "" }),
                    field_text(FieldText {
                        value: &p.base_cost,
                        classes: "text-end tabular-nums",
                    }),
                    field_text(FieldText {
                        value: &p.sales_price,
                        classes: "text-end tabular-nums",
                    }),
                    field_text(FieldText { value: &p.hsn_code, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: product_filter_form(&self.filter_name, &self.filter_reference),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_products.ProductCreateForm",
                    href: &ProductCreateGetRouteTag.url(),
                    form_post_url: &ProductCreateGetRouteTag.path(),
                    modal_uid: ProductCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<ProductTableKey>(
            &self.path_and_query,
            self.products.number,
            self.products.num_pages,
        );
        data_table_list_refresh::<ProductTableKey>(
            "Products",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for ProductListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, products_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
    pub product_type: String,
    pub reference: String,
    pub remarks: String,
    pub base_cost: String,
    pub sales_price: String,
    pub hsn_code: String,
    pub taxes: String,
    pub can_edit: bool,
}

impl ProductDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label_inline("Type", field_text(FieldText { value: &self.product_type, classes: "" })))
                    (label_inline("Reference", field_text(FieldText { value: &self.reference, classes: "" })))
                    (label_inline("Remarks", field_text(FieldText { value: &self.remarks, classes: "" })))
                    (label_inline("Taxes", field_text(FieldText { value: &self.taxes, classes: "" })))
                    (label_inline("Base cost", field_text(FieldText { value: &self.base_cost, classes: "" })))
                    (label_inline("Sales price", field_text(FieldText { value: &self.sales_price, classes: "" })))
                    (label_inline("HSN code", field_text(FieldText { value: &self.hsn_code, classes: "" })))
                    @if self.can_edit {
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = product_crumbs(self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
    pub product_type: String,
    pub reference: String,
    pub remarks: String,
    pub base_cost: String,
    pub sales_price: String,
    pub hsn_code: i64,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for ProductEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let choices = ProductForm::product_type_choices();
        modal_keyed::<ProductEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit product" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<ProductEditModalKey>(&modal_edit_post_url(
                        ProductEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: ProductForm::render_inputs(
                        &FormCtx::form::<ProductForm>()
                            .value(ProductFormField::Name, &self.name)
                            .value(ProductFormField::ProductType, &self.product_type)
                            .value(ProductFormField::Reference, &self.reference)
                            .value(ProductFormField::Remarks, &self.remarks)
                            .value(ProductFormField::BaseCost, &self.base_cost)
                            .value(ProductFormField::SalesPrice, &self.sales_price)
                            .value(ProductFormField::HsnCode, self.hsn_code.to_string())
                            .m2m(ProductFormField::TaxIds, &self.tax_items)
                            .choices(
                                ProductFormField::ProductType,
                                &choices
                                    .iter()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect::<Vec<_>>(),
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_delete_post_route(
                            ProductDeletePostRouteTag::new(self.id),
                            ButtonDeletePost {
                                label: "Delete",
                                confirm: "Permanently delete this product?",
                                classes: "btn-error",
                            },
                        ))
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
    pub name: String,
    pub product_type: String,
    pub reference: String,
    pub remarks: String,
    pub base_cost: String,
    pub sales_price: String,
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
        let choices = ProductForm::product_type_choices();
        modal_keyed::<ProductCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Product",
                subtitle: "Create a new product",
                classes: "@container",
                attrs: form_hx_post_url::<ProductCreateModalKey>(
                    &modal_create_post_url(
                        ProductCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: ProductForm::render_inputs(
                    &FormCtx::form::<ProductForm>()
                        .value(ProductFormField::Name, &self.name)
                        .value(ProductFormField::ProductType, &self.product_type)
                        .value(ProductFormField::Reference, &self.reference)
                        .value(ProductFormField::Remarks, &self.remarks)
                        .value(ProductFormField::BaseCost, &self.base_cost)
                        .value(ProductFormField::SalesPrice, &self.sales_price)
                        .value(ProductFormField::HsnCode, self.hsn_code.to_string())
                        .m2m(ProductFormField::TaxIds, &self.tax_items)
                        .choices(
                            ProductFormField::ProductType,
                            &choices
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect::<Vec<_>>(),
                        ),
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
            }),
        )
    }
}

#[derive(Generic)]
pub struct ProductSelectPage {
    pub products: ObjectList<ProductRow>,
    pub filter_name: String,
    pub filter_reference: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl RenderPickerSelect<ProductSelectTableKey, ProductSelectModalKey> for ProductSelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .products
            .items
            .iter()
            .map(|p| TableRow {
                attrs: row_attr_select_extra(
                    &self.target_input,
                    &p.id.to_string(),
                    &p.name,
                    &[("sales_price", p.sales_price_value.as_str())],
                ),
                cells: vec![
                    field_text(FieldText { value: &p.name, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: product_select_filter_form(
                    &self.filter_name,
                    &self.filter_reference,
                    &self.target_input,
                ),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_products.ProductCreateForm",
                    href: &ProductCreateGetRouteTag.url(),
                    form_post_url: &ProductCreateGetRouteTag.path(),
                    modal_uid: ProductCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<ProductSelectTableKey>(
            &self.path_and_query,
            self.products.number,
            self.products.num_pages,
        );
        data_table_list_refresh::<ProductSelectTableKey>(
            "Select Product",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for ProductSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
