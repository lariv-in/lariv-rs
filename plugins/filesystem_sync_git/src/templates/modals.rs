use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, delete_confirmation, form, form_hx_post_selector, form_hx_post_url, modal,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query, modal_edit_post_url};

use crate::forms::{GitSyncForm, GitSyncFormField};
use crate::keys::{GIT_SYNC_CREATE_MODAL, GIT_SYNC_DELETE_MODAL, GIT_SYNC_EDIT_MODAL};
use crate::routes::{
    GitSyncCreateGetRouteTag, GitSyncCreatePostRouteTag, GitSyncDeletePostRouteTag,
    GitSyncEditPostRouteTag,
};

pub const GIT_SYNC_CREATE: CreateModal<GitSyncCreateGetRouteTag, GitSyncCreatePostRouteTag> =
    CreateModal {
        key: GIT_SYNC_CREATE_MODAL,
        form_name: "p_filesystem_sync_git.GitSyncForm",
        get: GitSyncCreateGetRouteTag,
        post: GitSyncCreatePostRouteTag,
    };

#[derive(Generic)]
pub struct GitSyncCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub git_url: String,
    pub git_identifier: String,
    pub subpath: String,
    pub directory_vnode_id: String,
    pub directory: String,
    pub error: String,
}

fn form_inputs(page: &GitSyncCreateModalPage) -> Markup {
    GitSyncForm::render_inputs(
        &FormCtx::form::<GitSyncForm>(CsrfToken::current())
            .value(GitSyncFormField::GitUrl, &page.git_url)
            .value(GitSyncFormField::GitIdentifier, &page.git_identifier)
            .value(GitSyncFormField::Subpath, &page.subpath)
            .value(GitSyncFormField::DirectoryVnodeId, &page.directory_vnode_id)
            .display(GitSyncFormField::DirectoryVnodeId, &page.directory),
    )
}

impl RenderTemplate for GitSyncCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            GIT_SYNC_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New git sync" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(GIT_SYNC_CREATE_MODAL, &modal_create_post_query(
                        GitSyncCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: form_inputs(self),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct GitSyncEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub git_url: String,
    pub git_identifier: String,
    pub subpath: String,
    pub directory_vnode_id: String,
    pub directory: String,
    pub error: String,
}

impl RenderTemplate for GitSyncEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = crate::routes::GitSyncDeleteGetRouteTag::new(self.id).url();
        let values = GitSyncCreateModalPage {
            form_name: self.form_name.clone(),
            refresh_table: String::new(),
            target_input: String::new(),
            git_url: self.git_url.clone(),
            git_identifier: self.git_identifier.clone(),
            subpath: self.subpath.clone(),
            directory_vnode_id: self.directory_vnode_id.clone(),
            directory: self.directory.clone(),
            error: self.error.clone(),
        };
        modal_keyed(
            GIT_SYNC_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit git sync" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(GIT_SYNC_EDIT_MODAL, &modal_edit_post_url(
                        GitSyncEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: form_inputs(&values),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_filesystem_sync_git.GitSyncDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: GIT_SYNC_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct GitSyncConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for GitSyncConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        modal(lariv_core::components::Modal {
            uid: &self.modal_uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(
                    &GitSyncDeletePostRouteTag::new(self.id).url(),
                    &target,
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
