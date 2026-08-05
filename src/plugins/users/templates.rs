//! Maud page templates for auth, user/role CRUD, and self-profile views.
use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    components::{
        ButtonClear, ButtonLink, ButtonModalForm, ButtonSubmit, DeleteConfirmation, FieldCheckbox,
        FieldPhone, FieldSubtitle, FieldText, FieldTitle, FormOpts, LayoutSidebar, ObjectList,
        PaginationPage, RenderSlot, ShellAuth, ShellChrome, ShellScaffold,
        SidebarMenu, SidebarMenuBack, SidebarMenuItem, SlotCapability, SlotRegistrar, SlotCtx, SwapKey,
        TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear, button_link,
        button_modal_form, button_submit, column_sort_url, container_column, container_row,
        data_table_list, delete_confirmation, detail, field_checkbox, field_phone, field_subtitle,
        field_text, field_title, form, form_hx_get_route, form_hx_post_main, form_hx_post_selector,
        hx_nav_app_layout, label_inline, layout_sidebar, modal, modal_keyed,
        pagination_pages, row_attr_navigate_route, row_attr_select, shell_auth, shell_scaffold,
        sidebar_menu,
        sidebar_menu_item, sort_indicator, table_button_filter, table_pagination,
    },
    capability::define_register_items,
    html_form::{FormCtx, HtmlForm},
    http::{ProvideRequestCaps, AppPaneGet, RouteUrl},
    picker::RenderPickerSelect,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::forms::{
    LoginForm, PasswordForm, RoleForm, RoleFormField, RoleNameFilterForm, RoleNameFilterFormField,
    SelfEditForm, SelfEditFormField, SignupForm, SignupFormField, UserFilterForm, UserFilterFormField, UserForm,
    UserFormField, UserSelectFilterForm, UserSelectFilterFormField,
};
use super::keys::{
    RoleCreateModalKey, RoleDeleteModalKey, RoleSelectModalKey, RoleSelectTableKey, RoleTableKey,
    UserDeleteModalKey, UserSelectModalKey, UserSelectTableKey, UserTableKey,
};
use super::routes::{
    UsersChangePasswordGetRouteTag, UsersChangePasswordPostRouteTag, UsersCreateGetRouteTag,
    UsersCreatePostRouteTag, UsersDeleteGetRouteTag, UsersDeletePostRouteTag,
    UsersDetailRouteTag, UsersEditGetRouteTag, UsersEditPostRouteTag, UsersListRouteTag,
    UsersLoginGetRouteTag, UsersLoginPostRouteTag, UsersRolesCreateGetRouteTag,
    UsersRolesCreatePostRouteTag, UsersRolesDeleteGetRouteTag, UsersRolesDeletePostRouteTag,
    UsersRolesDetailRouteTag, UsersRolesEditGetRouteTag, UsersRolesEditPostRouteTag,
    UsersRolesListRouteTag, UsersRolesSelectRouteTag, UsersSelectRouteTag,
    UsersSelfChangePasswordGetRouteTag, UsersSelfChangePasswordPostRouteTag,
    UsersSelfEditGetRouteTag, UsersSelfEditPostRouteTag, UsersSelfRouteTag,
    UsersSignupGetRouteTag, UsersSignupPostRouteTag, UsersLogoutGetRouteTag,
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
        SignupIdx: UsersSignupPageTag => SignupPage,
        UnauthIdx: UsersUnauthenticatedPageTag => UnauthenticatedPage,
        SelfDetailIdx: UsersSelfDetailPageTag => SelfDetailPage,
        SelfEditIdx: UsersSelfEditPageTag => SelfEditPage,
        ChangePasswordIdx: UsersChangePasswordPageTag => ChangePasswordPage,
        UserListIdx: UsersUserListPageTag => UserListPage,
        UserFormIdx: UsersUserFormPageTag => UserFormPage,
        UserDetailIdx: UsersUserDetailPageTag => UserDetailPage,
        ConfirmDeleteIdx: UsersConfirmDeletePageTag => ConfirmDeletePage,
        UserSelectIdx: UsersUserSelectPageTag => UserSelectPage,
        RoleListIdx: UsersRoleListPageTag => RoleListPage,
        RoleFormIdx: UsersRoleFormPageTag => RoleFormPage,
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

/// `#app-layout` fragment (sidebar + main) for fine-grained HTMX swaps.
fn scaffold_pane(sidebar: Markup, body: Markup) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        content: body,
    })
}

