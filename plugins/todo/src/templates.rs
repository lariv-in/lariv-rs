//! Maud page templates for todo CRUD views.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DataTable, DataTableDisplay, DeleteConfirmation,
    FieldCheckbox, FieldText, FieldTextarea, FieldTitle, FilterClauseView, FormOpts, LayoutMain,
    LayoutSidebar, ListFilterBuilder, ObjectList, PaginationPage, ShellChrome, ShellScaffold,
    SidebarMenu, SidebarMenuItem, SidebarNavLink, SwapKey, TableButtonFilter, TableColumnHeader,
    TableListContent, TablePagination, TableRow, breadcrumbs, button_modal_form, button_submit,
    column_sort_url, container_column, container_row, data_table, detail, field_checkbox,
    field_text, field_textarea, field_title, filter_error_banner, form, form_hx_get_route,
    form_hx_post_route, form_hx_post_url, label, layout_main, layout_sidebar, list_filter_builder,
    modal, modal_keyed, pagination_pages, row_attr_navigate_route, shell_scaffold, sidebar_menu,
    sidebar_menu_item_pane, sidebar_nav_items_pane, sort_indicator, table_button_columns,
    table_button_filter, table_create_button, table_list_content, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use super::forms::{TodoForm, TodoFormField};
use super::keys::{TodoCreateModalKey, TodoDeleteModalKey, TodoEditModalKey, TodoTableKey};
use super::routes::{
    TodoCreatePostRouteTag, TodoDeleteGetRouteTag, TodoDeletePostRouteTag, TodoDetailRouteTag,
    TodoEditPostRouteTag, TodoListRouteTag, TodoToggleRouteTag,
};

define_register_items! {
    plugin: TodoTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        TodoListIdx: TodoListPageTag => TodoListPage,
        TodoDetailIdx: TodoDetailPageTag => TodoDetailPage,
        TodoEditModalIdx: TodoEditModalPageTag => TodoEditModalPage,
        TodoCreateModalIdx: TodoCreateModalPageTag => TodoCreateModalPage,
        ConfirmDeleteIdx: TodoConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

fn app_scaffold(chrome: &ShellChrome, sidebar: Markup, crumbs: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
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
) -> lariv_core::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "ToDo",
        href: None,
    }])
}

