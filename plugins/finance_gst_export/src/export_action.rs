//! Excel download registered on the import/export page.

use async_trait::async_trait;
use maud::{Markup, html};

use lariv_plugin_import_export::export_actions::{
    ExportPageAction, ExportPageActionRegistrar, ExportPageActionRegistry,
};

use crate::routes::GstWorkbookRouteTag;

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl ExportPageActionRegistrar for Hook {
    fn register_export_actions(
        self,
        registry: ExportPageActionRegistry,
    ) -> ExportPageActionRegistry {
        registry.push(std::sync::Arc::new(GstWorkbookButton))
    }
}

struct GstWorkbookButton;

#[async_trait]
impl ExportPageAction for GstWorkbookButton {
    async fn render(&self) -> Markup {
        html! {
            div class="mt-8" {
                h2 class="text-xl font-semibold mb-2" { "GST Excel" }
                p class="text-sm text-base-content/70 mb-4" {
                    "Download posted invoices as an Excel workbook for GST filing."
                }
                a class="btn btn-outline" href=(GstWorkbookRouteTag.path()) { "Download GST Excel" }
            }
        }
    }
}
