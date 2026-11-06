use maud::{Markup, html};

use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};
use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::routes::JournalEntryDetailRouteTag;

use lariv_core::components::{Crumb, FieldLink, FieldText, breadcrumbs, field_link, field_text};

use crate::logic::PaymentTermLineDisplayRow;

use crate::routes::{
    CancelledInvoiceDetailRouteTag, DraftInvoiceDetailRouteTag, InvoiceDefaultRouteTag,
    PaidInvoiceDetailRouteTag, PartiallyPaidInvoiceDetailRouteTag, PostedInvoiceDetailRouteTag,
    QuotationInvoiceDetailRouteTag,
};

pub(super) fn invoices_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Invoices",
        href: None,
    }])
}

pub(super) fn field_payment_term_schedule(rows: &[PaymentTermLineDisplayRow]) -> Markup {
    if rows.is_empty() {
        return html! { p { "—" } };
    }
    html! {
        table class="table table-sm w-full max-w-lg [&_th]:pl-0 [&_td]:pl-0" {
            thead {
                tr {
                    th class="text-xs" { "Due" }
                    th class="text-xs" { "Amount" }
                }
            }
            tbody {
                @for row in rows {
                    tr {
                        td { (row.due_display) }
                        td { (row.amount_display) }
                    }
                }
            }
        }
    }
}

pub(super) fn invoice_preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Invoice preferences",
        href: None,
    }])
}

pub(super) fn invoice_number_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

pub(super) fn quotation_invoice_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("Quotation #{id}")
    } else {
        format!("Quotation {number}")
    }
}

pub(super) fn quotation_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = quotation_invoice_label(id, number);
    let detail = QuotationInvoiceDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Quotation Detail",
        url: detail,
        active: true,
    }];
    detail_sidebar_menu(format!("Quotation: {label}"), &nav, None, html! {})
}

pub(super) fn draft_invoice_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("Draft invoice #{id}")
    } else {
        format!("Draft invoice {number}")
    }
}

pub(super) fn invoice_section_crumbs(
    label: &str,
    detail_url: &str,
    action: Option<&str>,
) -> Markup {
    let list_url = InvoiceDefaultRouteTag.url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Invoices",
                href: Some(&list_url),
            },
            Crumb {
                label: label,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Invoices",
                href: Some(&list_url),
            },
            Crumb {
                label: label,
                href: Some(detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}
pub(super) fn tab_href(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(InvoiceDefaultRouteTag)
        .query("tab", tab)
        .build()
}

pub(super) fn draft_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    let detail_url = DraftInvoiceDetailRouteTag::new(id).url();
    let nav = vec![DetailMenuNavItem {
        title: "Draft Invoice Detail",
        url: detail_url,
        active: true,
    }];
    detail_sidebar_menu(format!("Draft invoice: {label}"), &nav, None, html! {})
}

pub(super) fn posted_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Posted invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Posted Invoice Detail",
            url: PostedInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

pub(super) fn cancelled_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Cancelled invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Cancelled Invoice Detail",
            url: CancelledInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

pub(super) fn paid_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Paid invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Paid Invoice Detail",
            url: PaidInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

pub(super) fn partially_paid_invoice_detail_menu(id: i64, number: &str) -> Markup {
    let label = if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    };
    detail_sidebar_menu(
        format!("Partially paid invoice: {label}"),
        &[DetailMenuNavItem {
            title: "Partially Paid Invoice Detail",
            url: PartiallyPaidInvoiceDetailRouteTag::new(id).url(),
            active: true,
        }],
        None,
        html! {},
    )
}

pub(super) fn cancelled_detail_link(href: &Option<String>, label: &str) -> Markup {
    if let Some(url) = href {
        field_link(FieldLink {
            href: url,
            label,
            classes: "link link-hover",
        })
    } else if label.is_empty() {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    } else {
        field_text(FieldText {
            value: label,
            classes: "",
        })
    }
}

pub(super) fn customer_link(
    bill_to_individual: bool,
    party_id: i64,
    customer_name: &str,
) -> Markup {
    if party_id > 0 {
        let href = if bill_to_individual {
            ContactDetailRouteTag::new(party_id).url()
        } else {
            CompanyDetailRouteTag::new(party_id).url()
        };
        field_link(FieldLink {
            href: &href,
            label: customer_name,
            classes: "link link-hover",
        })
    } else if customer_name.is_empty() {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    } else {
        field_text(FieldText {
            value: customer_name,
            classes: "",
        })
    }
}

pub(super) fn journal_entry_link(id: i64) -> Markup {
    if id > 0 {
        field_link(FieldLink {
            href: &JournalEntryDetailRouteTag::new(id).url(),
            label: &format!("Entry #{id}"),
            classes: "link link-hover",
        })
    } else {
        field_text(FieldText {
            value: "—",
            classes: "",
        })
    }
}
