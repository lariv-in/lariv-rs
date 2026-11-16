//! Per-user audit log page, inside the user detail chrome.

use maud::Markup;

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, layout_main, layout_sidebar,
    shell_scaffold,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_plugin_logging::templates::LogListPage;
use lariv_plugin_users::templates::detail::{user_crumbs, user_detail_menu};

define_register_items! {
    plugin: UserLogTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        UserLogIdx: UserLogPageTag => UserLogPage,
    ]
}

fn app_scaffold(chrome: &ShellChrome, sidebar: Markup, crumbs: Markup, body: Markup) -> Markup {
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
) -> lariv_core::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

/// Audit log for one user, shown with that user's detail sidebar.
pub struct UserLogPage {
    pub user_id: i64,
    pub user_name: String,
    pub show_change_password: bool,
    pub logs: LogListPage,
}

impl UserLogPage {
    fn sidebar(&self) -> Markup {
        user_detail_menu(
            self.user_id,
            &self.user_name,
            "logs",
            self.show_change_password,
        )
    }

    fn crumbs(&self) -> Markup {
        user_crumbs(self.user_id, &self.user_name, Some("Logs"))
    }
}

impl lariv_core::template::RenderAppPane for UserLogPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(self.sidebar(), self.crumbs(), self.logs.render_table())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.logs.render_table())
    }
}

impl RenderTemplate for UserLogPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            self.sidebar(),
            self.crumbs(),
            self.logs.render_table(),
        )
    }
}
