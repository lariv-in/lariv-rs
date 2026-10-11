//! Purchase PDF: Minijinja template → Typst → PDF (via typst crate).

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use hex::ToHex;
use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};

use crate::logic::BillTo;
use lariv_plugin_finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_format,
};
use lariv_plugin_finance_common::{decimal::decimal_display_currency, typst};
use lariv_plugin_finance_products::entities::product::Entity as ProductEntity;
use lariv_plugin_finance_products::pricing;
use lariv_plugin_finance_taxes::entities::tax::{self, TaxKind};
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;
use minijinja::{Environment, UndefinedBehavior};
use num2words::{Lang, Num2Words};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait,     QueryFilter, Statement,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::entities::preferences;
use crate::entities::{
    CancelledPurchaseEntity, DraftPurchaseEntity, DraftPurchaseLineEntity, PostedPurchaseEntity,
    PostedPurchaseLineEntity,
};
use crate::entities::{draft_purchase_line, posted_purchase, posted_purchase_line};
use crate::purchase_pdf_addon::{collect_purchase_pdf_extras, collect_purchase_pdf_sample_extras};
use crate::purchase_pdf_assets::VnodeImageContext;
use crate::purchase_pdf_template::DEFAULT_PURCHASE_PDF_TEMPLATE;
use crate::logic::draft_payment_term::{
    draft_payment_term_line_display, load_draft_purchase_payment_term_lines,
    load_posted_payment_term_for_cancelled, load_posted_payment_term_for_posted,
    posted_payment_term_line_display, resolve_due_date,
};
use crate::logic::preferences::{
    format_pref_calendar_date, format_pref_datetime, purchase_date_format, purchase_datetime_format,
    load_purchase_preferences,
};
use crate::logic::tax_assoc::{
    load_cancelled_purchase_tax_ids, load_cancelled_line_tax_ids, load_draft_purchase_tax_ids,
    load_draft_line_tax_ids, load_posted_purchase_tax_ids, load_posted_line_tax_ids,
};
use crate::logic::tax_calculations::{
    PurchaseLinesTotals, purchase_line_amounts, purchase_receivable_grand_total,
    merge_purchase_line_tax_ids,
};
use lariv_plugin_filesystem::state::FilesystemState;

#[derive(Debug, thiserror::Error)]
pub enum PurchasePdfError {
    #[error("{0}")]
    Message(String),
    #[error("not found")]
    NotFound,
}

impl PurchasePdfError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

