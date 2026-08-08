//! Maud page templates for blog and tag CRUD views.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, FieldManyToMany,
        FieldMarkdown, FieldText, FieldTitle, FormOpts, LayoutMain, LayoutSidebar, ManyToManyItem,
        ObjectList, PaginationPage, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuItem,
        SidebarNavLink, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, breadcrumbs, button_clear, button_modal_form, button_submit,
        column_sort_url, container_column, container_row, data_table_list_refresh, detail,
        field_many_to_many, field_markdown, field_text, field_title, form, form_hx_get_route,
        form_hx_post_main, form_hx_post_url, label_inline, layout_main, layout_sidebar, modal,
        modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select_multi,
        shell_scaffold, sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane,
        sort_indicator, table_button_filter, table_pagination,
    },
    capability::define_register_items,
    html_form::{FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    picker::RenderPickerSelect,
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::modal_create_post_url,
};

use super::forms::{
    BlogForm, BlogFormField, BlogTitleFilterForm, BlogTitleFilterFormField, TagForm, TagFormField,
    TagNameFilterForm, TagNameFilterFormField,
};
use super::keys::{
    BlogDeleteModalKey, BlogCreateModalKey, BlogTableKey, TagCreateModalKey, TagDeleteModalKey, TagSelectModalKey,
    TagSelectTableKey, TagTableKey,
};
use super::routes::{
    BlogCreateGetRouteTag, BlogCreatePostRouteTag, BlogDeleteGetRouteTag, BlogDeletePostRouteTag,
    BlogDetailRouteTag, BlogEditGetRouteTag, BlogEditPostRouteTag, BlogListRouteTag,
    BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag, BlogTagsDeleteGetRouteTag,
    BlogTagsDeletePostRouteTag, BlogTagsDetailRouteTag, BlogTagsEditGetRouteTag,
    BlogTagsEditPostRouteTag, BlogTagsListRouteTag, BlogTagsSelectRouteTag,
};

define_register_items! {
    plugin: BlogTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        BlogListIdx: BlogListPageTag => BlogListPage,
        BlogDetailIdx: BlogDetailPageTag => BlogDetailPage,
        BlogFormIdx: BlogFormPageTag => BlogFormPage,
        BlogCreateModalIdx: BlogCreateModalPageTag => BlogCreateModalPage,
        TagListIdx: TagListPageTag => TagListPage,
        TagDetailIdx: TagDetailPageTag => TagDetailPage,
        TagFormIdx: TagFormPageTag => TagFormPage,
        TagCreateModalIdx: TagCreateModalPageTag => TagCreateModalPage,
        TagSelectIdx: TagSelectPageTag => TagSelectPage,
        ConfirmDeleteIdx: BlogConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

fn app_scaffold(
    _title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

/// `#app-layout` fragment (sidebar + main) for fine-grained HTMX swaps.
fn scaffold_pane(sidebar: Markup, crumbs: Markup, body: Markup) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

/// `<main id="main-content">` fragment for in-scaffold sidebar menu navigation.
fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn blog_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Blog",
        href: None,
    }])
}

fn blog_tags_list_crumbs() -> Markup {
    let list_url = BlogListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Blog",
            href: Some(&list_url),
        },
        Crumb {
            label: "Blog Tags",
            href: None,
        },
    ])
}

fn blog_article_crumbs(id: i64, title: &str, action: Option<&str>) -> Markup {
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

fn blog_tag_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
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

fn blog_menu(current_path: &str) -> Markup {
    let articles_url = BlogListRouteTag.url();
    let tags_url = BlogTagsListRouteTag.url();
    let links = [
        SidebarNavLink {
            key: "articles",
            title: "All Articles",
            url: &articles_url,
            icon_name: None,
            match_prefixes: &[],
        },
        SidebarNavLink {
            key: "tags",
            title: "Blog Tags",
            url: &tags_url,
            icon_name: None,
            match_prefixes: &[],
        },
    ];
    sidebar_menu(SidebarMenu {
        title: "Blog Admin",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn blog_detail_menu(blog_id: i64, title: &str, active: &str) -> Markup {
    let menu_title = format!("Article: {title}");
    let detail_url = BlogDetailRouteTag::new(blog_id).url();
    let edit_url = BlogEditGetRouteTag::new(blog_id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Article Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Edit Article",
                url: &edit_url,
                active: active == "edit",
                ..Default::default()
            }))
        },
    })
}

