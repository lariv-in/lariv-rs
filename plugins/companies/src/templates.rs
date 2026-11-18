//! Maud page templates for company CRUD views.

use maud::Markup;

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, layout_main, layout_sidebar,
    shell_scaffold,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod create;
pub mod detail;
pub mod edit;
pub mod list;

pub use create::{COMPANY_CREATE, CompanyCreateModalPage};
pub use detail::{CompanyDetailPage, CompanyMemberRow};
pub use edit::{CompanyEditModalPage, ConfirmDeletePage};
pub use list::{CompanyListPage, CompanyRow, CompanySelectPage};

define_register_items! {
    plugin: CompaniesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        CompanyListIdx: CompanyListPageTag => CompanyListPage,
        CompanyDetailIdx: CompanyDetailPageTag => CompanyDetailPage,
        CompanyEditModalIdx: CompanyEditModalPageTag => CompanyEditModalPage,
        CompanyCreateModalIdx: CompanyCreateModalPageTag => CompanyCreateModalPage,
        CompanySelectIdx: CompanySelectPageTag => CompanySelectPage,
        ConfirmDeleteIdx: CompanyConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

pub(crate) fn app_scaffold(
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

pub(crate) fn scaffold_main(
    crumbs: Markup,
    body: Markup,
) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}
