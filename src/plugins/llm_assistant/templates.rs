use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    components::{
        ButtonClear, ButtonLink, ButtonModal, ButtonModalForm, ButtonSubmit, DeleteConfirmation, FieldManyToMany,
        FieldMarkdown, FieldText, FieldTitle, FormOpts, InputFile, LayoutSidebar, ManyToManyItem,
        ObjectList, PaginationPage, RenderSlot, RightSidebarSlotTag, ShellChrome,
        ShellScaffold, SidebarMenu, SidebarMenuBack, SidebarMenuItem, SlotCapability, SlotRegistrar, SlotCtx, SlotOf,
        SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear,
        button_link, button_modal, button_modal_form, button_submit, column_sort_url, container_column, container_row,
        data_table_list, detail, field_many_to_many, field_markdown, field_text, field_title, form,
        form_hx_get, form_hx_post_main, icon, input_file, label_inline, layout_sidebar, modal,
        pagination_pages, row_attr_navigate, shell_scaffold, sidebar_menu, sidebar_menu_item, sort_indicator,
        table_button_filter, table_pagination,
    },
    capability::define_register_items,
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::LlmAssistantTag;
use super::forms::{SkillForm, SkillNameFilterForm};
use super::keys::{HistoryTableKey, SkillDeleteModalKey, SkillImportModalKey, SkillsTableKey};

define_register_items! {
    plugin: LlmAssistantTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ChatIdx: ChatPageTag => ChatPage,
        ChatSessionIdx: ChatSessionPageTag => ChatSessionPage,
        HistoryListIdx: HistoryListPageTag => HistoryListPage,
        SkillListIdx: SkillListPageTag => SkillListPage,
        SkillDetailIdx: SkillDetailPageTag => SkillDetailPage,
        SkillFormIdx: SkillFormPageTag => SkillFormPage,
        ConfirmDeleteIdx: SkillConfirmDeletePageTag => ConfirmDeletePage,
        SkillImportIdx: SkillImportPageTag => SkillImportPage,
    ]
}

fn app_scaffold(_title: &str, chrome: &ShellChrome, sidebar: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(sidebar: Markup, body: Markup) -> Markup {
    layout_sidebar(LayoutSidebar {
        sidebar,
        content: body,
    })
}

fn scaffold_main(body: Markup) -> Markup {
    use crate::components::layout::layout_main;
    layout_main(body)
}

fn assistant_menu() -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Assistant",
        back: Some(SidebarMenuBack {
            title: "Back to All Apps",
            url: "/dashboard/",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Chat",
                url: "/llm-assistant/",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "History",
                url: "/llm-assistant/history/",
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Skills",
                url: "/llm-assistant/skills/",
                ..Default::default()
            }))
        },
    })
}

fn skill_detail_menu(skill_id: i64, name: &str) -> Markup {
    let menu_title = format!("Skill: {name}");
    let detail_url = format!("/llm-assistant/skills/{skill_id}/");
    let edit_url = format!("/llm-assistant/skills/{skill_id}/update/");
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        back: Some(SidebarMenuBack {
            title: "Back to All Skills",
            url: "/llm-assistant/skills/",
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Skill Details",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit Skill",
                url: &edit_url,
                ..Default::default()
            }))
        },
    })
}

fn skill_filter_form<K: SwapKey>(name: &str, action: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get::<K>(action),
        inputs: SkillNameFilterForm::render_inputs(&FormCtx::new().value("Name", name)),
        actions: html! {
            (container_row(
                "flex gap-2",
                html! {
                    (button_submit(ButtonSubmit {
                        label: "Apply Filters",
                        ..Default::default()
                    }))
                    (button_clear(ButtonClear {
                        label: "Clear",
                        ..Default::default()
                    }))
                },
            ))
        },
        ..Default::default()
    })
}

fn render_pagination<K: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
    push_url: bool,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, push_url);
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