/// `<main id="main-content">` fragment for in-scaffold sidebar menu navigation.
fn scaffold_main(body: Markup) -> crate::components::MainContentHtml {
    use crate::components::layout::layout_main;
    layout_main(body)
}

/// Auth card body wrapped as `#app-layout` for HTMX swaps.
fn auth_pane(body: Markup) -> crate::components::AppLayoutHtml {
    use crate::components::app_layout_pane;
    app_layout_pane(body)
}

fn auth_main(body: Markup) -> crate::components::MainContentHtml {
    use crate::components::layout::layout_main;
    layout_main(body)
}

fn user_menu(_users_active: bool, _roles_active: bool) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "Users",
        back: Some(SidebarMenuBack {
            title: "Back to Home",
            url: &DashboardAppsRouteTag.url(),
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "All Users",
                url: &UsersListRouteTag.url(),
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Roles",
                url: &UsersRolesListRouteTag.url(),
                ..Default::default()
            }))
        },
    })
}

fn user_detail_menu(user_id: i64, user_name: &str, _active: &str, show_change_password: bool) -> Markup {
    let title = format!("User: {user_name}");
    let detail_url = UsersDetailRouteTag::new(user_id).url();
    let edit_url = UsersEditGetRouteTag::new(user_id).url();
    let pw_url = UsersChangePasswordGetRouteTag::new(user_id).url();
    let back_url = UsersListRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        back: Some(SidebarMenuBack {
            title: "Back to All Users",
            url: &back_url,
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "User Detail",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit User",
                url: &edit_url,
                ..Default::default()
            }))
            @if show_change_password {
                (sidebar_menu_item(SidebarMenuItem {
                    title: "Change Password",
                    url: &pw_url,
                    ..Default::default()
                }))
            }
        },
    })
}

fn user_self_menu(user_name: &str, _active: &str) -> Markup {
    let title = format!("My account: {user_name}");
    let back_url = DashboardAppsRouteTag.url();
    let detail_url = UsersSelfRouteTag.url();
    let edit_url = UsersSelfEditGetRouteTag.url();
    let pw_url = UsersSelfChangePasswordGetRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        back: Some(SidebarMenuBack {
            title: "Back to Home",
            url: &back_url,
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "My Profile",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit My Profile",
                url: &edit_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Change Password",
                url: &pw_url,
                ..Default::default()
            }))
        },
    })
}

fn role_detail_menu(role_id: i64, role_name: &str, _active: &str) -> Markup {
    let title = format!("Role: {role_name}");
    let detail_url = UsersRolesDetailRouteTag::new(role_id).url();
    let edit_url = UsersRolesEditGetRouteTag::new(role_id).url();
    let back_url = UsersRolesListRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        back: Some(SidebarMenuBack {
            title: "Back to All Roles",
            url: &back_url,
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Role Detail",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit Role",
                url: &edit_url,
                ..Default::default()
            }))
        },
    })
}

fn user_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + RouteUrl + Copy + Default>(
    name: &str,
    email: &str,
    phone: &str,
) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: UserFilterForm::render_inputs(
            &FormCtx::form::<UserFilterForm>()
                .value(UserFilterFormField::Name, name)
                .value(UserFilterFormField::Email, email)
                .value(UserFilterFormField::Phone, phone),
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

fn role_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + RouteUrl + Copy + Default>(name: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: RoleNameFilterForm::render_inputs(
            &FormCtx::form::<RoleNameFilterForm>().value(RoleNameFilterFormField::Name, name),
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
                            (button_submit(ButtonSubmit {
                                label: "Login",
                                classes: "w-full mb-4",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Don't have an account? Sign up",
                                href: &UsersSignupGetRouteTag.url(),
                                classes: "w-full",
                                ..Default::default()
                            }))
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
pub struct SignupPage {
    pub error: String,
}

impl SignupPage {
    fn body(&self) -> Markup {
        html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: "Create an Account",
                        classes: "",
                    }))
                    (form(FormOpts {
                        attrs: form_hx_post_main(UsersSignupPostRouteTag),
                        form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                        inputs: SignupForm::render_inputs(
                            &FormCtx::form::<SignupForm>()
                                .value(
                                    SignupFormField::Timezone,
                                    crate::datetime::DEFAULT_TIMEZONE,
                                )
                                .choices(
                                    SignupFormField::Timezone,
                                    crate::datetime::timezone_choices(),
                                ),
                        ),
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Sign Up",
                                classes: "w-full",
                                ..Default::default()
                            }))
                            (button_link(ButtonLink {
                                label: "Already have an account? Login",
                                href: &UsersLoginGetRouteTag.url(),
                                classes: "w-full",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    }))
                },
            ))
        }
    }
}

