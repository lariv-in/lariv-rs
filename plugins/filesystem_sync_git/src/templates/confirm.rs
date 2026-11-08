use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FieldText, FormOpts, Modal, ShellChrome, button_submit, field_text, form,
    form_hx_post_selector, label, modal,
};
use lariv_core::html_form::CsrfToken;
use lariv_core::template::RenderTemplate;

use crate::keys::GIT_SYNC_CONFIRM_MODAL;
use crate::routes::{GitSyncConfirmPostRouteTag, GitSyncRunPostRouteTag};

#[derive(Generic)]
pub struct GitSyncConfirmPage {
    pub directory_id: i64,
    pub directory_name: String,
    pub git_url: String,
    pub git_identifier: String,
    pub subpath: String,
    /// Empty when this directory has never synced.
    pub last_synced_commit_hash: String,
    /// Set after a fetch, including a skipped sync.
    pub resolved_commit: String,
    pub force: bool,
    pub notice: String,
    pub error: String,
}

impl RenderTemplate for GitSyncConfirmPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", GIT_SYNC_CONFIRM_MODAL.id());
        let post = if self.force {
            GitSyncConfirmPostRouteTag::new(self.directory_id).url()
        } else {
            GitSyncRunPostRouteTag::new(self.directory_id).url()
        };
        let subpath = if self.subpath.is_empty() {
            "(repository root)"
        } else {
            self.subpath.as_str()
        };
        let last_synced = if self.last_synced_commit_hash.is_empty() {
            "(never synced)"
        } else {
            self.last_synced_commit_hash.as_str()
        };
        let title = if self.force { "Force Sync" } else { "Git Sync" };
        let submit = if self.force {
            "Replace directory contents"
        } else {
            "Sync"
        };
        modal(Modal {
            uid: GIT_SYNC_CONFIRM_MODAL.id(),
            children: html! {
                h2 class="text-xl font-bold" { (title) }
                @if self.force {
                    p class="my-2" {
                        "Confirming deletes the existing data in this directory and overwrites it with the git tree. Directories with the same name are kept and their contents are updated. Files with the same name are overwritten in place. Everything else in the directory is deleted."
                    }
                } @else {
                    p class="my-2" {
                        "This updates the directory only when the resolved commit differs from the last synced commit. Directories with the same name are kept and their contents are updated. Files with the same name are overwritten in place. Everything else in the directory is deleted."
                    }
                }
                @if !self.notice.is_empty() {
                    p class="my-2" { (self.notice) }
                }
                (label("Directory", field_text(FieldText { value: &self.directory_name, classes: "" })))
                (label("Git URL", field_text(FieldText { value: &self.git_url, classes: "" })))
                (label("Git identifier", field_text(FieldText { value: &self.git_identifier, classes: "" })))
                (label("Subpath", field_text(FieldText { value: subpath, classes: "" })))
                (label("Last synced commit", field_text(FieldText { value: last_synced, classes: "" })))
                @if !self.resolved_commit.is_empty() {
                    (label("Resolved commit", field_text(FieldText { value: &self.resolved_commit, classes: "" })))
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_selector(&post, &target),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: html! {},
                    actions: html! {
                        (button_submit(ButtonSubmit { label: submit, ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
            ..Default::default()
        })
    }
}
