//! Maud page templates for auth, user/role CRUD, and self-profile views.
use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    ButtonLink, ButtonModalForm, ButtonPost, ButtonSubmit, Crumb, DataTable, DataTableDisplay,
    DeleteConfirmation, FieldPhone, FieldSubtitle, FieldText, FieldTitle, FilterClauseView,
    FormOpts, InputForeignKey, LayoutMain, LayoutSidebar, ListFilterBuilder, ObjectList,
    PaginationPage, RenderSlot, ShellAuth, ShellChrome, ShellScaffold, SidebarMenu,
    SidebarMenuItem, SidebarNavLink, SlotCapability, SlotCtx, SlotOf, SlotRegistrar, SwapKey,
    TableButtonFilter, TableColumnHeader, TableListContent, TablePagination, TableRow,
    TopbarItemsSlotTag, breadcrumbs, button_fk_select, button_link, button_modal_form, button_post,
    button_submit, column_sort_url, container_column, container_row, data_table,
    delete_confirmation, detail, field_phone, field_subtitle, field_text, field_title,
    filter_error_banner, form, form_hx_boost_post_main, form_hx_get_picker_route,
    form_hx_get_route, form_hx_post_main, form_hx_post_selector, form_hx_post_url,
    hx_nav_app_layout, input_foreign_key, label, layout_main, layout_sidebar, list_filter_builder,
    modal, modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select, shell_auth,
    shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane, sort_indicator,
    table_button_columns, table_button_filter, table_create_button, table_list_content,
    table_pagination, table_pagination_picker,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, csrf_hidden_field};
use lariv_core::http::{AppPaneGet, ProvideRequestCaps, RouteUrl};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_core::web::{modal_create_post_query, modal_edit_post_url};

use super::forms::{
    LoginForm, PasswordForm, SelfEditForm, SelfEditFormField, UserForm, UserFormField,
};
use super::keys::{
    SelfEditModalKey, UserCreateModalKey, UserDeleteModalKey, UserEditModalKey, UserSelectModalKey,
    UserSelectTableKey, UserTableKey,
};
use super::routes::{
    UsersChangePasswordGetRouteTag, UsersChangePasswordPostRouteTag, UsersCreatePostRouteTag,
    UsersDeleteGetRouteTag, UsersDeletePostRouteTag, UsersDetailRouteTag, UsersEditGetRouteTag,
    UsersEditPostRouteTag, UsersEffectiveUserGetRouteTag, UsersEffectiveUserPostRouteTag,
    UsersListRouteTag, UsersLoginGetRouteTag, UsersLoginPostRouteTag, UsersLogoutGetRouteTag,
    UsersLogoutPostRouteTag, UsersSelectRouteTag, UsersSelfChangePasswordGetRouteTag,
    UsersSelfChangePasswordPostRouteTag, UsersSelfEditGetRouteTag, UsersSelfEditPostRouteTag,
    UsersSelfRouteTag,
};

define_register_items! {
    plugin: UsersTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        LoginIdx: UsersLoginPageTag => LoginPage,
        UnauthIdx: UsersUnauthenticatedPageTag => UnauthenticatedPage,
        SelfDetailIdx: UsersSelfDetailPageTag => SelfDetailPage,
        SelfEditModalIdx: UsersSelfEditModalPageTag => SelfEditModalPage,
        ChangePasswordIdx: UsersChangePasswordPageTag => ChangePasswordPage,
        UserListIdx: UsersUserListPageTag => UserListPage,
        UserEditModalIdx: UsersUserEditModalPageTag => UserEditModalPage,
        UserCreateModalIdx: UsersUserCreateModalPageTag => UserCreateModalPage,
        UserDetailIdx: UsersUserDetailPageTag => UserDetailPage,
        ConfirmDeleteIdx: UsersConfirmDeletePageTag => ConfirmDeletePage,
        UserSelectIdx: UsersUserSelectPageTag => UserSelectPage,
    ]
}

// Identity tag for the users plugin topbar nav slot contributor.
pub struct UsersTopbarNavTag;

