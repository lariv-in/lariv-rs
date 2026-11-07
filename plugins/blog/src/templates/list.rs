use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::components::{
    ButtonModalForm, Crumb, DataTable, DataTableDisplay, FieldText, FilterClauseView,
    ListFilterBuilder, ObjectList, ShellChrome, SidebarMenu, SidebarNavLink, SwapKey,
    TableButtonFilter, TableColumnHeader, TableListContent, TableRow, breadcrumbs,
    button_modal_form, column_sort_url, data_table, field_text, filter_error_banner,
    form_hx_get_route, list_filter_builder, render_pagination_push, row_attr_navigate_route,
    row_attr_select_multi, sidebar_menu, sidebar_nav_items_pane, sort_indicator,
    table_button_columns, table_button_filter, table_create_button, table_list_content,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::RenderTemplate;

use crate::keys::{BLOG_CREATE_MODAL, BLOG_TABLE, TAG_SELECT_MODAL, TAG_SELECT_TABLE, TAG_TABLE};
use crate::routes::{
    BlogCreateGetRouteTag, BlogDetailRouteTag, BlogListRouteTag, BlogTagsDetailRouteTag,
    BlogTagsListRouteTag, BlogTagsSelectRouteTag,
};

use super::create::TAG_CREATE;
use super::{app_scaffold, scaffold_main, scaffold_pane};

pub(crate) fn data_table_filtered(
    key: SwapKey,
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let hx_target = key.selector();
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: &hx_target,
        }))
    };
    let grid = html! {
        (filter_error_banner(errors))
        (lariv_core::components::table::table_grid_content(headers, rows))
    };
    let column_keys: Vec<&str> = headers
        .iter()
        .map(|h| h.key)
        .filter(|k| !k.is_empty())
        .collect();
    let toolbar = if headers.len() > 1 {
        html! {
            (table_button_columns(headers))
            (actions)
        }
    } else {
        actions
    };
    data_table(DataTable {
        uid: key.id(),
        title,
        subtitle: "",
        classes: "w-full",
        actions: toolbar,
        displays: vec![
            DataTableDisplay {
                name: "Grid".into(),
                html: grid,
            },
            DataTableDisplay {
                name: "List".into(),
                html: list,
            },
        ],
        default_view: "List",
        pagination,
        oob: false,
        refresh_url,
        column_keys: &column_keys,
        instance_id: "",
        body_only: false,
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

pub(crate) fn blog_menu(current_path: &str) -> Markup {
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
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl BlogListPage {
    /// Fine-grained table fragment for HTMX swaps targeting [`BLOG_TABLE`].
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
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(BlogListRouteTag),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "p_blog.BlogCreateForm",
                href: &BlogCreateGetRouteTag.url(),
                form_post_url: &BlogCreateGetRouteTag.path(),
                modal_uid: BLOG_CREATE_MODAL.id(),
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        let pagination = render_pagination_push(
            BLOG_TABLE,
            &self.path_and_query,
            self.blogs.number,
            self.blogs.num_pages,
            true,
        );
        data_table_filtered(
            BLOG_TABLE,
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl lariv_core::template::RenderAppPane for BlogListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            blog_menu(&self.path_and_query),
            blog_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
pub struct TagListPage {
    pub tags: ObjectList<TagRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
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
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        BlogTagsListRouteTag,
                    ),
                    apply_label: "Apply Filters",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (table_create_button(TAG_TABLE, TAG_CREATE,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination_push(
            TAG_TABLE,
            &self.path_and_query,
            self.tags.number,
            self.tags.num_pages,
            true,
        );
        data_table_filtered(
            TAG_TABLE,
            "",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl lariv_core::template::RenderAppPane for TagListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            blog_menu(&self.path_and_query),
            blog_tags_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
pub struct TagSelectPage {
    pub tags: ObjectList<TagOption>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect for TagSelectPage {
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
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        BlogTagsSelectRouteTag,
                    )
                    .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
            (picker_create_button(TAG_CREATE,
                &self.target_input,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_pagination_push(
            TAG_SELECT_TABLE,
            &self.path_and_query,
            self.tags.number,
            self.tags.num_pages,
            false,
        );
        data_table_filtered(
            TAG_SELECT_TABLE,
            "Select Tags",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for TagSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(TAG_SELECT_MODAL).into_inner()
    }
}
