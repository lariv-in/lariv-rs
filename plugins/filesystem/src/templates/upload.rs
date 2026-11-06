use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_create_post_url;

use crate::forms::{
    VNodeMultiUploadForm, VNodeMultiUploadFormField, VNodeZipUploadForm, VNodeZipUploadFormField,
};
use crate::keys::{V_NODE_MULTI_UPLOAD_MODAL, V_NODE_ZIP_UPLOAD_MODAL};
use crate::routes::{
    VNodeUploadPostInRouteTag, VNodeUploadPostRouteTag, VNodeZipUploadPostInRouteTag,
    VNodeZipUploadPostRouteTag,
};

/// Multi-file upload modal.
#[derive(Generic)]
pub struct VNodeMultiUploadModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeMultiUploadModalPage {
    fn post_url(&self, form_name: &str) -> String {
        if self.parent_id == 0 {
            modal_create_post_url(VNodeUploadPostRouteTag, form_name, &self.refresh_table)
        } else {
            modal_create_post_url(
                VNodeUploadPostInRouteTag::new(self.parent_id),
                form_name,
                &self.refresh_table,
            )
        }
    }
}

impl RenderTemplate for VNodeMultiUploadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_filesystem.VNodeMultiUploadForm"
        } else {
            self.form_name.as_str()
        };
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let ctx = FormCtx::form::<VNodeMultiUploadForm>(CsrfToken::current())
            .value(VNodeMultiUploadFormField::ParentId, parent_id_s.as_str())
            .display(
                VNodeMultiUploadFormField::ParentId,
                self.parent_display.as_str(),
            );
        modal_keyed(
            V_NODE_MULTI_UPLOAD_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Bulk Upload",
                    subtitle: "Upload multiple files at once",
                    attrs: form_hx_post_url(V_NODE_MULTI_UPLOAD_MODAL, &self.post_url(form_name))
                        .set("hx-encoding", "multipart/form-data"),
                    enctype: Some("multipart/form-data"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: VNodeMultiUploadForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Upload",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

/// Zip-upload modal: replaces the target directory's contents.
#[derive(Generic)]
pub struct VNodeZipUploadModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeZipUploadModalPage {
    fn post_url(&self, form_name: &str) -> String {
        if self.parent_id == 0 {
            modal_create_post_url(VNodeZipUploadPostRouteTag, form_name, &self.refresh_table)
        } else {
            modal_create_post_url(
                VNodeZipUploadPostInRouteTag::new(self.parent_id),
                form_name,
                &self.refresh_table,
            )
        }
    }
}

impl RenderTemplate for VNodeZipUploadModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_filesystem.VNodeZipUploadForm"
        } else {
            self.form_name.as_str()
        };
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let ctx = FormCtx::form::<VNodeZipUploadForm>(CsrfToken::current())
            .value(VNodeZipUploadFormField::ParentId, parent_id_s.as_str())
            .display(
                VNodeZipUploadFormField::ParentId,
                self.parent_display.as_str(),
            );
        modal_keyed(
            V_NODE_ZIP_UPLOAD_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Upload Zip",
                    subtitle: "Replaces the contents of the destination folder",
                    attrs: form_hx_post_url(V_NODE_ZIP_UPLOAD_MODAL, &self.post_url(form_name))
                        .set("hx-encoding", "multipart/form-data"),
                    enctype: Some("multipart/form-data"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: VNodeZipUploadForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Upload",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
