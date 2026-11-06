use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, container_column, form, form_hx_post_url, modal, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, csrf_hidden_field};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::forms::{VNodeEditForm, VNodeEditFormField, VNodeEditFormFlag};
use crate::keys::{V_NODE_BULK_DELETE_MODAL, V_NODE_DELETE_MODAL, V_NODE_EDIT_MODAL};
use crate::routes::{
    VNodeBulkDeletePostRouteTag, VNodeDeleteGetRouteTag, VNodeDeletePostRouteTag,
    VNodeEditPostRouteTag,
};

/// Edit modal for filesystem nodes. Create uses [`VNodeCreateModalPage`].
#[derive(Generic)]
pub struct VNodeEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub is_directory: bool,
    pub has_file: bool,
    pub error: String,
}

impl RenderTemplate for VNodeEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = VNodeDeleteGetRouteTag::new(self.id).url();
        let file_label = if self.has_file {
            "Replace File"
        } else {
            "File"
        };
        let show_file_field = !self.is_directory;
        let ctx = FormCtx::form::<VNodeEditForm>(CsrfToken::current())
            .value(VNodeEditFormField::Name, self.name.as_str())
            .flag(VNodeEditFormFlag::ShowFile, show_file_field)
            .label(VNodeEditFormField::File, file_label);
        let inputs = VNodeEditForm::render_inputs(&ctx);
        modal_keyed(
            V_NODE_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit item" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(V_NODE_EDIT_MODAL, &modal_edit_post_url(
                        VNodeEditPostRouteTag::new(self.id),
                        &self.form_name,
                    ))
                    .set("hx-encoding", "multipart/form-data"),
                    enctype: Some("multipart/form-data"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: container_column("", inputs),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_filesystem.VNodeDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: V_NODE_DELETE_MODAL.id(),
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

/// Confirmation modal for deleting a filesystem node.
#[derive(Generic)]
pub struct VNodeConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for VNodeConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", V_NODE_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            V_NODE_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        modal(lariv_core::components::Modal {
            uid,
            children: lariv_core::components::delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: lariv_core::components::form_hx_post_selector(
                    &VNodeDeletePostRouteTag::new(self.id).url(),
                    &target,
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

/// Confirmation modal for bulk-deleting filesystem nodes.
#[derive(Generic)]
pub struct VNodeConfirmBulkDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub ids: String,
    pub return_to: String,
    pub error: String,
    pub can_submit: bool,
}

impl RenderTemplate for VNodeConfirmBulkDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", V_NODE_BULK_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            V_NODE_BULK_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = VNodeBulkDeletePostRouteTag.url();
        let form_attrs = lariv_core::components::form_hx_post_selector(&post_url, &target);
        modal(lariv_core::components::Modal {
            uid,
            children: html! {
                div class="container mx-auto" {
                    h2 class="text-xl font-bold text-error" { "Confirm Deletion" }
                    p class="my-2" { (self.message) }
                    @if !self.error.is_empty() {
                        div class="alert alert-error my-2 text-sm" { (self.error) }
                    }
                    @if self.can_submit {
                        (PreEscaped(format!(
                            r#"<form class="flex flex-col gap-2 my-4"{}>"#,
                            form_attrs.as_string(),
                        )))
                        (csrf_hidden_field(&CsrfToken::current()))
                        input type="hidden" name="ids" value=(self.ids);
                        input type="hidden" name="return" value=(self.return_to);
                        div class="my-2" {
                            (button_submit(ButtonSubmit {
                                label: "Confirm Delete",
                                classes: "btn-error my-2",
                                ..Default::default()
                            }))
                        }
                        (PreEscaped("</form>"))
                    }
                }
            },
            ..Default::default()
        })
    }
}
