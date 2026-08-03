//! Maud page templates for blog and tag CRUD views.

use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonLink, ButtonModalForm, ButtonSubmit, DeleteConfirmation, FieldManyToMany,
        FieldMarkdown, FieldText, FieldTitle, FormOpts, LayoutSidebar, ManyToManyItem, ObjectList,
        PaginationPage, ShellChrome, ShellScaffold, SidebarMenu, SidebarMenuBack,
        SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, button_clear, button_link, button_modal_form, button_submit,
        column_sort_url, container_column, container_row, data_table_list, detail, field_many_to_many,
        field_markdown, field_text, field_title, form, form_hx_get_route, form_hx_post_main,
        label_inline,
        layout_sidebar, modal, modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select_multi,
        shell_scaffold, sidebar_menu, sidebar_menu_item, sort_indicator, table_button_filter,
        table_pagination,
    },
    capability::define_register_items,
    html_form::{FormCtx, HtmlForm},
    http::{ProvideRequestCaps},
    template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use super::forms::{BlogForm, BlogTitleFilterForm, TagForm, TagNameFilterForm};
use super::keys::{
    BlogDeleteModalKey, BlogTableKey, TagDeleteModalKey, TagSelectModalKey, TagSelectTableKey,
    TagTableKey,
};
use super::routes::{
    BlogCreateGetRouteTag, BlogCreatePostRouteTag, BlogDeleteGetRouteTag, BlogDeletePostRouteTag,
    BlogDetailRouteTag, BlogEditGetRouteTag, BlogEditPostRouteTag, BlogListRouteTag,
    BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag, BlogTagsDeleteGetRouteTag,
    BlogTagsDeletePostRouteTag, BlogTagsDetailRouteTag, BlogTagsEditGetRouteTag,
    BlogTagsEditPostRouteTag, BlogTagsListRouteTag, BlogTagsSelectRouteTag,
};
use crate::plugins::dashboard::routes::DashboardAppsRouteTag;

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
        TagListIdx: TagListPageTag => TagListPage,
        TagDetailIdx: TagDetailPageTag => TagDetailPage,
        TagFormIdx: TagFormPageTag => TagFormPage,
        TagSelectIdx: TagSelectPageTag => TagSelectPage,
        ConfirmDeleteIdx: BlogConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

fn app_scaffold(_title: &str, chrome: &ShellChrome, sidebar: Markup, body: Markup) -> Markup {
    shell_scaffold(ShellScaffold {
        title: "Lariv",
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        body,
        ..Default::default()
    })
}

/// `#app-layout` fragment (sidebar + main) for fine-grained HTMX swaps.
fn scaffold_pane(sidebar: Markup, body: Markup) -> Markup {
    layout_sidebar(LayoutSidebar {
        sidebar,
        content: body,
    })
}

/// `<main id="main-content">` fragment for in-scaffold sidebar menu navigation.
fn scaffold_main(body: Markup) -> Markup {
    use crate::components::layout::layout_main;
    layout_main(body)
}

fn blog_menu() -> Markup {
    let back_url = DashboardAppsRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: "Blog Admin",
        back: Some(SidebarMenuBack {
            title: "Back to Home",
            url: &back_url,
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "All Articles",
                url: &BlogListRouteTag.url(),
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Blog Tags",
                url: &BlogTagsListRouteTag.url(),
                ..Default::default()
            }))
        },
    })
}

fn blog_detail_menu(blog_id: i64, title: &str) -> Markup {
    let menu_title = format!("Article: {title}");
    let detail_url = BlogDetailRouteTag::new(blog_id).url();
    let edit_url = BlogEditGetRouteTag::new(blog_id).url();
    let back_url = BlogListRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        back: Some(SidebarMenuBack {
            title: "Back to All Articles",
            url: &back_url,
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Article Detail",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit Article",
                url: &edit_url,
                ..Default::default()
            }))
        },
    })
}

fn tag_detail_menu(tag_id: i64, name: &str) -> Markup {
    let menu_title = format!("Tag: {name}");
    let detail_url = BlogTagsDetailRouteTag::new(tag_id).url();
    let edit_url = BlogTagsEditGetRouteTag::new(tag_id).url();
    let back_url = BlogTagsListRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        back: Some(SidebarMenuBack {
            title: "Back to Blog Tags",
            url: &back_url,
        }),
        children: html! {
            (sidebar_menu_item(SidebarMenuItem {
                title: "Tag Detail",
                url: &detail_url,
                ..Default::default()
            }))
            (sidebar_menu_item(SidebarMenuItem {
                title: "Edit Tag",
                url: &edit_url,
                ..Default::default()
            }))
        },
    })
}

