#[cfg(test)]
mod tests {
    use maud::Markup;

    use crate::components::{
        ButtonSubmit, FormOpts, InputEmail, InputPassword, ShellAuth, ShellBase, ShellChrome,
        SlotCtx, TopbarItemsSlotTag, button_submit, form, input_email, input_password, shell_auth,
        shell_base,
    };
    use crate::components::slots::SlotCapability;
    use crate::plugins::dashboard::templates::{
        AppsPage, DashboardAppsPageButton, DashboardAppsPageButtonTag, DashboardThemeButton,
        DashboardThemeButtonTag, DashboardUserDropdown, DashboardUserDropdownTag,
    };
    use crate::plugins::users::templates::{LoginPage, UnauthenticatedPage};
    use crate::template::RenderTemplate;

    fn markup_str(m: Markup) -> String {
        m.into_string()
    }

    #[test]
    fn shell_base_includes_cdn_stack() {
        let html = markup_str(shell_base(ShellBase {
            title: "Test",
            body: maud::html! { p { "hi" } },
            ..Default::default()
        }));
        assert!(html.contains("htmx.org@4.0.0-beta6"));
        assert!(html.contains("hx-alpine-compat"));
        assert!(html.contains("daisyui@5"));
        assert!(html.contains("@tailwindcss/browser@4"));
        assert!(html.contains("alpinejs"));
        assert!(html.contains(r#"name="htmx-config""#));
        assert!(html.contains("outerHTML"));
        assert!(html.contains("hx-boost:inherited=\"true\""));
        assert!(html.contains("hx-target:inherited=\"#app-layout\""));
        assert!(html.contains("hx-select:inherited=\"#app-layout\""));
        assert!(html.contains("hx-swap:inherited=\"outerHTML\""));
        assert!(!html.contains("hx-ext="));
        assert!(!html.contains("htmx.org@2"));
        assert!(!html.contains("alpine-morph"));
        assert!(!html.contains("htmx-2-compat"));
        assert!(!html.contains("htmx-ext-ws"));
        assert!(html.contains("hx-ws.min.js"));
        assert!(!html.contains("apexcharts"));
    }

    #[test]
    fn login_page_uses_auth_shell_and_form() {
        let chrome = ShellChrome::default();
        let html = markup_str(
            LoginPage {
                error: "bad creds".into(),
            }
            .render(&chrome),
        );
        assert!(html.contains("daisyui@5"));
        assert!(html.contains("Login"));
        assert!(html.contains("bad creds"));
        assert!(html.contains(r#"name="Email""#));
        assert!(html.contains(r#"name="Password""#));
        assert!(html.contains("card-body") || html.contains("card shadow-xl"));
        assert!(html.contains("hx-post"));
        assert!(html.contains("app-layout"));
        assert!(!html.contains("lariv-form-submit"));
    }

    #[test]
    fn unauthenticated_and_apps_pages_render() {
        let chrome = ShellChrome::default();
        let unauth = markup_str(UnauthenticatedPage {}.render(&chrome));
        assert!(unauth.contains("Welcome"));
        assert!(unauth.contains("/users/login"));

        let apps = markup_str(
            AppsPage {
                name: "Ada".into(),
                role: "Admin".into(),
                avatar: "A".into(),
                is_superuser: true,
                apps: vec![],
            }
            .render(&chrome),
        );
        assert!(apps.contains("Search apps"));
        assert!(apps.contains("daisyui@5") || apps.contains("@container"));
        assert!(apps.contains(r#"id="app-layout""#));
    }

    #[test]
    fn form_builder_composes_inputs() {
        let html = markup_str(shell_auth(ShellAuth {
            title: "x",
            body: form(FormOpts {
                title: "Sign in",
                action: Some("/users/login"),
                inputs: maud::html! {
                    (input_email(InputEmail {
                        required: true,
                        ..Default::default()
                    }))
                    (input_password(InputPassword {
                        required: true,
                        ..Default::default()
                    }))
                },
                actions: button_submit(ButtonSubmit {
                    label: "Go",
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }));
        assert!(html.contains("Sign in"));
        assert!(html.contains("btn btn-primary"));
        assert!(html.contains(r#"type="email""#));
    }

    #[test]
    fn dashboard_topbar_slots_match_go_chrome() {
        let slots = SlotCapability::new()
            .add::<DashboardAppsPageButtonTag, TopbarItemsSlotTag, DashboardAppsPageButton>()
            .add::<DashboardThemeButtonTag, TopbarItemsSlotTag, DashboardThemeButton>()
            .add::<DashboardUserDropdownTag, TopbarItemsSlotTag, DashboardUserDropdown>();
        let chrome = slots.fold_chrome(&SlotCtx {
            name: Some("Ada".into()),
            role: Some("Admin".into()),
            is_superuser: true,
        });
        let html = markup_str(chrome.topbar_items);
        assert!(html.contains("squares-2x2") || html.contains("/dashboard"));
        assert!(html.contains("toggleTheme()"));
        assert!(html.contains("dropdown dropdown-end"));
        assert!(html.contains("My Account"));
        assert!(html.contains("Logout"));
        assert!(html.contains("Ada"));
    }

    #[test]
    fn apps_page_uses_folded_dashboard_chrome() {
        let slots = SlotCapability::new()
            .add::<DashboardAppsPageButtonTag, TopbarItemsSlotTag, DashboardAppsPageButton>()
            .add::<DashboardThemeButtonTag, TopbarItemsSlotTag, DashboardThemeButton>()
            .add::<DashboardUserDropdownTag, TopbarItemsSlotTag, DashboardUserDropdown>();
        let chrome = slots.fold_chrome(&SlotCtx {
            name: Some("Ada".into()),
            role: Some("User".into()),
            is_superuser: false,
        });
        let html = markup_str(
            AppsPage {
                name: "Ada".into(),
                role: "User".into(),
                avatar: "A".into(),
                is_superuser: false,
                apps: vec![crate::plugins::dashboard::AppTile {
                    key: "p_users".into(),
                    verbose_name: "Users".into(),
                    href: "/users".into(),
                    icon: "users".into(),
                    plugin_type: crate::plugins::dashboard::PluginType::App,
                    roles: vec![],
                }],
            }
            .render(&chrome),
        );
        assert!(html.contains("Search apps"));
        assert!(html.contains("@md:grid-cols-4"));
        assert!(html.contains("x-model=\"search\""));
        assert!(html.contains("/users"));
        assert!(html.contains("toggleTheme()"));
        assert!(html.contains("dropdown dropdown-end"));
    }

    #[test]
    fn parity_components_render() {
        use crate::components::{
            ButtonModalForm, DeleteConfirmation, FieldText, InputForeignKey, AppLayoutKey, Modal,
            SidebarMenu, SidebarMenuBack, SidebarMenuItem, SwapKey, TableButtonFilter,
            TableColumnHeader, TableRow, button_modal_form, data_table_list, delete_confirmation,
            detail, field_text, form_hx_post, input_foreign_key, modal, nav_main_attrs,
            sidebar_menu, sidebar_menu_item, table_button_filter,
        };
        use crate::plugins::users::keys::{UserCreateModalKey, UserTableKey};

        let menu = markup_str(sidebar_menu(SidebarMenu {
            title: "Users",
            back: Some(SidebarMenuBack {
                title: "Back",
                url: "/dashboard",
            }),
            children: sidebar_menu_item(SidebarMenuItem {
                title: "All Users",
                url: "/users",
                ..Default::default()
            }),
        }));
        assert!(menu.contains("All Users"));
        assert!(menu.contains("/dashboard"));
        assert!(menu.contains("hx-target=\"#main-content\""));
        assert!(!menu.contains("hx-target=\"#app-layout\""));

        let headers = [TableColumnHeader {
            label: "Name",
            sort_url: Some("/users?sort=Name+ASC"),
            push_url: true,
        }];
        let rows = [TableRow {
            attrs: nav_main_attrs("/users/u/1"),
            cells: vec![field_text(FieldText {
                value: "Ada",
                classes: "",
            })],
        }];
        let table = markup_str(data_table_list::<UserTableKey>(
            "Users",
            table_button_filter(TableButtonFilter {
                panel: maud::html! { "filters" },
                ..Default::default()
            }),
            &headers,
            &rows,
            maud::Markup::default(),
        ));
        assert!(table.contains(UserTableKey::ID));
        assert!(table.contains("data-table-container"));
        assert!(table.contains(UserTableKey::SELECTOR));
        assert!(table.contains("Ada"));
        assert!(table.contains(AppLayoutKey::SELECTOR));
        assert!(!table.contains("closest .data-table-container"));

        let dlg = markup_str(modal(Modal {
            uid: UserCreateModalKey::ID,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: "Sure?",
                attrs: form_hx_post::<UserCreateModalKey>("/users/u/1/delete/"),
                ..Default::default()
            }),
            ..Default::default()
        }));
        assert!(dlg.contains(UserCreateModalKey::ID));
        assert!(dlg.contains("Confirm Delete"));
        assert!(dlg.contains("hx-post"));
        assert!(!dlg.contains("lariv-form-submit"));

        let fk = markup_str(input_foreign_key(InputForeignKey {
            label: "Role",
            name: "role_id",
            value: "1",
            display: "unassigned",
            url: "/users/roles/select",
            uid: "fk-user-role",
            required: true,
            ..Default::default()
        }));
        assert!(fk.contains("role_id"));
        assert!(fk.contains("target_input"));
        assert!(fk.contains("fk-select"));
        assert!(fk.contains(r#"id="fk-user-role""#));

        let btn = markup_str(button_modal_form(ButtonModalForm {
            href: "/users/create",
            name: "p_users.UserCreateForm",
            form_post_url: "/users/create",
            modal_uid: UserCreateModalKey::ID,
            icon_name: Some("plus"),
            classes: "btn-square btn-outline btn-sm",
            ..Default::default()
        }));
        assert!(btn.contains("hx-get"));
        assert!(btn.contains("/users/create"));
        assert!(btn.contains("p_users.UserCreateForm"));
        assert!(btn.contains("hx-target=\"body\""));
        assert!(btn.contains("hx-select=\"\""));
        assert!(!btn.contains("lariv-form-submit"));
        assert!(!btn.contains("htmx.ajax"));

        let d = markup_str(detail(maud::html! { "hello" }));
        assert!(d.contains("hello"));
    }
}
