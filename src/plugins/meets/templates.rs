use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, DetailHeader, FieldText,
        FormOpts, LayoutMain, LayoutSidebar, MeetsMediaSource, MeetsVideoStage, ObjectList,
        PaginationPage, ShellAuth, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem,
        SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, breadcrumbs, button_link_route, button_modal_form,
        button_modal_route, button_post_fragment_route_swap, button_post_route, button_submit,
        column_sort_url, container_column, data_table_list_refresh, delete_confirmation, detail,
        detail_header, field_text, form, form_hx_get_route, form_hx_post_main_url,
        form_hx_post_redirect, form_hx_post_selector, form_hx_post_url, label, layout_main,
        layout_sidebar, meets_media_source, meets_video_stage, modal, modal_keyed,
        pagination_pages, row_attr_navigate_route, shell_auth, shell_scaffold, sidebar_menu,
        sidebar_menu_item_pane, sort_indicator, table_button_filter, table_create_button,
        table_pagination, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_url, modal_edit_post_url},
};

use super::forms::{
    AnonymousJoinForm, AnonymousJoinFormField, CreateRoomFilterForm, CreateRoomFilterFormField,
    CreateRoomForm, CreateRoomFormField,
};
use super::keys::{
    MeetsCreateModalKey, MeetsDeleteModalKey, MeetsEditModalKey, MeetsHubTableKey,
    MeetsRecordingsModalKey, MeetsRoomChromeKey,
};
use super::routes::{
    HubRouteTag, JoinPostRouteTag, RecordingsRouteTag, RoomCallRouteTag, RoomCreatePostRouteTag,
    RoomDeleteGetRouteTag, RoomDeletePostRouteTag, RoomEditGetRouteTag, RoomEditPostRouteTag,
    RoomEnterRouteTag, RoomLeaveRouteTag, RoomLobbyRouteTag, RoomLockRouteTag, RoomRouteTag,
    RoomStopRouteTag,
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
        RoomEditModalIdx: MeetsEditModalPageTag => MeetsEditModalPage,
        RoomDeleteModalIdx: MeetsConfirmDeletePageTag => MeetsConfirmDeletePage,
        LobbyIdx: MeetsLobbyPageTag => MeetsLobbyPage,
        CallIdx: MeetsCallPageTag => MeetsCallPage,
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

fn room_crumbs_with(code: &str, suffix: Option<&str>) -> Markup {
    let list_url = HubRouteTag.url();
    let room_url = RoomRouteTag::new(code.to_string()).url();
    let mut crumbs = vec![
        Crumb {
            label: "Meets",
            href: Some(&list_url),
        },
        Crumb {
            label: code,
            href: Some(&room_url),
        },
    ];
    if let Some(label) = suffix {
        crumbs.push(Crumb { label, href: None });
    }
    breadcrumbs(&crumbs)
}

fn room_crumbs(code: &str) -> Markup {
    room_crumbs_with(code, None)
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
    pub joined_at: String,
    pub duration: String,
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
    pub can_manage: bool,
    pub authenticated: bool,
    pub scheduled_start_label: String,
    pub started_at_label: String,
    pub ended_at_label: String,
    pub has_joined: bool,
    pub roster: Vec<RosterEntry>,
}

fn display_or_dash(value: &str) -> &str {
    if value.is_empty() { "—" } else { value }
}

impl MeetsRoomPage {
    pub fn render_chrome(&self) -> Markup {
        let lock_label = if self.joining_allowed {
            "Lock joining"
        } else {
            "Unlock joining"
        };
        let actions = html! {
            @if self.live {
                span class="text-sm opacity-70" { "Live" }
            } @else if !self.ended_at_label.is_empty() {
                span class="text-sm opacity-70" { "Ended" }
            } @else if !self.scheduled_start_label.is_empty() {
                span class="text-sm opacity-70" {
                    "Starts " (self.scheduled_start_label)
                }
            }
            @if self.can_manage && !self.live && self.ended_at_label.is_empty() {
                (button_link_route(
                    RoomLobbyRouteTag::new(self.code.clone()),
                    "Start meeting",
                    "btn-primary",
                ))
            }
            @if (self.joining_allowed || self.can_manage) && self.ended_at_label.is_empty() {
                @if self.live && self.has_joined {
                    (button_link_route(
                        RoomCallRouteTag::new(self.code.clone()),
                        "Enter call",
                        "btn-primary",
                    ))
                } @else if !self.is_host || !self.has_joined {
                    (button_link_route(
                        RoomLobbyRouteTag::new(self.code.clone()),
                        "Join meeting",
                        "btn-primary",
                    ))
                }
            }
            @if self.can_manage && self.live {
                (button_post_route(
                    RoomStopRouteTag::new(self.code.clone()),
                    "Stop meeting",
                    "btn-outline btn-error",
                ))
            }
            @if self.can_manage {
                (button_post_fragment_route_swap(
                    RoomLockRouteTag::new(self.code.clone()),
                    lock_label,
                    "btn-outline",
                    "outerHTML",
                ))
            }
            (button_post_route(
                RoomLeaveRouteTag::new(self.code.clone()),
                "Leave",
                "btn-ghost",
            ))
            @if self.can_manage {
                (button_modal_form(ButtonModalForm {
                    name: "p_meets.RoomEditForm",
                    href: &RoomEditGetRouteTag::new(self.code.clone()).url(),
                    form_post_url: &RoomEditPostRouteTag::new(self.code.clone()).path(),
                    modal_uid: MeetsEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
                (button_modal_route(RecordingsRouteTag::new(self.code.clone()), "Recordings", "btn-outline"))
            }
        };
        html! {
            div id=(MeetsRoomChromeKey::ID) class="mb-4" {
                (detail_header(DetailHeader {
                    title: &format!("Meeting {}", self.code),
                    actions,
                }))
            }
        }
    }

    pub fn render_details(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (label(
                        "Scheduled start",
                        field_text(FieldText {
                            value: display_or_dash(&self.scheduled_start_label),
                            classes: "",
                        }),
                    ))
                    (label(
                        "Started",
                        field_text(FieldText {
                            value: display_or_dash(&self.started_at_label),
                            classes: "",
                        }),
                    ))
                    (label(
                        "Ended",
                        field_text(FieldText {
                            value: display_or_dash(&self.ended_at_label),
                            classes: "",
                        }),
                    ))
                    div class="mt-4" {
                        h4 class="font-semibold mb-2" { "Participants" }
                        @if self.roster.is_empty() {
                            p class="text-sm opacity-70" { "No participants yet." }
                        } @else {
                            ul class="flex flex-col gap-2" {
                                @for p in &self.roster {
                                    li class="text-sm border border-base-300 rounded p-3" {
                                        div class="font-medium" {
                                            (p.name)
                                            span class="opacity-60 ml-2 font-normal" {
                                                "(" (p.user_type) ")"
                                            }
                                        }
                                        div class="opacity-70 mt-1" {
                                            "Joined " (display_or_dash(&p.joined_at))
                                            " · Duration " (display_or_dash(&p.duration))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }))
            }))
        }
    }

    pub fn render_body(&self) -> Markup {
        html! {
            (self.render_chrome())
            (self.render_details())
        }
    }
}

#[derive(Clone)]
pub struct MeetsLobbyPage {
    pub code: String,
    pub is_host: bool,
    pub can_manage: bool,
    pub live: bool,
    pub relay_url: String,
    pub moq_jwt: String,
}

impl MeetsLobbyPage {
    fn body(&self) -> Markup {
        let title = if self.can_manage && !self.live {
            "Start meeting"
        } else {
            "Join meeting"
        };
        html! {
            div class="meets-lobby flex flex-col gap-4" {
                h2 class="text-lg font-semibold" { (title) }
                @if !self.can_manage && !self.live {
                    p class="text-sm text-warning" {
                        "Waiting for the host to start the meeting. Configure your devices below, then press Join when ready."
                    }
                } @else {
                    p class="text-sm opacity-70" {
                        "Choose your camera, microphone, and screen sharing settings, then join."
                    }
                }
                (meets_media_source(MeetsMediaSource {
                    room_code: &self.code,
                    preview_only: true,
                    show_screen_share: true,
                    relay_url: &self.relay_url,
                    moq_jwt: &self.moq_jwt,
                    joined_user_id: 0,
                }))
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_redirect(RoomEnterRouteTag::new(self.code.clone()))
                        .set("data-meets-enter-form", "true"),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Join meeting",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                        (button_link_route(
                            RoomRouteTag::new(self.code.clone()),
                            "Back",
                            "btn-ghost",
                        ))
                    },
                    ..Default::default()
                }))
            }
        }
    }
}

