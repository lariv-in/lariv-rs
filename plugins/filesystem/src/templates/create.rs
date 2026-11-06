use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, container_column, form, form_hx_post_url,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_create_post_query;

use crate::create_modals::V_NODE_CREATE;
use crate::forms::{VNodeForm, VNodeFormField, VNodeFormFlag, VNodeKind, VNodeKindField};
use crate::keys::V_NODE_CREATE_MODAL;
use crate::routes::{VNodeCreatePostInRouteTag, VNodeCreatePostRouteTag};

#[derive(Generic)]
pub struct VNodeCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub is_directory: bool,
    pub parent_id: i64,
    pub parent_display: String,
    pub error: String,
}

impl VNodeCreateModalPage {
    fn post_url(&self, form_name: &str) -> String {
        if self.parent_id == 0 {
            modal_create_post_query(
                VNodeCreatePostRouteTag,
                form_name,
                &self.refresh_table,
                &self.target_input,
            )
        } else {
            modal_create_post_query(
                VNodeCreatePostInRouteTag::new(self.parent_id),
                form_name,
                &self.refresh_table,
                &self.target_input,
            )
        }
    }

    fn inputs(&self) -> Markup {
        let parent_id_s = if self.parent_id == 0 {
            String::new()
        } else {
            self.parent_id.to_string()
        };
        let parent_val = if self.parent_id == 0 {
            ""
        } else {
            parent_id_s.as_str()
        };
        let ctx = FormCtx::form::<VNodeForm>(CsrfToken::current())
            .value(VNodeFormField::Name, self.name.as_str())
            .flag(VNodeFormFlag::CreateMode, true)
            .kind::<VNodeKind>("File")
            .value(VNodeFormField::ParentId, parent_val)
            .display(VNodeFormField::ParentId, self.parent_display.as_str())
            .label(VNodeKindField::File, "File");
        container_column("", VNodeForm::render_inputs(&ctx))
    }
}

impl RenderTemplate for VNodeCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            V_NODE_CREATE.form_name
        } else {
            self.form_name.as_str()
        };
        let post_url = self.post_url(form_name);
        modal_keyed(
            V_NODE_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Item",
                    subtitle: "Add a new file or folder",
                    classes: "@container",
                    attrs: form_hx_post_url(V_NODE_CREATE_MODAL, &post_url)
                        .set("hx-encoding", "multipart/form-data"),
                    enctype: Some("multipart/form-data"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: self.inputs(),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Save",
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
