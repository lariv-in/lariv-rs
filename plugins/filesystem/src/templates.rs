//! Filesystem plugin pages (Maud).

use maud::{Markup, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    Crumb, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem,
    SidebarMenuModalForm, SidebarNavLink, SlotCapability, SlotRegistrar, breadcrumbs, layout_main,
    layout_sidebar, shell_scaffold, sidebar_menu, sidebar_menu_item_pane,
    sidebar_menu_modal_form_item, sidebar_nav_items_pane_with_competitors,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

use crate::routes::{
    FilesystemRootPermissionsGetRouteTag, VNodeBrowseRouteTag, VNodeCreateGetInRouteTag,
    VNodeCreateGetRouteTag, VNodeDetailRouteTag, VNodeListRouteTag, VNodeMoveGetRouteTag,
    VNodePermissionsGetRouteTag, VNodeUploadGetInRouteTag, VNodeUploadGetRouteTag,
    VNodeZipUploadGetInRouteTag, VNodeZipUploadGetRouteTag,
};

pub mod create;
pub mod detail;
pub mod edit;
pub mod list;
pub mod move_form;
pub mod permissions;
pub mod upload;

pub use create::VNodeCreateModalPage;
pub use detail::VNodeDetailPage;
pub use edit::{VNodeConfirmBulkDeletePage, VNodeConfirmDeletePage, VNodeEditModalPage};
pub use list::{VNodeListPage, VNodeOption, VNodeRow, VNodeSelectPage};
pub use move_form::{VNodeBulkMoveFormPage, VNodeMoveFormPage};
pub use permissions::{FilesystemRootPermissionsPage, VNodePermissionsFormPage};
pub use upload::{VNodeMultiUploadModalPage, VNodeZipUploadModalPage};

define_register_items! {
    plugin: FilesystemTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ListIdx: VNodeListPageTag => VNodeListPage,
        DetailIdx: VNodeDetailPageTag => VNodeDetailPage,
        EditModalIdx: VNodeEditModalPageTag => VNodeEditModalPage,
        MoveIdx: VNodeMoveFormPageTag => VNodeMoveFormPage,
        PermissionsIdx: VNodePermissionsFormPageTag => VNodePermissionsFormPage,
        RootPermissionsIdx: FilesystemRootPermissionsPageTag => FilesystemRootPermissionsPage,
        BulkMoveIdx: VNodeBulkMoveFormPageTag => VNodeBulkMoveFormPage,
        CreateModalIdx: VNodeCreateModalPageTag => VNodeCreateModalPage,
        MultiUploadModalIdx: VNodeMultiUploadModalPageTag => VNodeMultiUploadModalPage,
        ZipUploadModalIdx: VNodeZipUploadModalPageTag => VNodeZipUploadModalPage,
        SelectIdx: VNodeSelectPageTag => VNodeSelectPage,
        ConfirmIdx: VNodeConfirmDeletePageTag => VNodeConfirmDeletePage,
        ConfirmBulkIdx: VNodeConfirmBulkDeletePageTag => VNodeConfirmBulkDeletePage,
    ]
}

define_register_items! {
    plugin: FilesystemTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
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

pub(crate) fn vnode_create_get_url(parent_id: i64) -> String {
    if parent_id == 0 {
        VNodeCreateGetRouteTag.url()
    } else {
        VNodeCreateGetInRouteTag::new(parent_id).url()
    }
}

pub(crate) fn filesystem_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Filesystem",
        href: None,
    }])
}

pub(crate) fn filesystem_browse_crumbs(parent_name: &str) -> Markup {
    let list_url = VNodeListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Filesystem",
            href: Some(&list_url),
        },
        Crumb {
            label: parent_name,
            href: None,
        },
    ])
}

pub(crate) fn filesystem_item_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = VNodeListRouteTag.url();
    let detail_url = VNodeDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Filesystem",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Filesystem",
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

pub(crate) fn can_change_permissions() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        super::routes::FilesystemPermissions,
    >())
}

