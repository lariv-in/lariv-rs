//! Data tables, list content, pagination, and toolbar buttons (Go `DataTable` ports).

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::button::{ButtonLink, button_link};
use crate::components::swap::SwapKey;
use crate::components::text::icon;

/// Paginated collection payload (Go `ObjectList`).
#[derive(Clone, Debug, Default)]
pub struct ObjectList<T> {
    pub items: Vec<T>,
    pub number: u32,
    pub num_pages: u32,
    pub total: u64,
}

impl<T> ObjectList<T> {
    pub fn from_page(items: Vec<T>, page: u32, page_size: u32, total: u64) -> Self {
        let page = page.max(1);
        let page_size = page_size.max(1);
        let num_pages = ((total as u32).saturating_add(page_size - 1) / page_size).max(1);
        Self {
            items,
            number: page,
            num_pages,
            total,
        }
    }
}

pub struct TableColumnHeader<'a> {
    pub label: &'a str,
    pub sort_url: Option<&'a str>,
    pub push_url: bool,
}

pub struct TableRow {
    pub attrs: HtmlAttrs,
    pub cells: Vec<Markup>,
}

pub struct TableListContent<'a> {
    pub headers: &'a [TableColumnHeader<'a>],
    pub rows: &'a [TableRow],
    /// HTMX target selector for sort links (from a [`SwapKey`]).
    pub hx_target: &'a str,
}

