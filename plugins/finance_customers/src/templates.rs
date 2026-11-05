//! Accounting chrome for the customer list, detail, and journal.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonDownload, Crumb, DetailHeader, FieldLink, FieldText, HtmlAttrs, ShellChrome,
    SlotCapability, SlotRegistrar, breadcrumbs, button_download, detail, detail_header, field_link,
    field_text, label,
};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_plugin_customer::templates::CustomerListPage;
use lariv_plugin_finance_accounts::accounting_detail_menu::{
    DetailMenuNavItem, detail_sidebar_menu,
};
use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use crate::routes::{
    CustomerDetailRouteTag, CustomerJournalRouteTag, CustomerListRouteTag,
    CustomerStatementExportRouteTag,
};

fn statement_export_button(id: i64) -> Markup {
    let href = CustomerStatementExportRouteTag::new(id).path();
    button_download(ButtonDownload {
        label: "Export Excel",
        href: &href,
        icon_name: Some("arrow-down-tray"),
        classes: "btn-outline btn-sm",
        attrs: HtmlAttrs::new()
            .set("aria-label", "Export Excel")
            .set("title", "Export invoices and payments"),
    })
}

fn customers_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Customers",
        href: None,
    }])
}

fn customer_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = CustomerListRouteTag.url();
    let detail_url = CustomerDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Customers",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(action) => breadcrumbs(&[
            Crumb {
                label: "Customers",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: Some(&detail_url),
            },
            Crumb {
                label: action,
                href: None,
            },
        ]),
    }
}

fn customer_menu(id: i64, name: &str, active: &str) -> Markup {
    let detail_url = CustomerDetailRouteTag::new(id).url();
    let journal_url = CustomerJournalRouteTag::new(id).url();
    let mut nav = vec![
        DetailMenuNavItem {
            title: "Detail",
            url: detail_url,
            active: active == "detail",
        },
        DetailMenuNavItem {
            title: "Journal",
            url: journal_url,
            active: active == "journal",
        },
    ];
    for link in crate::journal::customer_sidebar_links() {
        nav.push(DetailMenuNavItem {
            title: link.title,
            url: (link.url_fn)(id),
            active: active == link.key,
        });
    }
    detail_sidebar_menu(format!("Customer: {name}"), &nav, None, html! {})
}

/// Finance hub page that renders [`CustomerListPage`]'s table under Accounting chrome.
pub struct CustomersListPage {
    pub inner: CustomerListPage,
}

impl CustomersListPage {
    pub fn render_table(&self) -> Markup {
        self.inner.render_table::<CustomerListRouteTag>()
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for CustomersListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &self.inner.path_and_query,
            customers_list_crumbs(),
            self.body(),
        )
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(customers_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CustomersListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Customers",
            chrome,
            customers_list_crumbs(),
            self.body(),
            &self.inner.path_and_query,
        )
    }
}

pub struct CustomerDetailPage {
    pub id: i64,
    pub name: String,
    pub kind: &'static str,
    pub party_href: String,
    pub party_label: &'static str,
}

impl CustomerDetailPage {
    fn menu(&self) -> Markup {
        customer_menu(self.id, &self.name, "detail")
    }

    fn crumbs(&self) -> Markup {
        customer_crumbs(&self.name, self.id, None)
    }

    fn body(&self) -> Markup {
        let party = if self.party_href.is_empty() {
            field_text(FieldText {
                value: "—",
                classes: "",
            })
        } else {
            field_link(FieldLink {
                href: &self.party_href,
                label: &self.name,
                classes: "link link-hover",
            })
        };
        detail(html! {
            (detail_header(DetailHeader {
                title: &self.name,
                actions: statement_export_button(self.id),
            }))
            (label("Kind", field_text(FieldText { value: self.kind, classes: "" })))
            (label(self.party_label, party))
        })
    }
}

impl RenderAppPane for CustomerDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.menu(), self.crumbs(), self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.body())
    }
}

impl RenderTemplate for CustomerDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar("Customer", chrome, self.menu(), self.crumbs(), self.body())
    }
}

pub struct CustomerJournalPage {
    pub id: i64,
    pub name: String,
    pub sections_html: String,
    pub empty: bool,
}

impl CustomerJournalPage {
    fn menu(&self) -> Markup {
        customer_menu(self.id, &self.name, "journal")
    }

    fn crumbs(&self) -> Markup {
        customer_crumbs(&self.name, self.id, Some("Journal"))
    }

    fn body(&self) -> Markup {
        let empty = if self.empty {
            html! {
                p class="opacity-70" {
                    "Quotations, invoices, cancelled invoices, and payments appear here when those plugins are installed."
                }
            }
        } else {
            html! {}
        };
        html! {
            div class="mb-4 flex justify-end" {
                (statement_export_button(self.id))
            }
            div class="flex flex-col gap-8" {
                (empty)
                (PreEscaped(self.sections_html.as_str()))
            }
        }
    }
}

impl RenderAppPane for CustomerJournalPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.menu(), self.crumbs(), self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.body())
    }
}

impl RenderTemplate for CustomerJournalPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar(
            "Customer journal",
            chrome,
            self.menu(),
            self.crumbs(),
            self.body(),
        )
    }
}

/// One customer sidebar destination (quotations, invoices, payments, …).
pub struct CustomerSectionPage {
    pub id: i64,
    pub name: String,
    pub active: &'static str,
    pub action: &'static str,
    pub body: Markup,
}

impl CustomerSectionPage {
    fn menu(&self) -> Markup {
        customer_menu(self.id, &self.name, self.active)
    }

    fn crumbs(&self) -> Markup {
        customer_crumbs(&self.name, self.id, Some(self.action))
    }

    fn body(&self) -> Markup {
        self.body.clone()
    }
}

impl RenderAppPane for CustomerSectionPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_entity_sidebar_crumbs(self.menu(), self.crumbs(), self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(self.crumbs(), self.body())
    }
}

impl RenderTemplate for CustomerSectionPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold_with_sidebar(self.action, chrome, self.menu(), self.crumbs(), self.body())
    }
}

lariv_core::define_register_items! {
    plugin: FinanceCustomersTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        CustomersListIdx: CustomersListPageTag => CustomersListPage,
        CustomerDetailIdx: CustomerDetailPageTag => CustomerDetailPage,
        CustomerJournalIdx: CustomerJournalPageTag => CustomerJournalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: FinanceCustomersTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
