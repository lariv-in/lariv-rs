use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, Crumb, DetailHeader, FieldText, ShellChrome, breadcrumbs, button_modal_form,
    button_modal_route, container_column, detail, detail_header, field_text, label,
};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_plugin_filesystem::routes::{VNodeDetailRouteTag, VNodeListRouteTag};
use lariv_plugin_filesystem::templates::vnode_menu;

use crate::keys::{GIT_SYNC_CREATE_MODAL, GIT_SYNC_EDIT_MODAL};
use crate::routes::{
    GitSyncConfirmGetRouteTag, GitSyncCreateForGetRouteTag, GitSyncCreatePostRouteTag,
    GitSyncEditGetRouteTag, GitSyncEditPostRouteTag, GitSyncRunGetRouteTag,
};

use super::{app_scaffold, scaffold_main, scaffold_pane};

fn crumbs(id: i64, directory: &str) -> Markup {
    let list_url = VNodeListRouteTag.url();
    let detail_url = VNodeDetailRouteTag::new(id).url();
    breadcrumbs(&[
        Crumb {
            label: "Filesystem",
            href: Some(&list_url),
        },
        Crumb {
            label: directory,
            href: Some(&detail_url),
        },
        Crumb {
            label: "Git Sync",
            href: None,
        },
    ])
}

#[derive(Generic)]
pub struct GitSyncDetailPage {
    /// Directory vnode id. This is also the git sync primary key.
    pub id: i64,
    pub configured: bool,
    pub git_url: String,
    pub git_identifier: String,
    pub last_synced: String,
    pub subpath: String,
    pub directory: String,
}

impl GitSyncDetailPage {
    fn body(&self) -> Markup {
        let edit_get = GitSyncEditGetRouteTag::new(self.id).url();
        let edit_post = GitSyncEditPostRouteTag::new(self.id).path();
        let create_get = GitSyncCreateForGetRouteTag::new(self.id).url();
        let create_post = GitSyncCreatePostRouteTag.url();
        let directory_url = VNodeDetailRouteTag::new(self.id).url();
        let subpath = if self.subpath.is_empty() {
            "(repository root)".to_string()
        } else {
            self.subpath.clone()
        };
        detail(html! {
            (container_column("", html! {
                (detail_header(DetailHeader {
                    title: &self.directory,
                    actions: html! {
                        @if self.configured {
                            (button_modal_route(
                                GitSyncRunGetRouteTag::new(self.id),
                                "Git Sync",
                                "btn-outline",
                            ))
                            (button_modal_route(
                                GitSyncConfirmGetRouteTag::new(self.id),
                                "Force Sync",
                                "btn-outline",
                            ))
                            (button_modal_form(ButtonModalForm {
                                name: "p_filesystem_sync_git.GitSyncForm",
                                href: &edit_get,
                                form_post_url: &edit_post,
                                modal_uid: GIT_SYNC_EDIT_MODAL.id(),
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        } @else {
                            (button_modal_form(ButtonModalForm {
                                name: "p_filesystem_sync_git.GitSyncForm",
                                href: &create_get,
                                form_post_url: &create_post,
                                modal_uid: GIT_SYNC_CREATE_MODAL.id(),
                                label: "Create",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                        }
                    },
                }))
                (label("Directory", html! {
                    a href=(directory_url) { (self.directory) }
                }))
                @if !self.configured {
                    p class="my-2" { "This directory has no git sync yet." }
                }
                (label("Git URL", field_text(FieldText { value: &self.git_url, classes: "" })))
                (label("Git identifier", field_text(FieldText { value: &self.git_identifier, classes: "" })))
                (label("Last synced commit", field_text(FieldText { value: &self.last_synced, classes: "" })))
                (label("Subpath", field_text(FieldText { value: &subpath, classes: "" })))
            }))
        })
    }
}

impl RenderAppPane for GitSyncDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            vnode_menu(self.id, &self.directory, true, "git-sync"),
            crumbs(self.id, &self.directory),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(crumbs(self.id, &self.directory), self.body())
    }
}

impl RenderTemplate for GitSyncDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Git Sync — Lariv",
            chrome,
            vnode_menu(self.id, &self.directory, true, "git-sync"),
            crumbs(self.id, &self.directory),
            self.body(),
        )
    }
}