pub fn table_list_content(opts: TableListContent<'_>) -> Markup {
    let col_span = opts.headers.len().max(1);
    let target = opts.hx_target;
    html! {
        div class="table-container flex flex-col rounded-box border border-base-300 bg-base-100" {
            div class="overflow-x-auto" {
                table class="table table-zebra" {
                    thead {
                        tr {
                            @for h in opts.headers {
                                th class="whitespace-nowrap min-w-[100px]" {
                                    @if let Some(url) = h.sort_url {
                                        (PreEscaped(format!(
                                            r#"<a href="{}" hx-get="{}" hx-target="{}" hx-select="" hx-swap="outerMorph" hx-push-url="{}" class="link link-hover link-neutral no-underline hover:underline cursor-pointer font-inherit text-inherit inline-flex items-center gap-1">"#,
                                            escape_attr(url),
                                            escape_attr(url),
                                            escape_attr(target),
                                            if h.push_url { "true" } else { "false" }
                                        )))
                                        (h.label)
                                        (PreEscaped("</a>"))
                                    } @else {
                                        (h.label)
                                    }
                                }
                            }
                        }
                    }
                    tbody {
                        @if opts.rows.is_empty() {
                            tr {
                                td colspan=(col_span) class="text-center opacity-50 py-8" {
                                    "Table is empty"
                                }
                            }
                        } @else {
                            @for row in opts.rows {
                                (PreEscaped(format!("<tr{}>", row.attrs.as_string())))
                                @for cell in &row.cells {
                                    td class="whitespace-nowrap truncate max-w-xs min-w-[100px]" {
                                        (cell)
                                    }
                                }
                                (PreEscaped("</tr>"))
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Grid card view (Go `TableGridContent`). First column = card title; rest = labeled fields.
pub fn table_grid_content(headers: &[TableColumnHeader<'_>], rows: &[TableRow]) -> Markup {
    html! {
        div class="flex flex-col gap-4, @container" {
            div class="overflow-x-auto" {
                div class="grid grid-cols-1 @md:grid-cols-2 @2xl:grid-cols-3 @3xl:grid-cols-4 gap-2" {
                    @if rows.is_empty() {
                        div class="col-span-full text-center opacity-50 py-8" { "Table is empty" }
                    } @else {
                        @for row in rows {
                            (PreEscaped(format!("<div{}>", grid_row_attrs(row).as_string())))
                            @if let Some(title) = row.cells.first() {
                                div class="font-semibold text-md truncate" { (title) }
                            }
                            @for (i, cell) in row.cells.iter().enumerate().skip(1) {
                                div class="text-sm flex gap-2 truncate" {
                                    @if let Some(h) = headers.get(i) {
                                        @if !h.label.is_empty() {
                                            span class="font-semibold text-primary" { (h.label) }
                                        }
                                    }
                                    span { (cell) }
                                }
                            }
                            (PreEscaped("</div>"))
                        }
                    }
                }
            }
        }
    }
}

/// Grid card classes from row attrs (Go `RowAttrNavigateClick` grid branch).
fn grid_row_attrs(row: &TableRow) -> HtmlAttrs {
    let base =
        "border border-base-300 rounded-box flex flex-col bg-base-100 p-2 cursor-pointer transition-colors";
    let mut attrs = row.attrs.clone();
    if attrs.attrs.contains_key(":class") {
        attrs.attrs.insert("class".into(), base.into());
    } else {
        attrs.attrs.insert(
            "class".into(),
            format!("{base} hover:bg-base-200"),
        );
    }
    attrs
}

pub struct PaginationPage<'a> {
    pub ellipsis: bool,
    pub url: &'a str,
    pub push_url: bool,
    pub active: bool,
    pub label: &'a str,
}

pub struct TablePagination<'a> {
    pub pages: &'a [PaginationPage<'a>],
    /// HTMX target selector for page links (from a [`SwapKey`]).
    pub hx_target: &'a str,
}

pub fn table_pagination(opts: TablePagination<'_>) -> Markup {
    if opts.pages.is_empty() {
        return Markup::default();
    }
    let target = opts.hx_target;
    html! {
        div class="flex flex-col justify-center items-center gap-2 p-4" {
            div class="join" {
                @for p in opts.pages {
                    @if p.ellipsis {
                        button disabled class="join-item btn btn-sm" { "..." }
                    } @else {
                        (PreEscaped(format!(
                            r#"<a href="{}" hx-get="{}" hx-target="{}" hx-select="" hx-swap="outerMorph" hx-push-url="{}" class="{}">"#,
                            escape_attr(p.url),
                            escape_attr(p.url),
                            escape_attr(target),
                            if p.push_url { "true" } else { "false" },
                            escape_attr(&format!(
                                "join-item btn btn-sm{}",
                                if p.active { " btn-active" } else { "" }
                            ))
                        )))
                        (p.label)
                        (PreEscaped("</a>"))
                    }
                }
            }
        }
    }
}

/// Build pagination page entries for a window around the current page.
pub fn pagination_pages(
    path_and_query: &str,
    current: u32,
    num_pages: u32,
    push_url: bool,
) -> Vec<(bool, String, bool, bool, String)> {
    // (ellipsis, url, push_url, active, label) — owned for callers that store URLs
    if num_pages <= 1 {
        return Vec::new();
    }
    let n = current.max(1);
    let np = num_pages.max(1);
    let window_size = 5u32;
    let mut start = n.saturating_sub(window_size / 2).max(1);
    let mut end = start + window_size - 1;
    if end > np {
        end = np;
        start = end.saturating_sub(window_size - 1).max(1);
    }

    let mut out = Vec::new();
    if start > 1 {
        out.push((
            false,
            page_url(path_and_query, 1),
            push_url,
            n == 1,
            "1".into(),
        ));
        out.push((true, String::new(), push_url, false, String::new()));
    }
    for p in start..=end {
        out.push((
            false,
            page_url(path_and_query, p),
            push_url,
            p == n,
            p.to_string(),
        ));
    }
    if end < np {
        out.push((true, String::new(), push_url, false, String::new()));
        out.push((
            false,
            page_url(path_and_query, np),
            push_url,
            n == np,
            np.to_string(),
        ));
    }
    out
}

fn page_url(path_and_query: &str, page: u32) -> String {
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_query, ""),
    };
    let mut pairs: Vec<(String, String)> = if query.is_empty() {
        Vec::new()
    } else {
        query
            .split('&')
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .filter(|(k, _)| k != "page")
            .collect()
    };
    pairs.push(("page".into(), page.to_string()));
    let qs = pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{qs}")
}

/// Cycle sort clause for a column: ASC → DESC → cleared. Different column → ASC.
pub fn next_sort_clause(current: &str, column_key: &str) -> Option<String> {
    let current = current.trim();
    if current.is_empty() {
        return Some(format!("{column_key} ASC"));
    }
    let parts: Vec<&str> = current.split_whitespace().collect();
    if parts.is_empty() {
        return Some(format!("{column_key} ASC"));
    }
    let cur_col = parts[0];
    let cur_dir = parts
        .last()
        .map(|s| s.to_ascii_uppercase())
        .unwrap_or_else(|| "ASC".into());
    if cur_col.eq_ignore_ascii_case(column_key) {
        if cur_dir == "DESC" {
            None
        } else {
            Some(format!("{column_key} DESC"))
        }
    } else {
        Some(format!("{column_key} ASC"))
    }
}

pub fn sort_indicator(current_sort: &str, column_key: &str) -> &'static str {
    let current = current_sort.trim();
    if current.is_empty() {
        return "";
    }
    let parts: Vec<&str> = current.split_whitespace().collect();
    if parts.is_empty() || !parts[0].eq_ignore_ascii_case(column_key) {
        return "";
    }
    if parts.len() >= 2 && parts.last().is_some_and(|d| d.eq_ignore_ascii_case("DESC")) {
        " ▼"
    } else {
        " ▲"
    }
}

/// Build a sort URL preserving query string, cycling the column, resetting page to 1.
pub fn column_sort_url(path_and_query: &str, column_key: &str, current_sort: &str) -> String {
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_query, ""),
    };
    let mut pairs: Vec<(String, String)> = if query.is_empty() {
        Vec::new()
    } else {
        query
            .split('&')
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .filter(|(k, _)| k != "sort" && k != "page")
            .collect()
    };
    if let Some(next) = next_sort_clause(current_sort, column_key) {
        // Go emits page before sort.
        pairs.push(("page".into(), "1".into()));
        pairs.push(("sort".into(), next.replace(' ', "+")));
    } else {
        pairs.push(("page".into(), "1".into()));
    }
    let qs = pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{qs}")
}

pub struct DataTableDisplay {
    pub name: String,
    pub html: Markup,
}

pub struct DataTable<'a> {
    pub uid: &'a str,
    pub title: &'a str,
    pub subtitle: &'a str,
    pub classes: &'a str,
    pub actions: Markup,
    pub displays: Vec<DataTableDisplay>,
    pub default_view: &'a str,
    pub pagination: Markup,
    /// When true, emit `hx-swap-oob="true"` on the root (for multi-fragment responses).
    pub oob: bool,
}

