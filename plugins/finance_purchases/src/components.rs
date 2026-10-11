//! Purchase UI fragments.

mod input_payment_term_lines_draft;
mod input_purchase_lines_draft;

pub use input_payment_term_lines_draft::{
    PURCHASE_PAYMENT_TERM_DATE_KINDS, InputPaymentTermLinesDraft, PaymentTermDateKindOption,
    field_payment_term_lines, input_payment_term_lines_draft,
};
pub use input_purchase_lines_draft::{
    InputPurchaseLinesDraft, field_purchase_lines, input_purchase_lines_draft,
};

use maud::{Markup, html};

use crate::scope::PURCHASE_FISCAL_YEAR_COOKIE;
use lariv_core::components::{SwapKey, swap::MainContentKey};

#[derive(Clone)]
pub struct FiscalYearOption {
    /// Calendar year of the April 1 FY start.
    pub start_year: i32,
    pub label: String,
}

/// Fiscal year dropdown persisted in the `environment` cookie.
pub fn fiscal_year_environment_selector(
    fiscal_years: &[FiscalYearOption],
    selected_start_year: Option<i32>,
) -> Markup {
    let selected = selected_start_year
        .map(|y| y.to_string())
        .unwrap_or_default();
    let reload_js = format!(
        "htmx.ajax('GET',window.location.pathname+window.location.search,{{target:'{target}',select:'{target}',swap:'outerHTML',pushUrl:false}})",
        target = MainContentKey::SELECTOR,
    );
    let on_change = format!(
        r#"(function(){{
        var env={{}};
        try{{
            var c=document.cookie.split('; ').find(function(r){{return r.startsWith('environment=')}});
            if(c) env=JSON.parse(decodeURIComponent(c.split('=').slice(1).join('=')));
        }}catch(e){{}}
        env[{key:?}]=this.value;
        document.cookie='environment='+encodeURIComponent(JSON.stringify(env))+'; path=/';
        {reload_js};
    }}).call(this)"#,
        key = PURCHASE_FISCAL_YEAR_COOKIE,
    );

    html! {
        div class="my-1 w-full" {
            label class="label text-sm font-bold" { "Fiscal year" }
            select class="select select-bordered w-full" name="fiscal_year" onchange=(on_change) {
                option value="" selected[selected.is_empty()] { "—" }
                @for fy in fiscal_years {
                    option value=(fy.start_year.to_string()) selected[selected == fy.start_year.to_string()] {
                        (fy.label.as_str())
                    }
                }
            }
        }
    }
}
