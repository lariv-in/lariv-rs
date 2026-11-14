//! Download form registered on the import/export page.

use async_trait::async_trait;
use chrono::{Datelike, FixedOffset, NaiveDate, Utc};
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, CsrfToken, FormOpts, HtmlAttrs, InputSingleChoiceCombobox, InputText,
    button_submit, form, form_post_download_route, input_single_choice_combobox, input_text,
};
use lariv_plugin_import_export::export_actions::{
    ExportPageAction, ExportPageActionRegistrar, ExportPageActionRegistry,
};

use crate::routes::{GstWorkbookRouteTag, Gstr1DownloadRouteTag};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl ExportPageActionRegistrar for Hook {
    fn register_export_actions(
        self,
        registry: ExportPageActionRegistry,
    ) -> ExportPageActionRegistry {
        registry.push(std::sync::Arc::new(Gstr1Button))
    }
}

struct Gstr1Button;

#[async_trait]
impl ExportPageAction for Gstr1Button {
    async fn render(&self) -> Markup {
        let months = return_months();
        let current = current_period();
        html! {
            div class="mt-8" {
                h2 class="text-xl font-semibold mb-2" { "GSTR-1" }
                p class="text-sm text-base-content/70 mb-4" {
                    "Download a GST3.2.4 JSON file for one return month. Leave gross turnover blank to use the untaxed value of posted invoices, reduced by credit notes."
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_post_download_route(Gstr1DownloadRouteTag),
                    inputs: html! {
                        div class="grid gap-3 sm:grid-cols-2" {
                            div class="sm:col-span-2" {
                                (input_single_choice_combobox(InputSingleChoiceCombobox {
                                    label: "Month",
                                    name: "period",
                                    choices: &months,
                                    value: &current,
                                    placeholder: "Search months…",
                                    ..Default::default()
                                }))
                            }
                            (input_text(InputText {
                                label: "Gross turnover",
                                name: "gt",
                                attrs: HtmlAttrs::new().set("placeholder", "From invoices if blank"),
                                ..Default::default()
                            }))
                            (input_text(InputText {
                                label: "Current gross turnover",
                                name: "cur_gt",
                                attrs: HtmlAttrs::new().set("placeholder", "From invoices if blank"),
                                ..Default::default()
                            }))
                        }
                    },
                    actions: html! {
                        div class="mt-4" {
                            (button_submit(ButtonSubmit { label: "Download GSTR-1 JSON", ..Default::default() }))
                        }
                    },
                    ..Default::default()
                }))
                p class="mt-6" {
                    a class="btn btn-outline" href=(GstWorkbookRouteTag.path()) { "Download GST Excel" }
                }
            }
        }
    }
}

/// July 2017 through the current Indian month, newest first. Keys are `YYYY-MM`.
fn return_months() -> Vec<(String, String)> {
    let today = Utc::now().with_timezone(&ist()).date_naive();
    let mut year = today.year();
    let mut month = today.month();
    let mut months = Vec::new();
    loop {
        let Some(date) = NaiveDate::from_ymd_opt(year, month, 1) else {
            break;
        };
        months.push((
            format!("{year:04}-{month:02}"),
            date.format("%B %Y").to_string(),
        ));
        if year == 2017 && month == 7 {
            break;
        }
        if month == 1 {
            month = 12;
            year -= 1;
        } else {
            month -= 1;
        }
    }
    months
}

fn current_period() -> String {
    let today = Utc::now().with_timezone(&ist()).date_naive();
    format!("{:04}-{:02}", today.year(), today.month())
}

#[expect(clippy::expect_used, reason = "19800 seconds is a valid UTC offset")]
fn ist() -> FixedOffset {
    FixedOffset::east_opt(19_800).expect("IST offset")
}
