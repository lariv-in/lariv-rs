use lariv_core::html_form::{CsrfToken, html_form};

use lariv_plugin_finance_taxes::routes::TaxMultiSelectRouteTag;

#[html_form]
pub struct GstPreferencesForm {
    #[form(
        label = "IGST",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "gst-igst-taxes",
        placeholder = "Select IGST taxes…"
    )]
    pub igst: Vec<i64>,

    #[form(
        label = "CGST",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "gst-cgst-taxes",
        placeholder = "Select CGST taxes…"
    )]
    pub cgst: Vec<i64>,

    #[form(
        label = "SGST/UTGST",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "gst-sgst-taxes",
        placeholder = "Select SGST / UTGST taxes…"
    )]
    pub sgst: Vec<i64>,

    #[form(
        label = "Cess",
        widget = ManyToMany,
        route = TaxMultiSelectRouteTag,
        swap_key = "gst-cess-taxes",
        placeholder = "Select cess taxes…"
    )]
    pub cess: Vec<i64>,
}

impl GstPreferencesForm {
    pub fn empty() -> Self {
        Self {
            igst: Vec::new(),
            cgst: Vec::new(),
            sgst: Vec::new(),
            cess: Vec::new(),
            csrf: CsrfToken::default(),
        }
    }
}
