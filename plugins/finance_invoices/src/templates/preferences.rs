//! Invoice preferences placeholder page.

use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{FieldTitle, ShellChrome, field_title};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_finance_accounts::templates::{
    app_scaffold, layout_main_with_crumbs, layout_with_sidebar_crumbs,
};

use crate::routes::{InvoiceDefaultRouteTag, InvoicePreferencesRouteTag};

use super::common::invoice_preferences_crumbs;

#[derive(Generic)]
pub struct InvoicePreferencesPage {
    pub can_edit: bool,
}

impl InvoicePreferencesPage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: "Invoice preferences", classes: "" }))
            p { "Invoice accounts come from each product's journal entry template." }
        }
    }
}

impl RenderAppPane for InvoicePreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &InvoicePreferencesRouteTag.url(),
            invoice_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(invoice_preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for InvoicePreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Invoice Preferences",
            chrome,
            invoice_preferences_crumbs(),
            self.body(),
            &InvoiceDefaultRouteTag.url(),
        )
    }
}