// Topbar nav — kept for tests / optional registration; not registered by default
//.
#[derive(Default)]
pub struct UsersTopbarNav;

impl RenderSlot for UsersTopbarNav {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        users_nav()
    }
}

fn users_nav_link<R: AppPaneGet + RouteUrl + Copy + Default>(label: &str) -> Markup {
    let route = R::default();
    users_nav_href(&route.url(), label)
}

fn users_nav_href(href: &str, label: &str) -> Markup {
    html! {
        (PreEscaped(format!(
            r#"<a class="btn btn-ghost btn-sm" href="{href}"{hx}>{label}</a>"#,
            href = href,
            hx = lariv_core::components::swap::hx_nav_app_layout_for_url(href).as_string(),
            label = label,
        )))
    }
}

#[derive(Default)]
pub struct UsersUserDropdown;

impl RenderSlot for UsersUserDropdown {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        let name = ctx.name.as_deref().unwrap_or("");
        let avatar = name
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".into());
        let user_ok = ctx.name.is_some();
        html! {
            (PreEscaped(
                r##"<details class="dropdown dropdown-end" @click.outside="$el.removeAttribute('open')">"##,
            ))
            summary class="btn btn-sm btn-circle avatar placeholder" {
                div class="rounded-full w-10" {
                    span class="text-xl" { (avatar) }
                }
            }
            div class="card w-64 my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-50 bg-base-100 p-4" {
                div class="flex flex-col gap-1" {
                    div class="font-bold text-lg" { (name) }
                }
                @if user_ok {
                    div class="flex flex-col gap-1 mt-2 pt-2 border-t border-base-300" {
                        (PreEscaped(format!(
                            r##"<a class="btn justify-start w-full" href="{self_url}"{}>My Account</a>"##,
                            hx_nav_app_layout(UsersSelfRouteTag).as_string(),
                            self_url = UsersSelfRouteTag.url(),
                        )))
                        (button_post(ButtonPost {
                            label: "Logout",
                            action: &UsersLogoutPostRouteTag.url(),
                            classes: "btn btn-error justify-start w-full",
                            icon_name: Some("arrow-right-start-on-rectangle"),
                            ..Default::default()
                        }))
                    }
                }
            }
            (PreEscaped("</details>"))
        }
    }
}

/// Top-bar control that lets a superuser choose who the request acts as.
#[derive(Default)]
pub struct EffectiveUserSelector;

const EFFECTIVE_USER_FIELD: &str = "effective_user";

impl RenderSlot for EffectiveUserSelector {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        let show = crate::role_authorization::actual_auth()
            .is_some_and(|auth| crate::roles::Superuser::matches(&auth.role));
        if !show {
            return Markup::default();
        }
        let effective = crate::role_authorization::current_auth();
        let value = effective
            .as_ref()
            .map(|auth| auth.user.id.to_string())
            .unwrap_or_default();
        let display = ctx
            .name
            .clone()
            .filter(|name| !name.is_empty())
            .or_else(|| effective.map(|auth| auth.user.name))
            .unwrap_or_default();
        let action = UsersEffectiveUserPostRouteTag.path();
        let attrs = form_hx_boost_post_main(&action);
        let on_select = "const d=$event.detail; if(!d||String(d.name)!=='effective_user') return; const form=$el.closest('form'); if(!form) return; $nextTick(()=>{ if(window.htmx) window.htmx.trigger(form,'submit'); else form.requestSubmit(); })";
        html! {
            (PreEscaped(format!(
                r##"<form class="effective-user-selector"{}>"##,
                attrs.as_string()
            )))
            (csrf_hidden_field(&CsrfToken::current()))
            div "@fk-select.window"=(on_select) {
                (input_foreign_key(InputForeignKey {
                    name: EFFECTIVE_USER_FIELD,
                    value: &value,
                    display: &display,
                    placeholder: "Switch user",
                    url: &UsersEffectiveUserGetRouteTag.path(),
                    uid: "effective-user",
                    classes: "max-w-56",
                    ..Default::default()
                }))
            }
            (PreEscaped("</form>"))
        }
    }
}

