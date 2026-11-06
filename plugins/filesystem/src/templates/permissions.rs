use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, Crumb, FormOpts, ShellChrome, breadcrumbs, button_submit, form, form_hx_post_main,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::forms::{VNodePermissionsForm, VNodePermissionsFormField, VNodePermissionsFormFlag};
use crate::permissions::NodePermissions;
use crate::routes::{
    FilesystemRootPermissionsGetRouteTag, FilesystemRootPermissionsPostRouteTag, VNodeListRouteTag,
    VNodePermissionsPostRouteTag,
};

use super::{
    app_scaffold, filesystem_item_crumbs, main_menu, scaffold_main, scaffold_pane, vnode_menu,
};

/// Permissions page: owner, role, and who can view, change, or open an item.
#[derive(Generic)]
pub struct VNodePermissionsFormPage {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub owner_id: i64,
    pub owner_display: String,
    pub role: String,
    pub permissions: NodePermissions,
    pub apply_inside: bool,
    pub error: String,
}

impl VNodePermissionsFormPage {
    fn crumbs(&self) -> Markup {
        filesystem_item_crumbs(self.id, &self.name, Some("Permissions"))
    }

    fn id_value(id: i64) -> String {
        if id == 0 {
            String::new()
        } else {
            id.to_string()
        }
    }

    fn pane_body(&self) -> Markup {
        let owner_id = Self::id_value(self.owner_id);
        let permissions = self.permissions;
        let ctx = FormCtx::form::<VNodePermissionsForm>(CsrfToken::current())
            .flag(VNodePermissionsFormFlag::IsDirectory, self.is_directory)
            .value(VNodePermissionsFormField::OwnerId, owner_id.as_str())
            .display(
                VNodePermissionsFormField::OwnerId,
                self.owner_display.as_str(),
            )
            .value(VNodePermissionsFormField::Role, self.role.as_str())
            .checked(
                VNodePermissionsFormField::OwnerView,
                permissions.owner_view(),
            )
            .checked(
                VNodePermissionsFormField::OwnerChange,
                permissions.owner_change(),
            )
            .checked(
                VNodePermissionsFormField::OwnerOpen,
                permissions.owner_open(),
            )
            .checked(VNodePermissionsFormField::RoleView, permissions.role_view())
            .checked(
                VNodePermissionsFormField::RoleChange,
                permissions.role_change(),
            )
            .checked(VNodePermissionsFormField::RoleOpen, permissions.role_open())
            .checked(
                VNodePermissionsFormField::OtherView,
                permissions.other_view(),
            )
            .checked(
                VNodePermissionsFormField::OtherChange,
                permissions.other_change(),
            )
            .checked(
                VNodePermissionsFormField::OtherOpen,
                permissions.other_open(),
            )
            .checked(
                VNodePermissionsFormField::AnyoneView,
                permissions.anyone_view(),
            )
            .checked(
                VNodePermissionsFormField::AnyoneChange,
                permissions.anyone_change(),
            )
            .checked(
                VNodePermissionsFormField::AnyoneOpen,
                permissions.anyone_open(),
            )
            .checked(VNodePermissionsFormField::ApplyInside, self.apply_inside);
        let subtitle = format!("Choose who can use \"{}\"", self.name);
        form(
            &CsrfToken::current(),
            FormOpts {
                title: "Permissions",
                subtitle: &subtitle,
                attrs: form_hx_post_main(VNodePermissionsPostRouteTag::new(self.id)),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: VNodePermissionsForm::render_inputs(&ctx),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Save",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for VNodePermissionsFormPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "permissions"),
            self.crumbs(),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for VNodePermissionsFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Permissions {} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "permissions"),
            self.crumbs(),
            self.pane_body(),
        )
    }
}

/// Global permissions applied to the filesystem root.
#[derive(Generic)]
pub struct FilesystemRootPermissionsPage {
    pub owner_id: i64,
    pub owner_display: String,
    pub role: String,
    pub permissions: NodePermissions,
    pub apply_all: bool,
    pub error: String,
}

impl FilesystemRootPermissionsPage {
    fn crumbs(&self) -> Markup {
        let list_url = VNodeListRouteTag.url();
        breadcrumbs(&[
            Crumb {
                label: "Filesystem",
                href: Some(&list_url),
            },
            Crumb {
                label: "Permissions",
                href: None,
            },
        ])
    }

    fn id_value(id: i64) -> String {
        if id == 0 {
            String::new()
        } else {
            id.to_string()
        }
    }

    fn pane_body(&self) -> Markup {
        let owner_id = Self::id_value(self.owner_id);
        let permissions = self.permissions;
        let ctx = FormCtx::form::<VNodePermissionsForm>(CsrfToken::current())
            .flag(VNodePermissionsFormFlag::IsDirectory, false)
            .flag(VNodePermissionsFormFlag::IsRoot, true)
            .value(VNodePermissionsFormField::OwnerId, owner_id.as_str())
            .display(
                VNodePermissionsFormField::OwnerId,
                self.owner_display.as_str(),
            )
            .value(VNodePermissionsFormField::Role, self.role.as_str())
            .checked(
                VNodePermissionsFormField::OwnerView,
                permissions.owner_view(),
            )
            .checked(
                VNodePermissionsFormField::OwnerChange,
                permissions.owner_change(),
            )
            .checked(
                VNodePermissionsFormField::OwnerOpen,
                permissions.owner_open(),
            )
            .checked(VNodePermissionsFormField::RoleView, permissions.role_view())
            .checked(
                VNodePermissionsFormField::RoleChange,
                permissions.role_change(),
            )
            .checked(VNodePermissionsFormField::RoleOpen, permissions.role_open())
            .checked(
                VNodePermissionsFormField::OtherView,
                permissions.other_view(),
            )
            .checked(
                VNodePermissionsFormField::OtherChange,
                permissions.other_change(),
            )
            .checked(
                VNodePermissionsFormField::OtherOpen,
                permissions.other_open(),
            )
            .checked(
                VNodePermissionsFormField::AnyoneView,
                permissions.anyone_view(),
            )
            .checked(
                VNodePermissionsFormField::AnyoneChange,
                permissions.anyone_change(),
            )
            .checked(
                VNodePermissionsFormField::AnyoneOpen,
                permissions.anyone_open(),
            )
            .checked(VNodePermissionsFormField::ApplyInside, false)
            .checked(VNodePermissionsFormField::ApplyAll, self.apply_all);
        form(
            &CsrfToken::current(),
            FormOpts {
                title: "Permissions",
                subtitle: "Choose who can use the filesystem root.",
                attrs: form_hx_post_main(FilesystemRootPermissionsPostRouteTag),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: VNodePermissionsForm::render_inputs(&ctx),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Save",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for FilesystemRootPermissionsPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            main_menu(&FilesystemRootPermissionsGetRouteTag.url()),
            self.crumbs(),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for FilesystemRootPermissionsPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Permissions — Lariv",
            chrome,
            main_menu(&FilesystemRootPermissionsGetRouteTag.url()),
            self.crumbs(),
            self.pane_body(),
        )
    }
}