pub struct PurchasePdfResult {
    pub bytes: Vec<u8>,
    pub filename_base: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfRoot {
    #[serde(rename = "ID")]
    id: i64,
    number: Option<String>,
    reference: Option<String>,
    payment_reference: Option<String>,
    bank_account: Option<String>,
    remarks: Option<String>,
    datetime: String,
    datetime_display: String,
    datetime_year: i32,
    datetime_month: u32,
    datetime_day: u32,
    delivery_date: String,
    delivery_date_display: String,
    vendor_id: i64,
    vendor: PdfVendor,
    payment_term: PdfPaymentTerm,
    taxes: Vec<PdfTax>,
    lines: Vec<PdfLine>,
    payments: Vec<PdfPayment>,
    #[serde(rename = "company_name")]
    company_name: String,
    #[serde(rename = "company_address")]
    company_address: String,
    #[serde(rename = "company_phone")]
    company_phone: String,
    #[serde(rename = "company_gstin")]
    company_gstin: String,
    #[serde(rename = "place_of_supply")]
    place_of_supply: String,
    #[serde(rename = "company_logo_vnode_id")]
    company_logo_vnode_id: Option<i64>,
    #[serde(rename = "company_signature_vnode_id")]
    company_signature_vnode_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfVendor {
    #[serde(rename = "ID")]
    id: i64,
    vendor_type: String,
    name: String,
    address: Option<String>,
    address_line_1: Option<String>,
    address_line_2: Option<String>,
    city: Option<String>,
    pincode: Option<String>,
    state: Option<String>,
    #[serde(rename = "GSTIN")]
    gstin: Option<String>,
    #[serde(rename = "CIN")]
    cin: Option<String>,
    #[serde(rename = "PAN")]
    pan: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    website: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfPaymentTermLine {
    due_date: String,
    due_date_display: String,
    /// Alias for templates written when due dates were timestamps.
    due_datetime: String,
    due_datetime_display: String,
    amount: String,
}

impl PdfPaymentTermLine {
    fn formatted(due: String, amount: impl Into<String>) -> Self {
        Self {
            due_date: due.clone(),
            due_date_display: due.clone(),
            due_datetime: due.clone(),
            due_datetime_display: due,
            amount: amount.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfPaymentTerm {
    #[serde(rename = "ID")]
    id: i64,
    summary: String,
    lines: Vec<PdfPaymentTermLine>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfTax {
    #[serde(rename = "ID")]
    id: i64,
    name: String,
    percentage: String,
    #[serde(rename = "TaxType")]
    tax_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfProduct {
    #[serde(rename = "ID")]
    id: i64,
    name: String,
    product_type: String,
    #[serde(rename = "HSNCode")]
    hsn_code: i64,
    reference: String,
    remarks: String,
    /// Product variable schema: name → type (`length`, `quantity`, …). Keys are unchanged.
    variable_types: serde_json::Map<String, serde_json::Value>,
    base_price_formula: String,
    sales_price_formula: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfLine {
    #[serde(rename = "ID")]
    id: i64,
    product_id: i64,
    product: PdfProduct,
    quantity: String,
    rate: String,
    amount: String,
    remarks: String,
    /// Values typed on this line, keyed by variable name.
    ///
    /// Length and weight are `{"value","unit"}` (`"2"` cm, `"1.5"` kg). Other
    /// variables stay scalars. Line `Quantity` is separate and multiplies `Rate`.
    variables: serde_json::Map<String, serde_json::Value>,
    /// Schema-formatted `name: value` lines, one per variable.
    variable_lines: Vec<String>,
    taxes: Vec<PdfTax>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfPayment {
    #[serde(rename = "ID")]
    id: i64,
    amount: String,
    datetime: String,
    datetime_display: String,
}

fn purchase_date_parts(dt: DateTime<Utc>, tz: &str, fmt: &str) -> (String, i32, u32, u32) {
    let display = format_pref_datetime(dt, tz, fmt);
    let local = dt.with_timezone(&lariv_core::datetime::parse_timezone(tz));
    (display, local.year(), local.month(), local.day())
}

fn dec_str(d: Decimal) -> String {
    d.normalize().to_string()
}

/// Money for Typst: pad to currency minor units, no symbol (template adds ₹ / parses floats).
fn money_str(d: Decimal, currency: &CurrencyFormat) -> String {
    decimal_display_currency(d, currency.minor_unit, "")
}

fn tax_to_pdf(t: &tax::Model) -> PdfTax {
    PdfTax {
        id: t.id,
        name: t.name.clone(),
        percentage: dec_str(t.percentage),
        tax_type: t.tax_type.as_str().to_string(),
    }
}

fn pdf_vendor_from_contact(c: contact::Model) -> PdfVendor {
    PdfVendor {
        id: c.id,
        vendor_type: "individual".into(),
        name: c.name,
        address: None,
        address_line_1: None,
        address_line_2: None,
        city: None,
        pincode: None,
        state: None,
        gstin: None,
        cin: None,
        pan: None,
        phone: c.phone,
        email: c.email,
        website: None,
    }
}

fn pdf_vendor_from_company(c: company::Model) -> PdfVendor {
    let address = c.formatted_address_for_typst();
    PdfVendor {
        id: c.id,
        vendor_type: "business".into(),
        name: c.name,
        address,
        address_line_1: c.address_line_1,
        address_line_2: c.address_line_2,
        city: c.city,
        pincode: c.pincode,
        state: c.state,
        gstin: c.gstin,
        cin: c.cin,
        pan: c.pan,
        phone: c.phone,
        email: c.email,
        website: c.website,
    }
}

async fn load_vendor(
    db: &DatabaseConnection,
    party: BillTo,
) -> Result<PdfVendor, PurchasePdfError> {
    if party.vendor_is_individual {
        let id = party
            .vendor_contact_id
            .filter(|id| *id > 0)
            .ok_or(PurchasePdfError::NotFound)?;
        let c = ContactEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?
            .ok_or(PurchasePdfError::NotFound)?;
        Ok(pdf_vendor_from_contact(c))
    } else {
        let id = party
            .vendor_company_id
            .filter(|id| *id > 0)
            .ok_or(PurchasePdfError::NotFound)?;
        let c = CompanyEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?
            .ok_or(PurchasePdfError::NotFound)?;
        Ok(pdf_vendor_from_company(c))
    }
}

/// JSON object stored on a line or product. Non-objects become an empty map.
fn json_object(raw: &str) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(raw.trim()) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    }
}

async fn load_draft_payment_term_pdf(
    db: &DatabaseConnection,
    draft_id: i64,
    anchor: DateTime<Utc>,
    tz: &str,
    date_fmt: &str,
    _currency: &CurrencyFormat,
) -> Result<PdfPaymentTerm, PurchasePdfError> {
    let lines = load_draft_purchase_payment_term_lines(db, draft_id)
        .await
        .map_err(|e| PurchasePdfError::msg(e))?;
    let pdf_lines: Vec<PdfPaymentTermLine> = lines
        .iter()
        .map(|l| {
            let display = draft_payment_term_line_display(l, date_fmt);
            let due_date = resolve_due_date(l, anchor, tz).unwrap_or_else(|_| {
                l.due_date.unwrap_or_else(|| {
                    anchor
                        .with_timezone(&lariv_core::datetime::parse_timezone(tz))
                        .date_naive()
                })
            });
            let due_date_display = format_pref_calendar_date(due_date, date_fmt);
            PdfPaymentTermLine::formatted(due_date_display, display.amount_display)
        })
        .collect();
    let summary = pdf_lines
        .iter()
        .map(|l| format!("{}: {}", l.due_date_display, l.amount))
        .collect::<Vec<_>>()
        .join("; ");
    Ok(PdfPaymentTerm {
        id: draft_id,
        summary,
        lines: pdf_lines,
    })
}

async fn load_posted_payment_term_pdf(
    db: &DatabaseConnection,
    posted_purchase_id: Option<i64>,
    cancelled_purchase_id: Option<i64>,
    _tz: &str,
    date_fmt: &str,
    currency: &CurrencyFormat,
) -> Result<PdfPaymentTerm, PurchasePdfError> {
    let loaded = if let Some(pid) = posted_purchase_id {
        load_posted_payment_term_for_posted(db, pid).await
    } else if let Some(cid) = cancelled_purchase_id {
        load_posted_payment_term_for_cancelled(db, cid).await
    } else {
        Ok(None)
    }
    .map_err(|e| PurchasePdfError::msg(e))?;

    let Some((term, lines)) = loaded else {
        return Ok(PdfPaymentTerm {
            id: 0,
            summary: String::new(),
            lines: vec![],
        });
    };

    let pdf_lines: Vec<PdfPaymentTermLine> = lines
        .iter()
        .map(|l| {
            let display = posted_payment_term_line_display(
                l,
                currency.minor_unit,
                &currency.symbol,
                date_fmt,
            );
            let due_date_display = format_pref_calendar_date(l.due_date, date_fmt);
            PdfPaymentTermLine::formatted(due_date_display, display.amount_display)
        })
        .collect();
    let summary = pdf_lines
        .iter()
        .map(|l| format!("{}: {}", l.due_date_display, l.amount))
        .collect::<Vec<_>>()
        .join("; ");
    Ok(PdfPaymentTerm {
        id: term.id,
        summary,
        lines: pdf_lines,
    })
}

async fn load_product_pdf(
    db: &DatabaseConnection,
    id: i64,
) -> Result<(PdfProduct, String), PurchasePdfError> {
    let p = ProductEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?
        .ok_or(PurchasePdfError::NotFound)?;
    let schema = p.variables.clone();
    Ok((
        PdfProduct {
            id: p.id,
            name: p.name,
            product_type: p.product_type.as_str().to_string(),
            hsn_code: p.hsn_code,
            reference: p.reference.unwrap_or_default(),
            remarks: p.remarks.unwrap_or_default(),
            variable_types: json_object(&p.variables),
            base_price_formula: p.base_price_formula,
            sales_price_formula: p.sales_price_formula,
        },
        schema,
    ))
}

struct LineRow {
    id: i64,
    product_id: i64,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
    remarks: Option<String>,
    variable_values: String,
}

#[derive(Clone, Copy)]
enum LineTaxSource {
    Draft,
    Posted,
    Cancelled,
}

async fn line_tax_ids(
    db: &DatabaseConnection,
    source: LineTaxSource,
    line_id: i64,
) -> Result<Vec<i64>, PurchasePdfError> {
    let ids = match source {
        LineTaxSource::Draft => load_draft_line_tax_ids(db, line_id).await,
        LineTaxSource::Posted => load_posted_line_tax_ids(db, line_id).await,
        LineTaxSource::Cancelled => load_cancelled_line_tax_ids(db, line_id).await,
    }
    .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    Ok(ids)
}

async fn load_draft_lines(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<Vec<LineRow>, PurchasePdfError> {
    let rows = DraftPurchaseLineEntity::find()
        .filter(draft_purchase_line::Column::DraftPurchaseId.eq(draft_id))
        .all(db)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|l| LineRow {
            id: l.id,
            product_id: l.product_id,
            quantity: l.quantity,
            rate: l.rate,
            pre_tax: l.pre_tax_amount,
            remarks: l.remarks,
            variable_values: l.variable_values,
        })
        .collect())
}

async fn load_posted_lines(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<Vec<LineRow>, PurchasePdfError> {
    let rows = PostedPurchaseLineEntity::find()
        .filter(posted_purchase_line::Column::PostedPurchaseId.eq(posted_id))
        .all(db)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|l| LineRow {
            id: l.id,
            product_id: l.product_id,
            quantity: l.quantity,
            rate: l.rate,
            pre_tax: l.pre_tax_amount,
            remarks: l.remarks,
            variable_values: l.variable_values,
        })
        .collect())
}

async fn load_cancelled_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Result<Vec<LineRow>, PurchasePdfError> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, product_id, rate, quantity, pre_tax_amount, remarks, variable_values \
             FROM cancelled_purchase_lines \
             WHERE cancelled_purchase_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        let id: i64 = r
            .try_get("", "id")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        let product_id: i64 = r
            .try_get("", "product_id")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        let rate: Decimal = r
            .try_get("", "rate")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        let quantity: Decimal = r
            .try_get("", "quantity")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        let pre_tax: Decimal = r
            .try_get("", "pre_tax_amount")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        let remarks: Option<String> = r
            .try_get("", "remarks")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        let variable_values: String = r
            .try_get("", "variable_values")
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
        out.push(LineRow {
            id,
            product_id,
            quantity,
            rate,
            pre_tax,
            remarks,
            variable_values,
        });
    }
    Ok(out)
}

async fn build_pdf_lines(
    db: &DatabaseConnection,
    rows: &[LineRow],
    tax_source: LineTaxSource,
    currency: &CurrencyFormat,
) -> Result<Vec<PdfLine>, PurchasePdfError> {
    let mut lines = Vec::with_capacity(rows.len());
    for row in rows {
        let tax_ids = line_tax_ids(db, tax_source, row.id).await?;
        let taxes = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| PurchasePdfError::msg(e.to_string()))?
            .into_iter()
            .map(|t| tax_to_pdf(&t))
            .collect();
        let (product, schema) = load_product_pdf(db, row.product_id).await?;
        let variable_lines = pricing::format_variable_lines(&schema, &row.variable_values);
        lines.push(PdfLine {
            id: row.id,
            product_id: row.product_id,
            product,
            quantity: dec_str(row.quantity),
            rate: money_str(row.rate, currency),
            amount: dec_str(row.pre_tax),
            remarks: row.remarks.clone().unwrap_or_default(),
            variables: json_object(&row.variable_values),
            variable_lines,
            taxes,
        });
    }
    Ok(lines)
}

/// Company presentation fields from purchase preferences (Finance → Preferences).
fn company_fields_from_prefs(
    prefs: &preferences::Model,
) -> (
    String,
    String,
    String,
    String,
    String,
    Option<i64>,
    Option<i64>,
) {
    (
        prefs.company_name.clone().unwrap_or_default(),
        typst::typst_address_lines(&prefs.company_address.clone().unwrap_or_default()),
        prefs.company_phone.clone().unwrap_or_default(),
        prefs.company_gstin.clone().unwrap_or_default(),
        prefs.place_of_supply.clone().unwrap_or_default(),
        prefs.purchase_logo_vnode_id.filter(|&id| id > 0),
        prefs.purchase_signature_vnode_id.filter(|&id| id > 0),
    )
}

fn apply_pdf_presentation_prefs(root: &mut PdfRoot, prefs: &preferences::Model) {
    let (name, address, phone, gstin, place, logo, signature) = company_fields_from_prefs(prefs);
    root.company_name = name;
    root.company_address = address;
    root.company_phone = phone;
    root.company_gstin = gstin;
    root.place_of_supply = place;
    root.company_logo_vnode_id = logo;
    root.company_signature_vnode_id = signature;
}

async fn build_pdf_root(
    db: &DatabaseConnection,
    id: i64,
    number: Option<String>,
    reference: Option<String>,
    payment_reference: Option<String>,
    bank_account: Option<String>,
    remarks: Option<String>,
    datetime: DateTime<Utc>,
    delivery_date: Option<NaiveDate>,
    party: BillTo,
    header_tax_ids: Vec<i64>,
    line_rows: Vec<LineRow>,
    tax_source: LineTaxSource,
    payments: Vec<PdfPayment>,
    payment_term: PdfPaymentTerm,
    tz: &str,
    currency: &CurrencyFormat,
    prefs: &preferences::Model,
) -> Result<PdfRoot, PurchasePdfError> {
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?
        .into_iter()
        .map(|t| tax_to_pdf(&t))
        .collect();
    let lines = build_pdf_lines(db, &line_rows, tax_source, currency).await?;
    let date_fmt = purchase_date_format(prefs);
    let datetime_fmt = purchase_datetime_format(prefs);
    let (datetime_display, datetime_year, datetime_month, datetime_day) =
        purchase_date_parts(datetime, tz, datetime_fmt);
    let delivery_date_display = delivery_date
        .map(|d| format_pref_calendar_date(d, date_fmt))
        .unwrap_or_default();
    let (
        company_name,
        company_address,
        company_phone,
        company_gstin,
        place_of_supply,
        logo,
        signature,
    ) = company_fields_from_prefs(prefs);
    Ok(PdfRoot {
        id,
        number,
        reference,
        payment_reference,
        bank_account: bank_account.and_then(|s| {
            let lines = typst::typst_address_lines(&s);
            if lines.is_empty() { None } else { Some(lines) }
        }),
        remarks,
        datetime: datetime_display.clone(),
        datetime_display,
        datetime_year,
        datetime_month,
        datetime_day,
        delivery_date: delivery_date_display.clone(),
        delivery_date_display,
        vendor_id: party.party_id(),
        vendor: load_vendor(db, party).await?,
        payment_term,
        taxes: header_taxes,
        lines,
        payments,
        company_name,
        company_address,
        company_phone,
        company_gstin,
        place_of_supply,
        company_logo_vnode_id: logo,
        company_signature_vnode_id: signature,
    })
}

pub async fn render_draft_purchase_pdf(
    fs: &FilesystemState,
    id: i64,
    tz: &str,
) -> Result<PurchasePdfResult, PurchasePdfError> {
    let db = &fs.db;
    let draft = DraftPurchaseEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?
        .ok_or(PurchasePdfError::NotFound)?;
    let header_tax_ids = load_draft_purchase_tax_ids(db, draft.id)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    let line_rows = load_draft_lines(db, draft.id).await?;
    let prefs = load_purchase_preferences(db).await;
    let currency = match prefs.journal_id.filter(|&id| id > 0) {
        Some(jid) => load_journal_currency_format(db, jid).await,
        None => load_default_currency_format(db).await,
    };
    let date_fmt = purchase_date_format(&prefs);
    let payment_term =
        load_draft_payment_term_pdf(db, draft.id, draft.datetime, tz, date_fmt, &currency).await?;
    let root = build_pdf_root(
        db,
        draft.id,
        draft.number.clone(),
        draft.reference.clone(),
        draft.payment_reference.clone(),
        draft.bank_account.clone(),
        draft.remarks.clone(),
        draft.datetime,
        draft.delivery_date,
        BillTo::new(
            draft.vendor_is_individual,
            draft.vendor_contact_id,
            draft.vendor_company_id,
        ),
        header_tax_ids,
        line_rows,
        LineTaxSource::Draft,
        vec![],
        payment_term,
        tz,
        &currency,
        &prefs,
    )
    .await?;
    let base = pdf_filename_base(
        draft.number.as_deref(),
        &format!("draft-purchase-{}", draft.id),
    );
    render_pdf_from_prefs(fs, &root, &base, Some(draft.id)).await
}

pub async fn render_posted_purchase_pdf(
    fs: &FilesystemState,
    posted: posted_purchase::Model,
    tz: &str,
) -> Result<PurchasePdfResult, PurchasePdfError> {
    let db = &fs.db;
    let header_tax_ids = load_posted_purchase_tax_ids(db, posted.id)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    let line_rows = load_posted_lines(db, posted.id).await?;
    let prefs = load_purchase_preferences(db).await;
    let currency = load_journal_currency_format(db, posted.journal_id).await;
    let date_fmt = purchase_date_format(&prefs);
    let payments = Vec::new();
    let payment_term =
        load_posted_payment_term_pdf(db, Some(posted.id), None, tz, date_fmt, &currency).await?;
    let root = build_pdf_root(
        db,
        posted.id,
        Some(posted.number.clone()),
        posted.reference.clone(),
        posted.payment_reference.clone(),
        posted.bank_account.clone(),
        posted.remarks.clone(),
        posted.datetime,
        posted.delivery_date,
        BillTo::new(
            posted.vendor_is_individual,
            posted.vendor_contact_id,
            posted.vendor_company_id,
        ),
        header_tax_ids,
        line_rows,
        LineTaxSource::Posted,
        payments,
        payment_term,
        tz,
        &currency,
        &prefs,
    )
    .await?;
    let base = pdf_filename_base(Some(&posted.number), &format!("purchase-{}", posted.id));
    render_pdf_from_prefs(fs, &root, &base, Some(posted.draft_purchase_id)).await
}

pub async fn render_cancelled_purchase_pdf(
    fs: &FilesystemState,
    id: i64,
    tz: &str,
) -> Result<PurchasePdfResult, PurchasePdfError> {
    let db = &fs.db;
    let inv = CancelledPurchaseEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?
        .ok_or(PurchasePdfError::NotFound)?;
    let header_tax_ids = load_cancelled_purchase_tax_ids(db, inv.id)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?;
    let line_rows = load_cancelled_lines(db, inv.id).await?;
    let prefs = load_purchase_preferences(db).await;
    let currency = load_journal_currency_format(db, inv.journal_id).await;
    let date_fmt = purchase_date_format(&prefs);
    let payments = Vec::new();
    let payment_term =
        load_posted_payment_term_pdf(db, None, Some(inv.id), tz, date_fmt, &currency).await?;
    let root = build_pdf_root(
        db,
        inv.id,
        Some(inv.number.clone()),
        inv.reference.clone(),
        inv.payment_reference.clone(),
        inv.bank_account.clone(),
        inv.remarks.clone(),
        inv.datetime,
        inv.delivery_date,
        BillTo::new(
            inv.vendor_is_individual,
            inv.vendor_contact_id,
            inv.vendor_company_id,
        ),
        header_tax_ids,
        line_rows,
        LineTaxSource::Cancelled,
        payments,
        payment_term,
        tz,
        &currency,
        &prefs,
    )
    .await?;
    let draft_purchase_id = PostedPurchaseEntity::find_by_id(inv.posted_purchase_id)
        .one(db)
        .await
        .map_err(|e| PurchasePdfError::msg(e.to_string()))?
        .map(|p| p.draft_purchase_id);
    let base = pdf_filename_base(Some(&inv.number), &format!("cancelled-purchase-{}", inv.id));
    render_pdf_from_prefs(fs, &root, &base, draft_purchase_id).await
}

fn sample_purchase_pdf_root(tz: &str, date_fmt: &str, datetime_fmt: &str) -> PdfRoot {
    let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
    let (datetime_display, datetime_year, datetime_month, datetime_day) =
        purchase_date_parts(dt, tz, datetime_fmt);
    let delivery = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    let due1 = NaiveDate::from_ymd_opt(2026, 2, 23).unwrap();
    let due2 = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
    let delivery_display = format_pref_calendar_date(delivery, date_fmt);
    let due1_display = format_pref_calendar_date(due1, date_fmt);
    let due2_display = format_pref_calendar_date(due2, date_fmt);
    PdfRoot {
        id: 1,
        number: Some("INV/2025-26/0042".into()),
        reference: Some("PO-1001".into()),
        payment_reference: Some("Payment ref: SAMPLE-001".into()),
        bank_account: Some("1234567890 - Sample Bank".into()),
        remarks: Some("Goods delivered in good condition.".into()),
        datetime: datetime_display.clone(),
        datetime_display,
        datetime_year,
        datetime_month,
        datetime_day,
        delivery_date: delivery_display.clone(),
        delivery_date_display: delivery_display,
        vendor_id: 1,
        vendor: PdfVendor {
            id: 1,
            vendor_type: "business".into(),
            name: "Acme Industries Pvt. Ltd.".into(),
            address: Some(
                "123 Example Street, \\ \
                 Business Park, \\ \
                 Mumbai 400001 \\ \
                 Maharashtra \\ \
                 India"
                    .into(),
            ),
            address_line_1: Some("123 Example Street".into()),
            address_line_2: Some("Business Park".into()),
            city: Some("Mumbai".into()),
            pincode: Some("400001".into()),
            state: Some("Maharashtra".into()),
            gstin: Some("27AAAAA0000A1Z5".into()),
            cin: None,
            pan: Some("AAAAA0000A".into()),
            phone: Some("+91 98765 43210".into()),
            email: Some("billing@example.com".into()),
            website: None,
        },
        payment_term: PdfPaymentTerm {
            id: 1,
            summary: format!("{due1_display}: 38232; {due2_display}: 25488"),
            lines: vec![
                PdfPaymentTermLine::formatted(due1_display, "38232"),
                PdfPaymentTermLine::formatted(due2_display, "25488"),
            ],
        },
        taxes: vec![
            PdfTax {
                id: 1,
                name: "SGST 9%".into(),
                percentage: "9".into(),
                tax_type: "levied".into(),
            },
            PdfTax {
                id: 2,
                name: "CGST 9%".into(),
                percentage: "9".into(),
                tax_type: "levied".into(),
            },
        ],
        lines: vec![PdfLine {
            id: 1,
            product_id: 1,
            product: PdfProduct {
                id: 1,
                name: "Consulting services — monthly retainer".into(),
                product_type: "Services".into(),
                hsn_code: 9983,
                reference: "Project: Alpha".into(),
                remarks: String::new(),
                variable_types: serde_json::Map::new(),
                base_price_formula: r#"decimal("27000")"#.into(),
                sales_price_formula: r#"decimal("27000")"#.into(),
            },
            quantity: "1".into(),
            rate: "54000".into(),
            amount: "54000".into(),
            remarks: "Delivered to the warehouse dock.".into(),
            variables: serde_json::Map::new(),
            variable_lines: Vec::new(),
            taxes: vec![],
        }],
        payments: vec![],
        company_name: String::new(),
        company_address: String::new(),
        company_phone: String::new(),
        company_gstin: String::new(),
        place_of_supply: "Maharashtra".into(),
        company_logo_vnode_id: None,
        company_signature_vnode_id: None,
    }
}

/// Render a sample purchase PDF using an optional template override (blank → built-in example).
///
/// When `date_format` / `datetime_format` are non-empty they override saved preferences for the
/// preview (so unsaved preference form values are reflected).
pub async fn render_purchase_pdf_preview(
    fs: &FilesystemState,
    template_src: Option<&str>,
    tz: &str,
    date_format: Option<&str>,
    datetime_format: Option<&str>,
) -> Result<PurchasePdfResult, PurchasePdfError> {
    let tmpl_src = template_src
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_PURCHASE_PDF_TEMPLATE);
    let prefs = load_purchase_preferences(&fs.db).await;
    let date_fmt = date_format
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| purchase_date_format(&prefs));
    let datetime_fmt = datetime_format
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| purchase_datetime_format(&prefs));
    let mut root = sample_purchase_pdf_root(tz, date_fmt, datetime_fmt);
    apply_pdf_presentation_prefs(&mut root, &prefs);
    let work_dir = typst::typst_work_dir();
    let vnode_ctx = Arc::new(VnodeImageContext::new(
        fs.db.clone(),
        Arc::clone(&fs.store),
        work_dir.clone(),
    ));
    let typst_src = render_template(
        tmpl_src,
        &root,
        collect_purchase_pdf_sample_extras(),
        &work_dir,
        Some(vnode_ctx.as_ref()),
    )?;
    let pdf_bytes = typst::typst_compile_in(&work_dir, &typst_src)
        .await
        .map_err(|e| PurchasePdfError::msg(format!("Typst compile failed:\n{e}")))?;
    if let Err(e) = std::fs::remove_dir_all(&work_dir) {
        tracing::warn!(error = %e, path = %work_dir.display(), "failed to remove purchase pdf preview work dir");
    }
    Ok(PurchasePdfResult {
        bytes: pdf_bytes,
        filename_base: "purchase-preview".to_string(),
    })
}