fn tag_detail_menu(tag_id: i64, name: &str, active: &str) -> Markup {
    let menu_title = format!("Tag: {name}");
    let detail_url = BlogTagsDetailRouteTag::new(tag_id).url();
    let edit_url = BlogTagsEditGetRouteTag::new(tag_id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Tag Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Edit Tag",
                url: &edit_url,
                active: active == "edit",
                ..Default::default()
            }))
        },
    })
}

fn blog_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + crate::http::RouteUrl + Copy + Default>(title: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: BlogTitleFilterForm::render_inputs(
            &FormCtx::form::<BlogTitleFilterForm>().value(BlogTitleFilterFormField::Title, title),
        ),
        actions: html! {
            (container_row(
                "flex gap-2",
                html! {
                    (button_submit(ButtonSubmit {
                        label: "Apply Filters",
                        ..Default::default()
                    }))
                    (button_clear(ButtonClear {
                        label: "Clear",
                        ..Default::default()
                    }))
                },
            ))
        },
        ..Default::default()
    })
}

fn tag_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + crate::http::RouteUrl + Copy + Default>(name: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: TagNameFilterForm::render_inputs(
            &FormCtx::form::<TagNameFilterForm>().value(TagNameFilterFormField::Name, name),
        ),
        actions: html! {
            (container_row(
                "flex gap-2",
                html! {
                    (button_submit(ButtonSubmit {
                        label: "Apply Filters",
                        ..Default::default()
                    }))
                    (button_clear(ButtonClear {
                        label: "Clear",
                        ..Default::default()
                    }))
                },
            ))
        },
        ..Default::default()
    })
}

fn render_pagination<K: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
    push_url: bool,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, push_url);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

/// Row payload for [`BlogListPage`].
#[derive(Clone)]
pub struct BlogRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub author_name: String,
    pub updated_at: String,
}

/// Row payload for [`TagListPage`].
#[derive(Clone)]
pub struct TagRow {
    pub id: i64,
    pub name: String,
    pub updated_at: String,
}

/// Selectable option for [`TagSelectPage`].
#[derive(Clone)]
pub struct TagOption {
    pub id: i64,
    pub name: String,
}

#[derive(Generic)]
pub struct BlogListPage {
    pub blogs: ObjectList<BlogRow>,
    pub filter_title: String,
    pub sort: String,
    pub path_and_query: String,
}

