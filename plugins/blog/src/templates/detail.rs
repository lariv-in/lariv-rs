use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, Crumb, FieldManyToMany, FieldMarkdown, FieldText, FieldTitle, ShellChrome,
    SidebarMenu, SidebarMenuItem, breadcrumbs, button_modal_form, container_column, container_row,
    detail, field_many_to_many, field_markdown, field_text, field_title, label, sidebar_menu,
    sidebar_menu_item_pane,
};
use lariv_core::template::RenderTemplate;

use crate::keys::{BLOG_EDIT_MODAL, TAG_EDIT_MODAL};
use crate::routes::{
    BlogDetailRouteTag, BlogEditGetRouteTag, BlogEditPostRouteTag, BlogListRouteTag,
    BlogTagsDetailRouteTag, BlogTagsEditGetRouteTag, BlogTagsEditPostRouteTag,
    BlogTagsListRouteTag,
};

use super::{app_scaffold, scaffold_main, scaffold_pane};

pub(crate) fn blog_article_crumbs(id: i64, title: &str, action: Option<&str>) -> Markup {
    let list_url = BlogListRouteTag.url();
    let detail_url = BlogDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Blog",
                href: Some(&list_url),
            },
            Crumb {
                label: "All Articles",
                href: Some(&list_url),
            },
            Crumb {
                label: title,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Blog",
                href: Some(&list_url),
            },
            Crumb {
                label: "All Articles",
                href: Some(&list_url),
            },
            Crumb {
                label: title,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

pub(crate) fn blog_tag_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = BlogTagsListRouteTag.url();
    let detail_url = BlogTagsDetailRouteTag::new(id).url();
    let blog_url = BlogListRouteTag.url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Blog",
                href: Some(&blog_url),
            },
            Crumb {
                label: "Blog Tags",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Blog",
                href: Some(&blog_url),
            },
            Crumb {
                label: "Blog Tags",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

fn blog_detail_menu(blog_id: i64, title: &str, active: &str) -> Markup {
    let menu_title = format!("Article: {title}");
    let detail_url = BlogDetailRouteTag::new(blog_id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Article Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
        },
    })
}

fn tag_detail_menu(tag_id: i64, name: &str, active: &str) -> Markup {
    let menu_title = format!("Tag: {name}");
    let detail_url = BlogTagsDetailRouteTag::new(tag_id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Tag Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
        },
    })
}

#[derive(Generic)]
pub struct BlogDetailPage {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub author_name: String,
    pub tags: Vec<(i64, String)>,
    pub content: String,
}

impl BlogDetailPage {
    fn pane_body(&self) -> Markup {
        let tag_pairs: Vec<(String, String)> = self
            .tags
            .iter()
            .map(|(id, name)| (name.clone(), BlogTagsDetailRouteTag::new(*id).url()))
            .collect();
        let tag_items: Vec<(&str, Option<&str>)> = tag_pairs
            .iter()
            .map(|(name, href)| (name.as_str(), Some(href.as_str())))
            .collect();
        let edit_get = BlogEditGetRouteTag::new(self.id).url();
        let edit_post = BlogEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.title,
                        classes: "",
                    }))
                    (label("Slug", field_text(FieldText {
                        value: &self.slug,
                        classes: "",
                    })))
                    (label("Description", field_text(FieldText {
                        value: &self.description,
                        classes: "",
                    })))
                    (label("Author", field_text(FieldText {
                        value: &self.author_name,
                        classes: "",
                    })))
                    (label("Tags", field_many_to_many(FieldManyToMany {
                        items: &tag_items,
                        classes: "",
                    })))
                    (field_markdown(FieldMarkdown {
                        value: &self.content,
                        classes: "mt-4",
                    }))
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_blog.BlogEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: BLOG_EDIT_MODAL.id(),
                            label: "Edit",
                            classes: "btn-outline",
                            ..Default::default()
                        }))
                    }))
                },
            ))
        })
    }
}

impl lariv_core::template::RenderAppPane for BlogDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            blog_detail_menu(self.id, &self.title, "detail"),
            blog_article_crumbs(self.id, &self.title, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(
            blog_article_crumbs(self.id, &self.title, None),
            self.pane_body(),
        )
    }
}

impl RenderTemplate for BlogDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.title),
            chrome,
            blog_detail_menu(self.id, &self.title, "detail"),
            blog_article_crumbs(self.id, &self.title, None),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct TagDetailPage {
    pub id: i64,
    pub name: String,
    pub blogs: Vec<(i64, String)>,
}

impl TagDetailPage {
    fn pane_body(&self) -> Markup {
        let blog_pairs: Vec<(String, String)> = self
            .blogs
            .iter()
            .map(|(id, title)| (title.clone(), BlogDetailRouteTag::new(*id).url()))
            .collect();
        let blog_items: Vec<(&str, Option<&str>)> = blog_pairs
            .iter()
            .map(|(title, href)| (title.as_str(), Some(href.as_str())))
            .collect();
        let edit_get = BlogTagsEditGetRouteTag::new(self.id).url();
        let edit_post = BlogTagsEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (label("Articles", field_many_to_many(FieldManyToMany {
                        items: &blog_items,
                        classes: "",
                    })))
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_blog.TagEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: TAG_EDIT_MODAL.id(),
                            label: "Edit",
                            classes: "btn-outline",
                            ..Default::default()
                        }))
                    }))
                },
            ))
        })
    }
}

impl lariv_core::template::RenderAppPane for TagDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            tag_detail_menu(self.id, &self.name, "detail"),
            blog_tag_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(blog_tag_crumbs(self.id, &self.name, None), self.pane_body())
    }
}

impl RenderTemplate for TagDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            tag_detail_menu(self.id, &self.name, "detail"),
            blog_tag_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
}