async fn render_pdf_from_prefs(
    fs: &FilesystemState,
    root: &PdfRoot,
    filename_base: &str,
    draft_purchase_id: Option<i64>,
) -> Result<PurchasePdfResult, PurchasePdfError> {
    let prefs = load_purchase_preferences(&fs.db).await;
    // Blank saved template → shipped example. Render/compile errors are returned as-is
    // (never retried with the default template).
    let tmpl_src = prefs
        .purchase_pdf_template
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_PURCHASE_PDF_TEMPLATE);
    let extras = match draft_purchase_id {
        Some(id) => collect_purchase_pdf_extras(&fs.db, id)
            .await
            .map_err(PurchasePdfError::msg)?,
        None => serde_json::json!({}),
    };
    let work_dir = typst::typst_work_dir();
    let vnode_ctx = VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.clone());
    let typst_src = render_template(tmpl_src, root, extras, &work_dir, Some(&vnode_ctx))?;
    let pdf_bytes = typst::typst_compile_in(&work_dir, &typst_src)
        .await
        .map_err(|e| PurchasePdfError::msg(format!("Typst compile failed:\n{e}")))?;
    if let Err(e) = std::fs::remove_dir_all(&work_dir) {
        tracing::warn!(error = %e, path = %work_dir.display(), "failed to remove purchase pdf work dir");
    }
    Ok(PurchasePdfResult {
        bytes: pdf_bytes,
        filename_base: filename_base.to_string(),
    })
}