fn blog_filter_form<K: SwapKey, R: crate::http::FragmentGet<K> + crate::http::RouteUrl + Copy + Default>(title: &str) -> Markup {
    form(FormOpts {
        attrs: form_hx_get_route::<K, R>(R::default()),
        inputs: BlogTitleFilterForm::render_inputs(
            &FormCtx::new().value("Title", title),
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
        inputs: TagNameFilterForm::render_inputs(&FormCtx::new().value("Name", name)),
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
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let headers = [
            TableColumnHeader {
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                label: "Slug",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Author",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                label: "Updated At",
                sort_url: None,
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
            (button_link(ButtonLink {
                href: &BlogCreateGetRouteTag.url(),
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
        data_table_list::<BlogTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for BlogListPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(blog_menu(), self.render_table())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for BlogListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Blog — Lariv", chrome, blog_menu(), self.render_table())
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
    fn render_pane(&self) -> Markup {
        scaffold_pane(blog_detail_menu(self.id, &self.title), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for BlogDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.title),
            chrome,
            blog_detail_menu(self.id, &self.title),
            self.pane_body(),
        )
    }
}

/// Create/edit form for blogs. `id == 0` is create.
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
        if self.id == 0 {
            blog_menu()
        } else {
            blog_detail_menu(self.id, &self.title)
        }
    }

    fn pane_body(&self) -> Markup {
        let is_create = self.id == 0;
        let form_attrs = if is_create {
            form_hx_post_main(BlogCreatePostRouteTag)
        } else {
            form_hx_post_main(BlogEditPostRouteTag::new(self.id))
        };
        let delete_url = BlogDeleteGetRouteTag::new(self.id).url();
        let created_by_id_s = if self.created_by_id == 0 {
            String::new()
        } else {
            self.created_by_id.to_string()
        };
        let ctx = FormCtx::new()
            .value("Title", self.title.as_str())
            .value("Slug", self.slug.as_str())
            .value("Description", self.description.as_str())
            .value("CreatedByID", created_by_id_s.as_str())
            .display("author", self.author_display.as_str())
            .m2m("Tags", &self.tags)
            .value("Content", self.content.as_str());
        form(FormOpts {
            title: if is_create { "Create Article" } else { "Edit Article" },
            subtitle: if is_create {
                "Publish a new article"
            } else {
                "Update article details"
            },
            classes: "@container",
            attrs: form_attrs,
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
                                @if !is_create {
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
                                }
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
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for BlogFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let title = if self.id == 0 {
            "Create article — Lariv"
        } else {
            "Edit article — Lariv"
        };
        app_scaffold(title, chrome, self.menu(), self.pane_body())
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
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                label: "Updated At",
                sort_url: None,
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
            (button_link(ButtonLink {
                href: &BlogTagsCreateGetRouteTag.url(),
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
        data_table_list::<TagTableKey>("", actions, &headers, &rows, pagination)
    }
}

impl crate::template::RenderAppPane for TagListPage {
    fn render_pane(&self) -> Markup {
        scaffold_pane(blog_menu(), self.render_table())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.render_table())
    }
}

impl RenderTemplate for TagListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold("Blog Tags — Lariv", chrome, blog_menu(), self.render_table())
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
    fn render_pane(&self) -> Markup {
        scaffold_pane(tag_detail_menu(self.id, &self.name), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for TagDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            tag_detail_menu(self.id, &self.name),
            self.pane_body(),
        )
    }
}

/// Create/edit form for tags.
/// `id == 0` is create.
#[derive(Generic)]
pub struct TagFormPage {
    pub id: i64,
    pub name: String,
    pub error: String,
}

impl TagFormPage {
    fn menu(&self) -> Markup {
        if self.id == 0 {
            blog_menu()
        } else {
            tag_detail_menu(self.id, &self.name)
        }
    }

    fn pane_body(&self) -> Markup {
        let is_create = self.id == 0;
        let form_attrs = if is_create {
            form_hx_post_main(BlogTagsCreatePostRouteTag)
        } else {
            form_hx_post_main(BlogTagsEditPostRouteTag::new(self.id))
        };
        let delete_url = BlogTagsDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::new().value("Name", self.name.as_str());
        form(FormOpts {
            title: if is_create { "Create Tag" } else { "Edit Tag" },
            subtitle: if is_create {
                "Create a new blog tag"
            } else {
                "Update tag details"
            },
            attrs: form_attrs,
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
                                @if !is_create {
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
                                }
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
    fn render_pane(&self) -> Markup {
        scaffold_pane(self.menu(), self.pane_body())
    }
    fn render_main(&self) -> Markup {
        scaffold_main(self.pane_body())
    }
}

impl RenderTemplate for TagFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let title = if self.id == 0 {
            "Create tag — Lariv"
        } else {
            "Edit tag — Lariv"
        };
        app_scaffold(title, chrome, self.menu(), self.pane_body())
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

impl TagSelectPage {
    pub fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "Tags"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [TableColumnHeader {
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
                        &FormCtx::new().value("Name", self.filter_name.as_str()),
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
            (button_link(ButtonLink {
                href: &BlogTagsCreateGetRouteTag.url(),
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
        data_table_list::<TagSelectTableKey>(
            "Select Tags",
            actions,
            &headers,
            &rows,
            pagination,
        )
    }
}

impl RenderTemplate for TagSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<TagSelectModalKey>("", self.render_table())
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
