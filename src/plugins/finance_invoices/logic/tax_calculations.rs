//! Tax calculation helpers ported from tax_calculations.go.

use rust_decimal::Decimal;
use std::collections::HashSet;

use crate::plugins::finance_common::decimal;
use crate::plugins::finance_taxes::entities::{TaxKind, tax};

#[derive(Clone, Debug, Default)]
pub struct InvoiceLinesTotals {
    pub untaxed_subtotal: Decimal,
    pub lines_levied: Decimal,
    pub lines_withholding: Decimal,
}

impl InvoiceLinesTotals {
    pub fn lines_gross_before_withholding(&self) -> Decimal {
        decimal::dec_sum(self.untaxed_subtotal, self.lines_levied)
    }
}

pub fn taxes_levied(taxes: &[tax::Model]) -> Vec<&tax::Model> {
    taxes
        .iter()
        .filter(|t| t.tax_type != TaxKind::Withholding)
        .collect()
}

pub fn taxes_withholding(taxes: &[tax::Model]) -> Vec<&tax::Model> {
    taxes
        .iter()
        .filter(|t| t.tax_type == TaxKind::Withholding)
        .collect()
}

pub fn sum_tax_percents(taxes: &[&tax::Model]) -> Decimal {
    taxes.iter().map(|t| t.percentage).sum::<Decimal>()
}

pub fn tax_amount_on_base(base: Decimal, pct_sum: Decimal) -> Decimal {
    if decimal::dec_is_zero(base) || decimal::dec_is_zero(pct_sum) {
        return Decimal::ZERO;
    }
    decimal::dec_mul(base, pct_sum / Decimal::from(100))
}

pub fn tax_amount_for_tax(base: Decimal, tax: &tax::Model) -> Decimal {
    tax_amount_on_base(base, tax.percentage)
}

pub fn invoice_line_amount_breakdown(
    qty: Decimal,
    rate: Decimal,
    taxes: &[tax::Model],
) -> (Decimal, Decimal, Decimal, Decimal) {
    let untaxed = decimal::dec_mul(qty, rate);
    let levied = tax_amount_on_base(untaxed, sum_tax_percents(&taxes_levied(taxes)));
    let withholding = tax_amount_on_base(untaxed, sum_tax_percents(&taxes_withholding(taxes)));
    let net = decimal::dec_sub(decimal::dec_sum(untaxed, levied), withholding);
    (untaxed, levied, withholding, net)
}

pub fn merge_invoice_line_tax_ids(into: &mut HashSet<i64>, taxes: &[tax::Model]) {
    for t in taxes {
        if t.id != 0 {
            into.insert(t.id);
        }
    }
}

pub fn document_level_header_taxes(
    header: &[tax::Model],
    line_tax_ids: &HashSet<i64>,
) -> Vec<tax::Model> {
    header
        .iter()
        .filter(|t| t.id != 0 && !line_tax_ids.contains(&t.id))
        .cloned()
        .collect()
}

pub fn invoice_receivable_grand_total(
    totals: &InvoiceLinesTotals,
    header_taxes: &[tax::Model],
    line_tax_ids: &HashSet<i64>,
) -> Decimal {
    let (header_levied, header_withholding) =
        header_tax_split(totals.untaxed_subtotal, header_taxes, line_tax_ids);
    let gross = decimal::dec_sum(totals.lines_gross_before_withholding(), header_levied);
    let withheld = decimal::dec_sum(totals.lines_withholding, header_withholding);
    decimal::dec_sub(gross, withheld)
}

fn header_tax_split(
    untaxed_subtotal: Decimal,
    header_taxes: &[tax::Model],
    line_tax_ids: &HashSet<i64>,
) -> (Decimal, Decimal) {
    let mut levied = Decimal::ZERO;
    let mut withholding = Decimal::ZERO;
    for tax in document_level_header_taxes(header_taxes, line_tax_ids) {
        let amt = tax_amount_for_tax(untaxed_subtotal, &tax);
        if tax.tax_type == TaxKind::Withholding {
            withholding = decimal::dec_sum(withholding, amt);
        } else {
            levied = decimal::dec_sum(levied, amt);
        }
    }
    (levied, withholding)
}

pub fn withholding_tax_account_id(t: &tax::Model) -> Result<i64, String> {
    match t.account_id {
        Some(id) if id > 0 => Ok(id),
        _ => {
            let name = if t.name.trim().is_empty() {
                format!("#{}", t.id)
            } else {
                t.name.clone()
            };
            Err(format!("withholding tax {name} requires a ledger account"))
        }
    }
}

pub fn validate_withholding_tax_accounts(taxes: &[tax::Model]) -> Result<(), String> {
    for t in taxes {
        if t.tax_type == TaxKind::Withholding {
            withholding_tax_account_id(t)?;
        }
    }
    Ok(())
}

/// Untaxed portion of a payment settlement used as the withholding base.
///
/// Collection-time withholding (e.g. TDS) applies to taxable value excluding levied tax,
/// pro-rated by `settlement / invoice_total` for partial payments.
pub fn payment_withholding_base(
    settlement: Decimal,
    invoice_total: Decimal,
    untaxed_subtotal: Decimal,
) -> Decimal {
    let settlement = decimal::normalize(settlement);
    let invoice_total = decimal::normalize(invoice_total);
    let untaxed_subtotal = decimal::normalize(untaxed_subtotal);
    if decimal::dec_is_zero(settlement)
        || decimal::dec_is_zero(invoice_total)
        || decimal::dec_is_zero(untaxed_subtotal)
    {
        return Decimal::ZERO;
    }
    if decimal::dec_cmp(settlement, invoice_total) == std::cmp::Ordering::Equal {
        return untaxed_subtotal;
    }
    decimal::normalize(decimal::dec_mul(untaxed_subtotal, settlement) / invoice_total)
}

pub fn payment_withholding_total(withholding_base: Decimal, taxes: &[tax::Model]) -> Decimal {
    tax_amount_on_base(
        withholding_base,
        sum_tax_percents(&taxes_withholding(taxes)),
    )
}

pub fn payment_bank_amount(
    settlement: Decimal,
    withholding_base: Decimal,
    taxes: &[tax::Model],
) -> Decimal {
    decimal::dec_sub(
        settlement,
        payment_withholding_total(withholding_base, taxes),
    )
}

pub fn validate_payment_taxes(taxes: &[tax::Model]) -> Result<(), String> {
    validate_withholding_tax_accounts(taxes)?;
    for t in taxes {
        if t.tax_type != TaxKind::Withholding {
            let name = if t.name.trim().is_empty() {
                format!("#{}", t.id)
            } else {
                t.name.clone()
            };
            return Err(format!(
                "levied tax {name} cannot be applied on a payment; use withholding taxes only"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn payment_withholding_base_uses_untaxed_not_gross() {
        // Full pay of 118 receivable on 100 untaxed → base 100
        assert_eq!(
            payment_withholding_base(d("118"), d("118"), d("100")),
            d("100")
        );
        // Half pay → half untaxed
        assert_eq!(
            payment_withholding_base(d("59"), d("118"), d("100")),
            d("50")
        );
    }

    #[test]
    fn payment_bank_amount_withholds_from_untaxed_base() {
        let tax = tax::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            name: "TDS".into(),
            percentage: d("10"),
            tax_type: TaxKind::Withholding,
            account_id: Some(1),
        };
        // 10% of 100 untaxed = 10; bank = 118 - 10
        assert_eq!(
            payment_bank_amount(d("118"), d("100"), &[tax]),
            d("108")
        );
    }
}