fn merge_pdf_context(
    root: &PdfRoot,
    extras: serde_json::Value,
) -> Result<serde_json::Value, PurchasePdfError> {
    let mut value = serde_json::to_value(root)
        .map_err(|e| PurchasePdfError::msg(format!("serialize purchase PDF context: {e}")))?;
    let Some(obj) = value.as_object_mut() else {
        return Err(PurchasePdfError::msg("purchase PDF context is not an object"));
    };
    if let serde_json::Value::Object(extra) = extras {
        for (key, extra_value) in extra {
            obj.entry(key).or_insert(extra_value);
        }
    }
    Ok(value)
}

fn render_template(
    tmpl_src: &str,
    root: &PdfRoot,
    extras: serde_json::Value,
    asset_dir: &Path,
    vnode_ctx: Option<&VnodeImageContext>,
) -> Result<String, PurchasePdfError> {
    let grand_words = purchase_amount_words_from_decimal(pdf_receivable_grand_total(root));
    let ctx = merge_pdf_context(root, extras)?;
    let mut env = Environment::new();
    // Print/iterate of missing names must fail; a silent empty substitution would
    // still compile the rest of the layout and look like the default template.
    env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
    env.add_function("num2words", num2words_fn);
    env.add_function("num2wordsAnd", num2words_and_fn);
    env.add_function("num2wordsRupees", num2words_rupees_fn);
    let asset_dir = asset_dir.to_path_buf();
    env.add_function(
        "urlImage",
        move |url: String| -> Result<String, minijinja::Error> {
            url_image_sync(&url, &asset_dir)
                .map_err(|e| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e))
        },
    );
    if let Some(ctx) = vnode_ctx {
        let ctx = ctx.clone();
        env.add_function(
            "vnodeImage",
            move |vnode_id: i64| -> Result<String, minijinja::Error> {
                ctx.resolve_sync(vnode_id)
                    .map_err(|e| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e))
            },
        );
    }
    env.add_function(
        "purchaseGrandTotalWords",
        move || -> Result<String, minijinja::Error> { Ok(grand_words.clone()) },
    );
    let tmpl = env
        .template_from_str(tmpl_src)
        .map_err(|e| PurchasePdfError::msg(format!("invalid purchase PDF template: {e}")))?;
    tmpl.render(ctx)
        .map_err(|e| PurchasePdfError::msg(format!("rendering purchase PDF template failed: {e}")))
}

