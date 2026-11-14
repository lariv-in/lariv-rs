//! Maud page templates for assistant test-case CRUD.

use maud::Markup;

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, SlotCapability, SlotRegistrar,
    layout_main, layout_sidebar, shell_scaffold,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod create;
pub mod detail;
pub mod edit;
pub mod list;

pub use create::TestCaseCreateModalPage;
pub use detail::TestCaseDetailPage;
pub use edit::{ConfirmDeletePage, TestCaseEditModalPage};
pub use list::{TestCaseListPage, TestCaseRow};

define_register_items! {
    plugin: LlmAssistantTesterTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        TestCaseListIdx: TestCaseListPageTag => TestCaseListPage,
        TestCaseDetailIdx: TestCaseDetailPageTag => TestCaseDetailPage,
        TestCaseEditModalIdx: TestCaseEditModalPageTag => TestCaseEditModalPage,
        TestCaseCreateModalIdx: TestCaseCreateModalPageTag => TestCaseCreateModalPage,
        ConfirmDeleteIdx: TestCaseConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

pub(crate) fn app_scaffold(
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

define_register_items! {
    plugin: LlmAssistantTesterTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
