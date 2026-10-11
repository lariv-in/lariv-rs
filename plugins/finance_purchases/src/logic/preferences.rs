//! Purchase preferences and payment preferences singletons.

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use lariv_plugin_finance_accounts::{
    logic::journal::{credit_balance_type, debit_balance_type},
    validate_leaf_account_balance_type,
};
use lariv_plugin_finance_products::preferences::optional_i64;

use crate::entities::preferences::{self, Entity as PurchasePreferencesEntity};

pub async fn load_purchase_preferences(db: &DatabaseConnection) -> preferences::Model {
    if let Ok(Some(p)) = PurchasePreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let default_format = "PUR-{{YYYY}}-{{POSTED_SEQ}}".to_string();
    let am = preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        purchase_number_format: Set(Some(default_format.clone())),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        account_payable_id: None,
        account_expense_id: None,
        account_input_tax_id: None,
        journal_id: None,
        purchase_number_format: Some(default_format),
        purchase_date_format: None,
        purchase_datetime_format: None,
        purchase_pdf_template: None,
        purchase_logo_vnode_id: None,
        purchase_signature_vnode_id: None,
        company_name: None,
        company_address: None,
        company_phone: None,
        company_gstin: None,
        place_of_supply: None,
        default_bank_account: None,
    })
}

/// Chrono strftime for calendar dates (delivery, payment-term due dates).
/// Blank preference → [`lariv_core::datetime::DATE_FMT`] (`%d/%m/%Y`).
pub fn purchase_date_format(prefs: &preferences::Model) -> &str {
    prefs
        .purchase_date_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(lariv_core::datetime::DATE_FMT)
}

/// Chrono strftime for datetimes (purchase date, payment times).
/// Blank preference → [`lariv_core::datetime::DATE_FMT`] (`%d/%m/%Y`) to match prior PDF output.
pub fn purchase_datetime_format(prefs: &preferences::Model) -> &str {
    prefs
        .purchase_datetime_format
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(lariv_core::datetime::DATE_FMT)
}

/// Resolved chrono format strings from purchase preferences.
#[derive(Clone, Debug)]
pub struct PurchaseDateFormats {
    pub date: String,
    pub datetime: String,
}

impl PurchaseDateFormats {
    pub fn from_prefs(prefs: &preferences::Model) -> Self {
        Self {
            date: purchase_date_format(prefs).to_string(),
            datetime: purchase_datetime_format(prefs).to_string(),
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

pub async fn load_purchase_date_formats(db: &DatabaseConnection) -> PurchaseDateFormats {
    PurchaseDateFormats::from_prefs(&load_purchase_preferences(db).await)
}

/// Format a calendar date with a chrono strftime (`fmt` from purchase preferences).
pub fn format_pref_calendar_date(d: NaiveDate, fmt: &str) -> String {
    d.format(fmt).to_string()
}

pub fn format_pref_calendar_date_opt(d: Option<NaiveDate>, fmt: &str) -> String {
    d.map(|d| format_pref_calendar_date(d, fmt))
        .unwrap_or_default()
}

/// Format a UTC instant in `tz` with a chrono strftime (`fmt` from purchase preferences).
pub fn format_pref_datetime(dt: DateTime<Utc>, tz: &str, fmt: &str) -> String {
    dt.with_timezone(&lariv_core::datetime::parse_timezone(tz))
        .format(fmt)
        .to_string()
}

pub async fn validate_purchase_preferences_for_posting(
    db: &DatabaseConnection,
    prefs: &preferences::Model,
) -> Result<(), String> {
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_payable_id),
        credit_balance_type(),
        "accounts payable",
    )
    .await
    .map_err(|e| e.to_string())?;
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_expense_id),
        debit_balance_type(),
        "expense account",
    )
    .await
    .map_err(|e| e.to_string())?;
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_input_tax_id),
        debit_balance_type(),
        "input tax account",
    )
    .await
    .map_err(|e| e.to_string())?;
    if optional_i64(prefs.journal_id) == 0 {
        return Err("journal is required in purchase preferences".to_string());
    }
    Ok(())
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
            account_payable_id: None,
            account_expense_id: None,
            account_input_tax_id: None,
            journal_id: None,
            purchase_number_format: None,
            purchase_date_format: date.map(|s| s.to_string()),
            purchase_datetime_format: datetime.map(|s| s.to_string()),
            purchase_pdf_template: None,
            purchase_logo_vnode_id: None,
            purchase_signature_vnode_id: None,
            company_name: None,
            company_address: None,
            company_phone: None,
            company_gstin: None,
            place_of_supply: None,
            default_bank_account: None,
        }
    }

    #[test]
    fn blank_prefs_use_day_first_date() {
        let prefs = prefs_with(None, Some("   "));
        let fmts = PurchaseDateFormats::from_prefs(&prefs);
        let d = NaiveDate::from_ymd_opt(2026, 2, 8).unwrap();
        assert_eq!(fmts.calendar(d), "08/02/2026");
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        assert_eq!(fmts.datetime(dt, "Asia/Kolkata"), "08/02/2026");
    }

    #[test]
    fn custom_prefs_format_calendar_and_datetime() {
        let prefs = prefs_with(Some("%Y-%m-%d"), Some("%d %b %Y %H:%M"));
        let fmts = PurchaseDateFormats::from_prefs(&prefs);
        let d = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        assert_eq!(fmts.calendar(d), "2026-02-15");
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        assert_eq!(fmts.datetime(dt, "Asia/Kolkata"), "08 Feb 2026 05:30");
        assert_eq!(fmts.calendar_or_dash(None), "—");
    }
}