/// Main sidebar: browsing the filesystem root.
pub fn main_menu(current_path: &str) -> Markup {
    let list_url = VNodeListRouteTag.url();
    let permissions_url = FilesystemRootPermissionsGetRouteTag.url();
    let create_url = VNodeCreateGetRouteTag.url();
    let upload_url = VNodeUploadGetRouteTag.url();
    let zip_url = VNodeZipUploadGetRouteTag.url();
    let mut nav = vec![SidebarNavLink {
        key: "list",
        title: "All Files",
        url: &list_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    if can_change_permissions() {
        nav.push(SidebarNavLink {
            key: "permissions",
            title: "Permissions",
            url: &permissions_url,
            icon_name: None,
            match_prefixes: &[],
        });
    }
    let addon_urls = crate::sidebar_items::root_urls();
    let addon_refs: Vec<&str> = addon_urls.iter().map(String::as_str).collect();
    sidebar_menu(SidebarMenu {
        title: "Filesystem",
        children: html! {
            (sidebar_nav_items_pane_with_competitors(&nav, &addon_refs, current_path))
            (crate::sidebar_items::root_items(current_path))
            (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                label: "Create Item",
                href: &create_url,
                name: "p_filesystem.VNodeCreateForm",
                ..Default::default()
            }))
            (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                label: "Bulk Upload",
                href: &upload_url,
                name: "p_filesystem.VNodeMultiUploadForm",
                ..Default::default()
            }))
            (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                label: "Upload Zip",
                href: &zip_url,
                name: "p_filesystem.VNodeZipUploadForm",
                ..Default::default()
            }))
        },
    })
}

/// Sidebar for a specific node. `active` selects
/// which entry (if any) is highlighted (`"detail"`, `"move"`, `"browse"`, `"git-sync"`).
pub fn vnode_menu(id: i64, name: &str, is_directory: bool, active: &str) -> Markup {
    let menu_title = format!("Item: {name}");
    let detail_url = VNodeDetailRouteTag::new(id).url();
    let move_url = VNodeMoveGetRouteTag::new(id).url();
    let permissions_url = VNodePermissionsGetRouteTag::new(id).url();
    let browse_url = VNodeBrowseRouteTag::new(id).url();
    let create_get_url = VNodeCreateGetInRouteTag::new(id).url();
    let upload_get_url = VNodeUploadGetInRouteTag::new(id).url();
    let zip_upload_get_url = VNodeZipUploadGetInRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "View Details",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Move",
                url: &move_url,
                active: active == "move",
                ..Default::default()
            }))
            @if can_change_permissions() {
                (sidebar_menu_item_pane(SidebarMenuItem {
                    title: "Permissions",
                    url: &permissions_url,
                    active: active == "permissions",
                    ..Default::default()
                }))
            }
            @if is_directory {
                (crate::sidebar_items::directory_items(id, active))
                (sidebar_menu_item_pane(SidebarMenuItem {
                    title: "Browse Contents",
                    url: &browse_url,
                    active: active == "browse",
                    ..Default::default()
                }))
                (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                    label: "Add New Item",
                    href: &create_get_url,
                    name: "p_filesystem.VNodeCreateForm",
                    ..Default::default()
                }))
                (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                    label: "Bulk Upload",
                    href: &upload_get_url,
                    name: "p_filesystem.VNodeMultiUploadForm",
                    ..Default::default()
                }))
                (sidebar_menu_modal_form_item(SidebarMenuModalForm {
                    label: "Upload Zip",
                    href: &zip_upload_get_url,
                    name: "p_filesystem.VNodeZipUploadForm",
                    ..Default::default()
                }))
            }
        },
    })
}

#[cfg(test)]
mod vnode_form_page_tests {
    use chrono::Utc;

