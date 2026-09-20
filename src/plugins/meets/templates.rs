use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonSubmit, Crumb, DetailHeader, FieldText, FormOpts, LayoutMain, LayoutSidebar,
        ObjectList, PaginationPage, ShellAuth, ShellChrome, ShellScaffold, SidebarMenu,
        SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter,
        TableColumnHeader, TablePagination, TableRow, WebrtcMediaSource, WebrtcVideoStage,
        breadcrumbs, button_submit, column_sort_url, data_table_list_refresh, detail_header,
        field_text, form, form_hx_get_route, form_hx_post_main_url, form_hx_post_redirect,
        form_hx_post_route, form_hx_post_url, label, layout_main, layout_sidebar, modal_keyed,
        pagination_pages, row_attr_navigate_route, shell_auth, shell_scaffold, sidebar_menu,
        sidebar_menu_item_pane, sort_indicator, table_button_filter, table_create_button,
        table_pagination, webrtc_media_source, webrtc_video_stage, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::modal_create_post_url,
};

use super::forms::{
    AnonymousJoinForm, AnonymousJoinFormField, CreateRoomFilterForm, CreateRoomFilterFormField,
    CreateRoomForm, CreateRoomFormField,
};
use super::keys::{MeetsCreateModalKey, MeetsHubTableKey, MeetsRoomChromeKey, MeetsRoomRosterKey};
use super::routes::{
    HubRouteTag, JoinPostRouteTag, RecordingsRouteTag, RoomCreatePostRouteTag, RoomLeaveRouteTag,
    RoomLockRouteTag, RoomRouteTag, RoomStartRouteTag,
};

crate::define_register_items! {
    plugin: MeetsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        HubIdx: MeetsHubPageTag => MeetsHubPage,
        CreateModalIdx: MeetsCreateModalPageTag => MeetsCreateModalPage,
        RoomIdx: MeetsRoomPageTag => MeetsRoomPage,
        JoinIdx: MeetsJoinPageTag => MeetsJoinPage,
        RecordingsIdx: MeetsRecordingsPageTag => MeetsRecordingsPage,
    ]
}

crate::define_register_items! {
    plugin: MeetsTag;
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

pub fn meets_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Meets",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Rooms",
                url: &HubRouteTag.url(),
                active: active == "rooms",
                ..Default::default()
            }))
        },
    })
}

fn hub_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Meets",
        href: None,
    }])
}

fn room_crumbs(code: &str) -> Markup {
    let list_url = HubRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Meets",
            href: Some(&list_url),
        },
        Crumb {
            label: code,
            href: None,
        },
    ])
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
pub struct RoomRow {
    pub code: String,
    pub created_at: String,
    pub joining_allowed: bool,
    pub live: bool,
}

