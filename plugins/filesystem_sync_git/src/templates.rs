//! Git sync pages.

use maud::Markup;

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, SlotCapability, SlotRegistrar,
    layout_main, layout_sidebar, shell_scaffold,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod confirm;
pub mod detail;
pub mod list;
pub mod modals;

pub use confirm::GitSyncConfirmPage;
pub use detail::GitSyncDetailPage;
pub use list::{GitSyncListPage, GitSyncRow};
pub use modals::{
    GIT_SYNC_CREATE, GitSyncConfirmDeletePage, GitSyncCreateModalPage, GitSyncEditModalPage,
};

define_register_items! {
    plugin: FilesystemSyncGitTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ListIdx: GitSyncListPageTag => GitSyncListPage,
        DetailIdx: GitSyncDetailPageTag => GitSyncDetailPage,
        EditModalIdx: GitSyncEditModalPageTag => GitSyncEditModalPage,
        CreateModalIdx: GitSyncCreateModalPageTag => GitSyncCreateModalPage,
        ConfirmDeleteIdx: GitSyncConfirmDeletePageTag => GitSyncConfirmDeletePage,
        ConfirmSyncIdx: GitSyncConfirmPageTag => GitSyncConfirmPage,
    ]
}

define_register_items! {
    plugin: FilesystemSyncGitTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
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
