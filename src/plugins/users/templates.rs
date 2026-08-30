//! Maud page templates for auth, user/role CRUD, and self-profile views.
use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        ButtonClear, ButtonLink, ButtonModalForm, ButtonPost, ButtonSubmit, Crumb,
        DeleteConfirmation, FieldCheckbox, FieldPhone, FieldSubtitle, FieldText, FieldTitle,
        FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage, RenderSlot, ShellAuth,
        ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem, SidebarNavLink, SlotCapability,
        SlotCtx, SlotOf, SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, TopbarItemsSlotTag, breadcrumbs, button_clear, button_fk_select,
        button_link, button_modal_form, button_post, button_submit, column_sort_url,
        container_column, container_row, data_table_list_refresh, delete_confirmation, detail,
        field_checkbox, field_phone, field_subtitle, field_text, field_title, form,
        form_hx_get_picker_route, form_hx_get_route, form_hx_post_main, form_hx_post_selector,
        form_hx_post_url, hx_nav_app_layout, label, layout_main, layout_sidebar, modal,
        modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select, shell_auth,
        shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane,
        sort_indicator, table_button_filter, table_create_button, table_pagination,
        table_pagination_picker, with_list_filter_common,
    },
    html_form::{FormCtx, HtmlForm},
    http::{AppPaneGet, ProvideRequestCaps, RouteUrl},
    picker::{RenderPickerSelect, picker_create_button},
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_query, modal_edit_post_url},
};

use super::forms::{
    LoginForm, PasswordForm, RoleForm, RoleFormField, RoleNameFilterForm, RoleNameFilterFormField,
    SelfEditForm, SelfEditFormField, UserFilterForm, UserFilterFormField, UserForm, UserFormField,
    UserFormFlag, UserSelectFilterForm, UserSelectFilterFormField,
};
use super::keys::{
    RoleCreateModalKey, RoleDeleteModalKey, RoleEditModalKey, RoleSelectModalKey,
    RoleSelectTableKey, RoleTableKey, SelfEditModalKey, UserCreateModalKey, UserDeleteModalKey,
    UserEditModalKey, UserSelectModalKey, UserSelectTableKey, UserTableKey,
};
use super::routes::{
    UsersChangePasswordGetRouteTag, UsersChangePasswordPostRouteTag, UsersCreatePostRouteTag,
    UsersDeleteGetRouteTag, UsersDeletePostRouteTag, UsersDetailRouteTag, UsersEditGetRouteTag,
    UsersEditPostRouteTag, UsersListRouteTag, UsersLoginGetRouteTag, UsersLoginPostRouteTag,
    UsersLogoutGetRouteTag, UsersRolesCreatePostRouteTag, UsersRolesDeleteGetRouteTag,
    UsersRolesDeletePostRouteTag, UsersRolesDetailRouteTag, UsersRolesEditGetRouteTag,
    UsersRolesEditPostRouteTag, UsersRolesListRouteTag, UsersRolesSelectRouteTag,
    UsersSelectRouteTag, UsersSelfChangePasswordGetRouteTag, UsersSelfChangePasswordPostRouteTag,
    UsersSelfEditGetRouteTag, UsersSelfEditPostRouteTag, UsersSelfRouteTag,
};
use crate::plugins::dashboard::routes::DashboardAppsRouteTag;

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
        RoleListIdx: UsersRoleListPageTag => RoleListPage,
        RoleEditModalIdx: UsersRoleEditModalPageTag => RoleEditModalPage,
        RoleCreateModalIdx: UsersRoleCreateModalPageTag => RoleCreateModalPage,
        RoleDetailIdx: UsersRoleDetailPageTag => RoleDetailPage,
        RoleSelectIdx: UsersRoleSelectPageTag => RoleSelectPage,
    ]
}

// Identity tag for the users plugin topbar nav slot contributor.
pub struct UsersTopbarNavTag;

// Topbar nav — kept for tests / optional registration; not registered by default
//.
#[derive(Default)]
pub struct UsersTopbarNav;

impl RenderSlot for UsersTopbarNav {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        users_nav(ctx.is_staff)
    }
}

