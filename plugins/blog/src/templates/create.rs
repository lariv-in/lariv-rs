use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ManyToManyItem, ShellChrome, button_submit, container_row, form,
    form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_query, modal_create_post_url};

use crate::forms::{BlogForm, BlogFormField, TagForm, TagFormField};
use crate::keys::{BLOG_CREATE_MODAL, TAG_CREATE_MODAL};
use crate::routes::{
    BlogCreatePostRouteTag, BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag,
};

pub const TAG_CREATE: CreateModal<BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag> =
    CreateModal {
        key: TAG_CREATE_MODAL,
        form_name: "p_blog.TagCreateForm",
        get: BlogTagsCreateGetRouteTag,
        post: BlogTagsCreatePostRouteTag,
    };

#[derive(Generic)]
pub struct BlogCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub created_by_id: i64,
    pub author_display: String,
    pub tags: Vec<ManyToManyItem>,
    pub content: String,
    pub error: String,
}

impl RenderTemplate for BlogCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_blog.BlogCreateForm"
        } else {
            self.form_name.as_str()
        };
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
            BLOG_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Article",
                    subtitle: "Publish a new article",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        BLOG_CREATE_MODAL,
                        &modal_create_post_url(
                            BlogCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: BlogForm::render_inputs(&ctx),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Article",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

#[derive(Generic)]
pub struct TagCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub name: String,
    pub error: String,
}

impl RenderTemplate for TagCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_blog.TagCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            TAG_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Tag",
                    subtitle: "Create a new blog tag",
                    attrs: lariv_core::components::swap::form_hx_post_for_url(
                        TAG_CREATE_MODAL,
                        &modal_create_post_query(
                            BlogTagsCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                            &self.target_input,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TagForm::render_inputs(
                        &FormCtx::form::<TagForm>(CsrfToken::current())
                            .value(TagFormField::Name, self.name.as_str()),
                    ),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Tag",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
