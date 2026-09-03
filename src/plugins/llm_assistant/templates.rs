//! Maud templates for chat, history, and skills management pages.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        AppLayoutKey, ButtonClear, ButtonModal, ButtonModalForm, ButtonSubmit, Crumb,
        DeleteConfirmation, FieldManyToMany, FieldMarkdown, FieldText, FieldTitle, FormOpts,
        HtmlAttrs, LayoutMain, LayoutSidebar, MainContentKey, ManyToManyItem, ObjectList,
        PaginationPage, RenderSlot, RightSidebarSlotTag, ShellChrome, ShellScaffold, SidebarMenu,
        SidebarMenuItem, SidebarNavLink, SlotCapability, SlotCtx, SlotOf, SlotRegistrar, SwapKey,
        TableButtonFilter, TableColumnHeader, TablePagination, TableRow, breadcrumbs, button_clear,
        button_modal, button_modal_form, button_submit, column_sort_url, container_column,
        container_row, data_table_list, data_table_list_refresh, detail, field_many_to_many,
        field_markdown, field_text, field_title, form, form_hx_get_route, form_hx_post_selector,
        form_hx_post_url, icon, label, layout_main, layout_sidebar, modal, modal_keyed,
        page_size_only_filter_form, pagination_pages, row_attr_navigate_route, shell_scaffold,
        sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane, sort_indicator,
        table_button_filter, table_pagination, with_list_filter_common,
    },
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use super::forms::{
    PreferencesForm, PreferencesFormField, SkillForm, SkillFormField, SkillImportForm,
    SkillNameFilterForm, SkillNameFilterFormField,
};
use super::keys::{
    HistoryTableKey, SkillCreateModalKey, SkillDeleteModalKey, SkillEditModalKey,
    SkillImportModalKey, SkillsTableKey,
};
use super::preferences::mail_encryption_choices;
use super::routes::{
    ChatIndexRouteTag, ChatSessionRouteTag, HistoryListRouteTag, PrefsGetRouteTag,
    PrefsPostRouteTag, SkillsCreateGetRouteTag, SkillsCreatePostRouteTag, SkillsDeleteGetRouteTag,
    SkillsDeletePostRouteTag, SkillsDetailRouteTag, SkillsExportRouteTag, SkillsImportGetRouteTag,
    SkillsImportPostRouteTag, SkillsListRouteTag, SkillsUpdateGetRouteTag,
    SkillsUpdatePostRouteTag,
};
use crate::plugins::filesystem::routes::VNodeDetailRouteTag;

define_register_items! {
    plugin: LlmAssistantTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ChatSessionIdx: ChatSessionPageTag => ChatSessionPage,
        HistoryListIdx: HistoryListPageTag => HistoryListPage,
        PrefsIdx: LlmAssistantPreferencesPageTag => LlmAssistantPreferencesPage,
        SkillListIdx: SkillListPageTag => SkillListPage,
        SkillDetailIdx: SkillDetailPageTag => SkillDetailPage,
        SkillEditModalIdx: SkillEditModalPageTag => SkillEditModalPage,
        SkillCreateModalIdx: SkillCreateModalPageTag => SkillCreateModalPage,
        ConfirmDeleteIdx: SkillConfirmDeletePageTag => ConfirmDeletePage,
        SkillImportIdx: SkillImportPageTag => SkillImportPage,
    ]
}

fn app_scaffold(
    _title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
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

fn assistant_chat_crumbs(title: &str) -> Markup {
    let index_url = ChatIndexRouteTag.url();
    let history_url = HistoryListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Assistant",
            href: Some(&index_url),
        },
        Crumb {
            label: "History",
            href: Some(&history_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

fn assistant_history_crumbs() -> Markup {
    let index_url = ChatIndexRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Assistant",
            href: Some(&index_url),
        },
        Crumb {
            label: "History",
            href: None,
        },
    ])
}

fn assistant_skills_list_crumbs() -> Markup {
    let index_url = ChatIndexRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Assistant",
            href: Some(&index_url),
        },
        Crumb {
            label: "Skills",
            href: None,
        },
    ])
}

