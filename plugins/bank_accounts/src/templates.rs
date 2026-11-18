//! Maud page templates for bank account CRUD views.

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

pub use create::{BANK_ACCOUNT_CREATE, BankAccountCreateModalPage};
pub use detail::BankAccountDetailPage;
pub use edit::{BankAccountEditModalPage, ConfirmDeletePage};
pub use list::{BankAccountListPage, BankAccountRow, BankRow, BankSelectPage, MicrSelectPage};

define_register_items! {
    plugin: BankAccountsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        BankAccountListIdx: BankAccountListPageTag => BankAccountListPage,
        BankAccountDetailIdx: BankAccountDetailPageTag => BankAccountDetailPage,
        BankAccountEditModalIdx: BankAccountEditModalPageTag => BankAccountEditModalPage,
        BankAccountCreateModalIdx: BankAccountCreateModalPageTag => BankAccountCreateModalPage,
        BankSelectIdx: BankSelectPageTag => BankSelectPage,
        MicrSelectIdx: MicrSelectPageTag => MicrSelectPage,
        ConfirmDeleteIdx: BankAccountConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

define_register_items! {
    plugin: BankAccountsTag;
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