impl Default for DataTable<'_> {
    fn default() -> Self {
        Self {
            uid: "table-container",
            title: "",
            subtitle: "",
            classes: "",
            actions: Markup::default(),
            displays: Vec::new(),
            default_view: "List",
            pagination: Markup::default(),
            oob: false,
        }
    }
}

pub fn data_table(opts: DataTable<'_>) -> Markup {
    let uid = if opts.uid.is_empty() {
        "table-container"
    } else {
        opts.uid
    };
    let initial = if opts
        .displays
        .iter()
        .any(|d| d.name == opts.default_view)
    {
        opts.default_view
    } else {
        opts.displays
            .first()
            .map(|d| d.name.as_str())
            .unwrap_or("List")
    };
    let x_data = serde_json::json!({ "view": initial }).to_string();
    let oob_attr = if opts.oob {
        r#" hx-swap-oob="true""#
    } else {
        ""
    };
    // Go templates HTML-escape JSON into a double-quoted x-data attribute.
    html! {
        (PreEscaped(format!(
            r#"<div id="{}" class="w-full data-table-container {}" x-data="{}"{}>"#,
            escape_attr(uid),
            escape_attr(opts.classes),
            escape_attr(&x_data),
            oob_attr
        )))
        div class="flex justify-between items-center my-2" {
            div {
                @if !opts.title.is_empty() {
                    div class="text-xl font-semibold" { (opts.title) }
                }
                @if !opts.subtitle.is_empty() {
                    div class="text-sm text-gray-500" { (opts.subtitle) }
                }
            }
            div class="flex items-center gap-2" {
                @if opts.displays.len() > 1 {
                    select class="select select-md" x-model="view" {
                        @for d in &opts.displays {
                            option value=(d.name) { (d.name) }
                        }
                    }
                }
                (opts.actions)
            }
        }
        div class="relative my-2" {
            @for d in &opts.displays {
                // Static `hidden` on non-default views keeps rows visible after HTMX
                // outerMorph before Alpine re-inits; `:class` takes over once Alpine runs.
                (PreEscaped(format!(
                    r#"<div :class="{{ 'hidden': view !== '{}' }}" class="{}">"#,
                    escape_attr(&d.name),
                    if d.name == initial { "" } else { "hidden" },
                )))
                (d.html)
                (PreEscaped("</div>"))
            }
            (opts.pagination)
        }
        (PreEscaped("</div>"))
    }
}