const ASSISTANT_CHAT_SCRIPT: &str = r#"
document.body.addEventListener("htmx:before:ws:request", function(event) {
  if (!event || !event.detail || !event.detail.body) return;
  if (!event.target || event.target.id !== "llm_assistant_chat_form") return;
  var raw = event.detail.body.session_id;
  if (raw === undefined || raw === null || raw === "") {
    event.detail.body.session_id = 0;
    return;
  }
  var parsed = Number(raw);
  if (!Number.isNaN(parsed)) {
    event.detail.body.session_id = parsed;
  }
});
document.body.addEventListener("keydown", function(event) {
  if (!event.target || event.target.id !== "llm_assistant_chat_message") return;
  if (event.key !== "Enter" || event.shiftKey) return;
  event.preventDefault();
  var form = event.target.form;
  if (form) form.requestSubmit();
});
document.body.addEventListener("htmx:after:ws:request", function(event) {
  if (!event.target || event.target.id !== "llm_assistant_chat_form") return;
  var ta = document.getElementById("llm_assistant_chat_message");
  var btn = document.getElementById("llm_assistant_chat_send");
  if (ta) ta.value = "";
  if (btn) btn.disabled = true;
  var formEl = document.getElementById("llm_assistant_chat_form");
  if (formEl && window.Alpine) {
    var data = window.Alpine.$data(formEl);
    if (data) data.items = [];
  }
});
function llmAssistantScrollToBottom() {
  var transcript = document.getElementById("llm_assistant_transcript");
  if (transcript) transcript.scrollTop = transcript.scrollHeight;
}
document.body.addEventListener("htmx:after:ws:message", function() {
  llmAssistantScrollToBottom();
});
"#;

fn chat_form_html(hidden_val: &str, x_data: &str, file_select_url: &str) -> String {
    let icon_x = icon("x-mark", "heroicon-sm").into_string();
    let icon_upload = icon("arrow-up-tray", "heroicon-sm").into_string();
    let icon_clip = icon("paper-clip", "heroicon-sm").into_string();
    format!(
        r#"<form id="llm_assistant_chat_form" class="flex flex-col gap-2 max-w-2xl mx-auto w-full" hx-ws:send x-data="{x_data}" x-init="syncStore()" @fk-multi-select.window="eventHandler($event)">
<input id="llm_assistant_session_id" type="hidden" name="session_id" value="{hidden_val}">
<template x-for="item in items" :key="item.Key">
<input type="hidden" name="Files" :value="item.Key">
</template>
<div class="flex flex-wrap gap-2" x-show="items.length > 0">
<template x-for="item in items" :key="item.Key">
<div class="flex items-center gap-1 rounded-lg bg-base-200 pl-2 pr-1 py-1 text-xs">
<span class="truncate max-w-[150px]" x-text="item.Value"></span>
<button type="button" class="btn btn-ghost btn-square btn-xs shrink-0" @click.stop="removeItem(item.Key)">{icon_x}</button>
</div>
</template>
</div>
<textarea id="llm_assistant_chat_message" name="message" class="textarea textarea-bordered w-full" rows="3" placeholder="Message…" required></textarea>
<div class="flex justify-end items-center gap-2">
<label class="btn btn-outline btn-square" :class="uploading ? 'loading loading-spinner' : ''" title="Upload files from device">
<input type="file" class="hidden" multiple @change="uploadFiles($event.target)">
{icon_upload}
</label>
<button type="button" class="btn btn-outline btn-square" hx-get="{file_select_url}" hx-target="body" hx-swap="beforeend" hx-push-url="false">{icon_clip}</button>
<button id="llm_assistant_chat_send" type="submit" class="btn btn-primary">Send</button>
</div>
</form>"#,
        x_data = x_data.replace('"', "&quot;"),
        hidden_val = html_escape_attr(hidden_val),
        file_select_url = html_escape_attr(file_select_url),
        icon_x = icon_x,
        icon_upload = icon_upload,
        icon_clip = icon_clip,
    )
}

fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