    use super::{
        FilesystemRootPermissionsPage, VNodeCreateModalPage, VNodeDetailPage, VNodeEditModalPage,
        VNodeListPage, VNodeOption, VNodePermissionsFormPage, VNodeSelectPage,
    };
    use crate::permissions::NodePermissions;
    use crate::routes::FilesystemPermissions;
    use lariv_core::components::ObjectList;
    use lariv_core::components::picker::RenderPickerSelect;
    use lariv_core::template::{RenderAppPane, RenderTemplate};
    use lariv_plugin_users::entities::user::Model as User;
    use lariv_plugin_users::role_authorization::{RoleAuthorizationRegistry, with_principal};
    use lariv_plugin_users::state::AuthContext;

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

    fn filesystem_list_page() -> VNodeListPage {
        VNodeListPage {
            parent_id: 0,
            parent_name: String::new(),
            items: ObjectList::from_page(vec![], 1, 12, 0),
            fields: Vec::new(),
            filters: Vec::new(),
            filter_errors: Vec::new(),
            sort: String::new(),
            path_and_query: "/filesystem".into(),
            page_size: 12,
        }
    }

    #[test]
    fn permissions_nav_is_hidden_from_hr() {
        let registry =
            RoleAuthorizationRegistry::new().allow::<FilesystemPermissions>(vec!["admin".into()]);
        let page = filesystem_list_page();
        let admin = with_principal(auth("admin"), registry.clone(), || {
            page.render(&Default::default()).into_string()
        });
        assert!(
            admin.contains(">Permissions<"),
            "admin should see Permissions: {admin}"
        );
        let hr = with_principal(auth("hr"), registry.clone(), || {
            page.render(&Default::default()).into_string()
        });
        assert!(
            !hr.contains(">Permissions<"),
            "hr should not see Permissions: {hr}"
        );
        let superuser = with_principal(
            auth(lariv_plugin_users::roles::Superuser::NAME),
            registry,
            || page.render(&Default::default()).into_string(),
        );
        assert!(
            superuser.contains(">Permissions<"),
            "superuser should see Permissions: {superuser}"
        );
    }

    #[test]
    fn permissions_page_labels_and_inside_folder_checkbox() {
        let page = |is_directory: bool| VNodePermissionsFormPage {
            id: 1,
            name: "docs".into(),
            is_directory,
            owner_id: 0,
            owner_display: String::new(),
            role: String::new(),
            permissions: NodePermissions::for_directory(),
            apply_inside: false,
            error: String::new(),
        };
        let directory = page(true).render(&Default::default()).into_string();
        for text in [
            "Permissions",
            "Everyone else",
            "Anyone",
            "Can view",
            "Can change",
            "Can open",
            "Also update everything inside this folder",
        ] {
            assert!(directory.contains(text), "{text} missing from {directory}");
        }
        assert!(
            !directory.to_ascii_lowercase().contains("chmod"),
            "{directory}"
        );
        let file = page(false).render(&Default::default()).into_string();
        assert!(
            !file.contains("Also update everything inside this folder"),
            "{file}"
        );
        assert!(file.contains("Can open"), "{file}");
    }