fn users_nav() -> Markup {
    html! {
        (users_nav_href(lariv_core::components::nav_origin::DASHBOARD_URL, "Apps"))
        (users_nav_link::<UsersSelfRouteTag>("Profile"))
        (lariv_core::components::authorized_role(
            &crate::role_authorization::roles_for::<super::routes::UsersAdmin>(),
            html! {
                (users_nav_link::<UsersListRouteTag>("Users"))
            },
        ))
        (PreEscaped(format!(
            r#"<a class="btn btn-ghost btn-sm" href="{href}"{hx}>Logout</a>"#,
            href = UsersLogoutGetRouteTag.url(),
            hx = hx_nav_app_layout(UsersLogoutGetRouteTag).as_string(),
        )))
    }
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

/// `#app-layout` fragment (sidebar + main) for fine-grained HTMX swaps.
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

/// `<main id="main-content">` fragment for in-scaffold sidebar menu navigation.
fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn users_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Users",
        href: None,
    }])
}

fn user_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = UsersListRouteTag.url();
    let detail_url = UsersDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Users",
                href: Some(&list_url),
            },
            Crumb {
                label: "All Users",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Users",
                href: Some(&list_url),
            },
            Crumb {
                label: "All Users",
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

fn self_crumbs(name: &str, action: Option<&str>) -> Markup {
    let detail_url = UsersSelfRouteTag.url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "My account",
                href: Some(&detail_url),
            },
            Crumb {
                label: "My Profile",
                href: Some(&detail_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "My account",
                href: Some(&detail_url),
            },
            Crumb {
                label: "My Profile",
                href: Some(&detail_url),
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

fn user_menu(current_path: &str) -> Markup {
    let users_url = UsersListRouteTag.url();
    let links = [SidebarNavLink {
        key: "users",
        title: "All Users",
        url: &users_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "Users",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn user_detail_menu(
    user_id: i64,
    user_name: &str,
    active: &str,
    show_change_password: bool,
) -> Markup {
    let title = format!("User: {user_name}");
    let detail_url = UsersDetailRouteTag::new(user_id).url();
    let pw_url = UsersChangePasswordGetRouteTag::new(user_id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "User Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            @if show_change_password {
                (sidebar_menu_item_pane(SidebarMenuItem {
                    title: "Change Password",
                    url: &pw_url,
                    active: active == "password",
                    ..Default::default()
                }))
            }
        },
    })
}

fn user_self_menu(user_name: &str, active: &str) -> Markup {
    let title = format!("My account: {user_name}");
    let detail_url = UsersSelfRouteTag.url();
    let pw_url = UsersSelfChangePasswordGetRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "My Profile",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Change Password",
                url: &pw_url,
                active: active == "password",
                ..Default::default()
            }))
        },
    })
}

fn filtered_data_table<K: SwapKey>(
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
        (lariv_core::components::table::table_grid_content(headers, rows))
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

fn render_picker_pagination<M: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, false);
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
    table_pagination_picker(TablePagination {
        pages: &pages,
        hx_target: M::SELECTOR,
    })
}

#[derive(Generic)]
pub struct LoginPage {
    pub error: String,
}

impl LoginPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: "Login",
                        classes: "",
                    }))
                    (form(&CsrfToken::current(), FormOpts {
                        attrs: form_hx_post_main(UsersLoginPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: LoginForm::render_inputs(&FormCtx::form::<LoginForm>(CsrfToken::current())),
                        actions: html! {
                            (container_column(
                                "w-full gap-2",
                                html! {
                                    (button_submit(ButtonSubmit {
                                        label: "Login",
                                        classes: "w-full",
                                        ..Default::default()
                                    }))
                                },
                            ))
                        },
                        ..Default::default()
                    }))
                },
            ))
        }
    }
}

/// Auth card body wrapped as `#app-layout` for HTMX swaps.
fn auth_pane(body: Markup) -> lariv_core::components::AppLayoutHtml {
    use lariv_core::components::app_layout_pane;
    app_layout_pane(body)
}