impl RenderAppPane for MeetsLobbyPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            meets_menu("rooms"),
            room_crumbs_with(&self.code, Some("Lobby")),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(room_crumbs_with(&self.code, Some("Lobby")), self.body())
    }
}

impl RenderTemplate for MeetsLobbyPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Meeting {} — Lariv", self.code),
            chrome,
            meets_menu("rooms"),
            room_crumbs_with(&self.code, Some("Lobby")),
            self.body(),
        )
    }
}

#[derive(Clone)]
pub struct MeetsCallPage {
    pub code: String,
    pub is_host: bool,
    pub can_manage: bool,
    pub relay_url: String,
    pub moq_jwt: String,
    pub joined_user_id: i64,
}

impl MeetsCallPage {
    fn body(&self) -> Markup {
        html! {
            div class="flex flex-col gap-4" {
                div class="flex flex-wrap items-center justify-between gap-2" {
                    h2 class="text-lg font-semibold" { "Meeting " (self.code) }
                    div class="flex flex-wrap items-center gap-2" {
                        @if self.can_manage {
                            (button_post_route(
                                RoomStopRouteTag::new(self.code.clone()),
                                "Stop meeting",
                                "btn-outline btn-error",
                            ))
                        }
                        (button_post_route(
                            RoomLeaveRouteTag::new(self.code.clone()),
                            "Leave",
                            "btn-ghost",
                        ))
                        (button_link_route(
                            RoomRouteTag::new(self.code.clone()),
                            "Details",
                            "btn-outline",
                        ))
                    }
                }
                (meets_media_source(MeetsMediaSource {
                    room_code: &self.code,
                    preview_only: false,
                    show_screen_share: true,
                    relay_url: &self.relay_url,
                    moq_jwt: &self.moq_jwt,
                    joined_user_id: self.joined_user_id,
                }))
                (meets_video_stage(MeetsVideoStage {
                    room_code: &self.code,
                    relay_url: &self.relay_url,
                    moq_jwt: &self.moq_jwt,
                    joined_user_id: self.joined_user_id,
                }))
            }
        }
    }
}

