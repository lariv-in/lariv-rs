//! Maud pages for the to-do page list and one page's checklist.

use maud::{Markup, PreEscaped, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, FieldText, FormOpts, LayoutMain,
    LayoutSidebar, ObjectList, PaginationPage, ShellChrome, ShellScaffold, SidebarMenu,
    SidebarNavLink, SwapKey, TableColumnHeader, TablePagination, TableRow, breadcrumbs,
    button_modal_form, button_submit, column_sort_url, data_table_list, delete_confirmation,
    field_text, form, form_hx_post_selector, form_hx_post_url, icon, layout_main, layout_sidebar,
    modal, modal_keyed, pagination_pages, row_attr_navigate_route, shell_scaffold, sidebar_menu,
    sidebar_nav_items_pane, sort_indicator, table_create_button, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_core::web::{modal_create_post_url, modal_edit_post_url};

use super::forms::{TodoPageForm, TodoPageFormField};
use super::keys::{
    TodoPageCreateModalKey, TodoPageDeleteModalKey, TodoPageEditModalKey, TodoPageTableKey,
};
use super::routes::{
    TodoListRouteTag, TodoPageCreatePostRouteTag, TodoPageDeleteGetRouteTag,
    TodoPageDeletePostRouteTag, TodoPageDetailRouteTag, TodoPageEditGetRouteTag,
    TodoPageEditPostRouteTag, TodoWsRouteTag,
};

const TODO_SCRIPT: &str = include_str!("todo_client.js");

define_register_items! {
    plugin: TodoTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        TodoPageListIdx: TodoPageListPageTag => TodoPageListPage,
        TodoListIdx: TodoListPageTag => TodoListPage,
        TodoPageCreateModalIdx: TodoPageCreateModalPageTag => TodoPageCreateModalPage,
        TodoPageEditModalIdx: TodoPageEditModalPageTag => TodoPageEditModalPage,
        TodoPageDeleteModalIdx: TodoPageDeleteModalPageTag => TodoPageDeleteModalPage,
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
        key: "pages",
        title: "Pages",
        url: &list_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "ToDo",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn render_pagination(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
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
        hx_target: TodoPageTableKey::SELECTOR,
    })
}

pub struct TodoPageRow {
    pub id: i64,
    pub title: String,
}

pub struct TodoPageListPage {
    pub pages: ObjectList<TodoPageRow>,
    pub sort: String,
    pub path_and_query: String,
}

impl TodoPageListPage {
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let headers = [TableColumnHeader {
            key: "Title",
            label: &title_label,
            sort_url: Some(&title_sort),
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .pages
            .items
            .iter()
            .map(|page| TableRow {
                attrs: row_attr_navigate_route(TodoPageDetailRouteTag::new(page.id)),
                cells: vec![field_text(FieldText {
                    value: &page.title,
                    classes: "",
                })],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = table_create_button::<TodoPageTableKey, TodoPageCreateModalKey>(
            Some("plus"),
            "btn-primary btn-sm btn-square",
        );
        let pagination = render_pagination(
            &self.path_and_query,
            self.pages.number,
            self.pages.num_pages,
        );
        data_table_list::<TodoPageTableKey>("Pages", actions, &headers, &rows, pagination)
    }
}

impl lariv_core::template::RenderAppPane for TodoPageListPage {
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

impl RenderTemplate for TodoPageListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            todo_menu(&self.path_and_query),
            list_crumbs(),
            self.render_table(),
        )
    }
}

pub struct TodoListPage {
    pub user_id: i64,
    pub page_id: i64,
    pub title: String,
    pub path_and_query: String,
}

impl TodoListPage {
    fn render_body(&self) -> Markup {
        let ws_url = TodoWsRouteTag::new(self.page_id).path();
        let config = serde_json::json!({
            "userId": self.user_id,
            "pageId": self.page_id,
            "wsUrl": ws_url,
        })
        .to_string();
        let edit_url = TodoPageEditGetRouteTag::new(self.page_id).url();
        html! {
            div class="container mx-auto flex w-full max-w-2xl flex-col gap-4" {
                div class="flex items-center justify-between gap-2" {
                    h2 class="text-lg font-semibold" { (self.title) }
                    (button_modal_form(ButtonModalForm {
                        label: "Edit",
                        icon_name: Some("pencil"),
                        name: "p_todo.TodoPageEditForm",
                        href: &edit_url,
                        form_post_url: &edit_url,
                        modal_uid: TodoPageEditModalKey::ID,
                        classes: "btn-sm",
                        ..Default::default()
                    }))
                }
                ul id="todo-list" class="flex flex-col gap-2" {}
                button id="todo-add" type="button" class="btn btn-primary w-full" aria-label="Add" {
                    (icon("plus", "heroicon-sm"))
                    "Add"
                }
                p id="todo-status" class="text-sm opacity-60" {}
                script type="application/json" id="todo-config" { (PreEscaped(config)) }
                script { (PreEscaped(TODO_SCRIPT)) }
            }
        }
    }
}

impl lariv_core::template::RenderAppPane for TodoListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            todo_menu(&self.path_and_query),
            detail_crumbs(&self.title),
            self.render_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(detail_crumbs(&self.title), self.render_body())
    }
}

impl RenderTemplate for TodoListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            todo_menu(&self.path_and_query),
            detail_crumbs(&self.title),
            self.render_body(),
        )
    }
}

pub struct TodoPageCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub error: String,
}

impl RenderTemplate for TodoPageCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<TodoPageForm>(CsrfToken::current())
            .value(TodoPageFormField::Title, self.title.as_str());
        modal_keyed::<TodoPageCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New page" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TodoPageCreateModalKey>(&modal_create_post_url(
                        TodoPageCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TodoPageForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct TodoPageEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub error: String,
}

impl RenderTemplate for TodoPageEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TodoPageDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<TodoPageForm>(CsrfToken::current())
            .value(TodoPageFormField::Title, self.title.as_str());
        modal_keyed::<TodoPageEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit page" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TodoPageEditModalKey>(&modal_edit_post_url(
                        TodoPageEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TodoPageForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_todo.TodoPageDeleteForm",
                            href: &delete_url,
                            form_post_url: &TodoPageDeletePostRouteTag::new(self.id).url(),
                            modal_uid: TodoPageDeleteModalKey::ID,
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

pub struct TodoPageDeleteModalPage {
    pub id: i64,
}

impl RenderTemplate for TodoPageDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url = TodoPageDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: TodoPageDeleteModalKey::ID,
            children: delete_confirmation(DeleteConfirmation {
                title: "Delete page",
                message: "This deletes the page and every to-do on it.",
                attrs: form_hx_post_selector(&post_url, TodoPageDeleteModalKey::SELECTOR),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