impl crate::template::RenderAppPane for SignupPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        auth_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        auth_main(self.body())
    }
}

impl RenderTemplate for SignupPage {
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
                        value: "Please log in or create an account to continue.",
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
                            (button_link(ButtonLink {
                                label: "Sign Up",
                                href: &UsersSignupGetRouteTag.url(),
                                classes: "btn btn-outline w-full",
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
                    (crate::components::label_inline_with_classes(
                        "Phone",
                        "mt-2",
                        field_text(FieldText {
                            value: &self.phone,
                            classes: "",
                        }),
                    ))
                    (label_inline(
                        "Timezone",
                        field_text(FieldText {
                            value: &self.timezone,
                            classes: "",
                        }),
                    ))
                    @if self.is_superuser {
                        (label_inline(
                            "Superuser",
                            field_checkbox(FieldCheckbox {
                                checked: self.is_superuser,
                                classes: "",
                            }),
                        ))
                    }
                    (label_inline(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for SelfDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(user_self_menu(&self.name, "detail"), self.pane_body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for SelfDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Profile — Lariv",
            chrome,
            user_self_menu(&self.name, "detail"),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct SelfEditPage {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub error: String,
}

impl SelfEditPage {
    fn pane_body(&self) -> Markup {
        form(FormOpts {
            title: "Edit My Profile",
            subtitle: "Update your account details",
            classes: "@container",
            attrs: form_hx_post_main(UsersSelfEditPostRouteTag),
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
            actions: button_submit(ButtonSubmit {
                label: "Save Profile",
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for SelfEditPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(user_self_menu(&self.name, "edit"), self.pane_body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for SelfEditPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit profile — Lariv",
            chrome,
            user_self_menu(&self.name, "edit"),
            self.pane_body(),
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
        scaffold_pane(sidebar, self.pane_body(title, subtitle))
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let (_sidebar, title, subtitle) = self.sidebar_and_copy();
        scaffold_main(self.pane_body(title, subtitle))
    }
}

impl RenderTemplate for ChangePasswordPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let (sidebar, title, subtitle) = self.sidebar_and_copy();
        app_scaffold(
            &format!("{title} — Lariv"),
            chrome,
            sidebar,
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
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: true,
            },
            TableColumnHeader {
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
                ),
                ..Default::default()
            }))
            (button_link(ButtonLink {
                href: &UsersCreateGetRouteTag.url(),
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<UserTableKey>(
            &self.path_and_query,
            self.users.number,
            self.users.num_pages,
            true,
        );
        data_table_list::<UserTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for UserListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(user_menu(true, false), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for UserListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Users — Lariv",
            chrome,
            user_menu(true, false),
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
                    (crate::components::label_inline_with_classes(
                        "Phone",
                        "mt-2",
                        field_text(FieldText {
                            value: &self.phone,
                            classes: "",
                        }),
                    ))
                    (label_inline(
                        "Timezone",
                        field_text(FieldText {
                            value: &self.timezone,
                            classes: "",
                        }),
                    ))
                    (label_inline(
                        "Superuser",
                        field_checkbox(FieldCheckbox {
                            checked: self.user_is_superuser,
                            classes: "",
                        }),
                    ))
                    (label_inline(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for UserDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.pane_body())
    }
}
impl RenderTemplate for UserDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            self.pane_body(),
        )
    }
}

#[derive(Clone)]
pub struct RoleOption {
    pub id: i64,
    pub name: String,
}

/// Create/edit user form. `id == 0` is create (full page, not a modal).
#[derive(Generic)]
pub struct UserFormPage {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role_id: i64,
    pub role_display: String,
    pub error: String,
    pub show_change_password: bool,
}

impl UserFormPage {
    fn menu(&self) -> Markup {
        if self.id == 0 {
            user_menu(true, false)
        } else {
            user_detail_menu(self.id, &self.name, "edit", self.show_change_password)
        }
    }

    fn pane_body(&self) -> Markup {
        let is_create = self.id == 0;
        let form_attrs = if is_create {
            form_hx_post_main(UsersCreatePostRouteTag)
        } else {
            form_hx_post_main(UsersEditPostRouteTag::new(self.id))
        };
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
            .display(UserFormField::RoleId, self.role_display.as_str());
        form(FormOpts {
            title: if is_create { "Create User" } else { "Edit User" },
            subtitle: if is_create {
                "Create a new user"
            } else {
                "Update user details"
            },
            classes: "@container",
            attrs: form_attrs,
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: UserForm::render_inputs(&ctx),
            actions: html! {
                (container_row(
                    "flex flex-wrap justify-between gap-2 mt-2 items-center",
                    html! {
                        (container_row(
                            "flex justify-end gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save User",
                                    ..Default::default()
                                }))
                                @if !is_create {
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

impl crate::template::RenderAppPane for UserFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.pane_body())
    }
}
impl RenderTemplate for UserFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let title = if self.id == 0 {
            "Create user — Lariv"
        } else {
            "Edit user — Lariv"
        };
        app_scaffold(title, chrome, self.menu(), self.pane_body())
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
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
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: false,
            },
            TableColumnHeader {
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
                    attrs: form_hx_get_route::<UserSelectTableKey, UsersSelectRouteTag>(UsersSelectRouteTag)
                        .set("hx-push-url", "false"),
                    inputs: UserSelectFilterForm::render_inputs(
                        &FormCtx::form::<UserSelectFilterForm>()
                            .value(UserSelectFilterFormField::Name, self.filter_name.as_str())
                            .value(UserSelectFilterFormField::Email, self.filter_email.as_str()),
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
            (button_link(ButtonLink {
                href: &UsersCreateGetRouteTag.url(),
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<UserSelectTableKey>(
            &self.path_and_query,
            self.users.number,
            self.users.num_pages,
            false,
        );
        data_table_list::<UserSelectTableKey>(
            "Select User",
            actions,
            &headers,
            &rows,
            pagination,
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
}

impl RoleListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
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
                panel: role_filter_form::<RoleTableKey, UsersRolesListRouteTag>(&self.filter_name),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_users.RoleCreateForm",
                href: &UsersRolesCreateGetRouteTag.url(),
                form_post_url: &UsersRolesCreateGetRouteTag.path(),
                modal_uid: RoleCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<RoleTableKey>(
            &self.path_and_query,
            self.roles.number,
            self.roles.num_pages,
            true,
        );
        data_table_list::<RoleTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for RoleListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(user_menu(false, true), self.render_table())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for RoleListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Roles — Lariv",
            chrome,
            user_menu(false, true),
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
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for RoleDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            role_detail_menu(self.id, &self.name, "detail"),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.pane_body())
    }
}
impl RenderTemplate for RoleDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            role_detail_menu(self.id, &self.name, "detail"),
            self.pane_body(),
        )
    }
}

// Edit role form. Create uses [`RoleCreateModalPage`].
#[derive(Generic)]
pub struct RoleFormPage {
    pub id: i64,
    pub name: String,
    pub error: String,
}

impl RoleFormPage {
    fn pane_body(&self) -> Markup {
        let delete_url = UsersRolesDeleteGetRouteTag::new(self.id).url();
        form(FormOpts {
            title: "Edit Role",
            subtitle: "Update role details",
            attrs: form_hx_post_main(UsersRolesEditPostRouteTag::new(self.id)),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: RoleForm::render_inputs(
                &FormCtx::form::<RoleForm>().value(RoleFormField::Name, self.name.as_str()),
            ),
            actions: html! {
                (container_row(
                    "flex flex-wrap justify-between gap-2 mt-2 items-center",
                    html! {
                        (container_row(
                            "flex justify-end gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Role",
                                    ..Default::default()
                                }))
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
                        ))
                    },
                ))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for RoleFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            role_detail_menu(self.id, &self.name, "edit"),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.pane_body())
    }
}
impl RenderTemplate for RoleFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit role — Lariv",
            chrome,
            role_detail_menu(self.id, &self.name, "edit"),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct RoleCreateModalPage {
    pub form_name: String,
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
                    &UsersRolesCreatePostRouteTag.with_query().query("name", form_name).build_with_query(),
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
                    inputs: RoleNameFilterForm::render_inputs(
                        &FormCtx::form::<RoleNameFilterForm>()
                            .value(RoleNameFilterFormField::Name, self.filter_name.as_str()),
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
            (button_modal_form(ButtonModalForm {
                name: "p_users.RoleCreateForm",
                href: &UsersRolesCreateGetRouteTag.url(),
                form_post_url: &UsersRolesCreateGetRouteTag.path(),
                modal_uid: RoleCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<RoleSelectTableKey>(
            &self.path_and_query,
            self.roles.number,
            self.roles.num_pages,
            false,
        );
        data_table_list::<RoleSelectTableKey>(
            "Select Role",
            actions,
            &headers,
            &rows,
            pagination,
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
    bounds: [];
    items: [];
    hook: SlotsHook;
}