fn num2words_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(num2words_cardinal(n))
}

fn num2words_and_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(num2words_and(n))
}

fn num2words_rupees_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(purchase_amount_words(n))
}

pub fn pdf_filename_base(number: Option<&str>, fallback: &str) -> String {
    if let Some(n) = number.map(str::trim).filter(|s| !s.is_empty()) {
        sanitize_pdf_filename_base(n)
    } else {
        fallback.to_string()
    }
}

pub fn sanitize_pdf_filename_base(s: &str) -> String {
    let mut s = s.trim().to_string();
    for ch in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
        s = s.replace(ch, "-");
    }
    if s.is_empty() {
        "purchase".to_string()
    } else {
        s
    }
}

fn num2words_cardinal(n: i64) -> String {
    Num2Words::new(n)
        .lang(Lang::English)
        .to_words()
        .unwrap_or_default()
}

fn num2words_and(n: i64) -> String {
    let words = num2words_cardinal(n);
    if words.contains(" and ") {
        return words;
    }
    let parts: Vec<&str> = words.split_whitespace().collect();
    if parts.len() >= 3 {
        let mut out = parts[..parts.len() - 2].join(" ");
        out.push_str(" and ");
        out.push_str(parts[parts.len() - 2]);
        out.push(' ');
        out.push_str(parts[parts.len() - 1]);
        return out;
    }
    words
}