fn users_nav_link<R: AppPaneGet + RouteUrl + Copy + Default>(label: &str) -> Markup {
    let route = R::default();
    let href = route.url();
    html! {
        (PreEscaped(format!(
            r#"<a class="btn btn-ghost btn-sm" href="{href}"{hx}>{label}</a>"#,
            href = href,
            hx = hx_nav_app_layout(route).as_string(),
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
                            r##"<a class="btn justify-start w-full" href="/users/self/"{}>My Account</a>"##,
                            hx_nav_app_layout(UsersSelfRouteTag).as_string(),
                        )))
                        (button_post(ButtonPost {
                            label: "Logout",
                            action: "/users/logout/",
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

fn users_nav(is_staff: bool) -> Markup {
    html! {
        (users_nav_link::<DashboardAppsRouteTag>("Apps"))
        (users_nav_link::<UsersSelfRouteTag>("Profile"))
        @if is_staff {
            (users_nav_link::<UsersListRouteTag>("Users"))
            (users_nav_link::<UsersRolesListRouteTag>("Roles"))
        }
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
) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

/// `<main id="main-content">` fragment for in-scaffold sidebar menu navigation.
fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
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

fn roles_list_crumbs() -> Markup {
    let users_url = UsersListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Users",
            href: Some(&users_url),
        },
        Crumb {
            label: "Roles",
            href: None,
        },
    ])
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

fn role_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let users_url = UsersListRouteTag.url();
    let list_url = UsersRolesListRouteTag.url();
    let detail_url = UsersRolesDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Users",
                href: Some(&users_url),
            },
            Crumb {
                label: "Roles",
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
                href: Some(&users_url),
            },
            Crumb {
                label: "Roles",
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

/// Auth card body wrapped as `#app-layout` for HTMX swaps.
fn auth_pane(body: Markup) -> crate::components::AppLayoutHtml {
    use crate::components::app_layout_pane;
    app_layout_pane(body)
}

fn auth_main(body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: Markup::default(),
        content: body,
    })
}

fn user_menu(current_path: &str) -> Markup {
    let users_url = UsersListRouteTag.url();
    let roles_url = UsersRolesListRouteTag.url();
    let links = [
        SidebarNavLink {
            key: "users",
            title: "All Users",
            url: &users_url,
            icon_name: None,
            match_prefixes: &[],
        },
        SidebarNavLink {
            key: "roles",
            title: "Roles",
            url: &roles_url,
            icon_name: None,
            match_prefixes: &[],
        },
    ];
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

fn role_detail_menu(role_id: i64, role_name: &str, active: &str) -> Markup {
    let title = format!("Role: {role_name}");
    let detail_url = UsersRolesDetailRouteTag::new(role_id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Role Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
        },
    })
}

fn user_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + RouteUrl + Copy + Default>(
    name: &str,
    email: &str,
    phone: &str,
    page_size: u32,
) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: with_list_filter_common(
            UserFilterForm::render_inputs(
                &FormCtx::form::<UserFilterForm>()
                    .value(UserFilterFormField::Name, name)
                    .value(UserFilterFormField::Email, email)
                    .value(UserFilterFormField::Phone, phone),
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

fn role_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + RouteUrl + Copy + Default>(
    name: &str,
    page_size: u32,
) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: with_list_filter_common(
            RoleNameFilterForm::render_inputs(
                &FormCtx::form::<RoleNameFilterForm>().value(RoleNameFilterFormField::Name, name),
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
                    (form(FormOpts {
                        attrs: form_hx_post_main(UsersLoginPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: LoginForm::render_inputs(&FormCtx::form::<LoginForm>()),
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

impl crate::template::RenderAppPane for LoginPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
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

impl crate::template::RenderAppPane for UnauthenticatedPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
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
    pub is_superuser: bool,
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
                    @if self.is_superuser {
                        (label(
                            "Superuser",
                            field_checkbox(FieldCheckbox {
                                checked: self.is_superuser,
                                classes: "",
                            }),
                        ))
                    }
                    (label(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
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

impl crate::template::RenderAppPane for SelfDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = self_crumbs(&self.name, None);
        scaffold_pane(
            user_self_menu(&self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
                (form(FormOpts {
                    attrs: form_hx_post_url::<SelfEditModalKey>(&modal_edit_post_url(
                        UsersSelfEditPostRouteTag,
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: SelfEditForm::render_inputs(
                        &FormCtx::form::<SelfEditForm>()
                            .value(SelfEditFormField::Name, self.name.as_str())
                            .value(SelfEditFormField::Email, self.email.as_str())
                            .value(SelfEditFormField::Phone, self.phone.as_str())
                            .value(SelfEditFormField::Timezone, self.timezone.as_str())
                            .choices(
                                SelfEditFormField::Timezone,
                                crate::datetime::timezone_choices(),
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
        form(FormOpts {
            title,
            subtitle,
            attrs: if self.is_self {
                form_hx_post_main(UsersSelfChangePasswordPostRouteTag)
            } else {
                form_hx_post_main(UsersChangePasswordPostRouteTag::new(self.user_id))
            },
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: PasswordForm::render_inputs(&FormCtx::form::<PasswordForm>()),
            actions: button_submit(ButtonSubmit {
                label: "Change Password",
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for ChangePasswordPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let (sidebar, title, subtitle) = self.sidebar_and_copy();
        scaffold_pane(sidebar, self.crumbs(), self.pane_body(title, subtitle))
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
}

#[derive(Generic)]
pub struct UserListPage {
    pub users: ObjectList<UserRow>,
    pub filter_name: String,
    pub filter_email: String,
    pub filter_phone: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
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
        let headers = [
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
        let rows: Vec<TableRow> = self
            .users
            .items
            .iter()
            .map(|u| TableRow {
                attrs: row_attr_navigate_route(UsersDetailRouteTag::new(u.id)),
                cells: vec![
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
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: user_filter_form::<UserTableKey, UsersListRouteTag>(
                    &self.filter_name,
                    &self.filter_email,
                    &self.filter_phone,
                    self.page_size,
                ),
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
        data_table_list_refresh::<UserTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl crate::template::RenderAppPane for UserListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            user_menu(&self.path_and_query),
            users_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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
    pub user_is_superuser: bool,
    pub show_change_password: bool,
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
                        "Superuser",
                        field_checkbox(FieldCheckbox {
                            checked: self.user_is_superuser,
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

impl crate::template::RenderAppPane for UserDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = user_crumbs(self.id, &self.name, None);
        scaffold_pane(
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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

#[derive(Clone)]
pub struct RoleOption {
    pub id: i64,
    pub name: String,
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
    pub role_id: i64,
    pub role_display: String,
    pub is_superuser: bool,
    pub can_set_superuser: bool,
    pub error: String,
}

impl RenderTemplate for UserEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = UsersDeleteGetRouteTag::new(self.id).url();
        let role_id_s = if self.role_id == 0 {
            String::new()
        } else {
            self.role_id.to_string()
        };
        let ctx = FormCtx::form::<UserForm>()
            .value(UserFormField::Name, self.name.as_str())
            .value(UserFormField::Email, self.email.as_str())
            .value(UserFormField::Phone, self.phone.as_str())
            .value(UserFormField::Timezone, self.timezone.as_str())
            .choices(UserFormField::Timezone, crate::datetime::timezone_choices())
            .value(UserFormField::RoleId, role_id_s.as_str())
            .display(UserFormField::RoleId, self.role_display.as_str())
            .checked(UserFormField::IsSuperuser, self.is_superuser)
            .flag(UserFormFlag::CanSetSuperuser, self.can_set_superuser);
        modal_keyed::<UserEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit user" }
                (form(FormOpts {
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
    pub role_id: i64,
    pub role_display: String,
    pub is_superuser: bool,
    pub can_set_superuser: bool,
    pub error: String,
}

impl RenderTemplate for UserCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_users.UserCreateForm"
        } else {
            self.form_name.as_str()
        };
        let role_id_s = if self.role_id == 0 {
            String::new()
        } else {
            self.role_id.to_string()
        };
        let ctx = FormCtx::form::<UserForm>()
            .value(UserFormField::Name, self.name.as_str())
            .value(UserFormField::Email, self.email.as_str())
            .value(UserFormField::Phone, self.phone.as_str())
            .value(UserFormField::Timezone, self.timezone.as_str())
            .choices(UserFormField::Timezone, crate::datetime::timezone_choices())
            .value(UserFormField::RoleId, role_id_s.as_str())
            .display(UserFormField::RoleId, self.role_display.as_str())
            .checked(UserFormField::IsSuperuser, self.is_superuser)
            .flag(UserFormFlag::CanSetSuperuser, self.can_set_superuser);
        modal_keyed::<UserCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create User",
                subtitle: "Create a new user",
                classes: "@container",
                attrs: crate::components::swap::form_hx_post_for_url::<UserCreateModalKey>(
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
            }),
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
        let post_url = if self.modal_uid == RoleDeleteModalKey::ID {
            UsersRolesDeletePostRouteTag::new(self.id).url()
        } else {
            UsersDeletePostRouteTag::new(self.id).url()
        };
        modal(crate::components::Modal {
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

#[derive(Generic)]
pub struct UserSelectPage {
    pub users: ObjectList<UserRow>,
    pub filter_name: String,
    pub filter_email: String,
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
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_picker_route::<
                        UserSelectTableKey,
                        UserSelectModalKey,
                        UsersSelectRouteTag,
                    >(UsersSelectRouteTag)
                        .set("hx-push-url", "false"),
                    inputs: html! {
                        (with_list_filter_common(
                            UserSelectFilterForm::render_inputs(
                                &FormCtx::form::<UserSelectFilterForm>()
                                    .value(UserSelectFilterFormField::Name, self.filter_name.as_str())
                                    .value(UserSelectFilterFormField::Email, self.filter_email.as_str()),
                            ),
                            self.page_size,
                        ))
                        input type="hidden" name="target_input" value=(self.target_input.as_str()) {}
                    },
                    actions: html! {
                        (container_row(
                            "flex gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Apply",
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
        data_table_list_refresh::<UserSelectTableKey>(
            "Select User",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for UserSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct RoleListPage {
    pub roles: ObjectList<RoleOption>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RoleListPage {
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
            .roles
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_navigate_route(UsersRolesDetailRouteTag::new(r.id)),
                cells: vec![field_text(FieldText {
                    value: &r.name,
                    classes: "",
                })],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: role_filter_form::<RoleTableKey, UsersRolesListRouteTag>(&self.filter_name, self.page_size),
                ..Default::default()
            }))
            (table_create_button::<RoleTableKey, RoleCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination::<RoleTableKey>(
            &self.path_and_query,
            self.roles.number,
            self.roles.num_pages,
            true,
        );
        data_table_list_refresh::<RoleTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl crate::template::RenderAppPane for RoleListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            user_menu(&self.path_and_query),
            roles_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(roles_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for RoleListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Roles — Lariv",
            chrome,
            user_menu(&self.path_and_query),
            roles_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct RoleDetailPage {
    pub id: i64,
    pub name: String,
}

impl RoleDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = UsersRolesEditGetRouteTag::new(self.id).url();
        let edit_post = UsersRolesEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_users.RoleEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: RoleEditModalKey::ID,
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

impl crate::template::RenderAppPane for RoleDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = role_crumbs(self.id, &self.name, None);
        scaffold_pane(
            role_detail_menu(self.id, &self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(role_crumbs(self.id, &self.name, None), self.pane_body())
    }
}
impl RenderTemplate for RoleDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = role_crumbs(self.id, &self.name, None);
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            role_detail_menu(self.id, &self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
}

/// Edit role modal. Create uses [`RoleCreateModalPage`].
#[derive(Generic)]
pub struct RoleEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for RoleEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = UsersRolesDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<RoleEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit role" }
                (form(FormOpts {
                    attrs: form_hx_post_url::<RoleEditModalKey>(&modal_edit_post_url(
                        UsersRolesEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: RoleForm::render_inputs(
                        &FormCtx::form::<RoleForm>().value(RoleFormField::Name, self.name.as_str()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_users.RoleDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: RoleDeleteModalKey::ID,
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
pub struct RoleCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for RoleCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_users.RoleCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<RoleCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Role",
                subtitle: "Create a new role",
                attrs: crate::components::swap::form_hx_post_for_url::<RoleCreateModalKey>(
                    &modal_create_post_query(
                        UsersRolesCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                        &self.target_input,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: RoleForm::render_inputs(
                    &FormCtx::form::<RoleForm>().value(RoleFormField::Name, self.name.as_str()),
                ),
                actions: html! {
                    (container_row(
                        "flex justify-end gap-2 mt-2",
                        html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Role",
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
pub struct RoleSelectPage {
    pub roles: ObjectList<RoleOption>,
    pub filter_name: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RoleSelectPage {
    pub fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "RoleID"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .roles
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_select(target, &r.id.to_string(), &r.name),
                cells: vec![field_text(FieldText {
                    value: &r.name,
                    classes: "",
                })],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<RoleSelectTableKey, UsersRolesSelectRouteTag>(UsersRolesSelectRouteTag)
                        .set("hx-push-url", "false"),
                    inputs: with_list_filter_common(
                        RoleNameFilterForm::render_inputs(
                            &FormCtx::form::<RoleNameFilterForm>()
                                .value(RoleNameFilterFormField::Name, self.filter_name.as_str()),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (container_row(
                            "flex gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Apply",
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
                }),
                ..Default::default()
            }))
            (picker_create_button::<RoleCreateModalKey>(
                &self.target_input,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination::<RoleSelectTableKey>(
            &self.path_and_query,
            self.roles.number,
            self.roles.num_pages,
            false,
        );
        data_table_list_refresh::<RoleSelectTableKey>(
            "Select Role",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for RoleSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<RoleSelectModalKey>("", self.render_table())
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
    ]
}
