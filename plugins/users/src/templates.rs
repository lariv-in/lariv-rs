//! Maud page templates for auth, user/role CRUD, and self-profile views.

use maud::Markup;

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    Crumb, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, SlotCapability, SlotOf,
    SlotRegistrar, TopbarItemsSlotTag, breadcrumbs, layout_main, layout_sidebar, shell_scaffold,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

use crate::routes::UsersSelfRouteTag;

pub mod auth;
pub mod change_password;
pub mod create;
pub mod detail;
pub mod edit;
pub mod effective_user;
pub mod list;
pub mod select;
#[path = "templates/self.rs"]
pub mod self_;
pub mod topbar;

pub use auth::{LoginPage, UnauthenticatedPage};
pub use change_password::ChangePasswordPage;
pub use create::UserCreateModalPage;
pub use detail::{SelfDetailPage, UserDetailPage};
pub use edit::{ConfirmDeletePage, SelfEditModalPage, UserEditModalPage};
pub use effective_user::EffectiveUserSelector;
pub use list::{UserListPage, UserRow};
pub use select::UserSelectPage;
pub use topbar::{UsersTopbarNav, UsersTopbarNavTag, UsersUserDropdown};

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

pub(crate) fn app_scaffold(
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
pub(crate) fn scaffold_pane(
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
pub(crate) fn scaffold_main(
    crumbs: Markup,
    body: Markup,
) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

pub(crate) fn self_crumbs(name: &str, action: Option<&str>) -> Markup {
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

        use super::{EffectiveUserSelector, UsersUserDropdown};
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

        let menu = with_principal(
            auth(Superuser::NAME),
            RoleAuthorizationRegistry::new(),
            || UsersUserDropdown.render_slot(&ctx).into_string(),
        );
        let summary_at = menu.find("<summary").expect("avatar summary");
        let selector_at = menu
            .find("effective-user-selector")
            .expect("selector in the user menu");
        let details_end = menu.rfind("</details>").expect("menu close");
        assert!(
            summary_at < selector_at && selector_at < details_end,
            "{menu}"
        );
        assert_eq!(menu.matches("effective-user-selector").count(), 1, "{menu}");

        let employee_menu =
            with_principal(auth("employee"), RoleAuthorizationRegistry::new(), || {
                UsersUserDropdown.render_slot(&ctx).into_string()
            });
        assert!(
            !employee_menu.contains("effective-user-selector"),
            "{employee_menu}"
        );
    }
}