fn auth_main(body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: Markup::default(),
        content: body,
    })
}

impl lariv_core::template::RenderAppPane for LoginPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for LoginPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct UnauthenticatedPage {}

impl UnauthenticatedPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "w-80 items-center text-center",
                html! {
                    (field_title(FieldTitle {
                        value: "Welcome",
                        classes: "",
                    }))
                    (field_subtitle(FieldSubtitle {
                        value: "Please log in to continue.",
                        classes: "",
                    }))
                    (container_column(
                        "w-full mt-4 gap-2",
                        html! {
                            (button_link(ButtonLink {
                                label: "Login",
                                href: &UsersLoginGetRouteTag.url(),
                                classes: "btn btn-primary text-white w-full",
                                ..Default::default()
                            }))
                        },
                    ))
                },
            ))
        }
    }
}

impl lariv_core::template::RenderAppPane for UnauthenticatedPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for UnauthenticatedPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_auth(ShellAuth {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct SelfDetailPage {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    /// Set only when the viewer is a superuser.
    pub last_login_time: Option<String>,
}

impl SelfDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = UsersSelfEditGetRouteTag.url();
        let edit_post = UsersSelfEditPostRouteTag.path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (field_subtitle(FieldSubtitle {
                        value: &self.email,
                        classes: "",
                    }))
                    (label(
                        "Phone",
                        field_text(FieldText {
                            value: &self.phone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Timezone",
                        field_text(FieldText {
                            value: &self.timezone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
                    @if let Some(last_login_time) = &self.last_login_time {
                        (label(
                            "Last login",
                            field_text(FieldText {
                                value: last_login_time,
                                classes: "",
                            }),
                        ))
                    }
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_users.SelfEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: SelfEditModalKey::ID,
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

impl lariv_core::template::RenderAppPane for SelfDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = self_crumbs(&self.name, None);
        scaffold_pane(
            user_self_menu(&self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self_crumbs(&self.name, None), self.pane_body())
    }
}

impl RenderTemplate for SelfDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = self_crumbs(&self.name, None);
        app_scaffold(
            "Profile — Lariv",
            chrome,
            user_self_menu(&self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct SelfEditModalPage {
    pub form_name: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub error: String,
}

impl RenderTemplate for SelfEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<SelfEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit my profile" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<SelfEditModalKey>(&modal_edit_post_url(
                        UsersSelfEditPostRouteTag,
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: SelfEditForm::render_inputs(
                        &FormCtx::form::<SelfEditForm>(CsrfToken::current())
                            .value(SelfEditFormField::Name, self.name.as_str())
                            .value(SelfEditFormField::Email, self.email.as_str())
                            .value(SelfEditFormField::Phone, self.phone.as_str())
                            .value(SelfEditFormField::Timezone, self.timezone.as_str())
                            .choices(
                                SelfEditFormField::Timezone,
                                lariv_core::datetime::timezone_choices(),
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct ChangePasswordPage {
    pub user_id: i64,
    pub user_name: String,
    pub action: String,
    pub error: String,
    pub is_self: bool,
}

impl ChangePasswordPage {
    fn sidebar_and_copy(&self) -> (Markup, &'static str, &'static str) {
        if self.is_self {
            (
                user_self_menu(&self.user_name, "password"),
                "Change Password",
                "Update your password",
            )
        } else {
            (
                user_detail_menu(self.user_id, &self.user_name, "password", true),
                "Change Password",
                "Update user password",
            )
        }
    }

    fn crumbs(&self) -> Markup {
        if self.is_self {
            self_crumbs(&self.user_name, Some("Change Password"))
        } else {
            user_crumbs(self.user_id, &self.user_name, Some("Change Password"))
        }
    }

    fn pane_body(&self, title: &str, subtitle: &str) -> Markup {
        form(
            &CsrfToken::current(),
            FormOpts {
                title,
                subtitle,
                attrs: if self.is_self {
                    form_hx_post_main(UsersSelfChangePasswordPostRouteTag)
                } else {
                    form_hx_post_main(UsersChangePasswordPostRouteTag::new(self.user_id))
                },
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: PasswordForm::render_inputs(&FormCtx::form::<PasswordForm>(
                    CsrfToken::current(),
                )),
                actions: button_submit(ButtonSubmit {
                    label: "Change Password",
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
    }
}

impl lariv_core::template::RenderAppPane for ChangePasswordPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let (sidebar, title, subtitle) = self.sidebar_and_copy();
        scaffold_pane(sidebar, self.crumbs(), self.pane_body(title, subtitle))
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let (_sidebar, title, subtitle) = self.sidebar_and_copy();
        scaffold_main(self.crumbs(), self.pane_body(title, subtitle))
    }
}

impl RenderTemplate for ChangePasswordPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let (sidebar, title, subtitle) = self.sidebar_and_copy();
        app_scaffold(
            &format!("{title} — Lariv"),
            chrome,
            sidebar,
            self.crumbs(),
            self.pane_body(title, subtitle),
        )
    }
}

#[derive(Clone)]
pub struct UserRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub last_login_time: String,
}

pub struct UserListPage {
    pub users: ObjectList<UserRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub show_last_login: bool,
}

impl UserListPage {
    /// Fine-grained table fragment for HTMX swaps targeting [`UserTableKey`].
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let email_sort = column_sort_url(&self.path_and_query, "Email", &self.sort);
        let phone_sort = column_sort_url(&self.path_and_query, "Phone", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let email_label = format!("Email{}", sort_indicator(&self.sort, "Email"));
        let phone_label = format!("Phone{}", sort_indicator(&self.sort, "Phone"));
        let last_login_sort = column_sort_url(&self.path_and_query, "Last login", &self.sort);
        let last_login_label = format!("Last login{}", sort_indicator(&self.sort, "Last login"));
        let mut headers = vec![
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Email",
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Phone",
                label: &phone_label,
                sort_url: Some(&phone_sort),
                push_url: true,
            },
        ];
        if self.show_last_login {
            headers.push(TableColumnHeader {
                key: "Last login",
                label: &last_login_label,
                sort_url: Some(&last_login_sort),
                push_url: true,
            });
        }
        let rows: Vec<TableRow> = self
            .users
            .items
            .iter()
            .map(|u| {
                let mut cells = vec![
                    field_text(FieldText {
                        value: &u.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &u.email,
                        classes: "",
                    }),
                    field_phone(FieldPhone {
                        value: &u.phone,
                        classes: "",
                    }),
                ];
                if self.show_last_login {
                    cells.push(field_text(FieldText {
                        value: &u.last_login_time,
                        classes: "",
                    }));
                }
                TableRow {
                    attrs: row_attr_navigate_route(UsersDetailRouteTag::new(u.id)),
                    cells,
                    key: String::new(),
                    children: Vec::new(),
                }
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route::<UserTableKey, UsersListRouteTag>(UsersListRouteTag),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (table_create_button::<UserTableKey, UserCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination::<UserTableKey>(
            &self.path_and_query,
            self.users.number,
            self.users.num_pages,
            true,
        );
        filtered_data_table::<UserTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl lariv_core::template::RenderAppPane for UserListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            user_menu(&self.path_and_query),
            users_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(users_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for UserListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Users — Lariv",
            chrome,
            user_menu(&self.path_and_query),
            users_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct UserDetailPage {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    pub show_change_password: bool,
    /// Set only when the viewer is a superuser.
    pub last_login_time: Option<String>,
}

impl UserDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = UsersEditGetRouteTag::new(self.id).url();
        let edit_post = UsersEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (field_subtitle(FieldSubtitle {
                        value: &self.email,
                        classes: "",
                    }))
                    (label(
                        "Phone",
                        field_text(FieldText {
                            value: &self.phone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Timezone",
                        field_text(FieldText {
                            value: &self.timezone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
                    @if let Some(last_login_time) = &self.last_login_time {
                        (label(
                            "Last login",
                            field_text(FieldText {
                                value: last_login_time,
                                classes: "",
                            }),
                        ))
                    }
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_users.UserEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: UserEditModalKey::ID,
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

impl lariv_core::template::RenderAppPane for UserDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = user_crumbs(self.id, &self.name, None);
        scaffold_pane(
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(user_crumbs(self.id, &self.name, None), self.pane_body())
    }
}
impl RenderTemplate for UserDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = user_crumbs(self.id, &self.name, None);
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            crumbs,
            self.pane_body(),
        )
    }
}

/// Edit user modal. Create uses [`UserCreateModalPage`].
#[derive(Generic)]
pub struct UserEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    pub error: String,
}

impl RenderTemplate for UserEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = UsersDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<UserForm>(CsrfToken::current())
            .value(UserFormField::Name, self.name.as_str())
            .value(UserFormField::Email, self.email.as_str())
            .value(UserFormField::Phone, self.phone.as_str())
            .value(UserFormField::Timezone, self.timezone.as_str())
            .choices(
                UserFormField::Timezone,
                lariv_core::datetime::timezone_choices(),
            )
            .value(UserFormField::Role, self.role.as_str());
        modal_keyed::<UserEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit user" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<UserEditModalKey>(&modal_edit_post_url(
                        UsersEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: UserForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_users.UserDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: UserDeleteModalKey::ID,
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
pub struct UserCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    pub error: String,
}

impl RenderTemplate for UserCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_users.UserCreateForm"
        } else {
            self.form_name.as_str()
        };
        let ctx = FormCtx::form::<UserForm>(CsrfToken::current())
            .value(UserFormField::Name, self.name.as_str())
            .value(UserFormField::Email, self.email.as_str())
            .value(UserFormField::Phone, self.phone.as_str())
            .value(UserFormField::Timezone, self.timezone.as_str())
            .choices(
                UserFormField::Timezone,
                lariv_core::datetime::timezone_choices(),
            )
            .value(UserFormField::Role, self.role.as_str());
        modal_keyed::<UserCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create User",
                    subtitle: "Create a new user",
                    classes: "@container",
                    attrs: lariv_core::components::swap::form_hx_post_for_url::<UserCreateModalKey>(
                        &modal_create_post_query(
                            UsersCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                            &self.target_input,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: UserForm::render_inputs(&ctx),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save User",
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
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", UserDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            UserDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = UsersDeletePostRouteTag::new(self.id).url();
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

pub struct UserSelectPage {
    pub users: ObjectList<UserRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub current_user_id: i64,
    pub current_user_name: String,
    pub page_size: u32,
}

impl RenderPickerSelect<UserSelectTableKey, UserSelectModalKey> for UserSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "UserID"
        } else {
            self.target_input.as_str()
        };
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
            .users
            .items
            .iter()
            .map(|u| TableRow {
                attrs: row_attr_select(target, &u.id.to_string(), &u.name),
                cells: vec![
                    field_text(FieldText {
                        value: &u.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &u.email,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &u.phone,
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
                    attrs: form_hx_get_picker_route::<
                        UserSelectTableKey,
                        UserSelectModalKey,
                        UsersSelectRouteTag,
                    >(UsersSelectRouteTag)
                        .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input.as_str()) {}
                    },
                }),
                ..Default::default()
            }))
            (button_fk_select(
                "Me",
                target,
                &self.current_user_id.to_string(),
                &self.current_user_name,
            ))
            (picker_create_button::<UserCreateModalKey>(
                &self.target_input,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_picker_pagination::<UserSelectModalKey>(
            &self.path_and_query,
            self.users.number,
            self.users.num_pages,
        );
        filtered_data_table::<UserSelectTableKey>(
            "Select User",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for UserSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

define_register_items! {
    plugin: UsersTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    wrapper: SlotOf;
    bounds: [];
    hook: SlotsHook;
    items: [
        UserDropdownIdx: UsersUserDropdownTag, TopbarItemsSlotTag => UsersUserDropdown,
        EffectiveUserIdx: EffectiveUserSelectorTag, TopbarItemsSlotTag => EffectiveUserSelector,
    ]
}

#[cfg(test)]
mod tests {
    use super::{UserDetailPage, UserListPage, UserRow};
    use lariv_core::components::ObjectList;
    use lariv_core::template::RenderAppPane;

    fn detail(last_login_time: Option<String>) -> String {
        UserDetailPage {
            id: 1,
            name: "Ada".into(),
            email: "ada@example.com".into(),
            phone: "1".into(),
            timezone: "UTC".into(),
            role: "Employee".into(),
            show_change_password: false,
            last_login_time,
        }
        .render_main()
        .into_markup()
        .into_string()
    }

    fn list(show_last_login: bool) -> String {
        UserListPage {
            users: ObjectList::from_page(
                vec![UserRow {
                    id: 1,
                    name: "Ada".into(),
                    email: "ada@example.com".into(),
                    phone: "1".into(),
                    last_login_time: "01/01/2026 10:00:00".into(),
                }],
                1,
                12,
                1,
            ),
            fields: Vec::new(),
            filters: Vec::new(),
            filter_errors: Vec::new(),
            sort: String::new(),
            path_and_query: "/users/".into(),
            page_size: 12,
            show_last_login,
        }
        .render_table()
        .into_string()
    }

    #[test]
    fn last_login_is_visible_only_when_the_viewer_may_see_it() {
        let hidden = detail(None);
        assert!(!hidden.contains("Last login"), "{hidden}");

        let shown = detail(Some("01/01/2026 10:00:00".into()));
        assert!(shown.contains("Last login"), "{shown}");
        assert!(shown.contains("01/01/2026 10:00:00"), "{shown}");

        let list_hidden = list(false);
        assert!(!list_hidden.contains("Last login"), "{list_hidden}");
        assert!(
            !list_hidden.contains("01/01/2026 10:00:00"),
            "{list_hidden}"
        );

        let list_shown = list(true);
        assert!(list_shown.contains("Last login"), "{list_shown}");
        assert!(list_shown.contains("01/01/2026 10:00:00"), "{list_shown}");
    }

    #[test]
    fn effective_user_selector_follows_the_actual_user() {
        use chrono::Utc;

        use super::EffectiveUserSelector;
        use crate::entities::user::Model as User;
        use crate::role_authorization::{RoleAuthorizationRegistry, with_principal};
        use crate::roles::Superuser;
        use crate::state::AuthContext;
        use lariv_core::components::{RenderSlot, SlotCtx};

        fn auth(role: &str) -> AuthContext {
            AuthContext {
                user: User {
                    id: 1,
                    created_at: Some(Utc::now()),
                    updated_at: Some(Utc::now()),
                    name: "Ada".into(),
                    email: "ada@example.com".into(),
                    phone: "1".into(),
                    role: role.into(),
                    password_hash: Some(vec![]),
                    password_salt: Some(vec![]),
                    timezone: "UTC".into(),
                    last_login_time: None,
                },
                role: role.into(),
                timezone: "UTC".into(),
            }
        }

        let ctx = SlotCtx {
            name: Some("Bea".into()),
            role: Some("employee".into()),
        };
        let closed = EffectiveUserSelector.render_slot(&ctx).into_string();
        assert!(closed.is_empty(), "{closed}");

        let hidden = with_principal(auth("employee"), RoleAuthorizationRegistry::new(), || {
            EffectiveUserSelector.render_slot(&ctx).into_string()
        });
        assert!(hidden.is_empty(), "{hidden}");

        let shown = with_principal(
            auth(Superuser::NAME),
            RoleAuthorizationRegistry::new(),
            || EffectiveUserSelector.render_slot(&ctx).into_string(),
        );
        assert!(shown.contains("effective-user-selector"), "{shown}");
        assert!(shown.contains("data-fk-picker"), "{shown}");
        assert!(shown.contains("Bea"), "{shown}");
        assert!(shown.contains("/dashboard/users/effective-user"), "{shown}");
        assert!(shown.contains("effective_user"), "{shown}");
        assert!(!shown.contains("Loading"), "{shown}");
    }
}