impl BlogListPage {
    /// Fine-grained table fragment for HTMX swaps targeting [`BlogTableKey`].
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let slug_sort = column_sort_url(&self.path_and_query, "Slug", &self.sort);
        let updated_sort = column_sort_url(&self.path_and_query, "UpdatedAt", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let slug_label = format!("Slug{}", sort_indicator(&self.sort, "Slug"));
        let updated_label = format!("Updated At{}", sort_indicator(&self.sort, "UpdatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Slug",
                label: &slug_label,
                sort_url: Some(&slug_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Author",
                label: "Author",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "UpdatedAt",
                label: &updated_label,
                sort_url: Some(&updated_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .blogs
            .items
            .iter()
            .map(|b| TableRow {
                attrs: row_attr_navigate_route(BlogDetailRouteTag::new(b.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &b.title,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &b.slug,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &b.author_name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &b.updated_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: blog_filter_form::<BlogTableKey, BlogListRouteTag>(&self.filter_title),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_blog.BlogCreateForm",
                href: &BlogCreateGetRouteTag.url(),
                form_post_url: &BlogCreateGetRouteTag.path(),
                modal_uid: BlogCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<BlogTableKey>(
            &self.path_and_query,
            self.blogs.number,
            self.blogs.num_pages,
            true,
        );
        data_table_list_refresh::<BlogTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl crate::template::RenderAppPane for BlogListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            blog_menu(&self.path_and_query),
            blog_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(blog_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for BlogListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Blog — Lariv",
            chrome,
            blog_menu(&self.path_and_query),
            blog_list_crumbs(),
            self.render_table(),
        )
    }
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
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.title,
                        classes: "",
                    }))
                    (label_inline("Slug", field_text(FieldText {
                        value: &self.slug,
                        classes: "",
                    })))
                    (label_inline("Description", field_text(FieldText {
                        value: &self.description,
                        classes: "",
                    })))
                    (label_inline("Author", field_text(FieldText {
                        value: &self.author_name,
                        classes: "",
                    })))
                    (label_inline("Tags", field_many_to_many(FieldManyToMany {
                        items: &tag_items,
                        classes: "",
                    })))
                    (field_markdown(FieldMarkdown {
                        value: &self.content,
                        classes: "mt-4",
                    }))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for BlogDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            blog_detail_menu(self.id, &self.title, "detail"),
            blog_article_crumbs(self.id, &self.title, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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

/// Edit article form (full page). Create uses [`BlogCreateModalPage`].
#[derive(Generic)]
pub struct BlogFormPage {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub created_by_id: i64,
    pub author_display: String,
    pub tags: Vec<ManyToManyItem>,
    pub content: String,
    pub error: String,
}

impl BlogFormPage {
    fn menu(&self) -> Markup {
        blog_detail_menu(self.id, &self.title, "edit")
    }

    fn crumbs(&self) -> Markup {
        blog_article_crumbs(self.id, &self.title, Some("Edit"))
    }

    fn pane_body(&self) -> Markup {
        let delete_url = BlogDeleteGetRouteTag::new(self.id).url();
        let created_by_id_s = if self.created_by_id == 0 {
            String::new()
        } else {
            self.created_by_id.to_string()
        };
        let ctx = FormCtx::form::<BlogForm>()
            .value(BlogFormField::Title, self.title.as_str())
            .value(BlogFormField::Slug, self.slug.as_str())
            .value(BlogFormField::Description, self.description.as_str())
            .value(BlogFormField::CreatedById, created_by_id_s.as_str())
            .display(BlogFormField::CreatedById, self.author_display.as_str())
            .m2m(BlogFormField::Tags, &self.tags)
            .value(BlogFormField::Content, self.content.as_str());
        form(FormOpts {
            title: "Edit Article",
            subtitle: "Update article details",
            classes: "@container",
            attrs: form_hx_post_main(BlogEditPostRouteTag::new(self.id)),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: BlogForm::render_inputs(&ctx),
            actions: html! {
                (container_row(
                    "flex flex-wrap justify-between gap-2 mt-2 items-center",
                    html! {
                        (container_row(
                            "flex justify-end gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Article",
                                    ..Default::default()
                                }))
                                (button_modal_form(ButtonModalForm {
                                    label: "Delete",
                                    icon_name: Some("trash"),
                                    name: "p_blog.BlogDeleteForm",
                                    href: &delete_url,
                                    form_post_url: &delete_url,
                                    modal_uid: BlogDeleteModalKey::ID,
                                    classes: "btn-error",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                ))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for BlogFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.crumbs(), self.pane_body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for BlogFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit article — Lariv",
            chrome,
            self.menu(),
            self.crumbs(),
            self.pane_body(),
        )
    }
}

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
        let ctx = FormCtx::form::<BlogForm>()
            .value(BlogFormField::Title, self.title.as_str())
            .value(BlogFormField::Slug, self.slug.as_str())
            .value(BlogFormField::Description, self.description.as_str())
            .value(BlogFormField::CreatedById, created_by_id_s.as_str())
            .display(BlogFormField::CreatedById, self.author_display.as_str())
            .m2m(BlogFormField::Tags, &self.tags)
            .value(BlogFormField::Content, self.content.as_str());
        modal_keyed::<BlogCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Article",
                subtitle: "Publish a new article",
                classes: "@container",
                attrs: form_hx_post_url::<BlogCreateModalKey>(
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
            }),
        )
    }
}

#[derive(Generic)]
pub struct TagListPage {
    pub tags: ObjectList<TagRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
}

impl TagListPage {
    pub fn render_table(&self) -> Markup {
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let updated_sort = column_sort_url(&self.path_and_query, "UpdatedAt", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let updated_label = format!("Updated At{}", sort_indicator(&self.sort, "UpdatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "UpdatedAt",
                label: &updated_label,
                sort_url: Some(&updated_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .tags
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_navigate_route(BlogTagsDetailRouteTag::new(t.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &t.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &t.updated_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: tag_filter_form::<TagTableKey, BlogTagsListRouteTag>(&self.filter_name),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_blog.TagCreateForm",
                href: &BlogTagsCreateGetRouteTag.url(),
                form_post_url: &BlogTagsCreateGetRouteTag.path(),
                modal_uid: TagCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<TagTableKey>(
            &self.path_and_query,
            self.tags.number,
            self.tags.num_pages,
            true,
        );
        data_table_list_refresh::<TagTableKey>(
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl crate::template::RenderAppPane for TagListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            blog_menu(&self.path_and_query),
            blog_tags_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(blog_tags_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for TagListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Blog Tags — Lariv",
            chrome,
            blog_menu(&self.path_and_query),
            blog_tags_list_crumbs(),
            self.render_table(),
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
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (label_inline("Articles", field_many_to_many(FieldManyToMany {
                        items: &blog_items,
                        classes: "",
                    })))
                },
            ))
        })
    }
}

impl crate::template::RenderAppPane for TagDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            tag_detail_menu(self.id, &self.name, "detail"),
            blog_tag_crumbs(self.id, &self.name, None),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
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

/// Edit tag form (full page). Create uses [`TagCreateModalPage`].
#[derive(Generic)]
pub struct TagFormPage {
    pub id: i64,
    pub name: String,
    pub error: String,
}

impl TagFormPage {
    fn menu(&self) -> Markup {
        tag_detail_menu(self.id, &self.name, "edit")
    }

    fn crumbs(&self) -> Markup {
        blog_tag_crumbs(self.id, &self.name, Some("Edit"))
    }

    fn pane_body(&self) -> Markup {
        let delete_url = BlogTagsDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<TagForm>().value(TagFormField::Name, self.name.as_str());
        form(FormOpts {
            title: "Edit Tag",
            subtitle: "Update tag details",
            attrs: form_hx_post_main(BlogTagsEditPostRouteTag::new(self.id)),
            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
            inputs: TagForm::render_inputs(&ctx),
            actions: html! {
                (container_row(
                    "flex flex-wrap justify-between gap-2 mt-2 items-center",
                    html! {
                        (container_row(
                            "flex justify-end gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Tag",
                                    ..Default::default()
                                }))
                                (button_modal_form(ButtonModalForm {
                                    label: "Delete",
                                    icon_name: Some("trash"),
                                    name: "p_blog.TagDeleteForm",
                                    href: &delete_url,
                                    form_post_url: &delete_url,
                                    modal_uid: TagDeleteModalKey::ID,
                                    classes: "btn-error",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                ))
            },
            ..Default::default()
        })
    }
}

impl crate::template::RenderAppPane for TagFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.crumbs(), self.pane_body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for TagFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Edit tag — Lariv",
            chrome,
            self.menu(),
            self.crumbs(),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct TagCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
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
        modal_keyed::<TagCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Tag",
                subtitle: "Create a new blog tag",
                attrs: crate::components::swap::form_hx_post_for_url::<TagCreateModalKey>(
                    &modal_create_post_url(
                        BlogTagsCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: TagForm::render_inputs(
                    &FormCtx::form::<TagForm>().value(TagFormField::Name, self.name.as_str()),
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
            }),
        )
    }
}

#[derive(Generic)]
pub struct TagSelectPage {
    pub tags: ObjectList<TagOption>,
    pub filter_name: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
}

impl RenderPickerSelect<TagSelectTableKey, TagSelectModalKey> for TagSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "Tags"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: false,
        }];
        let rows: Vec<TableRow> = self
            .tags
            .items
            .iter()
            .map(|t| TableRow {
                attrs: row_attr_select_multi(target, &t.id.to_string(), &t.name),
                cells: vec![field_text(FieldText {
                    value: &t.name,
                    classes: "",
                })],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(FormOpts {
                    attrs: form_hx_get_route::<TagSelectTableKey, BlogTagsSelectRouteTag>(BlogTagsSelectRouteTag)
                        .set("hx-push-url", "false"),
                    inputs: TagNameFilterForm::render_inputs(
                        &FormCtx::form::<TagNameFilterForm>()
                            .value(TagNameFilterFormField::Name, self.filter_name.as_str()),
                    ),
                    actions: html! {
                        (container_row(
                            "flex gap-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Apply",
                                    ..Default::default()
                                }))
                                (button_clear(ButtonClear {
                                    label: "Clear",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_blog.TagCreateForm",
                href: &BlogTagsCreateGetRouteTag.url(),
                form_post_url: &BlogTagsCreateGetRouteTag.path(),
                modal_uid: TagCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination::<TagSelectTableKey>(
            &self.path_and_query,
            self.tags.number,
            self.tags.num_pages,
            false,
        );
        data_table_list_refresh::<TagSelectTableKey>(
            "Select Tags",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for TagSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", BlogDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            BlogDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.modal_uid == TagDeleteModalKey::ID {
            BlogTagsDeletePostRouteTag::new(self.id).url()
        } else {
            BlogDeletePostRouteTag::new(self.id).url()
        };
        modal(crate::components::Modal {
            uid,
            children: crate::components::delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: crate::components::form_hx_post_selector(&post_url, &target),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

define_register_items! {
    plugin: BlogTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