/// Convenience: List+Grid data table keyed by [`SwapKey`] (default view: List).
pub fn data_table_list<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
) -> Markup {
    data_table_list_opts::<K>(title, actions, headers, rows, pagination, false, "List")
}

/// Like [`data_table_list`], default view: Grid (matches Go client tables).
pub fn data_table_list_grid<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
) -> Markup {
    data_table_list_opts::<K>(title, actions, headers, rows, pagination, false, "Grid")
}

/// Like [`data_table_list`], optionally as an OOB fragment.
pub fn data_table_list_opts<K: SwapKey>(
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    oob: bool,
    default_view: &str,
) -> Markup {
    let list = table_list_content(TableListContent {
        headers,
        rows,
        hx_target: K::SELECTOR,
    });
    let grid = table_grid_content(headers, rows);
    // Go sorts view option names alphabetically: Grid, List.
    data_table(DataTable {
        uid: K::ID,
        title,
        classes: "w-full",
        actions,
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
        default_view,
        pagination,
        oob,
        ..Default::default()
    })
}

const TABLE_BUTTON_FILTER_DEFAULT_CONTENT: &str =
    "card w-64 my-1.5 card-body shadow dropdown-content border border-base-300 rounded-box z-2 bg-base-100";

pub struct TableButtonFilter {
    pub panel: Markup,
    pub content_classes: String,
}

impl Default for TableButtonFilter {
    fn default() -> Self {
        Self {
            panel: Markup::default(),
            content_classes: TABLE_BUTTON_FILTER_DEFAULT_CONTENT.into(),
        }
    }
}

pub fn table_button_filter(opts: TableButtonFilter) -> Markup {
    let content = if opts.content_classes.is_empty() {
        TABLE_BUTTON_FILTER_DEFAULT_CONTENT
    } else {
        opts.content_classes.as_str()
    };
    html! {
        (PreEscaped(
            r#"<details class="dropdown dropdown-end" @click.outside="if(!$event.target.closest('.fk-modal-container')){$el.removeAttribute('open')}">"#
        ))
        summary class="btn btn-square dropdown-toggle btn-primary btn-sm" {
            (icon("funnel", ""))
        }
        div class=(content) { (opts.panel) }
        (PreEscaped("</details>"))
    }
}

pub struct TableButtonCreate<'a> {
    pub href: &'a str,
    pub label: &'a str,
    pub icon_name: Option<&'a str>,
    pub classes: &'a str,
}

impl Default for TableButtonCreate<'_> {
    fn default() -> Self {
        Self {
            href: "#",
            label: "",
            icon_name: Some("plus"),
            classes: "btn-square btn-outline btn-sm",
        }
    }
}

pub fn table_button_create(opts: TableButtonCreate<'_>) -> Markup {
    button_link(ButtonLink {
        label: opts.label,
        href: opts.href,
        icon_name: opts.icon_name,
        classes: opts.classes,
        ..Default::default()
    })
}
