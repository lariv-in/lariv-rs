//! Maud page templates for tasks, statuses, and logs.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldText, FormOpts,
        LayoutMain, LayoutSidebar, ObjectList, PaginationPage, ShellChrome, ShellScaffold,
        SidebarMenu, SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter,
        TableColumnHeader, TablePagination, TableRow, button_modal_form, button_submit,
        column_sort_url, container_column, container_row, data_table_list_refresh,
        delete_confirmation, detail, detail_header, field_text, form, form_hx_get_route,
        form_hx_post_route, form_hx_post_selector, form_hx_post_url, label, layout_main,
        layout_sidebar, modal, modal_keyed, pagination_pages, row_attr_navigate, shell_scaffold,
        sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_filter,
        table_create_button, table_pagination, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use super::color::u24_to_hex;
use super::crumbs::{
    status_crumbs, statuses_list_crumbs, task_crumbs, task_log_crumbs, tasks_list_crumbs,
};
use super::detail_menu::{status_detail_menu, task_detail_menu};
use super::forms::{
    TaskFilterForm, TaskFilterFormField, TaskForm, TaskFormField, TaskLogForm, TaskLogFormField,
    TaskLogQuickForm, TaskStatusFilterForm, TaskStatusFilterFormField, TaskStatusForm,
    TaskStatusFormField,
};
use super::keys::{
    TASK_LOG_SAVED_EVENT, TaskCreateModalKey, TaskDeleteModalKey, TaskEditModalKey,
    TaskLogDeleteModalKey, TaskLogEditModalKey, TaskLogsKey, TaskStatusCreateModalKey,
    TaskStatusDeleteModalKey, TaskStatusEditModalKey, TaskStatusTableKey, TaskStatusTasksTableKey,
    TaskTableKey,
};
use super::routes::{
    TaskCreatePostRouteTag, TaskDefaultRouteTag, TaskDeleteGetRouteTag, TaskDeletePostRouteTag,
    TaskDetailRouteTag, TaskEditGetRouteTag, TaskEditPostRouteTag, TaskLogAddPostRouteTag,
    TaskLogDeleteGetRouteTag, TaskLogDeletePostRouteTag, TaskLogEditGetRouteTag,
    TaskLogEditPostRouteTag, TaskStatusCreatePostRouteTag, TaskStatusDefaultRouteTag,
    TaskStatusDeleteGetRouteTag, TaskStatusDeletePostRouteTag, TaskStatusDetailRouteTag,
    TaskStatusEditGetRouteTag, TaskStatusEditPostRouteTag,
};

