//! Invoice UI fragments.

mod input_invoice_lines_draft;
mod input_payment_batch_allocations;
mod input_payment_term_lines_draft;

pub use input_invoice_lines_draft::{
    InputInvoiceLinesDraft, field_invoice_lines, input_invoice_lines_draft,
};
pub use input_payment_batch_allocations::{
    InputPaymentBatchAllocations, input_payment_batch_allocations,
};
pub use input_payment_term_lines_draft::{
    INVOICE_PAYMENT_TERM_DATE_KINDS, InputPaymentTermLinesDraft, PaymentTermDateKindOption,
    field_payment_term_lines, input_payment_term_lines_draft,
};

use maud::{Markup, html};

use crate::components::{SwapKey, swap::MainContentKey};

use crate::plugins::finance_invoices::scope::INVOICE_FISCAL_YEAR_COOKIE;

#[derive(Clone)]
pub struct FiscalYearOption {
    pub id: i64,
    pub label: String,
}

/// Fiscal year dropdown persisted in the `environment` cookie (mirrors Go `Environment`).
pub fn fiscal_year_environment_selector(
    fiscal_years: &[FiscalYearOption],
    selected_id: Option<i64>,
) -> Markup {
    let selected = selected_id.map(|id| id.to_string()).unwrap_or_default();
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
        key = INVOICE_FISCAL_YEAR_COOKIE,
    );

    html! {
        div class="my-1 w-full" {
            label class="label text-sm font-bold" { "Fiscal year" }
            select class="select select-bordered w-full" name="fiscal_year" onchange=(on_change) {
                option value="" selected[selected.is_empty()] { "—" }
                @for fy in fiscal_years {
                    option value=(fy.id.to_string()) selected[selected == fy.id.to_string()] {
                        (fy.label.as_str())
                    }
                }
            }
        }
    }
}