fn chat_form_x_data() -> String {
    r#"{
        items: [],
        uploading: false,
        syncStore() {
            if (typeof Alpine !== 'undefined') {
                if (!Alpine.store('m2mSelections')) {
                    Alpine.store('m2mSelections', {});
                }
                Alpine.store('m2mSelections')['Files'] = this.items;
            }
        },
        hasItem(value) {
            value = String(value);
            return this.items.some(item => item.Key === value);
        },
        addItem(detail) {
            const value = String(detail.value);
            if (this.hasItem(value)) return;
            const display = detail.display ? String(detail.display) : value;
            this.items = [...this.items, { Key: value, Value: display }];
            this.syncStore();
        },
        removeItem(value) {
            this.items = this.items.filter(item => item.Key !== String(value));
            this.syncStore();
        },
        eventHandler(ev) {
            if (ev.detail.name === 'Files') {
                if (!this.hasItem(ev.detail.value)) {
                    this.addItem(ev.detail);
                } else {
                    this.removeItem(ev.detail.value);
                }
            }
        },
        async uploadFiles(fileInput) {
            if (!fileInput.files || fileInput.files.length === 0) return;
            this.uploading = true;
            try {
                const fd = new FormData();
                for (const f of fileInput.files) { fd.append('Files', f); }
                const resp = await fetch('/filesystem/chat-upload/', {
                    method: 'POST',
                    headers: { 'HX-Request': 'true' },
                    body: fd
                });
                const data = await resp.json();
                if (Array.isArray(data)) {
                    for (const node of data) {
                        this.addItem({ value: String(node.id), display: node.name });
                    }
                }
            } catch (e) {
                console.error('upload failed', e);
            } finally {
                this.uploading = false;
                fileInput.value = '';
            }
        }
    }"#
    .to_string()
}

pub fn chat_shell(
    session_id: Option<i64>,
    title: &str,
    transcript_html: &str,
    error: &str,
    compact: bool,
) -> Markup {
    let hidden_val = session_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "0".into());
    let new_session_url = "/llm-assistant/new-session/";
    let file_select_url = "/filesystem/file-select/?target_input=Files";
    let x_data = chat_form_x_data();
    let (root_class, transcript_class) = if compact {
        (
            "max-w-3xl mx-auto p-0 flex flex-col gap-4 h-full overflow-hidden",
            "flex flex-col gap-2 flex-1 overflow-y-auto border border-base-300 rounded-lg p-3 bg-base-200/40 min-h-0",
        )
    } else {
        (
            "flex flex-col h-full min-h-[24rem] gap-3",
            "flex-1 overflow-y-auto w-full max-w-2xl mx-auto border border-base-300 rounded-lg p-4 min-h-[12rem]",
        )
    };
    html! {
        div class=(root_class) {
            @if !compact {
                @if let Some(id) = session_id {
                    div class="text-sm opacity-70" {
                        (title) " · #" (id)
                    }
                } @else {
                    div class="flex items-center justify-between gap-2" {
                        p class="text-sm opacity-70" { "Start a new chat session to begin." }
                        form method="post" action=(new_session_url) {
                            (button_submit(ButtonSubmit {
                                label: "New session",
                                ..Default::default()
                            }))
                        }
                    }
                }
            }
            @if session_id.is_some() {
                (PreEscaped(format!(
                    r#"<div class="flex flex-col flex-1 gap-3 min-h-0" hx-ws:connect="/llm-assistant/ws/" hx-swap="none"><script>{}</script>"#,
                    ASSISTANT_CHAT_SCRIPT
                )))
            }
            div id="llm_assistant_errors" class="text-error text-sm max-w-2xl mx-auto w-full" {
                @if !error.is_empty() {
                    (error)
                }
            }
            div id="llm_assistant_transcript"
                class=(transcript_class)
                x-init="$nextTick(() => { $el.scrollTop = $el.scrollHeight })"
            {
                (PreEscaped(transcript_html))
            }
            div id="llm_assistant_stream"
                class="w-full max-w-2xl mx-auto mb-4 min-h-[1.5rem] border border-dashed border-base-300 rounded-lg p-4 text-sm" {}
            @if session_id.is_some() {
                (PreEscaped(chat_form_html(
                    &hidden_val,
                    &x_data,
                    file_select_url,
                )))
                (PreEscaped("</div>"))
            }
        }
    }
}

/// Landing chat page (no session).
#[derive(Generic)]
pub struct ChatPage;

impl ChatPage {
    fn pane_body(&self) -> Markup {
        chat_shell(None, "", "", "", false)
    }
}

impl crate::template::RenderAppPane for ChatPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(assistant_menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for ChatPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Assistant — Lariv", chrome, assistant_menu(), self.pane_body())
    }
}

