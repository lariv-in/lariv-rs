use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ManyToManyItem, ShellChrome,
    button_modal_form, button_submit, form, form_hx_post_url, modal, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::forms::{BlogForm, BlogFormField, TagForm, TagFormField};
use crate::keys::{BLOG_DELETE_MODAL, BLOG_EDIT_MODAL, TAG_DELETE_MODAL, TAG_EDIT_MODAL};
use crate::routes::{
    BlogDeleteGetRouteTag, BlogDeletePostRouteTag, BlogEditPostRouteTag, BlogTagsDeleteGetRouteTag,
    BlogTagsDeletePostRouteTag, BlogTagsEditPostRouteTag,
};

/// Edit article modal. Create uses [`super::create::BlogCreateModalPage`].
#[derive(Generic)]
pub struct BlogEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub created_by_id: i64,
    pub author_display: String,
    pub tags: Vec<ManyToManyItem>,
    pub content: String,
    pub error: String,
}

impl RenderTemplate for BlogEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = BlogDeleteGetRouteTag::new(self.id).url();
        let created_by_id_s = if self.created_by_id == 0 {
            String::new()
        } else {
            self.created_by_id.to_string()
        };
        let ctx = FormCtx::form::<BlogForm>(CsrfToken::current())
            .value(BlogFormField::Title, self.title.as_str())
            .value(BlogFormField::Slug, self.slug.as_str())
            .value(BlogFormField::Description, self.description.as_str())
            .value(BlogFormField::CreatedById, created_by_id_s.as_str())
            .display(BlogFormField::CreatedById, self.author_display.as_str())
            .m2m(BlogFormField::Tags, &self.tags)
            .value(BlogFormField::Content, self.content.as_str());
        modal_keyed(
            BLOG_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit article" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(BLOG_EDIT_MODAL, &modal_edit_post_url(
                        BlogEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: BlogForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_blog.BlogDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: BLOG_DELETE_MODAL.id(),
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

/// Edit tag modal. Create uses [`super::create::TagCreateModalPage`].
#[derive(Generic)]
pub struct TagEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for TagEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = BlogTagsDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<TagForm>(CsrfToken::current())
            .value(TagFormField::Name, self.name.as_str());
        modal_keyed(
            TAG_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit tag" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(TAG_EDIT_MODAL, &modal_edit_post_url(
                        BlogTagsEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TagForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_blog.TagDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: TAG_DELETE_MODAL.id(),
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
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", BLOG_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            BLOG_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.modal_uid == TAG_DELETE_MODAL.id() {
            BlogTagsDeletePostRouteTag::new(self.id).url()
        } else {
            BlogDeletePostRouteTag::new(self.id).url()
        };
        modal(lariv_core::components::Modal {
            uid,
            children: lariv_core::components::delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: lariv_core::components::form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
