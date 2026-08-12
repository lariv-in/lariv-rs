use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonDeletePost, ButtonSubmit, Crumb, FieldText, FieldTitle, FormOpts,
        ObjectList, PaginationPage, ShellChrome, SlotCapability, SlotRegistrar, SwapKey,
        TableButtonFilter, TableColumnHeader, TablePagination, TableRow, ButtonModalForm,
        breadcrumbs, button_clear, button_delete_post_route, button_modal_form, button_submit,
        container_column, container_row, data_table_list_refresh, detail, field_text, field_title,
        form, form_hx_get_route, form_hx_post_url, label_inline, modal_keyed, pagination_pages,
        row_attr_navigate_route, row_attr_select, column_sort_url, sort_indicator,
        table_button_filter, table_pagination,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::RenderPickerSelect,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

#[cfg(not(feature = "plugin-finance-customer"))]
use crate::{
    components::{
        LayoutMain, LayoutSidebar, ShellScaffold, SidebarMenu, SidebarMenuItem, layout_main,
        layout_sidebar, shell_scaffold, sidebar_menu, sidebar_menu_item_pane,
    },
    template::RenderAppPane,
};

use super::forms::{
    CustomerFilterForm, CustomerFilterFormField, CustomerForm, CustomerFormField,
};
use super::keys::{
    CustomerCreateModalKey, CustomerEditModalKey, CustomerSelectModalKey, CustomerSelectTableKey,
    CustomerTableKey,
};
use super::routes::{
    CustomerCreateGetRouteTag, CustomerCreatePostRouteTag, CustomerDefaultRouteTag,
    CustomerDeletePostRouteTag, CustomerDetailRouteTag, CustomerEditGetRouteTag,
    CustomerEditPostRouteTag, CustomerFkSelectRouteTag,
};

#[cfg(not(feature = "plugin-finance-customer"))]
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

#[cfg(not(feature = "plugin-finance-customer"))]
fn scaffold_pane(sidebar: Markup, crumbs: Markup, body: Markup) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

#[cfg(not(feature = "plugin-finance-customer"))]
fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

pub(crate) fn customers_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Customers",
        href: None,
    }])
}

pub(crate) fn customer_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = CustomerDefaultRouteTag.url();
    let detail_url = CustomerDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Customers",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Customers",
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

#[cfg(not(feature = "plugin-finance-customer"))]
fn customer_menu() -> Markup {
    let list_url = CustomerDefaultRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: "Customers",
        children: sidebar_menu_item_pane(SidebarMenuItem {
            title: "All Customers",
            url: &list_url,
            active: true,
            ..Default::default()
        }),
    })
}

#[cfg(not(feature = "plugin-finance-customer"))]
fn customer_detail_menu(id: i64, name: &str) -> Markup {
    let title = format!("Customer: {name}");
    let detail_url = CustomerDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Customer Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
        },
    })
}

crate::define_register_items! {
    plugin: CustomerTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        CustomerListIdx: CustomerListPageTag => CustomerListPage,
        CustomerDetailIdx: CustomerDetailPageTag => CustomerDetailPage,
        CustomerEditModalIdx: CustomerEditModalPageTag => CustomerEditModalPage,
        CustomerCreateModalIdx: CustomerCreateModalPageTag => CustomerCreateModalPage,
        CustomerSelectIdx: CustomerSelectPageTag => CustomerSelectPage,
    ]
}