fn assistant_skill_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let index_url = ChatIndexRouteTag.url();
    let list_url = SkillsListRouteTag.url();
    let detail_url = SkillsDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Assistant",
                href: Some(&index_url),
            },
            Crumb {
                label: "Skills",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Assistant",
                href: Some(&index_url),
            },
            Crumb {
                label: "Skills",
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

fn assistant_menu(current_path: &str) -> Markup {
    let history_url = HistoryListRouteTag.url();
    let skills_url = SkillsListRouteTag.url();
    let prefs_url = PrefsGetRouteTag.url();
    let links = [
        SidebarNavLink {
            key: "history",
            title: "History",
            url: &history_url,
            icon_name: None,
            match_prefixes: &[],
        },
        SidebarNavLink {
            key: "skills",
            title: "Skills",
            url: &skills_url,
            icon_name: None,
            match_prefixes: &[],
        },
        SidebarNavLink {
            key: "preferences",
            title: "Preferences",
            url: &prefs_url,
            icon_name: None,
            match_prefixes: &[],
        },
    ];
    sidebar_menu(SidebarMenu {
        title: "Assistant",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn assistant_prefs_crumbs() -> Markup {
    let index_url = ChatIndexRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Assistant",
            href: Some(&index_url),
        },
        Crumb {
            label: "Preferences",
            href: None,
        },
    ])
}

fn skill_detail_menu(skill_id: i64, name: &str, active: &str) -> Markup {
    let menu_title = format!("Skill: {name}");
    let detail_url = SkillsDetailRouteTag::new(skill_id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Skill Details",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
        },
    })
}

fn skill_filter_form<
    K: SwapKey,
    R: crate::http::FragmentGet<K> + crate::http::RouteUrl + Copy + Default,
>(
    name: &str,
    page_size: u32,
) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: with_list_filter_common(
            SkillNameFilterForm::render_inputs(
                &FormCtx::form::<SkillNameFilterForm>().value(SkillNameFilterFormField::Name, name),
            ),
            page_size,
        ),
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