impl RenderAppPane for MeetsCallPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            meets_menu("rooms"),
            room_crumbs_with(&self.code, Some("Call")),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(room_crumbs_with(&self.code, Some("Call")), self.body())
    }
}

impl RenderTemplate for MeetsCallPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Meeting {} — Lariv", self.code),
            chrome,
            meets_menu("rooms"),
            room_crumbs_with(&self.code, Some("Call")),
            self.body(),
        )
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
    fn modal_body(&self) -> Markup {
        html! {
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

    pub fn access_denied() -> Markup {
        modal_keyed::<MeetsRecordingsModalKey>(
            "",
            html! {
                h3 class="text-lg font-semibold mb-2" { "Recordings unavailable" }
                p class="text-error" { "You do not have access to these recordings." }
            },
        )
    }
}

impl RenderTemplate for MeetsRecordingsPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<MeetsRecordingsModalKey>(
            "",
            html! {
                h3 class="text-lg font-semibold mb-4" { "Recordings for " (self.code) }
                (self.modal_body())
            },
        )
    }
}

#[derive(Generic, Clone)]
pub struct MeetsEditModalPage {
    pub code: String,
    pub form_name: String,
    pub anonymous_allowed: bool,
    pub joining_allowed: bool,
    pub start_at: String,
    pub error: String,
}

impl RenderTemplate for MeetsEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = RoomDeleteGetRouteTag::new(self.code.clone()).url();
        let form_name = if self.form_name.is_empty() {
            "p_meets.RoomEditForm"
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
        modal_keyed::<MeetsEditModalKey>(
            form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit meeting " (self.code) }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<MeetsEditModalKey>(&modal_edit_post_url(
                        RoomEditPostRouteTag::new(self.code.clone()),
                        form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: CreateRoomForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_meets.RoomDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: MeetsDeleteModalKey::ID,
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

#[derive(Generic, Clone)]
pub struct MeetsConfirmDeletePage {
    pub code: String,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for MeetsConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", MeetsDeleteModalKey::ID);
        let post_url = RoomDeletePostRouteTag::new(self.code.clone()).url();
        modal(crate::components::Modal {
            uid: MeetsDeleteModalKey::ID,
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