crate::define_register_items! {
    plugin: CustomerTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn customer_filter_form(name: &str, email: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<CustomerTableKey, CustomerDefaultRouteTag>(
            CustomerDefaultRouteTag,
        ),
        inputs: CustomerFilterForm::render_inputs(
            &FormCtx::form::<CustomerFilterForm>()
                .value(CustomerFilterFormField::Name, name)
                .value(CustomerFilterFormField::Email, email),
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

fn customer_select_filter_form(name: &str, email: &str, target_input: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<CustomerSelectTableKey, CustomerFkSelectRouteTag>(
            CustomerFkSelectRouteTag,
        )
        .set("hx-push-url", "false"),
        inputs: html! {
            (CustomerFilterForm::render_inputs(
                &FormCtx::form::<CustomerFilterForm>()
                    .value(CustomerFilterFormField::Name, name)
                    .value(CustomerFilterFormField::Email, email),
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
pub struct CustomerRow {
    pub id: i64,
    pub customer_type: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub gstin: String,
}

#[derive(Generic)]
pub struct CustomerListPage {
    pub customers: ObjectList<CustomerRow>,
    pub filter_name: String,
    pub filter_email: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl CustomerListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
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
        ];
        let rows: Vec<TableRow> = self
            .customers
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_navigate_route(CustomerDetailRouteTag::new(c.id)),
                cells: vec![
                    field_text(FieldText { value: &c.name, classes: "" }),
                    field_text(FieldText { value: &c.customer_type, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: customer_filter_form(&self.filter_name, &self.filter_email),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_customer.CustomerCreateForm",
                    href: &CustomerCreateGetRouteTag.url(),
                    form_post_url: &CustomerCreateGetRouteTag.path(),
                    modal_uid: CustomerCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<CustomerTableKey>(
            &self.path_and_query,
            self.customers.number,
            self.customers.num_pages,
        );
        data_table_list_refresh::<CustomerTableKey>(
            "Customers",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

#[cfg(not(feature = "plugin-finance-customer"))]
impl RenderAppPane for CustomerListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(customer_menu(), customers_list_crumbs(), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(customers_list_crumbs(), self.render_table())
    }
}

#[cfg(not(feature = "plugin-finance-customer"))]
impl RenderTemplate for CustomerListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Customers — Lariv",
            chrome,
            customer_menu(),
            customers_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct CustomerDetailPage {
    pub id: i64,
    pub customer_type: String,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub gstin: String,
    pub pan: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub can_edit: bool,
}

impl CustomerDetailPage {
    pub(crate) fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (label_inline("Type", field_text(FieldText { value: &self.customer_type, classes: "" })))
                    (label_inline("Address line 1", field_text(FieldText { value: &self.address_line_1, classes: "" })))
                    (label_inline("Address line 2", field_text(FieldText { value: &self.address_line_2, classes: "" })))
                    (label_inline("City", field_text(FieldText { value: &self.city, classes: "" })))
                    (label_inline("Pincode", field_text(FieldText { value: &self.pincode, classes: "" })))
                    (label_inline("State", field_text(FieldText { value: &self.state, classes: "" })))
                    (label_inline("GSTIN", field_text(FieldText { value: &self.gstin, classes: "" })))
                    (label_inline("PAN", field_text(FieldText { value: &self.pan, classes: "" })))
                    (label_inline("Phone", field_text(FieldText { value: &self.phone, classes: "" })))
                    (label_inline("Email", field_text(FieldText { value: &self.email, classes: "" })))
                    (label_inline("Website", field_text(FieldText { value: &self.website, classes: "" })))
                    @if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_customer.CustomerEditForm",
                                href: &CustomerEditGetRouteTag::new(self.id).url(),
                                form_post_url: &CustomerEditPostRouteTag::new(self.id).path(),
                                modal_uid: CustomerEditModalKey::ID,
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

    #[cfg(not(feature = "plugin-finance-customer"))]
    fn menu(&self) -> Markup {
        customer_detail_menu(self.id, &self.name)
    }
}

#[cfg(not(feature = "plugin-finance-customer"))]
impl RenderAppPane for CustomerDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = customer_crumbs(self.id, &self.name, None);
        scaffold_pane(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(customer_crumbs(self.id, &self.name, None), self.body())
    }
}

#[cfg(not(feature = "plugin-finance-customer"))]
impl RenderTemplate for CustomerDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = customer_crumbs(self.id, &self.name, None);
        app_scaffold("Customer — Lariv", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct CustomerEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub customer_type: String,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub gstin: String,
    pub pan: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub error: String,
}

impl RenderTemplate for CustomerEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let choices = CustomerForm::customer_type_choices();
        modal_keyed::<CustomerEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit customer" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<CustomerEditModalKey>(&modal_edit_post_url(
                        CustomerEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CustomerForm::render_inputs(
                        &FormCtx::form::<CustomerForm>()
                            .value(CustomerFormField::CustomerType, &self.customer_type)
                            .value(CustomerFormField::Name, &self.name)
                            .value(CustomerFormField::AddressLine1, &self.address_line_1)
                            .value(CustomerFormField::AddressLine2, &self.address_line_2)
                            .value(CustomerFormField::City, &self.city)
                            .value(CustomerFormField::Pincode, &self.pincode)
                            .value(CustomerFormField::State, &self.state)
                            .value(CustomerFormField::Gstin, &self.gstin)
                            .value(CustomerFormField::Pan, &self.pan)
                            .value(CustomerFormField::Phone, &self.phone)
                            .value(CustomerFormField::Email, &self.email)
                            .value(CustomerFormField::Website, &self.website)
                            .choices(
                                CustomerFormField::CustomerType,
                                &choices
                                    .iter()
                                    .map(|(k, v)| (k.to_string(), v.to_string()))
                                    .collect::<Vec<_>>(),
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_delete_post_route(
                            CustomerDeletePostRouteTag::new(self.id),
                            ButtonDeletePost {
                                label: "Delete",
                                confirm: "Permanently delete this customer?",
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
pub struct CustomerCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub customer_type: String,
    pub name: String,
    pub address_line_1: String,
    pub address_line_2: String,
    pub city: String,
    pub pincode: String,
    pub state: String,
    pub gstin: String,
    pub pan: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub error: String,
}

impl RenderTemplate for CustomerCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_customer.CustomerCreateForm"
        } else {
            self.form_name.as_str()
        };
        let choices = CustomerForm::customer_type_choices();
        modal_keyed::<CustomerCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Customer",
                subtitle: "Create a new customer",
                classes: "@container",
                attrs: form_hx_post_url::<CustomerCreateModalKey>(
                    &modal_create_post_url(
                        CustomerCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: CustomerForm::render_inputs(
                    &FormCtx::form::<CustomerForm>()
                        .value(CustomerFormField::CustomerType, &self.customer_type)
                        .value(CustomerFormField::Name, &self.name)
                        .value(CustomerFormField::AddressLine1, &self.address_line_1)
                        .value(CustomerFormField::AddressLine2, &self.address_line_2)
                        .value(CustomerFormField::City, &self.city)
                        .value(CustomerFormField::Pincode, &self.pincode)
                        .value(CustomerFormField::State, &self.state)
                        .value(CustomerFormField::Gstin, &self.gstin)
                        .value(CustomerFormField::Pan, &self.pan)
                        .value(CustomerFormField::Phone, &self.phone)
                        .value(CustomerFormField::Email, &self.email)
                        .value(CustomerFormField::Website, &self.website)
                        .choices(
                            CustomerFormField::CustomerType,
                            &choices
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect::<Vec<_>>(),
                        ),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Save Customer",
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
pub struct CustomerSelectPage {
    pub customers: ObjectList<CustomerRow>,
    pub filter_name: String,
    pub filter_email: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl RenderPickerSelect<CustomerSelectTableKey, CustomerSelectModalKey> for CustomerSelectPage {
    fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let email_sort = column_sort_url(&self.path_and_query, "Email", &self.sort);
        let phone_sort = column_sort_url(&self.path_and_query, "Phone", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let email_label = format!("Email{}", sort_indicator(&self.sort, "Email"));
        let phone_label = format!("Phone{}", sort_indicator(&self.sort, "Phone"));
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
            TableColumnHeader {
                key: "Phone",
                label: &phone_label,
                sort_url: Some(&phone_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .customers
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_select(&self.target_input, &c.id.to_string(), &c.name),
                cells: vec![
                    field_text(FieldText { value: &c.name, classes: "" }),
                    field_text(FieldText { value: &c.email, classes: "" }),
                    field_text(FieldText { value: &c.phone, classes: "" }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: customer_select_filter_form(
                    &self.filter_name,
                    &self.filter_email,
                    &self.target_input,
                ),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_customer.CustomerCreateForm",
                    href: &CustomerCreateGetRouteTag.url(),
                    form_post_url: &CustomerCreateGetRouteTag.path(),
                    modal_uid: CustomerCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<CustomerSelectTableKey>(
            &self.path_and_query,
            self.customers.number,
            self.customers.num_pages,
        );
        data_table_list_refresh::<CustomerSelectTableKey>(
            "Select Customer",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for CustomerSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
