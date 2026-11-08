use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonLink, ButtonModalForm, ButtonSubmit, CodeEditorInput, DetailHeader, FieldText, FormOpts,
    MAIN_CONTENT, ShellChrome, button_link, button_modal_form, button_modal_route, button_submit,
    code_editor_input, container_column, detail, detail_header, field_text, form, form_hx_post_url,
    label,
};
use lariv_core::html_form::CsrfToken;
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::keys::V_NODE_EDIT_MODAL;
use crate::routes::{
    VNodeBrowseRouteTag, VNodeContentPostRouteTag, VNodeDownloadRouteTag, VNodeEditGetRouteTag,
    VNodeEditPostRouteTag, VNodePdfModalRouteTag,
};

use super::{app_scaffold, filesystem_item_crumbs, scaffold_main, scaffold_pane, vnode_menu};

#[derive(Generic)]
pub struct VNodeDetailPage {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub item_type: String,
    pub size_display: String,
    pub items_display: String,
    pub path: String,
    pub updated_at: String,
    pub text_content: Option<String>,
    pub save_error: String,
    /// Markup from [`crate::detail_actions`] contributors, already rendered.
    pub header_extra: String,
}

impl VNodeDetailPage {
    fn pane_body(&self) -> Markup {
        let browse_url = VNodeBrowseRouteTag::new(self.id).url();
        let edit_get = VNodeEditGetRouteTag::new(self.id).url();
        let edit_post = VNodeEditPostRouteTag::new(self.id).path();
        let content_post = VNodeContentPostRouteTag::new(self.id).path();
        let show_pdf = crate::node::is_typst_file(&self.name, self.is_directory);
        let header_actions = html! {
            @if show_pdf {
                (button_modal_route(VNodePdfModalRouteTag::new(self.id), "PDF", "btn-outline"))
            }
            (button_modal_form(ButtonModalForm {
                name: "p_filesystem.VNodeEditForm",
                href: &edit_get,
                form_post_url: &edit_post,
                modal_uid: V_NODE_EDIT_MODAL.id(),
                label: "Edit",
                classes: "btn-outline",
                ..Default::default()
            }))
            @if self.is_directory {
                (button_link(ButtonLink {
                    label: "Browse Contents",
                    href: &browse_url,
                    icon_name: Some("folder-open"),
                    ..Default::default()
                }))
            } @else {
                (lariv_core::components::button_download_route(VNodeDownloadRouteTag::new(self.id),
                    "Download", "",
                ))
            }
            @if !self.header_extra.is_empty() {
                (maud::PreEscaped(self.header_extra.as_str()))
            }
        };
        detail(html! {
            (container_column(
                "",
                html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions: header_actions,
                    }))
                    (label("Type", field_text(FieldText {
                        value: &self.item_type,
                        classes: "",
                    })))
                    (label("Size", field_text(FieldText {
                        value: &self.size_display,
                        classes: "",
                    })))
                    @if self.is_directory {
                        (label("Items", field_text(FieldText {
                            value: &self.items_display,
                            classes: "",
                        })))
                    }
                    (label("Path", field_text(FieldText {
                        value: &self.path,
                        classes: "",
                    })))
                    (label("Modified", field_text(FieldText {
                        value: &self.updated_at,
                        classes: "",
                    })))
                    @if let Some(content) = self.text_content.as_deref() {
                        (form(&CsrfToken::current(), FormOpts {
                            attrs: form_hx_post_url(MAIN_CONTENT, &content_post),
                            form_error: Some(self.save_error.as_str()).filter(|e| !e.is_empty()),
                            inputs: container_column(
                                "",
                                code_editor_input(CodeEditorInput {
                                    label: "Contents",
                                    name: "Content",
                                    value: content,
                                    language: crate::node::editor_language(&self.name),
                                    rows: 20,
                                    ..Default::default()
                                }),
                            ),
                            actions: html! {
                                (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                            },
                            ..Default::default()
                        }))
                    }
                },
            ))
        })
    }
}

impl RenderAppPane for VNodeDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "detail"),
            filesystem_item_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            filesystem_item_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}

impl RenderTemplate for VNodeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "detail"),
            filesystem_item_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}