const ASSISTANT_CHAT_SCRIPT: &str = r##"
(function() {
  if (window.__llmAssistantChatBound) return;
  window.__llmAssistantChatBound = true;
  // Coerce session_id to a number for the WS JSON body (hidden inputs are strings).
  document.body.addEventListener("htmx:before:ws:request", function(event) {
    if (!event || !event.detail || !event.detail.body) return;
    var form = event.target && event.target.closest
      ? event.target.closest("#llm_assistant_chat_form")
      : null;
    if (!form && event.target && event.target.id !== "llm_assistant_chat_form") return;
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
  var llmAssistantWsSocket = null;
  // After connect/reconnect, reattach to any in-flight turn for this session.
  document.body.addEventListener("htmx:after:ws:connection", function(event) {
    var conn = event.detail && event.detail.connection;
    var socket = conn && conn.socket;
    if (socket) llmAssistantWsSocket = socket;
    var sidEl = document.getElementById("llm_assistant_session_id");
    if (!sidEl) return;
    var sid = Number(sidEl.value);
    if (!sid || Number.isNaN(sid)) return;
    if (!socket || socket.readyState !== 1) return;
    try {
      socket.send(JSON.stringify({
        headers: { "HX-Request": "true", "HX-Request-Type": "partial" },
        body: { session_id: sid, message: "", attach: true }
      }));
    } catch (e) {
      console.warn("llm_assistant: attach send failed", e);
    }
  });
  document.body.addEventListener("click", function(event) {
    var hitlBtn = event.target && event.target.closest
      ? event.target.closest("[data-hitl-id]")
      : null;
    if (hitlBtn) {
      event.preventDefault();
      var hitlId = hitlBtn.getAttribute("data-hitl-id") || "";
      if (!hitlId) return;
      var sidEl = document.getElementById("llm_assistant_session_id");
      var sid = sidEl ? Number(sidEl.value) : 0;
      if (Number.isNaN(sid)) sid = 0;
      var socket = llmAssistantWsSocket;
      if (!socket || socket.readyState !== 1) return;
      var approve = hitlBtn.getAttribute("data-hitl-approve") === "true";
      var deny = hitlBtn.getAttribute("data-hitl-deny") === "true";
      try {
        socket.send(JSON.stringify({
          headers: { "HX-Request": "true", "HX-Request-Type": "partial" },
          body: {
            session_id: sid,
            message: "",
            hitl_id: hitlId,
            hitl_approve: approve,
            hitl_deny: deny
          }
        }));
      } catch (e) {
        console.warn("llm_assistant: hitl send failed", e);
      }
      return;
    }
    var btn = event.target && event.target.closest
      ? event.target.closest("#llm_assistant_chat_send")
      : null;
    if (!btn || btn.getAttribute("data-stop") !== "true") return;
    event.preventDefault();
    var sidEl = document.getElementById("llm_assistant_session_id");
    var sid = sidEl ? Number(sidEl.value) : 0;
    if (Number.isNaN(sid)) sid = 0;
    var socket = llmAssistantWsSocket;
    if (!socket || socket.readyState !== 1) return;
    try {
      socket.send(JSON.stringify({
        headers: { "HX-Request": "true", "HX-Request-Type": "partial" },
        body: { session_id: sid, message: "", stop: true }
      }));
    } catch (e) {
      console.warn("llm_assistant: stop send failed", e);
    }
  });
  document.body.addEventListener("keydown", function(event) {
    if (!event.target || event.target.id !== "llm_assistant_chat_message") return;
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    var btn = document.getElementById("llm_assistant_chat_send");
    if (btn && btn.getAttribute("data-stop") === "true") return;
    var form = event.target.form;
    if (form) form.requestSubmit();
  });
  function llmAssistantScrollToBottom() {
    var transcript = document.getElementById("llm_assistant_transcript");
    if (transcript) transcript.scrollTop = transcript.scrollHeight;
  }
  function llmAssistantSyncSessionOpened() {
    var sidEl = document.getElementById("llm_assistant_session_id");
    if (!sidEl) return;
    var id = Number(sidEl.value);
    if (!id || Number.isNaN(id)) return;
    window.dispatchEvent(new CustomEvent("llm-assistant-session-opened", { detail: { id: id } }));
  }
  function llmAssistantApplyWorkingClose() {
    var closer = document.getElementById("llm_assistant_working_close");
    if (!closer) return;
    var detailsId = closer.getAttribute("data-details-id");
    if (!detailsId) return;
    var details = document.getElementById(detailsId);
    if (details) details.removeAttribute("open");
    closer.removeAttribute("data-details-id");
  }
  document.body.addEventListener("htmx:after:ws:message", function() {
    llmAssistantApplyWorkingClose();
    llmAssistantScrollToBottom();
    llmAssistantSyncSessionOpened();
  });
})();
"##;

/// HTMX 4: clear composer + swap Send for Stop after WS send (`hx-on::after:ws:request`).
/// Send is restored via OOB `form_ready_oob` when the turn finishes or is stopped.
const CHAT_FORM_AFTER_WS_REQUEST: &str = r#"var ta=document.getElementById('llm_assistant_chat_message');if(ta)ta.value='';var btn=document.getElementById('llm_assistant_chat_send');if(btn){btn.disabled=false;btn.type='button';btn.textContent='Stop';btn.className='btn btn-error';btn.setAttribute('data-stop','true');}if(window.Alpine){var d=Alpine.$data(this);if(d){d.items=[];if(d.syncStore)d.syncStore();}}"#;

fn chat_form_html(hidden_val: &str, x_data: &str, file_select_url: &str) -> String {
    let icon_x = icon("x-mark", "heroicon-sm").into_string();
    let icon_upload = icon("arrow-up-tray", "heroicon-sm").into_string();
    let icon_clip = icon("paper-clip", "heroicon-sm").into_string();
    format!(
        r#"<form id="llm_assistant_chat_form" class="flex flex-col gap-2 w-full" hx-ws:send hx-on::after:ws:request="{after_ws}" x-data="{x_data}" x-init="syncStore()" @fk-multi-select.window="eventHandler($event)">
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
        after_ws = html_escape_attr(CHAT_FORM_AFTER_WS_REQUEST),
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
                const sidEl = document.getElementById('llm_assistant_session_id');
                fd.append('SessionId', sidEl ? sidEl.value : '0');
                for (const f of fileInput.files) { fd.append('Files', f); }
                const resp = await fetch('/llm-assistant/chat-upload/', {
                    method: 'POST',
                    headers: { 'HX-Request': 'true' },
                    body: fd
                });
                const data = await resp.json();
                if (data && Array.isArray(data.files)) {
                    if (data.session_id && sidEl) {
                        sidEl.value = String(data.session_id);
                        window.dispatchEvent(new CustomEvent('llm-assistant-session-opened', { detail: { id: data.session_id } }));
                    }
                    for (const node of data.files) {
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
    let file_select_url = "/filesystem/file-select/?target_input=Files&multi=1";
    let x_data = chat_form_x_data();
    let (root_class, transcript_class) = if compact {
        (
            "w-full p-0 flex flex-col gap-4 h-full overflow-hidden min-w-0",
            "flex flex-col gap-2 flex-1 overflow-y-auto min-h-0 min-w-0 w-full",
        )
    } else {
        (
            "flex flex-col h-full min-h-[24rem] gap-3 w-full min-w-0",
            "flex-1 overflow-y-auto w-full min-h-[12rem] min-w-0",
        )
    };
    html! {
        div class=(root_class) {
            @if !compact {
                @if let Some(id) = session_id {
                    div class="text-sm opacity-70" {
                        (title) " · #" (id)
                    }
                }
            }
            (PreEscaped(format!(
                r#"<div class="flex flex-col flex-1 gap-3 min-h-0 min-w-0 w-full" hx-ws:connect="/llm-assistant/ws/" hx-swap="none" hx-config="ws.pauseOnBackground:false"><script>{}</script>"#,
                ASSISTANT_CHAT_SCRIPT
            )))
            div id="llm_assistant_errors" class="text-error text-sm w-full" {
                @if !error.is_empty() {
                    (error)
                }
            }
            span id="llm_assistant_working_close" hidden {}
            div id="llm_assistant_transcript"
                class=(transcript_class)
                x-init="$nextTick(() => { $el.scrollTop = $el.scrollHeight })"
            {
                (PreEscaped(transcript_html))
            }
            (PreEscaped(chat_form_html(
                &hidden_val,
                &x_data,
                file_select_url,
            )))
            (PreEscaped("</div>"))
        }
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            assistant_menu(&ChatSessionRouteTag::new(self.id).url()),
            assistant_chat_crumbs(&self.title),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(assistant_chat_crumbs(&self.title), self.pane_body())
    }
}

impl RenderTemplate for ChatSessionPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.title),
            chrome,
            assistant_menu(&ChatSessionRouteTag::new(self.id).url()),
            assistant_chat_crumbs(&self.title),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct LlmAssistantPreferencesPage {
    pub api_key: String,
    pub chat_model: String,
    pub chat_model_choices: Vec<(String, String)>,
    pub cse_api_key: String,
    pub cse_cx: String,
    pub imap_server: String,
    pub imap_port: String,
    pub smtp_server: String,
    pub smtp_port: String,
    pub email: String,
    pub password: String,
    pub mail_encryption: String,
    pub email_filter: String,
    pub email_owner_user_id: i64,
    pub email_owner_display: String,
    pub email_attachments_parent_id: i64,
    pub email_attachments_parent_display: String,
    pub chat_attachments_parent_id: i64,
    pub chat_attachments_parent_display: String,
    pub error: String,
}

impl LlmAssistantPreferencesPage {
    fn body(&self) -> Markup {
        form(FormOpts {
            // Same-structure prefs save: swap `#main-content` (not `#app-layout`).
            attrs: form_hx_post_url::<MainContentKey>(&PrefsPostRouteTag.path())
                .set("hx-swap", "outerHTML"),
            title: "Assistant Preferences",
            subtitle: "Configure Gemini, Google Custom Search, and email credentials used for chat",
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: PreferencesForm::render_inputs(
                &FormCtx::form::<PreferencesForm>()
                    .value(PreferencesFormField::ApiKey, self.api_key.as_str())
                    .value(PreferencesFormField::ChatModel, self.chat_model.as_str())
                    .choices(PreferencesFormField::ChatModel, &self.chat_model_choices)
                    .value(PreferencesFormField::CseApiKey, self.cse_api_key.as_str())
                    .value(PreferencesFormField::CseCx, self.cse_cx.as_str())
                    .value(PreferencesFormField::ImapServer, self.imap_server.as_str())
                    .value(PreferencesFormField::ImapPort, self.imap_port.as_str())
                    .value(PreferencesFormField::SmtpServer, self.smtp_server.as_str())
                    .value(PreferencesFormField::SmtpPort, self.smtp_port.as_str())
                    .value(PreferencesFormField::Email, self.email.as_str())
                    .value(PreferencesFormField::Password, self.password.as_str())
                    .value(
                        PreferencesFormField::MailEncryption,
                        self.mail_encryption.as_str(),
                    )
                    .choices(
                        PreferencesFormField::MailEncryption,
                        &mail_encryption_choices(),
                    )
                    .value(
                        PreferencesFormField::EmailFilter,
                        self.email_filter.as_str(),
                    )
                    .value(
                        PreferencesFormField::EmailOwnerUserId,
                        if self.email_owner_user_id > 0 {
                            self.email_owner_user_id.to_string()
                        } else {
                            String::new()
                        }
                        .as_str(),
                    )
                    .display(
                        PreferencesFormField::EmailOwnerUserId,
                        self.email_owner_display.as_str(),
                    )
                    .value(
                        PreferencesFormField::EmailAttachmentsParentId,
                        if self.email_attachments_parent_id > 0 {
                            self.email_attachments_parent_id.to_string()
                        } else {
                            String::new()
                        }
                        .as_str(),
                    )
                    .display(
                        PreferencesFormField::EmailAttachmentsParentId,
                        self.email_attachments_parent_display.as_str(),
                    )
                    .value(
                        PreferencesFormField::ChatAttachmentsParentId,
                        if self.chat_attachments_parent_id > 0 {
                            self.chat_attachments_parent_id.to_string()
                        } else {
                            String::new()
                        }
                        .as_str(),
                    )
                    .display(
                        PreferencesFormField::ChatAttachmentsParentId,
                        self.chat_attachments_parent_display.as_str(),
                    ),
            ),
            actions: html! {
                (button_submit(ButtonSubmit {
                    label: "Save Preferences",
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for LlmAssistantPreferencesPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            assistant_menu(&PrefsGetRouteTag.url()),
            assistant_prefs_crumbs(),
            self.body(),
        )
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(assistant_prefs_crumbs(), self.body())
    }
}

impl RenderTemplate for LlmAssistantPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Assistant Preferences — Lariv",
            chrome,
            assistant_menu(&PrefsGetRouteTag.url()),
            assistant_prefs_crumbs(),
            self.body(),
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
    pub page_size: u32,
}

impl HistoryListPage {
    pub fn render_table(&self) -> Markup {
        let headers = [TableColumnHeader {
            key: "Chat",
            label: "Chat",
            sort_url: None,
            push_url: true,
        }];
        let rows: Vec<TableRow> = self
            .sessions
            .items
            .iter()
            .map(|s| TableRow {
                attrs: row_attr_open_sidebar_session(s.id),
                cells: vec![field_text(FieldText {
                    value: &s.label,
                    classes: "",
                })],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: page_size_only_filter_form::<HistoryTableKey, HistoryListRouteTag>(
                    self.page_size,
                ),
                ..Default::default()
            }))
            form method="post" action="/llm-assistant/new-session/" {
                (button_submit(ButtonSubmit {
                    label: "",
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            assistant_menu(&self.path_and_query),
            assistant_history_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(assistant_history_crumbs(), self.render_table())
    }
}

impl RenderTemplate for HistoryListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "History — Lariv",
            chrome,
            assistant_menu(&self.path_and_query),
            assistant_history_crumbs(),
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
    pub page_size: u32,
}

impl SkillListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let desc_sort = column_sort_url(&self.path_and_query, "Description", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let desc_label = format!("Description{}", sort_indicator(&self.sort, "Description"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Description",
                label: &desc_label,
                sort_url: Some(&desc_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .skills
            .items
            .iter()
            .map(|s| TableRow {
                attrs: row_attr_navigate_route(SkillsDetailRouteTag::new(s.id)),
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
                panel: skill_filter_form::<SkillsTableKey, SkillsListRouteTag>(&self.filter_name, self.page_size),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_llm_assistant.SkillCreateForm",
                href: &SkillsCreateGetRouteTag.url(),
                form_post_url: &SkillsCreateGetRouteTag.path(),
                modal_uid: SkillCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
            (button_modal(ButtonModal {
                label: "",
                icon_name: Some("arrow-up-tray"),
                href: &SkillsImportGetRouteTag.url(),
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
        data_table_list_refresh::<SkillsTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl crate::template::RenderAppPane for SkillListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            assistant_menu(&self.path_and_query),
            assistant_skills_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(assistant_skills_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for SkillListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Skills — Lariv",
            chrome,
            assistant_menu(&self.path_and_query),
            assistant_skills_list_crumbs(),
            self.render_table(),
        )
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
            .map(|(id, name)| (name.clone(), VNodeDetailRouteTag::new(*id).url()))
            .collect();
        let file_items: Vec<(&str, Option<&str>)> = file_pairs
            .iter()
            .map(|(name, href)| (name.as_str(), Some(href.as_str())))
            .collect();
        let edit_get = SkillsUpdateGetRouteTag::new(self.id).url();
        let edit_post = SkillsUpdatePostRouteTag::new(self.id).path();
        detail(html! {
            div class="flex justify-end mb-2" {
                a href={(SkillsExportRouteTag::new(self.id).url())} download class="btn btn-sm btn-square btn-outline" {
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
                    (label("Description", field_text(FieldText {
                        value: &self.description,
                        classes: "",
                    })))
                    (label("Content", field_markdown(FieldMarkdown {
                        value: &self.content,
                        classes: "",
                    })))
                    (label("Files", field_many_to_many(FieldManyToMany {
                        items: &file_items,
                        classes: "",
                    })))
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_llm_assistant.SkillEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: SkillEditModalKey::ID,
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

impl crate::template::RenderAppPane for SkillDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            skill_detail_menu(self.id, &self.name, "detail"),
            assistant_skill_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            assistant_skill_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}

impl RenderTemplate for SkillDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            skill_detail_menu(self.id, &self.name, "detail"),
            assistant_skill_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}

/// Edit skill modal. Create uses [`SkillCreateModalPage`].
#[derive(Generic)]
pub struct SkillEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub content_hint: String,
    pub files: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for SkillEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = SkillsDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<SkillForm>()
            .value(SkillFormField::Name, self.name.as_str())
            .value(SkillFormField::Description, self.description.as_str())
            .value(SkillFormField::Content, self.content.as_str())
            .hint(SkillFormField::Content, self.content_hint.as_str())
            .m2m(SkillFormField::Files, &self.files);
        modal_keyed::<SkillEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit skill" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<SkillEditModalKey>(&modal_edit_post_url(
                        SkillsUpdatePostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: SkillForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
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
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct SkillCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub content_hint: String,
    pub files: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for SkillCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_llm_assistant.SkillCreateForm"
        } else {
            self.form_name.as_str()
        };
        let ctx = FormCtx::form::<SkillForm>()
            .value(SkillFormField::Name, self.name.as_str())
            .value(SkillFormField::Description, self.description.as_str())
            .value(SkillFormField::Content, self.content.as_str())
            .hint(SkillFormField::Content, self.content_hint.as_str())
            .m2m(SkillFormField::Files, &self.files);
        modal_keyed::<SkillCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Skill",
                subtitle: "Define a new assistant skill",
                classes: "@container",
                attrs: crate::components::swap::form_hx_post_for_url::<SkillCreateModalKey>(
                    &modal_create_post_url(
                        SkillsCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: SkillForm::render_inputs(&ctx),
                actions: html! {
                    (container_row(
                        "flex justify-end gap-2 mt-2",
                        html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Skill",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        },
                    ))
                },
                ..Default::default()
            }),
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub name: String,
    pub id: i64,
    pub error: String,
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
                attrs: crate::components::form_hx_post_selector(
                    &SkillsDeletePostRouteTag::new(self.id).url(),
                    &target,
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
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
                attrs: form_hx_post_selector(
                    &SkillsImportPostRouteTag.path(),
                    AppLayoutKey::SELECTOR,
                )
                .set("hx-select", AppLayoutKey::SELECTOR)
                .set("hx-swap", "outerHTML")
                .set("hx-push-url", "true")
                .set("hx-encoding", "multipart/form-data"),
                enctype: Some("multipart/form-data"),
                inputs: SkillImportForm::render_inputs(&FormCtx::form::<SkillImportForm>()),
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

/// Row click: open the LLM right sidebar and load the selected session chat.
fn row_attr_open_sidebar_session(session_id: i64) -> HtmlAttrs {
    // Open the right drawer and load this session into the sidebar chat.
    // Use `llm-assistant-open-session` (not `session-opened`) so live WS turns do not
    // reload the form and re-enable Send mid-response.
    // If the panel host has not swapped in yet, stash the id for panel init.
    let click = format!(
        "window.dispatchEvent(new CustomEvent('llm-assistant-open-sidebar'));\
         window.dispatchEvent(new CustomEvent('llm-assistant-open-session',{{detail:{{id:{session_id}}}}}));\
         if(!document.getElementById('sidebar-chat-container')){{\
           window.__llmAssistantPendingSessionId={session_id};\
         }}"
    );
    HtmlAttrs::new()
        .set(
            "class",
            "cursor-pointer hover:bg-base-200 transition-colors",
        )
        .set("role", "button")
        .set("tabindex", "0")
        .set("onclick", click)
}

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
        r#"{{showModal: false, activeSessionId: $persist(0).as('llm-assistant-sidebar-active-session-id'), loadSession(id) {{ this.activeSessionId = id; const targetEl = document.getElementById('sidebar-chat-container'); if (targetEl) {{ htmx.ajax('GET', '/llm-assistant/sidebar-chat/' + id + '/', {{target: targetEl, swap: 'innerHTML', source: this.$el}}); }} }}, init() {{ const pending = window.__llmAssistantPendingSessionId; if (pending) {{ window.__llmAssistantPendingSessionId = null; this.loadSession(pending); return; }} this.activeSessionId = {open_session_id}; }}, openDraft() {{ this.activeSessionId = 0; this.showModal = false; const targetEl = document.getElementById('sidebar-chat-container'); if (targetEl) {{ htmx.ajax('GET', '/llm-assistant/sidebar-chat/0/', {{target: targetEl, swap: 'innerHTML', source: targetEl}}); }} }} }}"#,
        open_session_id = open_session_id,
    );
    html! {
        (PreEscaped(format!(
            r##"<div x-data="{x_data}" @new-session-created.window="showModal = false; loadSession($event.detail.id)" @llm-assistant-open-session.window="loadSession($event.detail.id)" @llm-assistant-session-opened.window="activeSessionId = $event.detail.id" class="flex flex-col gap-0 p-2 h-full overflow-hidden" hx-push-url="false">"##,
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
                        r##"<button type="button" class="btn btn-sm btn-ghost btn-circle" @click="openDraft()">"##,
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