fn title_word(w: &str) -> String {
    w.split('-')
        .map(|seg| {
            let mut c = seg.chars();
            match c.next() {
                None => String::new(),
                Some(f) => {
                    f.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn title_purchase_words(s: &str) -> String {
    s.split_whitespace()
        .map(|p| {
            if p.eq_ignore_ascii_case("and") {
                "And".to_string()
            } else {
                title_word(p)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn purchase_amount_words(amount: i64) -> String {
    let words = if amount < 0 {
        title_purchase_words(&num2words_and(amount))
    } else {
        title_purchase_words(&num2words_and(amount))
    };
    format!("{words} Rupees")
}

pub fn purchase_amount_words_from_decimal(d: Decimal) -> String {
    let rounded = d.round().to_string().parse::<i64>().unwrap_or(0);
    purchase_amount_words(rounded)
}

fn pdf_receivable_grand_total(root: &PdfRoot) -> Decimal {
    let mut totals = PurchaseLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &root.lines {
        let untaxed: Decimal = line.amount.parse().unwrap_or(Decimal::ZERO);
        let taxes: Vec<tax::Model> = line
            .taxes
            .iter()
            .map(|t| tax::Model {
                id: t.id,
                created_at: None,
                updated_at: None,
                name: t.name.clone(),
                percentage: t.percentage.parse().unwrap_or(Decimal::ZERO),
                tax_type: TaxKind::parse(&t.tax_type).unwrap_or(TaxKind::Levied),
                account_id: None,
            })
            .collect();
        let (untaxed, levied, withholding, _) = purchase_line_amounts(untaxed, &taxes);
        totals.untaxed_subtotal += untaxed;
        totals.lines_levied += levied;
        totals.lines_withholding += withholding;
        merge_purchase_line_tax_ids(&mut line_tax_ids, &taxes);
    }
    let header_taxes: Vec<tax::Model> = root
        .taxes
        .iter()
        .map(|t| tax::Model {
            id: t.id,
            created_at: None,
            updated_at: None,
            name: t.name.clone(),
            percentage: t.percentage.parse().unwrap_or(Decimal::ZERO),
            tax_type: TaxKind::parse(&t.tax_type).unwrap_or(TaxKind::Levied),
            account_id: None,
        })
        .collect();
    purchase_receivable_grand_total(&totals, &header_taxes, &line_tax_ids)
}

/// Download a remote image into `asset_dir` and return a local filename for Typst.
///
/// The filename is a SHA-256 hash of the URL (cache key), not the original name.
fn url_image_sync(url: &str, asset_dir: &Path) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err("urlImage: empty URL".into());
    }
    std::fs::create_dir_all(asset_dir).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash_name: String = hasher.finalize().encode_hex();
    let ext = Path::new(url)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .filter(|e| !e.contains('?'))
        .unwrap_or_else(|| ".png".to_string());
    let filename = format!("{hash_name}{ext}");
    let tmp_path = asset_dir.join(&filename);
    if is_valid_cached_image(&tmp_path) {
        return Ok(filename);
    }
    if let Err(e) = std::fs::remove_file(&tmp_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(error = %e, path = %tmp_path.display(), "failed to remove stale url image cache");
        }
    }
    // reqwest::blocking must not run on the tokio runtime thread — spawn a plain
    // std thread so the blocking client's internal runtime can shut down safely.
    let url = url.to_string();
    std::thread::Builder::new()
        .name("url-image-fetch".into())
        .spawn(move || download_url_to_file(&url, &tmp_path))
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "urlImage: download thread panicked".to_string())??;
    Ok(filename)
}

fn is_valid_cached_image(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    is_valid_image_bytes(&bytes)
}

fn is_valid_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8\xff")
        || (bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
}

fn download_url_to_file(url: &str, tmp_path: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("urlImage: HTTP client: {e}"))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("urlImage: fetch {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("urlImage: fetch {url}: HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| format!("urlImage: read {url}: {e}"))?;
    if !is_valid_image_bytes(&bytes) {
        return Err(format!(
            "urlImage: {url}: response is not a recognized image"
        ));
    }
    std::fs::write(tmp_path, &bytes).map_err(|e| format!("urlImage: write cache: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal::Decimal;
    fn sample_example_purchase_root() -> PdfRoot {
        sample_purchase_pdf_root(
            lariv_core::datetime::DEFAULT_TIMEZONE,
            lariv_core::datetime::DATE_FMT,
            lariv_core::datetime::DATE_FMT,
        )
    }

    #[test]
    fn individual_bill_to_omits_tax_identity() {
        let pdf = pdf_vendor_from_contact(contact::Model {
            id: 7,
            created_at: None,
            updated_at: None,
            company_id: None,
            name: "Ada".into(),
            email: Some("ada@example.com".into()),
            phone: Some("555".into()),
            is_primary: false,
        });
        assert_eq!(pdf.vendor_type, "individual");
        assert_eq!(pdf.name, "Ada");
        assert!(pdf.gstin.is_none());
        assert!(pdf.address.is_none());
        assert_eq!(pdf.email.as_deref(), Some("ada@example.com"));
    }

    #[test]
    fn company_bill_to_keeps_gstin() {
        let pdf = pdf_vendor_from_company(company::Model {
            id: 3,
            created_at: None,
            updated_at: None,
            name: "Acme".into(),
            address_line_1: Some("1 Road".into()),
            address_line_2: None,
            city: None,
            pincode: None,
            state: None,
            website: None,
            gstin: Some("27AAAAA0000A1Z5".into()),
            cin: None,
            pan: None,
            phone: None,
            email: None,
        });
        assert_eq!(pdf.vendor_type, "business");
        assert_eq!(pdf.gstin.as_deref(), Some("27AAAAA0000A1Z5"));
        assert_eq!(pdf.address_line_1.as_deref(), Some("1 Road"));
    }

    #[test]
    fn num2words_rupees_matches_go_sample() {
        assert_eq!(
            purchase_amount_words(63_720),
            "Sixty-Three Thousand Seven Hundred And Twenty Rupees"
        );
    }

    #[test]
    fn grand_total_words_for_sample_purchase() {
        let root = sample_example_purchase_root();
        let grand = pdf_receivable_grand_total(&root);
        assert_eq!(grand, Decimal::from(63_720));
        assert_eq!(
            purchase_amount_words_from_decimal(grand),
            "Sixty-Three Thousand Seven Hundred And Twenty Rupees"
        );
    }

    #[test]
    fn example_purchase_pdf_template_renders() {
        let root = sample_example_purchase_root();
        let asset_dir = std::env::temp_dir().join("lariv-purchase-pdf-test");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            DEFAULT_PURCHASE_PDF_TEMPLATE,
            &root,
            serde_json::json!({}),
            &asset_dir,
            None,
        )
        .expect("render");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(out.contains("Sixty-Three Thousand Seven Hundred And Twenty Rupees"));
        assert!(out.contains("Acme Industries Pvt. Ltd."));
        assert!(out.contains("INV/2025-26/0042"));
        assert!(out.contains("08/02/2026"));
        assert!(!out.contains("```"));
        assert!(out.contains("Mumbai 400001"));
        assert!(out.contains("GSTIN/UIN: 27AAAAA0000A1Z5"));
        assert!(out.contains("Place of Supply: Maharashtra"));
        assert!(out.contains("dict-sum-prefix(tax-totals, \"SGST\")"));
        assert!(out.contains("*Company's Bank Details*"));
        assert!(out.contains("1234567890 - Sample Bank"));
        assert!(out.contains("*Payment Schedule*"));
        assert!(out.contains(
            r#"#text(size: 7.5pt, style: "italic", fill: luma(110))[Delivered to the warehouse dock.]"#
        ));
        assert!(out.contains("This is a Computer Generated Purchase"));
        assert!(out.contains("Goods delivered in good condition."));
    }

    #[test]
    fn example_purchase_pdf_template_renders_sites() {
        let root = sample_example_purchase_root();
        let extras = serde_json::json!({
            "Sites": [{
                "ID": 1,
                "Name": "North Yard",
                "Address": "Plot 12, Industrial Area",
            }]
        });
        let asset_dir = std::env::temp_dir().join("lariv-purchase-pdf-test-sites");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            DEFAULT_PURCHASE_PDF_TEMPLATE,
            &root,
            extras,
            &asset_dir,
            None,
        )
        .expect("render");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(out.contains("*Sites:* North Yard"));
    }

    #[test]
    fn minijinja_renders_simple_template() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        let (datetime_display, datetime_year, datetime_month, datetime_day) = purchase_date_parts(
            dt,
            lariv_core::datetime::DEFAULT_TIMEZONE,
            lariv_core::datetime::DATE_FMT,
        );
        let root = PdfRoot {
            id: 1,
            number: Some("INV-1".into()),
            reference: None,
            payment_reference: None,
            bank_account: None,
            remarks: None,
            datetime: datetime_display.clone(),
            datetime_display,
            datetime_year,
            datetime_month,
            datetime_day,
            delivery_date: String::new(),
            delivery_date_display: String::new(),
            vendor_id: 1,
            vendor: PdfVendor {
                id: 1,
                vendor_type: "business".into(),
                name: "Acme".into(),
                address: None,
                address_line_1: None,
                address_line_2: None,
                city: None,
                pincode: None,
                state: None,
                gstin: None,
                cin: None,
                pan: None,
                phone: None,
                email: None,
                website: None,
            },
            payment_term: PdfPaymentTerm {
                id: 1,
                summary: String::new(),
                lines: vec![],
            },
            taxes: vec![],
            lines: vec![],
            payments: vec![],
            company_name: "Test Co".into(),
            company_address: "Test address".into(),
            company_phone: String::new(),
            company_gstin: String::new(),
            place_of_supply: String::new(),
            company_logo_vnode_id: None,
            company_signature_vnode_id: None,
        };
        let asset_dir = std::env::temp_dir().join("lariv-purchase-pdf-test-simple");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            "#set page(paper: \"a4\")\n= {{ Vendor.Name }}\n",
            &root,
            serde_json::json!({}),
            &asset_dir,
            None,
        )
        .expect("render");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(out.contains("Acme"));
    }

    #[test]
    fn render_template_errors_on_undefined_substitution() {
        let root = sample_example_purchase_root();
        let asset_dir = std::env::temp_dir().join("lariv-purchase-pdf-test-undef");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let err = render_template(
            "{{ DoesNotExist }}\n",
            &root,
            serde_json::json!({}),
            &asset_dir,
            None,
        )
        .expect_err("undefined substitution must fail");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let msg = err.to_string();
        assert!(
            msg.contains("rendering purchase PDF template failed"),
            "{msg}"
        );
    }

    #[test]
    fn pdf_line_variables_keep_input_keys() {
        let mut root = sample_example_purchase_root();
        let mut variables = serde_json::Map::new();
        variables.insert("length".into(), serde_json::json!("1000"));
        variables.insert("qty".into(), serde_json::json!("2"));
        root.lines[0].variables = variables;
        let mut types = serde_json::Map::new();
        types.insert("length".into(), serde_json::json!("length"));
        root.lines[0].product.variable_types = types;

        let v = serde_json::to_value(&root).expect("serialize");
        assert_eq!(v["Lines"][0]["Variables"]["length"], "1000");
        assert_eq!(v["Lines"][0]["Variables"]["qty"], "2");
        assert_eq!(
            v["Lines"][0]["Product"]["VariableTypes"]["length"],
            "length"
        );
        assert_eq!(v["Vendor"]["VendorType"], "business");
        assert_eq!(v["Vendor"]["AddressLine1"], "123 Example Street");
        assert_eq!(v["Lines"][0]["Product"]["ProductType"], "Services");

        let asset_dir = std::env::temp_dir().join("lariv-purchase-pdf-test-variables");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            "{{ Lines[0].Variables.length }} / {{ Lines[0].Product.VariableTypes.length }}\n",
            &root,
            serde_json::json!({}),
            &asset_dir,
            None,
        )
        .expect("render variable pair");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(out.contains("1000 / length"), "{out}");
    }

    #[test]
    fn example_template_prints_length_and_weight_units() {
        let mut root = sample_example_purchase_root();
        let mut variables = serde_json::Map::new();
        variables.insert(
            "length".into(),
            serde_json::json!({"value": "2", "unit": "cm"}),
        );
        variables.insert(
            "mass".into(),
            serde_json::json!({"value": "1.5", "unit": "kg"}),
        );
        variables.insert("quantity".into(), serde_json::json!("3"));
        root.lines[0].variables = variables;
        root.lines[0].variable_lines = vec![
            "Length: 2 cm".into(),
            "Mass: 1.5 kg".into(),
            "Quantity: 3".into(),
        ];
        let asset_dir = std::env::temp_dir().join("lariv-purchase-pdf-test-units");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            DEFAULT_PURCHASE_PDF_TEMPLATE,
            &root,
            serde_json::json!({}),
            &asset_dir,
            None,
        )
        .expect("render variables");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(
            out.contains(
                "#text(size: 7.5pt, fill: luma(90))[Length: 2 cm] \\\n#text(size: 7.5pt, fill: luma(90))[Mass: 1.5 kg] \\\n#text(size: 7.5pt, fill: luma(90))[Quantity: 3]"
            ),
            "{out}"
        );
        assert!(!out.contains("[*Variables*]"), "{out}");
    }

    #[test]
    fn pdf_context_dates_follow_strftime() {
        let root = sample_purchase_pdf_root("Asia/Kolkata", "%Y-%m-%d", "%d %b %Y %H:%M");
        let v = serde_json::to_value(&root).expect("serialize");
        assert_eq!(v["DeliveryDate"], "2026-02-15");
        assert_eq!(v["DeliveryDateDisplay"], "2026-02-15");
        assert_eq!(v["Datetime"], "08 Feb 2026 05:30");
        assert_eq!(v["DatetimeDisplay"], "08 Feb 2026 05:30");
        assert_eq!(v["PaymentTerm"]["Lines"][0]["DueDate"], "2026-02-23");
        assert_eq!(v["PaymentTerm"]["Lines"][0]["DueDateDisplay"], "2026-02-23");
        assert_eq!(v["PaymentTerm"]["Lines"][0]["DueDatetime"], "2026-02-23");
        assert_eq!(
            v["PaymentTerm"]["Lines"][0]["DueDatetimeDisplay"],
            "2026-02-23"
        );
        assert_eq!(v["PaymentTerm"]["Lines"][1]["DueDate"], "2026-03-10");
        assert!(
            v["PaymentTerm"]["Summary"]
                .as_str()
                .unwrap_or("")
                .contains("2026-02-23")
        );
    }
}
