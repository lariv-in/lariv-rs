use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonModalForm, FieldText, FieldTitle, ShellChrome, button_modal_form, container_column,
    container_row, detail, field_text, field_title, label,
};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_plugin_filesystem::routes::VNodeDetailRouteTag;

use crate::keys::{DOCUMENT_DELETE_MODAL, DOCUMENT_EDIT_MODAL};
use crate::routes::{
    DocumentDeleteGetRouteTag, DocumentDeletePostRouteTag, DocumentEditGetRouteTag,
    DocumentEditPostRouteTag,
};

use super::{app_scaffold, document_crumbs, document_detail_menu, scaffold_main, scaffold_pane};

#[derive(Generic)]
pub struct DocumentDetailPage {
    pub id: i64,
    pub document_type: String,
    pub name: String,
    pub vnode_id: i64,
    pub vnode_name: String,
    pub aadhar_number: String,
    pub pan_number: String,
    pub passport_number: String,
    pub gender: String,
    pub date_of_birth: String,
    pub address: String,
    pub nationality: String,
    pub expiry_date: String,
    pub error: String,
    pub extra_actions: String,
}

impl DocumentDetailPage {
    fn title(&self) -> &str {
        if self.name.is_empty() {
            "Document"
        } else {
            &self.name
        }
    }

    pub(crate) fn body(&self) -> Markup {
        let file_url = VNodeDetailRouteTag::new(self.vnode_id).url();
        let file = if self.vnode_id > 0 {
            html! { a class="link" href=(file_url) { (self.vnode_name) } }
        } else {
            field_text(FieldText {
                value: &self.vnode_name,
                classes: "",
            })
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: self.title(), classes: "" }))
                    @if !self.error.is_empty() {
                        div class="alert alert-error" { (self.error) }
                    }
                    (label("Document type", field_text(FieldText { value: &self.document_type, classes: "" })))
                    (label("Document file", file))
                    @if !self.aadhar_number.is_empty() {
                        (label("Aadhar number", field_text(FieldText { value: &self.aadhar_number, classes: "" })))
                    }
                    @if !self.pan_number.is_empty() {
                        (label("PAN", field_text(FieldText { value: &self.pan_number, classes: "" })))
                    }
                    @if !self.passport_number.is_empty() {
                        (label("Passport number", field_text(FieldText { value: &self.passport_number, classes: "" })))
                    }
                    (label("Name", field_text(FieldText { value: &self.name, classes: "" })))
                    @if !self.gender.is_empty() {
                        (label("Gender", field_text(FieldText { value: &self.gender, classes: "" })))
                    }
                    (label("Date of birth", field_text(FieldText { value: &self.date_of_birth, classes: "" })))
                    @if !self.address.is_empty() {
                        (label("Address", field_text(FieldText { value: &self.address, classes: "" })))
                    }
                    @if !self.nationality.is_empty() {
                        (label("Nationality", field_text(FieldText { value: &self.nationality, classes: "" })))
                    }
                    @if !self.expiry_date.is_empty() {
                        (label("Expiry date", field_text(FieldText { value: &self.expiry_date, classes: "" })))
                    }
                    @if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<crate::routes::DocumentsMutate>()) && self.error.is_empty() {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_documents.DocumentEditForm",
                                href: &DocumentEditGetRouteTag::new(self.id).url(),
                                form_post_url: &DocumentEditPostRouteTag::new(self.id).path(),
                                modal_uid: DOCUMENT_EDIT_MODAL.id(),
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                            (PreEscaped(self.extra_actions.clone()))
                        }))
                    } @else if lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<crate::routes::DocumentsMutate>()) {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                label: "Delete",
                                icon_name: Some("trash"),
                                name: "p_documents.DocumentDeleteForm",
                                href: &DocumentDeleteGetRouteTag::new(self.id).url(),
                                form_post_url: &DocumentDeletePostRouteTag::new(self.id).path(),
                                modal_uid: DOCUMENT_DELETE_MODAL.id(),
                                classes: "btn-error",
                                ..Default::default()
                            }))
                            (PreEscaped(self.extra_actions.clone()))
                        }))
                    } @else if !self.extra_actions.is_empty() {
                        (container_row("flex gap-2 mt-4", html! {
                            (PreEscaped(self.extra_actions.clone()))
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for DocumentDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = document_crumbs(self.id, &self.name, None);
        scaffold_pane(
            document_detail_menu(self.id, &self.name),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(document_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for DocumentDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = document_crumbs(self.id, &self.name, None);
        app_scaffold(
            "Document — Lariv",
            chrome,
            document_detail_menu(self.id, &self.name),
            crumbs,
            self.body(),
        )
    }
}