crate::define_register_items! {
    plugin: TasksTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        TaskListIdx: TaskListPageTag => TaskListPage,
        TaskDetailIdx: TaskDetailPageTag => TaskDetailPage,
        TaskLogsIdx: TaskLogsPageTag => TaskLogsPage,
        TaskEditModalIdx: TaskEditModalPageTag => TaskEditModalPage,
        TaskCreateModalIdx: TaskCreateModalPageTag => TaskCreateModalPage,
        TaskStatusListIdx: TaskStatusListPageTag => TaskStatusListPage,
        TaskStatusDetailIdx: TaskStatusDetailPageTag => TaskStatusDetailPage,
        TaskStatusEditModalIdx: TaskStatusEditModalPageTag => TaskStatusEditModalPage,
        TaskStatusCreateModalIdx: TaskStatusCreateModalPageTag => TaskStatusCreateModalPage,
        TaskLogDetailIdx: TaskLogDetailPageTag => TaskLogDetailPage,
        TaskLogEditModalIdx: TaskLogEditModalPageTag => TaskLogEditModalPage,
        ConfirmDeleteIdx: TasksConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

crate::define_register_items! {
    plugin: TasksTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

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

fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
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

fn color_swatch(color: u32, name: &str) -> Markup {
    let hex = u24_to_hex(color);
    html! {
        span class="inline-flex items-center gap-2" {
            span class="w-3 h-3 rounded-full shrink-0 border border-base-300" style=(format!("background-color: {hex}")) {}
            (name)
        }
    }
}

pub fn tasks_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Tasks",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Tasks",
                url: &TaskDefaultRouteTag.url(),
                active: active == "tasks",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Statuses",
                url: &TaskStatusDefaultRouteTag.url(),
                active: active == "statuses",
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

fn task_filter_clear_button(assigned_to_id: &str, assigned_to_display: &str) -> Markup {
    use crate::components::attrs::escape_attr;
    use maud::PreEscaped;
    let onclick = "const f=this.closest('form');f.querySelectorAll('input[name=Title],select[name=StatusId]').forEach(el=>el.value='');window.dispatchEvent(new CustomEvent('fk-select',{detail:{name:'AssignedToId',value:this.dataset.defaultAssignedToId,display:this.dataset.defaultAssignedToDisplay}}));";
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
    pub status: String,
    pub status_color: u32,
    pub priority: i32,
    pub due_datetime: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct TaskListPage {
    pub tasks: ObjectList<TaskRow>,
    pub filter_title: String,
    pub filter_assigned_to_id: String,
    pub filter_assigned_to_display: String,
    pub filter_status_id: String,
    pub status_choices: Vec<(String, String)>,
    pub default_assigned_to_id: String,
    pub default_assigned_to_display: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl TaskListPage {
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let assigned_sort = column_sort_url(&self.path_and_query, "AssignedTo", &self.sort);
        let status_sort = column_sort_url(&self.path_and_query, "Status", &self.sort);
        let priority_sort = column_sort_url(&self.path_and_query, "Priority", &self.sort);
        let due_sort = column_sort_url(&self.path_and_query, "DueDatetime", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let assigned_label = format!("Assigned To{}", sort_indicator(&self.sort, "AssignedTo"));
        let status_label = format!("Status{}", sort_indicator(&self.sort, "Status"));
        let priority_label = format!("Priority{}", sort_indicator(&self.sort, "Priority"));
        let due_label = format!("Due{}", sort_indicator(&self.sort, "DueDatetime"));
        let headers = [
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
                key: "Status",
                label: &status_label,
                sort_url: Some(&status_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Priority",
                label: &priority_label,
                sort_url: Some(&priority_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "DueDatetime",
                label: &due_label,
                sort_url: Some(&due_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .tasks
            .items
            .iter()
            .map(|t| {
                let priority = t.priority.to_string();
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
                        color_swatch(t.status_color, &t.status),
                        field_text(FieldText {
                            value: &priority,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: &t.due_datetime,
                            classes: "",
                        }),
                    ],
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<TaskTableKey, TaskDefaultRouteTag>(
                        TaskDefaultRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        TaskFilterForm::render_inputs(
                            &FormCtx::form::<TaskFilterForm>(CsrfToken::current())
                                .value(TaskFilterFormField::Title, &self.filter_title)
                                .value(
                                    TaskFilterFormField::AssignedToId,
                                    &self.filter_assigned_to_id,
                                )
                                .display(
                                    TaskFilterFormField::AssignedToId,
                                    &self.filter_assigned_to_display,
                                )
                                .value(TaskFilterFormField::StatusId, &self.filter_status_id)
                                .choices(TaskFilterFormField::StatusId, &self.status_choices),
                        ),
                        self.page_size,
                    ),
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
        if self.can_edit {
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
}

impl RenderAppPane for TaskListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            tasks_menu("tasks"),
            tasks_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(tasks_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for TaskListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Tasks — Lariv",
            chrome,
            tasks_menu("tasks"),
            tasks_list_crumbs(),
            self.render_table(),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn task_form_inputs(
    title: &str,
    description: &str,
    assigned_to_id: i64,
    assigned_to_display: &str,
    status_id: &str,
    status_choices: &[(String, String)],
    priority: &str,
    due_datetime: &str,
) -> Markup {
    let assigned_to_id_s = fk_value(assigned_to_id);
    TaskForm::render_inputs(
        &FormCtx::form::<TaskForm>(CsrfToken::current())
            .value(TaskFormField::Title, title)
            .value(TaskFormField::Description, description)
            .value(TaskFormField::AssignedToId, assigned_to_id_s.as_str())
            .display(TaskFormField::AssignedToId, assigned_to_display)
            .value(TaskFormField::StatusId, status_id)
            .choices(TaskFormField::StatusId, status_choices)
            .value(TaskFormField::Priority, priority)
            .value(TaskFormField::DueDatetime, due_datetime),
    )
}

#[derive(Generic)]
pub struct TaskDetailPage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub assigned_to: String,
    pub status: String,
    pub status_color: u32,
    pub priority: i32,
    pub due_datetime: String,
    pub can_edit: bool,
}

impl TaskDetailPage {
    fn actions(&self) -> Markup {
        if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_tasks.TaskEditForm",
                    href: &TaskEditGetRouteTag::new(self.id).url(),
                    form_post_url: &TaskEditPostRouteTag::new(self.id).path(),
                    modal_uid: TaskEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        }
    }

    fn body(&self) -> Markup {
        let priority = self.priority.to_string();
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.title,
                        actions: self.actions(),
                    }))
                    (label("Assigned To", field_text(FieldText { value: &self.assigned_to, classes: "" })))
                    (label("Status", color_swatch(self.status_color, &self.status)))
                    (label("Priority", field_text(FieldText { value: &priority, classes: "" })))
                    (label("Due", field_text(FieldText { value: &self.due_datetime, classes: "" })))
                    (label("Description", field_text(FieldText { value: &self.description, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for TaskDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            task_detail_menu(&self.title, self.id, "detail"),
            task_crumbs(&self.title, self.id, None),
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
pub struct TaskEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub description: String,
    pub assigned_to_id: i64,
    pub assigned_to_display: String,
    pub status_id: String,
    pub status_choices: Vec<(String, String)>,
    pub priority: String,
    pub due_datetime: String,
    pub error: String,
}

impl RenderTemplate for TaskEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TaskDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<TaskEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit task" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TaskEditModalKey>(&modal_edit_post_url(
                        TaskEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: task_form_inputs(
                        &self.title,
                        &self.description,
                        self.assigned_to_id,
                        &self.assigned_to_display,
                        &self.status_id,
                        &self.status_choices,
                        &self.priority,
                        &self.due_datetime,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_tasks.TaskDeleteForm",
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
    pub status_id: String,
    pub status_choices: Vec<(String, String)>,
    pub priority: String,
    pub due_datetime: String,
    pub error: String,
}

impl RenderTemplate for TaskCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<TaskCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New task" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TaskCreateModalKey>(&modal_create_post_url(
                        TaskCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: task_form_inputs(
                        &self.title,
                        &self.description,
                        self.assigned_to_id,
                        &self.assigned_to_display,
                        &self.status_id,
                        &self.status_choices,
                        &self.priority,
                        &self.due_datetime,
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

#[derive(Clone)]
pub struct TaskStatusRow {
    pub id: i64,
    pub name: String,
    pub color: u32,
}

#[derive(Generic)]
pub struct TaskStatusListPage {
    pub statuses: ObjectList<TaskStatusRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl TaskStatusListPage {
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
            .statuses
            .items
            .iter()
            .map(|s| TableRow {
                attrs: row_attr_navigate(&TaskStatusDetailRouteTag::new(s.id).url()),
                cells: vec![color_swatch(s.color, &s.name)],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<TaskStatusTableKey, TaskStatusDefaultRouteTag>(
                        TaskStatusDefaultRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        TaskStatusFilterForm::render_inputs(
                            &FormCtx::form::<TaskStatusFilterForm>(CsrfToken::current())
                                .value(TaskStatusFilterFormField::Name, &self.filter_name),
                        ),
                        self.page_size,
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
                (table_create_button::<TaskStatusTableKey, TaskStatusCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        data_table_list_refresh::<TaskStatusTableKey>(
            "Statuses",
            actions,
            &headers,
            &rows,
            render_pagination::<TaskStatusTableKey>(
                &self.path_and_query,
                self.statuses.number,
                self.statuses.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for TaskStatusListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            tasks_menu("statuses"),
            statuses_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(statuses_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for TaskStatusListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Task statuses — Lariv",
            chrome,
            tasks_menu("statuses"),
            statuses_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct StatusTaskRow {
    pub id: i64,
    pub title: String,
    pub due_datetime: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct TaskStatusDetailPage {
    pub id: i64,
    pub name: String,
    pub color: u32,
    pub can_edit: bool,
    pub tasks: ObjectList<StatusTaskRow>,
    pub sort: String,
    pub path_and_query: String,
}

impl TaskStatusDetailPage {
    pub fn render_tasks_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let due_sort = column_sort_url(&self.path_and_query, "DueDatetime", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let due_label = format!("Due{}", sort_indicator(&self.sort, "DueDatetime"));
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "DueDatetime",
                label: &due_label,
                sort_url: Some(&due_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .tasks
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_navigate(&t.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &t.title,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.due_datetime,
                        classes: "",
                    }),
                ],
            })
            .collect();
        data_table_list_refresh::<TaskStatusTasksTableKey>(
            "Tasks",
            html! {},
            &headers,
            &rows,
            render_pagination::<TaskStatusTasksTableKey>(
                &self.path_and_query,
                self.tasks.number,
                self.tasks.num_pages,
            ),
            &self.path_and_query,
        )
    }

    fn actions(&self) -> Markup {
        if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_tasks.TaskStatusEditForm",
                    href: &TaskStatusEditGetRouteTag::new(self.id).url(),
                    form_post_url: &TaskStatusEditPostRouteTag::new(self.id).path(),
                    modal_uid: TaskStatusEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        }
    }

    fn body(&self) -> Markup {
        let hex = u24_to_hex(self.color);
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions: self.actions(),
                    }))
                    (label("Color", html! {
                        span class="inline-flex items-center gap-2" {
                            span class="w-4 h-4 rounded-full shrink-0 border border-base-300" style=(format!("background-color: {hex}")) {}
                            span class="font-mono text-sm" { (hex) }
                        }
                    }))
                }))
            }))
            div class="mt-6" {
                (self.render_tasks_table())
            }
        }
    }
}

impl RenderAppPane for TaskStatusDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            status_detail_menu(&self.name, self.id, "detail"),
            status_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(status_crumbs(&self.name, self.id, None), self.body())
    }
}

impl RenderTemplate for TaskStatusDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Status — Lariv",
            chrome,
            status_detail_menu(&self.name, self.id, "detail"),
            status_crumbs(&self.name, self.id, None),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct TaskStatusEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub color: String,
    pub error: String,
}

impl RenderTemplate for TaskStatusEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TaskStatusDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<TaskStatusEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit status" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TaskStatusEditModalKey>(&modal_edit_post_url(
                        TaskStatusEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TaskStatusForm::render_inputs(
                        &FormCtx::form::<TaskStatusForm>(CsrfToken::current())
                            .value(TaskStatusFormField::Name, &self.name)
                            .value(TaskStatusFormField::Color, &self.color),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_tasks.TaskStatusDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: TaskStatusDeleteModalKey::ID,
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
pub struct TaskStatusCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub color: String,
    pub error: String,
}

impl RenderTemplate for TaskStatusCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<TaskStatusCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New status" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TaskStatusCreateModalKey>(&modal_create_post_url(
                        TaskStatusCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TaskStatusForm::render_inputs(
                        &FormCtx::form::<TaskStatusForm>(CsrfToken::current())
                            .value(TaskStatusFormField::Name, &self.name)
                            .value(TaskStatusFormField::Color, &self.color),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create status", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Clone)]
pub struct TaskLogItem {
    pub id: i64,
    pub datetime: String,
    pub description: String,
}

#[derive(Clone)]
pub struct TaskLogsPanel {
    pub task_id: i64,
    pub items: Vec<TaskLogItem>,
    pub can_edit: bool,
    pub default_datetime: String,
}

impl TaskLogsPanel {
    fn escape_js_str(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\'', "\\'")
    }

    fn render_add_form(&self) -> Markup {
        let default_dt = Self::escape_js_str(&self.default_datetime);
        let x_data = format!(
            "{{ datetime: $persist('{default_dt}').as('tasks-log-draft-datetime-{id}'), description: $persist('').as('tasks-log-draft-description-{id}'), clearDraft() {{ this.description = ''; this.datetime = '{default_dt}'; }} }}",
            id = self.task_id,
            default_dt = default_dt,
        );
        let attrs = form_hx_post_route::<TaskLogsKey, _>(TaskLogAddPostRouteTag::new(self.task_id))
            .set("x-data", x_data)
            .set(format!("@{TASK_LOG_SAVED_EVENT}"), "clearDraft()");
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs,
                inputs: TaskLogQuickForm::render_inputs(&FormCtx::form::<TaskLogQuickForm>(
                    CsrfToken::current(),
                )),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Add log",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }

    pub fn render_list(&self) -> Markup {
        html! {
            div id=(TaskLogsKey::ID) class="max-h-96 overflow-y-auto flex flex-col divide-y divide-base-300 border border-base-300 rounded-box" {
                @if self.items.is_empty() {
                    div class="text-sm opacity-60 px-3 py-4 text-center" { "No logs" }
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
                                        name: "p_tasks.TaskLogEditForm",
                                        href: &TaskLogEditGetRouteTag::new(item.id).url(),
                                        form_post_url: &TaskLogEditPostRouteTag::new(item.id).path(),
                                        modal_uid: TaskLogEditModalKey::ID,
                                        label: "Edit",
                                        classes: "btn-ghost btn-xs",
                                        ..Default::default()
                                    }))
                                    (button_modal_form(ButtonModalForm {
                                        name: "p_tasks.TaskLogDeleteForm",
                                        href: &TaskLogDeleteGetRouteTag::new(item.id).url(),
                                        form_post_url: &TaskLogDeleteGetRouteTag::new(item.id).url(),
                                        modal_uid: TaskLogDeleteModalKey::ID,
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
                div class="text-lg font-semibold" { "Logs" }
                @if self.can_edit {
                    (self.render_add_form())
                }
                (self.render_list())
            }
        }
    }
}

#[derive(Generic)]
pub struct TaskLogsPage {
    pub task_id: i64,
    pub task_title: String,
    pub logs: TaskLogsPanel,
}

impl TaskLogsPage {
    fn body(&self) -> Markup {
        self.logs.render()
    }
}

impl RenderAppPane for TaskLogsPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            task_detail_menu(&self.task_title, self.task_id, "logs"),
            task_crumbs(&self.task_title, self.task_id, Some("Logs")),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            task_crumbs(&self.task_title, self.task_id, Some("Logs")),
            self.body(),
        )
    }
}

impl RenderTemplate for TaskLogsPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Task logs — Lariv",
            chrome,
            task_detail_menu(&self.task_title, self.task_id, "logs"),
            task_crumbs(&self.task_title, self.task_id, Some("Logs")),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct TaskLogDetailPage {
    pub id: i64,
    pub task_id: i64,
    pub task_title: String,
    pub datetime: String,
    pub description: String,
    pub can_edit: bool,
}

impl TaskLogDetailPage {
    fn actions(&self) -> Markup {
        if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_tasks.TaskLogEditForm",
                    href: &TaskLogEditGetRouteTag::new(self.id).url(),
                    form_post_url: &TaskLogEditPostRouteTag::new(self.id).path(),
                    modal_uid: TaskLogEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        }
    }

    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.datetime,
                        actions: self.actions(),
                    }))
                    (label("Task", html! {
                        a class="link" href=(TaskDetailRouteTag::new(self.task_id).url()) {
                            (self.task_title)
                        }
                    }))
                    (label("Description", field_text(FieldText { value: &self.description, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderAppPane for TaskLogDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            task_detail_menu(&self.task_title, self.task_id, "logs"),
            task_log_crumbs(&self.task_title, self.task_id, &self.datetime),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            task_log_crumbs(&self.task_title, self.task_id, &self.datetime),
            self.body(),
        )
    }
}

impl RenderTemplate for TaskLogDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Task log — Lariv",
            chrome,
            task_detail_menu(&self.task_title, self.task_id, "logs"),
            task_log_crumbs(&self.task_title, self.task_id, &self.datetime),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct TaskLogEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub datetime: String,
    pub description: String,
    pub error: String,
}

impl RenderTemplate for TaskLogEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TaskLogDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<TaskLogEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit log" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TaskLogEditModalKey>(&modal_edit_post_url(
                        TaskLogEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TaskLogForm::render_inputs(
                        &FormCtx::form::<TaskLogForm>(CsrfToken::current())
                            .value(TaskLogFormField::Datetime, &self.datetime)
                            .value(TaskLogFormField::Description, &self.description),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_tasks.TaskLogDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: TaskLogDeleteModalKey::ID,
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
        let target = format!("#{}", self.modal_uid);
        let post_url = if self.modal_uid == TaskStatusDeleteModalKey::ID {
            TaskStatusDeletePostRouteTag::new(self.id).url()
        } else if self.modal_uid == TaskLogDeleteModalKey::ID {
            TaskLogDeletePostRouteTag::new(self.id).url()
        } else {
            TaskDeletePostRouteTag::new(self.id).url()
        };
        modal(crate::components::Modal {
            uid: self.modal_uid.as_str(),
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