    #[test]
    fn root_permissions_page_applies_to_filesystem_root() {
        let page = FilesystemRootPermissionsPage {
            owner_id: 0,
            owner_display: String::new(),
            role: String::new(),
            permissions: NodePermissions::legacy(),
            apply_all: false,
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(
            html.contains("Choose who can use the filesystem root."),
            "{html}"
        );
        assert!(html.contains("Permissions"), "{html}");
        assert!(html.contains("Also update every item"), "{html}");
        assert!(
            !html.contains("Also update everything inside this folder"),
            "{html}"
        );
        assert!(html.contains("/filesystem/permissions"), "{html}");
    }

    #[test]
    fn create_modal_shows_kind_radios() {
        let page = VNodeCreateModalPage {
            form_name: String::new(),
            refresh_table: String::new(),
            target_input: String::new(),
            name: String::new(),
            is_directory: false,
            parent_id: 0,
            parent_display: String::new(),
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(html.contains("name=\"Kind\""), "create: {html}");
        assert!(html.contains("type=\"radio\""), "create: {html}");
        assert!(html.contains("type=\"file\""), "create: {html}");
        assert!(html.contains("name=\"ParentID\""), "create: {html}");
        assert!(html.contains("multipart/form-data"), "create: {html}");
        assert!(html.contains("onchange="), "create: {html}");
        assert!(html.contains("files[0].name"), "create: {html}");
    }

    #[test]
    fn create_modal_post_includes_target_input() {
        let page = VNodeCreateModalPage {
            form_name: String::new(),
            refresh_table: String::new(),
            target_input: "ParentID".into(),
            name: String::new(),
            is_directory: false,
            parent_id: 0,
            parent_display: String::new(),
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(html.contains("target_input=ParentID"), "create: {html}");
    }

    fn select_page(parent_id: i64, target_input: &str) -> VNodeSelectPage {
        select_page_with(parent_id, target_input, "/filesystem/select", true, vec![])
    }

    fn select_page_with(
        parent_id: i64,
        target_input: &str,
        browse_base: &str,
        only_directories: bool,
        items: Vec<VNodeOption>,
    ) -> VNodeSelectPage {
        VNodeSelectPage {
            items: ObjectList::from_page(items, 1, 12, 0),
            fields: Vec::new(),
            filters: Vec::new(),
            filter_errors: Vec::new(),
            target_input: target_input.into(),
            browse_base: browse_base.into(),
            parent_id,
            up_parent_id: 0,
            current_path: if parent_id == 0 {
                "/".into()
            } else {
                "/docs".into()
            },
            exclude_id: 0,
            only_directories,
            select_any: false,
            multi: false,
            sort: String::new(),
            path_and_query: browse_base.into(),
            page_size: 12,
        }
    }

    #[test]
    fn select_modal_matches_other_pickers() {
        let html = select_page(0, "ParentID")
            .render(&Default::default())
            .into_string();
        assert!(html.contains("<dialog"), "select: {html}");
        assert!(html.contains("vnode-selection-modal"), "select: {html}");
        assert!(html.contains("modal-open"), "select: {html}");
        assert!(!html.contains("!bg-transparent"), "select: {html}");
    }

    #[test]
    fn select_table_is_fragment_not_dialog() {
        let html = select_page(0, "ParentID").render_table().into_string();
        assert!(!html.contains("<dialog"), "select table: {html}");
        assert!(html.contains("fk-modal-host"), "select table: {html}");
        assert!(html.contains("Select a Folder"), "select table: {html}");
    }

    #[test]
    fn select_table_create_button_at_root_embeds_target_input() {
        let html = select_page(0, "ParentID").render_table().into_string();
        assert!(html.contains("fk-modal-host"), "select: {html}");
        assert!(html.contains("/filesystem/create/"), "select: {html}");
        assert!(
            html.contains("name=p_filesystem.VNodeCreateForm"),
            "select: {html}"
        );
        assert!(html.contains("target_input=ParentID"), "select: {html}");
    }

    #[test]
    fn select_table_create_button_in_directory_uses_parent_route() {
        let html = select_page(7, "ParentID").render_table().into_string();
        assert!(html.contains("/filesystem/create/in/7/"), "select: {html}");
        assert!(html.contains("target_input=ParentID"), "select: {html}");
        assert!(
            !html.contains(r#"hx-get="/filesystem/create/?name="#),
            "select: {html}"
        );
        assert!(html.contains(">Up</button>"), "select: {html}");
        assert!(html.contains("Use here:"), "select: {html}");
    }

    #[test]
    fn file_select_browse_stays_on_file_select_routes() {
        let page = select_page_with(
            0,
            "PageID",
            "/filesystem/file-select",
            false,
            vec![
                VNodeOption {
                    id: 3,
                    name: "docs".into(),
                    is_directory: true,
                },
                VNodeOption {
                    id: 4,
                    name: "readme.txt".into(),
                    is_directory: false,
                },
            ],
        );
        let html = page.render_table().into_string();
        assert!(html.contains("Select a File"), "file select: {html}");
        assert!(
            html.contains("/filesystem/file-select/in/3"),
            "file select: {html}"
        );
        assert!(
            !html.contains("/filesystem/select/in/3"),
            "file select: {html}"
        );
        assert!(html.contains(">Open</button>"), "file select: {html}");
        assert!(html.contains("readme.txt"), "file select: {html}");
        assert!(html.contains("fk-select"), "file select single: {html}");
        assert!(
            !html.contains("fk-multi-select"),
            "file select single: {html}"
        );
    }

    #[test]
    fn file_select_multi_uses_multi_select_and_keeps_multi_query() {
        let mut page = select_page_with(
            0,
            "Files",
            "/filesystem/file-select",
            false,
            vec![
                VNodeOption {
                    id: 3,
                    name: "docs".into(),
                    is_directory: true,
                },
                VNodeOption {
                    id: 4,
                    name: "readme.txt".into(),
                    is_directory: false,
                },
            ],
        );
        page.multi = true;
        let html = page.render_table().into_string();
        assert!(html.contains("fk-multi-select"), "file multi: {html}");
        assert!(
            !html.contains("closest('dialog.modal')"),
            "file multi must not close modal: {html}"
        );
        assert!(
            html.contains(r#"name="multi" value="1""#),
            "file multi filter: {html}"
        );
        assert!(
            html.contains("multi=1"),
            "browse/open must preserve multi: {html}"
        );
    }

    #[test]
    fn list_page_has_bulk_actions() {
        let page = VNodeListPage {
            parent_id: 0,
            parent_name: String::new(),
            items: ObjectList::from_page(vec![], 1, 12, 0),
            fields: Vec::new(),
            filters: Vec::new(),
            filter_errors: Vec::new(),
            sort: String::new(),
            path_and_query: "/filesystem".into(),
            page_size: 12,
        };
        let html = page.render_table().into_string();
        assert!(
            html.contains("$persist(&quot;List&quot;)"),
            "filesystem list should default to List: {html}"
        );
        assert!(
            html.contains("ellipsis-vertical") && html.contains("Bulk actions"),
            "filesystem list should expose bulk actions: {html}"
        );
        assert!(
            html.contains("data-vnode-selection") || html.contains("setVisible"),
            "filesystem list table should wire selection helpers: {html}"
        );
    }

    #[test]
    fn detail_page_shows_breadcrumbs() {
        let page = VNodeDetailPage {
            id: 42,
            name: "docs".into(),
            is_directory: true,
            item_type: "Directory".into(),
            size_display: "—".into(),
            items_display: "3".into(),
            path: "/docs".into(),
            updated_at: "2026-01-01".into(),
            text_content: None,
            save_error: String::new(),
            header_extra: String::new(),
        };
        let html = page.render_main().into_markup().into_string();
        assert!(html.contains(r#"class="breadcrumbs"#), "detail: {html}");
        assert!(html.contains("/filesystem"), "detail: {html}");
        assert!(html.contains(">Filesystem</a>"), "detail: {html}");
        assert!(html.contains("<span>docs</span>"), "detail: {html}");
        assert!(
            html.contains(">Edit<"),
            "directory detail should show Edit: {html}"
        );
        assert!(
            html.contains("Browse Contents"),
            "directory detail should show Browse Contents: {html}"
        );
        let edit_at = html.find(">Edit<").expect("Edit");
        let modified_at = html.find("Modified").expect("Modified");
        assert!(
            edit_at < modified_at,
            "Edit should be in the header above Modified: {html}"
        );
        assert!(
            !html.contains(">PDF<"),
            "directory detail should not show PDF: {html}"
        );
        assert!(
            !html.contains("data-code-editor-root"),
            "directory detail should not show editor: {html}"
        );
    }

    fn detail_file_page(name: &str) -> VNodeDetailPage {
        detail_file_page_with_text(name, Some("hello\n"))
    }

    fn detail_file_page_with_text(name: &str, text_content: Option<&str>) -> VNodeDetailPage {
        VNodeDetailPage {
            id: 7,
            name: name.into(),
            is_directory: false,
            item_type: "File".into(),
            size_display: "12 B".into(),
            items_display: "-".into(),
            path: format!("/{name}"),
            updated_at: "2026-01-01".into(),
            text_content: text_content.map(str::to_string),
            save_error: String::new(),
            header_extra: String::new(),
        }
    }

    #[test]
    fn detail_page_typ_file_shows_pdf_button() {
        let html = detail_file_page("notes.typ")
            .render_main()
            .into_markup()
            .into_string();
        assert!(html.contains(">PDF<"), "typ detail: {html}");
        assert!(html.contains(">Edit<"), "typ detail Edit: {html}");
        assert!(html.contains("Download"), "typ detail Download: {html}");
        let pdf_at = html.find(">PDF<").expect("PDF");
        let modified_at = html.find("Modified").expect("Modified");
        assert!(
            pdf_at < modified_at,
            "PDF should be in the header above Modified: {html}"
        );
        assert!(
            html.contains("/filesystem/7/pdf"),
            "typ detail pdf route: {html}"
        );
        assert!(
            html.contains("data-code-editor-root"),
            "typ detail should also show text editor: {html}"
        );
        assert!(
            html.contains(r#"data-language="typst""#),
            "typ editor language: {html}"
        );
    }

    #[test]
    fn detail_page_non_typ_file_hides_pdf_button() {
        let html = detail_file_page("notes.txt")
            .render_main()
            .into_markup()
            .into_string();
        assert!(!html.contains(">PDF<"), "txt detail: {html}");
        assert!(
            !html.contains("/filesystem/7/pdf"),
            "txt detail pdf route: {html}"
        );
    }

    #[test]
    fn detail_page_text_file_shows_editor_and_save() {
        let html = detail_file_page("notes.txt")
            .render_main()
            .into_markup()
            .into_string();
        assert!(html.contains("data-code-editor-root"), "txt editor: {html}");
        assert!(html.contains("name=\"Content\""), "txt editor name: {html}");
        assert!(html.contains("hello"), "txt editor value: {html}");
        assert!(html.contains(">Save<"), "txt save: {html}");
        assert!(
            html.contains("/filesystem/7/content"),
            "txt content post: {html}"
        );
        assert!(
            html.contains(r#"data-language="plaintext""#),
            "txt language: {html}"
        );
    }

    #[test]
    fn detail_page_markdown_file_uses_markdown_language() {
        let html = detail_file_page("readme.md")
            .render_main()
            .into_markup()
            .into_string();
        assert!(
            html.contains(r#"data-language="markdown""#),
            "md language: {html}"
        );
    }

    #[test]
    fn detail_page_binary_file_hides_editor() {
        let html = detail_file_page_with_text("photo.png", None)
            .render_main()
            .into_markup()
            .into_string();
        assert!(
            !html.contains("data-code-editor-root"),
            "binary should not show editor: {html}"
        );
        assert!(
            !html.contains("/filesystem/7/content"),
            "binary should not post content: {html}"
        );
    }

    #[test]
    fn edit_dir_hides_file() {
        let page = VNodeEditModalPage {
            id: 1,
            form_name: "p_filesystem.VNodeEditForm".into(),
            name: "docs".into(),
            is_directory: true,
            has_file: false,
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(!html.contains("type=\"radio\""), "edit dir: {html}");
        assert!(!html.contains("type=\"file\""), "edit dir: {html}");
        assert!(html.contains("name=\"Name\""), "edit dir: {html}");
    }

    #[test]
    fn edit_file_shows_replace_file() {
        let page = VNodeEditModalPage {
            id: 2,
            form_name: "p_filesystem.VNodeEditForm".into(),
            name: "a.txt".into(),
            is_directory: false,
            has_file: true,
            error: String::new(),
        };
        let html = page.render(&Default::default()).into_string();
        assert!(html.contains("type=\"file\""), "edit file: {html}");
        assert!(html.contains("Replace File"), "edit file: {html}");
    }
}
