use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        Crumb, FieldText, FieldTitle, ObjectList, PaginationPage, ShellChrome, SlotCapability,
        SlotRegistrar, SwapKey, TableColumnHeader, TablePagination, TableRow, breadcrumbs,
        column_sort_url, container_column, data_table_list_refresh, detail, field_text,
        field_title, label, pagination_pages, row_attr_navigate_route, sort_indicator,
        table_pagination,
    },
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
};

use crate::plugins::finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use crate::plugins::finance_accounts::routes::JournalEntryDetailRouteTag;
use crate::plugins::finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use super::keys::CreditNoteTableKey;
use super::routes::{CreditNoteDefaultRouteTag, CreditNoteDetailRouteTag};

fn credit_notes_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Credit Notes",
        href: None,
    }])
}

fn credit_note_crumbs(label: &str) -> Markup {
    let list_url = CreditNoteDefaultRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Credit Notes",
            href: Some(&list_url),
        },
        Crumb {
            label: label,
            href: None,
        },
    ])
}

fn credit_note_detail_menu(id: i64) -> Markup {
    detail_sidebar_menu(
        format!("Credit note #{id}"),
        &[DetailMenuNavItem {
            title: "Credit Note Detail",
            url: CreditNoteDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

crate::define_register_items! {
    plugin: FinanceCreditnotesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        CreditNoteListIdx: CreditNoteListPageTag => CreditNoteListPage,
        CreditNoteDetailIdx: CreditNoteDetailPageTag => CreditNoteDetailPage,
    ]
}

crate::define_register_items! {
    plugin: FinanceCreditnotesTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
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

#[derive(Clone)]
pub struct CreditNoteRow {
    pub id: i64,
    pub datetime: String,
    pub reason: String,
    pub original_entry_label: String,
    pub reversal_entry_label: String,
}

#[derive(Generic)]
pub struct CreditNoteListPage {
    pub credit_notes: ObjectList<CreditNoteRow>,
    pub sort: String,
    pub path_and_query: String,
}

impl CreditNoteListPage {
    pub fn render_table(&self) -> Markup {
        let date_sort = column_sort_url(&self.path_and_query, "Date", &self.sort);
        let reason_sort = column_sort_url(&self.path_and_query, "Reason", &self.sort);
        let date_label = format!("Date{}", sort_indicator(&self.sort, "Date"));
        let reason_label = format!("Reason{}", sort_indicator(&self.sort, "Reason"));
        let headers = [
            TableColumnHeader {
                key: "Date",
                label: &date_label,
                sort_url: Some(&date_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Reason",
                label: &reason_label,
                sort_url: Some(&reason_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "OriginalEntry",
                label: "Original entry",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "ReversalEntry",
                label: "Reversal entry",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .credit_notes
            .items
            .iter()
            .map(|c| TableRow {
                attrs: row_attr_navigate_route(CreditNoteDetailRouteTag::new(c.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &c.datetime,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.reason,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.original_entry_label,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &c.reversal_entry_label,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let pagination = render_pagination::<CreditNoteTableKey>(
            &self.path_and_query,
            self.credit_notes.number,
            self.credit_notes.num_pages,
        );
        data_table_list_refresh::<CreditNoteTableKey>(
            "Credit Notes",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for CreditNoteListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &self.path_and_query,
            credit_notes_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(credit_notes_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CreditNoteListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Credit Notes",
            chrome,
            credit_notes_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct CreditNoteDetailPage {
    pub id: i64,
    pub datetime: String,
    pub reason: String,
    pub journal_entry_id: i64,
    pub reversed_journal_entry_id: i64,
}

impl CreditNoteDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle {
                        value: &format!("Credit note #{}", self.id),
                        classes: "",
                    }))
                    (label("Date", field_text(FieldText { value: &self.datetime, classes: "" })))
                    (label("Reason", field_text(FieldText { value: &self.reason, classes: "" })))
                    (label("Original journal entry", html! {
                        a href=(JournalEntryDetailRouteTag::new(self.journal_entry_id).url()) {
                            "Entry #" (self.journal_entry_id)
                        }
                    }))
                    (label("Reversal journal entry", html! {
                        a href=(JournalEntryDetailRouteTag::new(self.reversed_journal_entry_id).url()) {
                            "Entry #" (self.reversed_journal_entry_id)
                        }
                    }))
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        credit_note_detail_menu(self.id)
    }
}

impl RenderAppPane for CreditNoteDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let label = format!("#{}", self.id);
        let crumbs = credit_note_crumbs(&label);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        let label = format!("#{}", self.id);
        layout_main_with_crumbs(credit_note_crumbs(&label), self.body())
    }
}

impl RenderTemplate for CreditNoteDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = format!("#{}", self.id);
        let crumbs = credit_note_crumbs(&label);
        app_scaffold_with_sidebar("Credit Note", chrome, self.menu(), crumbs, self.body())
    }
}