#[derive(Clone)]
pub struct MeetsHubPage {
    pub rooms: ObjectList<RoomRow>,
    pub filter_code: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl MeetsHubPage {
    pub fn render_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let created_sort = column_sort_url(&self.path_and_query, "CreatedAt", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let created_label = format!("Created{}", sort_indicator(&self.sort, "CreatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Code",
                label: &code_label,
                sort_url: Some(&code_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "CreatedAt",
                label: &created_label,
                sort_url: Some(&created_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Status",
                label: "Status",
                sort_url: None,
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .rooms
            .items
            .iter()
            .map(|r| {
                let status = if r.live {
                    "Live"
                } else if r.joining_allowed {
                    "Open"
                } else {
                    "Locked"
                };
                TableRow {
                    attrs: row_attr_navigate_route(RoomRouteTag::new(r.code.clone())),
                    cells: vec![
                        field_text(FieldText {
                            value: &r.code,
                            classes: "font-mono",
                        }),
                        field_text(FieldText {
                            value: &r.created_at,
                            classes: "",
                        }),
                        field_text(FieldText {
                            value: status,
                            classes: "",
                        }),
                    ],
                }
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<MeetsHubTableKey, HubRouteTag>(HubRouteTag),
                    inputs: with_list_filter_common(
                        CreateRoomFilterForm::render_inputs(
                            &FormCtx::form::<CreateRoomFilterForm>(CsrfToken::current())
                                .value(CreateRoomFilterFormField::Code, &self.filter_code),
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
            (table_create_button::<MeetsHubTableKey, MeetsCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        data_table_list_refresh::<MeetsHubTableKey>(
            "Rooms",
            actions,
            &headers,
            &rows,
            render_pagination::<MeetsHubTableKey>(
                &self.path_and_query,
                self.rooms.number,
                self.rooms.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for MeetsHubPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(meets_menu("rooms"), hub_crumbs(), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(hub_crumbs(), self.render_table())
    }
}

impl RenderTemplate for MeetsHubPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Meets — Lariv",
            chrome,
            meets_menu("rooms"),
            hub_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic, Clone)]
pub struct MeetsCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub anonymous_allowed: bool,
    pub joining_allowed: bool,
    pub start_at: String,
    pub error: String,
}

impl RenderTemplate for MeetsCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_meets.RoomCreateForm"
        } else {
            self.form_name.as_str()
        };
        let ctx = FormCtx::form::<CreateRoomForm>(CsrfToken::current())
            .checked(
                CreateRoomFormField::AnonymousAllowed,
                self.anonymous_allowed,
            )
            .checked(CreateRoomFormField::JoiningAllowed, self.joining_allowed)
            .value(CreateRoomFormField::StartAt, self.start_at.as_str());
        modal_keyed::<MeetsCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "New meeting",
                    attrs: form_hx_post_url::<MeetsCreateModalKey>(&modal_create_post_url(
                        RoomCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CreateRoomForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Create",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

#[derive(Clone)]
pub struct RosterEntry {
    pub joined_user_id: i64,
    pub name: String,
    pub user_type: String,
}

#[derive(Clone)]
pub struct RecordingRow {
    pub id: i64,
    pub started_at: String,
    pub vnode_id: i64,
    pub filename: String,
}

#[derive(Clone)]
pub struct MeetsRoomPage {
    pub code: String,
    pub live: bool,
    pub joining_allowed: bool,
    pub anonymous_allowed: bool,
    pub is_host: bool,
    pub authenticated: bool,
    pub joined_user_id: Option<i64>,
    pub ice_servers_json: String,
    pub signaling_url: String,
    pub roster: Vec<RosterEntry>,
    pub recordings: Vec<RecordingRow>,
    pub start_at_label: String,
}

impl MeetsRoomPage {
    pub fn render_chrome(&self) -> Markup {
        let recordings_url = RecordingsRouteTag::new(self.code.clone()).url();
        let lock_label = if self.joining_allowed {
            "Lock joining"
        } else {
            "Unlock joining"
        };
        html! {
            div id=(MeetsRoomChromeKey::ID) class="flex flex-wrap items-center gap-2 mb-4" {
                (detail_header(DetailHeader {
                    title: &format!("Meeting {}", self.code),
                    actions: html! {
                        span class="text-sm opacity-70" {
                            @if self.live { "Live" } @else { "Lobby" }
                            @if !self.start_at_label.is_empty() {
                                " · Starts " (self.start_at_label)
                            }
                        }
                    },
                }))
                @if self.is_host && !self.live {
                    (form(&CsrfToken::current(), FormOpts {
                        attrs: form_hx_post_redirect(RoomStartRouteTag::new(self.code.clone())),
                        actions: html! {
                            (button_submit(ButtonSubmit { label: "Start meeting", classes: "btn-primary", ..Default::default() }))
                        },
                        ..Default::default()
                    }))
                }
                @if self.is_host {
                    (form(&CsrfToken::current(), FormOpts {
                        attrs: form_hx_post_route::<MeetsRoomChromeKey, RoomLockRouteTag>(
                            RoomLockRouteTag::new(self.code.clone()),
                        ).set("hx-swap", "outerHTML"),
                        actions: html! {
                            (button_submit(ButtonSubmit { label: lock_label, classes: "btn-outline", ..Default::default() }))
                        },
                        ..Default::default()
                    }))
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_redirect(RoomLeaveRouteTag::new(self.code.clone())),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Leave", classes: "btn-ghost", ..Default::default() }))
                    },
                    ..Default::default()
                }))
                @if self.is_host {
                    a class="btn btn-ghost btn-sm" href=(recordings_url) { "Recordings" }
                }
            }
        }
    }

    pub fn render_roster(&self) -> Markup {
        html! {
            div id=(MeetsRoomRosterKey::ID) class="mb-4" {
                h3 class="font-semibold mb-2" { "Participants" }
                ul class="flex flex-col gap-1" {
                    @for p in &self.roster {
                        li class="text-sm" {
                            span { (p.name) }
                            span class="opacity-60 ml-2" { "(" (p.user_type) ")" }
                        }
                    }
                }
            }
        }
    }

    pub fn render_media(&self) -> Markup {
        let Some(joined_user_id) = self.joined_user_id else {
            return html! {};
        };
        let preview_only = !self.live;
        let codec_priority = crate::plugins::meets::sfu::codec::video_codec_priority_json();
        html! {
            div class="flex flex-col gap-4" {
                (webrtc_media_source(WebrtcMediaSource {
                    room_code: &self.code,
                    preview_only,
                    signaling_url: &self.signaling_url,
                    joined_user_id,
                    ice_servers_json: &self.ice_servers_json,
                }))
                @if self.live {
                    (webrtc_video_stage(WebrtcVideoStage {
                        room_code: &self.code,
                        signaling_url: &self.signaling_url,
                        joined_user_id,
                        ice_servers_json: &self.ice_servers_json,
                        video_codec_priority_json: &codec_priority,
                    }))
                }
            }
        }
    }

    pub fn render_body(&self) -> Markup {
        html! {
            (self.render_chrome())
            (self.render_roster())
            (self.render_media())
        }
    }
}

impl RenderAppPane for MeetsRoomPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            meets_menu("rooms"),
            room_crumbs(&self.code),
            self.render_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(room_crumbs(&self.code), self.render_body())
    }
}

impl RenderTemplate for MeetsRoomPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        if self.authenticated {
            app_scaffold(
                &format!("Meeting {} — Lariv", self.code),
                chrome,
                meets_menu("rooms"),
                room_crumbs(&self.code),
                self.render_body(),
            )
        } else {
            shell_auth(ShellAuth {
                title: &format!("Meeting {} — Lariv", self.code),
                registry_head: chrome.head.clone(),
                body: self.render_body(),
                ..Default::default()
            })
        }
    }
}

#[derive(Clone)]
pub struct MeetsJoinPage {
    pub code: String,
    pub name: String,
    pub email: String,
    pub error: String,
    pub authenticated: bool,
}

impl MeetsJoinPage {
    fn body(&self) -> Markup {
        let post = JoinPostRouteTag::new(self.code.clone()).path();
        html! {
            (label("Meeting", field_text(FieldText { value: &self.code, classes: "font-mono" })))
            (form(&CsrfToken::current(), FormOpts {
                title: "Join as guest",
                attrs: form_hx_post_main_url(&post),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: AnonymousJoinForm::render_inputs(
                    &FormCtx::form::<AnonymousJoinForm>(CsrfToken::current())
                        .value(AnonymousJoinFormField::Name, &self.name)
                        .value(AnonymousJoinFormField::Email, &self.email),
                ),
                actions: html! {
                    (button_submit(ButtonSubmit { label: "Join", classes: "btn-primary w-full", ..Default::default() }))
                },
                ..Default::default()
            }))
        }
    }
}

impl RenderAppPane for MeetsJoinPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(meets_menu("rooms"), room_crumbs(&self.code), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(room_crumbs(&self.code), self.body())
    }
}

impl RenderTemplate for MeetsJoinPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        if self.authenticated {
            app_scaffold(
                "Join meeting — Lariv",
                chrome,
                meets_menu("rooms"),
                room_crumbs(&self.code),
                self.body(),
            )
        } else {
            shell_auth(ShellAuth {
                title: "Join meeting — Lariv",
                registry_head: chrome.head.clone(),
                body: self.body(),
                ..Default::default()
            })
        }
    }
}

#[derive(Clone)]
pub struct MeetsRecordingsPage {
    pub code: String,
    pub recordings: Vec<RecordingRow>,
}

impl MeetsRecordingsPage {
    fn body(&self) -> Markup {
        html! {
            h2 class="text-lg font-semibold mb-4" { "Recordings for " (self.code) }
            @if self.recordings.is_empty() {
                p class="opacity-70" { "No recordings yet." }
            } @else {
                ul class="flex flex-col gap-2" {
                    @for r in &self.recordings {
                        li {
                            a class="link" href=(format!("/filesystem/{}/download", r.vnode_id)) {
                                (r.filename) " — " (r.started_at)
                            }
                        }
                    }
                }
            }
        }
    }
}

impl RenderAppPane for MeetsRecordingsPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(meets_menu("rooms"), room_crumbs(&self.code), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(room_crumbs(&self.code), self.body())
    }
}

impl RenderTemplate for MeetsRecordingsPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Recordings — Lariv",
            chrome,
            meets_menu("rooms"),
            room_crumbs(&self.code),
            self.body(),
        )
    }
}