/// Session chat page (WebSocket streaming via HTMX 4 hx-ws).
#[derive(Generic)]
pub struct ChatSessionPage {
    pub id: i64,
    pub title: String,
    pub transcript_html: String,
    pub error: String,
}

impl ChatSessionPage {
    fn pane_body(&self) -> Markup {
        chat_shell(
            Some(self.id),
            &self.title,
            &self.transcript_html,
            &self.error,
            false,
        )
    }
}

impl crate::template::RenderAppPane for ChatSessionPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(assistant_menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for ChatSessionPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.title),
            chrome,
            assistant_menu(),
            self.pane_body(),
        )
    }
}

#[derive(Clone)]
pub struct HistoryRow {
    pub id: i64,
    pub label: String,
}

#[derive(Generic)]
pub struct HistoryListPage {
    pub sessions: ObjectList<HistoryRow>,
    pub path_and_query: String,
}

impl HistoryListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [TableColumnHeader {
            label: "Chat",
            sort_url: None,
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .sessions
            .items
            .iter()
            .map(|s| TableRow {
                attrs: row_attr_navigate(&format!("/llm-assistant/c/{}/", s.id)),
                cells: vec![field_text(FieldText {
                    value: &s.label,
                    classes: "",
                })],
            })
            .collect();
        let actions = html! {
            form method="post" action="/llm-assistant/new-session/" {
                (button_submit(ButtonSubmit {
                    label: "New session",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
        };
        let pagination = render_pagination::<HistoryTableKey>(
            &self.path_and_query,
            self.sessions.number,
            self.sessions.num_pages,
            true,
        );
        data_table_list::<HistoryTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for HistoryListPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(assistant_menu(), self.render_table())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for HistoryListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "History — Lariv",
            chrome,
            assistant_menu(),
            self.render_table(),
        )
    }
}

#[derive(Clone)]
pub struct SkillRow {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub updated_at: String,
}

#[derive(Generic)]
pub struct SkillListPage {
    pub skills: ObjectList<SkillRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
}

impl SkillListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                label: "Description",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .skills
            .items
            .iter()
            .map(|s| TableRow {
                attrs: row_attr_navigate(&format!("/llm-assistant/skills/{}/", s.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &s.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &s.description,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: skill_filter_form::<SkillsTableKey>(&self.filter_name, "/llm-assistant/skills/"),
                ..Default::default()
            }))
            (button_link(ButtonLink {
                href: "/llm-assistant/skills/create/",
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal(ButtonModal {
                label: "",
                icon_name: Some("arrow-up-tray"),
                href: "/llm-assistant/skills/import/",
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<SkillsTableKey>(
            &self.path_and_query,
            self.skills.number,
            self.skills.num_pages,
            true,
        );
        data_table_list::<SkillsTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for SkillListPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(assistant_menu(), self.render_table())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for SkillListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Skills — Lariv", chrome, assistant_menu(), self.render_table())
    }
}

#[derive(Generic)]
pub struct SkillDetailPage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub content: String,
    pub files: Vec<(i64, String)>,
}

impl SkillDetailPage {
    fn pane_body(&self) -> Markup {
        let file_pairs: Vec<(String, String)> = self
            .files
            .iter()
            .map(|(id, name)| (name.clone(), format!("/filesystem/{id}/")))
            .collect();
        let file_items: Vec<(&str, Option<&str>)> = file_pairs
            .iter()
            .map(|(name, href)| (name.as_str(), Some(href.as_str())))
            .collect();
        detail(html! {
            div class="flex justify-end mb-2" {
                a href={(format!("/llm-assistant/skills/{}/export/", self.id))} download class="btn btn-sm btn-square btn-outline" {
                    (icon("arrow-down-tray", ""))
                }
            }
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (label_inline("Description", field_text(FieldText {
                        value: &self.description,
                        classes: "",
                    })))
                    (label_inline("Content", field_markdown(FieldMarkdown {
                        value: &self.content,
                        classes: "mt-2",
                    })))
                    (label_inline("Files", field_many_to_many(FieldManyToMany {
                        items: &file_items,
                        classes: "",
                    })))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for SkillDetailPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(skill_detail_menu(self.id, &self.name), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for SkillDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            skill_detail_menu(self.id, &self.name),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct SkillFormPage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub content: String,
    pub files: Vec<ManyToManyItem>,
    pub error: String,
}

impl SkillFormPage {
    fn menu(&self) -> Markup {
        if self.id == 0 {
            assistant_menu()
        } else {
            skill_detail_menu(self.id, &self.name)
        }
    }

    fn pane_body(&self) -> Markup {
        let is_create = self.id == 0;
        let action = if is_create {
            "/llm-assistant/skills/create/".to_string()
        } else {
            format!("/llm-assistant/skills/{}/update/", self.id)
        };
        let delete_url = format!("/llm-assistant/skills/{}/delete/", self.id);
        let ctx = FormCtx::new()
            .value("Name", self.name.as_str())
            .value("Description", self.description.as_str())
            .value("Content", self.content.as_str())
            .m2m("Files", &self.files);
        form(FormOpts {
            title: if is_create {
                "Create Skill"
            } else {
                "Edit Skill"
            },
            subtitle: if is_create {
                "Define a new assistant skill"
            } else {
                "Update skill details"
            },
            classes: "@container",
            attrs: form_hx_post_main(&action),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: SkillForm::render_inputs(&ctx),
            actions: html! {
                (container_row(
                    "flex flex-wrap justify-between gap-2 mt-2 items-center",
                    html! {
                        (container_row(
                            "flex justify-end gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Skill",
                                    ..Default::default()
                                }))
                                @if !is_create {
                                    (button_modal_form(ButtonModalForm {
                                        label: "Delete",
                                        icon_name: Some("trash"),
                                        name: "p_llm_assistant.SkillDeleteForm",
                                        href: &delete_url,
                                        form_post_url: &delete_url,
                                        modal_uid: SkillDeleteModalKey::ID,
                                        classes: "btn-error",
                                        ..Default::default()
                                    }))
                                }
                            },
                        ))
                    },
                ))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for SkillFormPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for SkillFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let title = if self.id == 0 {
            "Create skill — Lariv"
        } else {
            "Edit skill — Lariv"
        };
        app_scaffold(title, chrome, self.menu(), self.pane_body())
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub name: String,
    pub action: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", SkillDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            SkillDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        modal(crate::components::Modal {
            uid,
            children: crate::components::delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: crate::components::form_hx_post_selector(&self.action, &target),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct SkillImportPage;

impl RenderTemplate for SkillImportPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal(crate::components::Modal {
            uid: SkillImportModalKey::ID,
            children: form(FormOpts {
                title: "Import Skill",
                subtitle: "Upload a skill zip file to import it",
                attrs: form_hx_post_main("/llm-assistant/skills/import/")
                    .set("hx-encoding", "multipart/form-data"),
                enctype: Some("multipart/form-data"),
                inputs: html! {
                    (input_file(InputFile {
                        label: "Skill Zip File",
                        name: "File",
                        required: true,
                        accept: ".zip",
                        ..Default::default()
                    }))
                },
                actions: html! {
                    (container_row(
                        "flex justify-end gap-2 mt-2",
                        html! {
                            (button_submit(ButtonSubmit {
                                label: "Import",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        },
                    ))
                },
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

/// Lazy-load host for the assistant history sidebar (DB-backed panel via HTMX).
#[derive(Default)]
pub struct HistorySidebarPanel;

pub fn session_list_items(sessions: &[(i64, String)]) -> Markup {
    if sessions.is_empty() {
        return html! {
            div class="p-4 text-sm opacity-60 text-center" { "No conversations yet." }
        };
    }
    html! {
        @for (id, label) in sessions {
            (PreEscaped(format!(
                r##"<button type="button" class="btn btn-ghost btn-sm justify-start w-full text-left truncate" hx-get="/llm-assistant/sidebar-chat/{id}/" hx-target="#sidebar-chat-container" hx-swap="innerHTML" hx-push-url="false" @click="showModal = false">{label}</button>"##,
                label = label
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            )))
        }
    }
}

pub fn modal_sessions_oob(sessions: &[(i64, String)]) -> Markup {
    html! {
        div id="modal-sessions-list" hx-swap-oob="true" class="max-h-60 overflow-y-auto flex flex-col bg-base-200 rounded border border-base-300" {
            (session_list_items(sessions))
        }
    }
}

pub fn history_sidebar_panel_html(
    active_session_name: &str,
    open_session_id: i64,
    initial_chat: Markup,
    sessions: &[(i64, String)],
) -> Markup {
    let x_data = format!(
        r#"{{showModal: false, activeSessionId: $persist(0).as('llm-assistant-sidebar-active-session-id'), init() {{const serverSessionId = {open_session_id}; if (serverSessionId !== 0) {{ this.activeSessionId = serverSessionId; }} else {{ this.$nextTick(() => {{ if (this.activeSessionId !== 0) {{ const targetEl = document.getElementById('sidebar-chat-container'); if (targetEl) {{ htmx.ajax('GET', '/llm-assistant/sidebar-chat/' + this.activeSessionId + '/', {{target: targetEl, swap: 'innerHTML', source: targetEl}}); }} }} }}); }} }} }}"#,
        open_session_id = open_session_id,
    );
    html! {
        (PreEscaped(format!(
            r##"<div x-data="{x_data}" @new-session-created.window="activeSessionId = $event.detail.id; showModal = false; htmx.ajax('GET', '/llm-assistant/sidebar-chat/' + activeSessionId + '/', {{target: '#sidebar-chat-container', swap: 'innerHTML', source: $el}})" class="flex flex-col gap-0 p-2 h-full overflow-hidden" hx-push-url="false">"##,
        )))
        {
            div class="flex justify-between items-center flex-none border-b border-base-300 pb-2 px-1" {
                div id="session-name-container" class="text-sm font-semibold truncate max-w-[70%]" {
                    (active_session_name)
                }
                div class="flex gap-1 flex-none" {
                    (PreEscaped(
                        r##"<button type="button" class="btn btn-sm btn-ghost btn-circle" @click="showModal = true">"##,
                    ))
                    (icon("clock", ""))
                    (PreEscaped("</button>"))
                    (PreEscaped(
                        r##"<button type="button" class="btn btn-sm btn-ghost btn-circle" hx-post="/llm-assistant/new-session/?sidebar=1" hx-swap="none" hx-push-url="false">"##,
                    ))
                    (icon("plus", ""))
                    (PreEscaped("</button>"))
                }
            }
            div id="sidebar-chat-container" class="flex-1 flex flex-col gap-4 overflow-hidden min-h-0" hx-push-url="false" {
                (initial_chat)
            }
            (PreEscaped(
                r##"<dialog x-show="showModal" :class="showModal ? 'modal modal-open' : 'modal'">"##,
            ))
            {
                div class="modal-box bg-base-100 max-w-lg border border-base-300 p-6 relative" {
                    (PreEscaped(
                        r##"<button type="button" class="btn btn-sm btn-circle btn-ghost absolute right-3 top-3" @click="showModal = false">"##,
                    ))
                    (icon("x-mark", ""))
                    (PreEscaped("</button>"))
                    h3 class="text-lg font-bold mb-4" { "Conversations" }
                    div id="modal-sessions-list" class="max-h-60 overflow-y-auto flex flex-col bg-base-200 rounded border border-base-300" {
                        (session_list_items(sessions))
                    }
                }
                form method="dialog" class="modal-backdrop" {
                    (PreEscaped(
                        r##"<button type="button" @click="showModal = false">close</button>"##,
                    ))
                }
            }
            (PreEscaped("</dialog>"))
        }
        (PreEscaped("</div>"))
    }
}

pub fn sidebar_chat_partial(session_name: &str, chat: Markup) -> Markup {
    html! {
        div id="session-name-container" hx-swap-oob="true" class="text-sm font-semibold truncate max-w-[70%]" {
            (session_name)
        }
        div class="flex-1 overflow-hidden min-h-0" {
            (chat)
        }
    }
}

impl RenderSlot for HistorySidebarPanel {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        html! {
            div id="llm-assistant-history-panel-host"
                hx-get="/llm-assistant/history-panel/"
                hx-trigger="load"
                hx-swap="outerHTML" {}
        }
    }
}

define_register_items! {
    plugin: LlmAssistantTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    wrapper: SlotOf;
    bounds: [];
    hook: SlotsHook;
    items: [
        HistoryPanelIdx: HistoryPanelTag, RightSidebarSlotTag => HistorySidebarPanel,
    ]
}