fn detail_crumbs(title: &str) -> Markup {
    let list_url = TodoListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "ToDo",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

fn todo_menu(current_path: &str) -> Markup {
    let list_url = TodoListRouteTag.url();
    let links = [SidebarNavLink {
        key: "todos",
        title: "All items",
        url: &list_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "ToDo",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn detail_menu(id: i64, title: &str) -> Markup {
    let menu_title = format!("To-do: {title}");
    let detail_url = TodoDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
        },
    })
}

fn data_table_filtered<K: SwapKey>(
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
        .map(|h| h.key)
        .filter(|k| !k.is_empty())
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
        title: "",
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

fn checked_toggle(id: i64, checked: bool) -> Markup {
    let route = TodoToggleRouteTag::new(id);
    let attrs =
        form_hx_post_route::<TodoTableKey, TodoToggleRouteTag>(route).set("hx-swap", "outerHTML");
    html! {
        div onclick="event.stopPropagation()" {
            (PreEscaped(format!(r#"<form{}>"#, attrs.as_string())))
            input type="hidden" name=(lariv_core::html_form::CSRF_FIELD) value=(CsrfToken::current().as_str());
            input type="checkbox" class="checkbox checkbox-sm" checked[checked] onchange="this.form.requestSubmit()";
            (PreEscaped("</form>"))
        }
    }
}

/// Row payload for [`TodoListPage`].
#[derive(Clone)]
pub struct TodoRow {
    pub id: i64,
    pub title: String,
    pub checked: bool,
    pub updated_at: String,
}

#[derive(Generic)]
pub struct TodoListPage {
    pub todos: ObjectList<TodoRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl TodoListPage {
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let checked_sort = column_sort_url(&self.path_and_query, "Checked", &self.sort);
        let updated_sort = column_sort_url(&self.path_and_query, "UpdatedAt", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let checked_label = format!("Checked{}", sort_indicator(&self.sort, "Checked"));
        let updated_label = format!("Updated At{}", sort_indicator(&self.sort, "UpdatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Checked",
                label: &checked_label,
                sort_url: Some(&checked_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "UpdatedAt",
                label: &updated_label,
                sort_url: Some(&updated_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .todos
            .items
            .iter()
            .map(|item| TableRow {
                attrs: row_attr_navigate_route(TodoDetailRouteTag::new(item.id)),
                cells: vec![
                    checked_toggle(item.id, item.checked),
                    field_text(FieldText {
                        value: &item.title,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &item.updated_at,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route::<TodoTableKey, TodoListRouteTag>(TodoListRouteTag),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (table_create_button::<TodoTableKey, TodoCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination::<TodoTableKey>(
            &self.path_and_query,
            self.todos.number,
            self.todos.num_pages,
        );
        data_table_filtered::<TodoTableKey>(
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl lariv_core::template::RenderAppPane for TodoListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            todo_menu(&self.path_and_query),
            list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for TodoListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            todo_menu(&self.path_and_query),
            list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct TodoDetailPage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub checked: bool,
}

impl TodoDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = super::routes::TodoEditGetRouteTag::new(self.id).url();
        let edit_post = TodoEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.title,
                        classes: "",
                    }))
                    (label("Checked", field_checkbox(FieldCheckbox {
                        checked: self.checked,
                        classes: "",
                    })))
                    (label("Description", field_textarea(FieldTextarea {
                        value: &self.description,
                        classes: "",
                    })))
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_todo.TodoEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: TodoEditModalKey::ID,
                            label: "Edit",
                            classes: "btn-outline",
                            ..Default::default()
                        }))
                    }))
                },
            ))
        })
    }
}

impl lariv_core::template::RenderAppPane for TodoDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            detail_menu(self.id, &self.title),
            detail_crumbs(&self.title),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(detail_crumbs(&self.title), self.pane_body())
    }
}

impl RenderTemplate for TodoDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            detail_menu(self.id, &self.title),
            detail_crumbs(&self.title),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct TodoEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub description: String,
    pub checked: bool,
    pub error: String,
}

impl RenderTemplate for TodoEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TodoDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<TodoForm>(CsrfToken::current())
            .value(TodoFormField::Title, self.title.as_str())
            .value(TodoFormField::Description, self.description.as_str())
            .checked(TodoFormField::Checked, self.checked);
        modal_keyed::<TodoEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit to-do" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TodoEditModalKey>(&modal_edit_post_url(
                        TodoEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TodoForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_todo.TodoDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: TodoDeleteModalKey::ID,
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
pub struct TodoCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub description: String,
    pub checked: bool,
    pub error: String,
}

impl RenderTemplate for TodoCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_todo.TodoCreateForm"
        } else {
            self.form_name.as_str()
        };
        let ctx = FormCtx::form::<TodoForm>(CsrfToken::current())
            .value(TodoFormField::Title, self.title.as_str())
            .value(TodoFormField::Description, self.description.as_str())
            .checked(TodoFormField::Checked, self.checked);
        modal_keyed::<TodoCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "New to-do",
                    subtitle: "Add an item to your list",
                    attrs: form_hx_post_url::<TodoCreateModalKey>(&modal_create_post_url(
                        TodoCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TodoForm::render_inputs(&ctx),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url = TodoDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: TodoDeleteModalKey::ID,
            children: lariv_core::components::delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: "Are you sure you want to delete this to-do?",
                attrs: lariv_core::components::form_hx_post_selector(
                    &post_url,
                    &format!("#{}", TodoDeleteModalKey::ID),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
