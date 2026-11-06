//! Maud page templates for document CRUD views.

use maud::{Markup, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    Crumb, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem,
    SidebarNavLink, SlotCapability, SlotRegistrar, breadcrumbs, layout_main, layout_sidebar,
    shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

use crate::routes::{DocumentDefaultRouteTag, DocumentDetailRouteTag, DocumentPrefsGetRouteTag};

pub mod create;
pub mod detail;
pub mod edit;
pub mod list;
pub mod preferences;

pub use create::{DOCUMENT_CREATE, DocumentCreateModalPage};
pub use detail::DocumentDetailPage;
pub use edit::{ConfirmDeletePage, DocumentEditModalPage};
pub use list::{DocumentListPage, DocumentOption, DocumentRow, DocumentSelectPage};
pub use preferences::DocumentPreferencesPage;

define_register_items! {
    plugin: DocumentsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        DocumentListIdx: DocumentListPageTag => DocumentListPage,
        DocumentDetailIdx: DocumentDetailPageTag => DocumentDetailPage,
        DocumentEditModalIdx: DocumentEditModalPageTag => DocumentEditModalPage,
        DocumentCreateModalIdx: DocumentCreateModalPageTag => DocumentCreateModalPage,
        ConfirmDeleteIdx: DocumentConfirmDeletePageTag => ConfirmDeletePage,
        PreferencesIdx: DocumentPreferencesPageTag => DocumentPreferencesPage,
        DocumentSelectIdx: DocumentSelectPageTag => DocumentSelectPage,
    ]
}

define_register_items! {
    plugin: DocumentsTag;
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

pub(crate) fn documents_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Documents",
        href: None,
    }])
}

pub(crate) fn document_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = DocumentDefaultRouteTag.url();
    let detail_url = DocumentDetailRouteTag::new(id).url();
    let label = if name.is_empty() { "Document" } else { name };
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Documents",
                href: Some(&list_url),
            },
            Crumb { label, href: None },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Documents",
                href: Some(&list_url),
            },
            Crumb {
                label,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

pub(crate) fn document_menu(current_path: &str) -> Markup {
    let list_url = DocumentDefaultRouteTag.url();
    let prefs_url = DocumentPrefsGetRouteTag.url();
    let links = [
        SidebarNavLink {
            key: "documents",
            title: "All Documents",
            url: &list_url,
            icon_name: None,
            match_prefixes: &[],
        },
        SidebarNavLink {
            key: "preferences",
            title: "Preferences",
            url: &prefs_url,
            icon_name: None,
            match_prefixes: &[],
        },
    ];
    sidebar_menu(SidebarMenu {
        title: "Documents",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

pub(crate) fn document_detail_menu(id: i64, name: &str) -> Markup {
    let title = if name.is_empty() {
        "Document".to_string()
    } else {
        format!("Document: {name}")
    };
    let detail_url = DocumentDetailRouteTag::new(id).url();
    let prefs_url = DocumentPrefsGetRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Document Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Preferences",
                url: &prefs_url,
                ..Default::default()
            }))
        },
    })
}

pub(crate) fn preferences_crumbs() -> Markup {
    let list_url = DocumentDefaultRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Documents",
            href: Some(&list_url),
        },
        Crumb {
            label: "Preferences",
            href: None,
        },
    ])
}
