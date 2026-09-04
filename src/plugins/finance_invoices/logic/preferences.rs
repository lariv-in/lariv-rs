//! Invoice preferences and payment preferences singletons.

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::plugins::finance_accounts::{
    logic::journal::{credit_balance_type, debit_balance_type},
    validate_leaf_account_balance_type,
};
use crate::plugins::finance_products::preferences::optional_i64;

use crate::plugins::finance_invoices::entities::{
    payment_preferences::{self, Entity as PaymentPreferencesEntity},
    preferences::{self, Entity as InvoicePreferencesEntity},
};

pub async fn load_invoice_preferences(db: &DatabaseConnection) -> preferences::Model {
    if let Ok(Some(p)) = InvoicePreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let default_format = "INV-{{YYYY}}-{{POSTED_SEQ}}".to_string();
    let am = preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        invoice_number_format: Set(Some(default_format.clone())),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        account_receivable_id: None,
        account_revenue_id: None,
        account_tax_payable_id: None,
        journal_id: None,
        invoice_number_format: Some(default_format),
        invoice_date_format: None,
        invoice_datetime_format: None,
        invoice_pdf_template: None,
        invoice_logo_vnode_id: None,
        invoice_signature_vnode_id: None,
        company_name: None,
        company_address: None,
        company_phone: None,
        company_gstin: None,
        place_of_supply: None,
    })
}

/// Chrono strftime for calendar dates (delivery, payment-term due dates).
/// Blank preference → [`crate::datetime::DATE_FMT`] (`%d/%m/%Y`).
pub fn invoice_date_format(prefs: &preferences::Model) -> &str {
    prefs
        .invoice_date_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(crate::datetime::DATE_FMT)
}

/// Chrono strftime for datetimes (invoice date, payment times).
/// Blank preference → [`crate::datetime::DATE_FMT`] (`%d/%m/%Y`) to match prior PDF output.
pub fn invoice_datetime_format(prefs: &preferences::Model) -> &str {
    prefs
        .invoice_datetime_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(crate::datetime::DATE_FMT)
}

/// Resolved chrono format strings from invoice preferences.
#[derive(Clone, Debug)]
pub struct InvoiceDateFormats {
    pub date: String,
    pub datetime: String,
}

impl InvoiceDateFormats {
    pub fn from_prefs(prefs: &preferences::Model) -> Self {
        Self {
            date: invoice_date_format(prefs).to_string(),
            datetime: invoice_datetime_format(prefs).to_string(),
        }
    }

    pub fn calendar(&self, d: NaiveDate) -> String {
        format_pref_calendar_date(d, &self.date)
    }

    pub fn calendar_opt(&self, d: Option<NaiveDate>) -> String {
        format_pref_calendar_date_opt(d, &self.date)
    }

    /// Calendar date for labels; em-dash when unset.
    pub fn calendar_or_dash(&self, d: Option<NaiveDate>) -> String {
        let s = self.calendar_opt(d);
        if s.is_empty() { "—".to_string() } else { s }
    }

    pub fn datetime(&self, dt: DateTime<Utc>, tz: &str) -> String {
        format_pref_datetime(dt, tz, &self.datetime)
    }
}

pub async fn load_invoice_date_formats(db: &DatabaseConnection) -> InvoiceDateFormats {
    InvoiceDateFormats::from_prefs(&load_invoice_preferences(db).await)
}

/// Format a calendar date with a chrono strftime (`fmt` from invoice preferences).
pub fn format_pref_calendar_date(d: NaiveDate, fmt: &str) -> String {
    d.format(fmt).to_string()
}

pub fn format_pref_calendar_date_opt(d: Option<NaiveDate>, fmt: &str) -> String {
    d.map(|d| format_pref_calendar_date(d, fmt))
        .unwrap_or_default()
}

/// Format a UTC instant in `tz` with a chrono strftime (`fmt` from invoice preferences).
pub fn format_pref_datetime(dt: DateTime<Utc>, tz: &str, fmt: &str) -> String {
    dt.with_timezone(&crate::datetime::parse_timezone(tz))
        .format(fmt)
        .to_string()
}

pub async fn validate_invoice_preferences_for_posting(
    db: &DatabaseConnection,
    prefs: &preferences::Model,
) -> Result<(), String> {
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_receivable_id),
        debit_balance_type(),
        "accounts receivable",
    )
    .await
    .map_err(|e| e.to_string())?;
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_revenue_id),
        credit_balance_type(),
        "revenue account",
    )
    .await
    .map_err(|e| e.to_string())?;
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_tax_payable_id),
        credit_balance_type(),
        "tax payable account",
    )
    .await
    .map_err(|e| e.to_string())?;
    if optional_i64(prefs.journal_id) == 0 {
        return Err("journal is required in invoice preferences".to_string());
    }
    Ok(())
}

pub async fn load_payment_preferences(db: &DatabaseConnection) -> payment_preferences::Model {
    if let Ok(Some(p)) = PaymentPreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = payment_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(payment_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        payment_account_id: None,
    })
}

pub async fn validate_payment_preferences_for_create(
    db: &DatabaseConnection,
    prefs: &payment_preferences::Model,
) -> Result<(), String> {
    let account_id = optional_i64(prefs.payment_account_id);
    if account_id == 0 {
        return Err("payment account is required in payment preferences".to_string());
    }
    validate_leaf_account_balance_type(db, account_id, debit_balance_type(), "payment account")
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn prefs_with(date: Option<&str>, datetime: Option<&str>) -> preferences::Model {
        preferences::Model {
            id: 1,
            created_at: None,
            updated_at: None,
            account_receivable_id: None,
            account_revenue_id: None,
            account_tax_payable_id: None,
            journal_id: None,
            invoice_number_format: None,
            invoice_date_format: date.map(|s| s.to_string()),
            invoice_datetime_format: datetime.map(|s| s.to_string()),
            invoice_pdf_template: None,
            invoice_logo_vnode_id: None,
            invoice_signature_vnode_id: None,
            company_name: None,
            company_address: None,
            company_phone: None,
            company_gstin: None,
            place_of_supply: None,
        }
    }

    #[test]
    fn blank_prefs_use_day_first_date() {
        let prefs = prefs_with(None, Some("   "));
        let fmts = InvoiceDateFormats::from_prefs(&prefs);
        let d = NaiveDate::from_ymd_opt(2026, 2, 8).unwrap();
        assert_eq!(fmts.calendar(d), "08/02/2026");
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        assert_eq!(fmts.datetime(dt, "Asia/Kolkata"), "08/02/2026");
    }

    #[test]
    fn custom_prefs_format_calendar_and_datetime() {
        let prefs = prefs_with(Some("%Y-%m-%d"), Some("%d %b %Y %H:%M"));
        let fmts = InvoiceDateFormats::from_prefs(&prefs);
        let d = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        assert_eq!(fmts.calendar(d), "2026-02-15");
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        assert_eq!(fmts.datetime(dt, "Asia/Kolkata"), "08 Feb 2026 05:30");
        assert_eq!(fmts.calendar_or_dash(None), "—");
    }
}
