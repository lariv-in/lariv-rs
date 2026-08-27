//! Invoice PDF: Minijinja template → Typst → PDF (via typst crate).

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use crate::plugins::customer::entities::customer::Entity as CustomerEntity;
use crate::plugins::finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_format,
};
use crate::plugins::finance_common::{decimal::decimal_display_currency, typst};
use crate::plugins::finance_products::entities::product::Entity as ProductEntity;
use crate::plugins::finance_taxes::entities::tax::{self, TaxKind};
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use hex::ToHex;
use minijinja::Environment;
use num2words::{Lang, Num2Words};
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Statement,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::plugins::filesystem::state::FilesystemState;
use crate::plugins::finance_invoices::entities::preferences;
use crate::plugins::finance_invoices::entities::{
    CancelledInvoiceEntity, DraftInvoiceEntity, DraftInvoiceLineEntity, PaidInvoiceEntity,
    PartiallyPaidInvoiceEntity, PaymentEntity, PostedInvoiceEntity, PostedInvoiceLineEntity,
};
use crate::plugins::finance_invoices::entities::{
    draft_invoice_line, payment, posted_invoice, posted_invoice_line,
};
use crate::plugins::finance_invoices::invoice_pdf_addon::{
    collect_invoice_pdf_extras, collect_invoice_pdf_sample_extras,
};
use crate::plugins::finance_invoices::invoice_pdf_assets::VnodeImageContext;
use crate::plugins::finance_invoices::invoice_pdf_template::DEFAULT_INVOICE_PDF_TEMPLATE;
use crate::plugins::finance_invoices::logic::draft_payment_term::{
    draft_payment_term_line_display, load_draft_payment_term_lines,
    load_posted_payment_term_for_cancelled, load_posted_payment_term_for_posted,
    posted_payment_term_line_display, resolve_due_datetime,
};
use crate::plugins::finance_invoices::logic::preferences::load_invoice_preferences;
use crate::plugins::finance_invoices::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_draft_invoice_tax_ids,
    load_draft_line_tax_ids, load_posted_invoice_tax_ids, load_posted_line_tax_ids,
};
use crate::plugins::finance_invoices::logic::tax_calculations::{
    InvoiceLinesTotals, invoice_line_amount_breakdown, invoice_receivable_grand_total,
    merge_invoice_line_tax_ids,
};

#[derive(Debug, thiserror::Error)]
pub enum InvoicePdfError {
    #[error("{0}")]
    Message(String),
    #[error("not found")]
    NotFound,
}

impl InvoicePdfError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

pub struct InvoicePdfResult {
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
    datetime: DateTime<Utc>,
    datetime_display: String,
    datetime_year: i32,
    datetime_month: u32,
    datetime_day: u32,
    customer_id: i64,
    customer: PdfCustomer,
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
struct PdfCustomer {
    #[serde(rename = "ID")]
    id: i64,
    name: String,
    address: Option<String>,
    #[serde(rename = "GSTIN")]
    gstin: Option<String>,
    #[serde(rename = "PAN")]
    pan: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    website: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfPaymentTermLine {
    due_datetime: DateTime<Utc>,
    due_datetime_display: String,
    amount: String,
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
    #[serde(rename = "HSNCode")]
    hsn_code: i64,
    reference: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfLine {
    #[serde(rename = "ID")]
    id: i64,
    product_id: i64,
    product: PdfProduct,
    rate: String,
    quantity: String,
    taxes: Vec<PdfTax>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PdfPayment {
    #[serde(rename = "ID")]
    id: i64,
    amount: String,
    datetime: DateTime<Utc>,
    datetime_display: String,
}

fn invoice_date_display(dt: DateTime<Utc>, tz: &str) -> String {
    dt.with_timezone(&crate::datetime::parse_timezone(tz))
        .format("%d/%m/%Y")
        .to_string()
}

fn invoice_date_parts(dt: DateTime<Utc>, tz: &str) -> (String, i32, u32, u32) {
    let local = dt.with_timezone(&crate::datetime::parse_timezone(tz));
    (
        local.format("%d/%m/%Y").to_string(),
        local.year(),
        local.month(),
        local.day(),
    )
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

async fn load_customer(db: &DatabaseConnection, id: i64) -> Result<PdfCustomer, InvoicePdfError> {
    let c = CustomerEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    let address = c.formatted_address_for_typst();
    Ok(PdfCustomer {
        id: c.id,
        name: c.name,
        address,
        gstin: c.gstin,
        pan: c.pan,
        phone: c.phone,
        email: c.email,
        website: c.website,
    })
}

async fn load_draft_payment_term_pdf(
    db: &DatabaseConnection,
    draft_id: i64,
    anchor: DateTime<Utc>,
    tz: &str,
    _currency: &CurrencyFormat,
) -> Result<PdfPaymentTerm, InvoicePdfError> {
    let lines = load_draft_payment_term_lines(db, draft_id)
        .await
        .map_err(|e| InvoicePdfError::msg(e))?;
    let pdf_lines: Vec<PdfPaymentTermLine> = lines
        .iter()
        .map(|l| {
            let display = draft_payment_term_line_display(l, tz);
            let due_datetime = resolve_due_datetime(l, anchor)
                .unwrap_or_else(|_| l.due_datetime.unwrap_or(anchor));
            PdfPaymentTermLine {
                due_datetime,
                due_datetime_display: display.due_display,
                amount: display.amount_display,
            }
        })
        .collect();
    let summary = pdf_lines
        .iter()
        .map(|l| format!("{}: {}", l.due_datetime_display, l.amount))
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
    posted_invoice_id: Option<i64>,
    cancelled_invoice_id: Option<i64>,
    tz: &str,
    currency: &CurrencyFormat,
) -> Result<PdfPaymentTerm, InvoicePdfError> {
    let loaded = if let Some(pid) = posted_invoice_id {
        load_posted_payment_term_for_posted(db, pid).await
    } else if let Some(cid) = cancelled_invoice_id {
        load_posted_payment_term_for_cancelled(db, cid).await
    } else {
        Ok(None)
    }
    .map_err(|e| InvoicePdfError::msg(e))?;

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
            let display =
                posted_payment_term_line_display(l, tz, currency.minor_unit, &currency.symbol);
            PdfPaymentTermLine {
                due_datetime: l.due_datetime,
                due_datetime_display: display.due_display,
                amount: display.amount_display,
            }
        })
        .collect();
    let summary = pdf_lines
        .iter()
        .map(|l| format!("{}: {}", l.due_datetime_display, l.amount))
        .collect::<Vec<_>>()
        .join("; ");
    Ok(PdfPaymentTerm {
        id: term.id,
        summary,
        lines: pdf_lines,
    })
}

async fn load_product_pdf(db: &DatabaseConnection, id: i64) -> Result<PdfProduct, InvoicePdfError> {
    let p = ProductEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    Ok(PdfProduct {
        id: p.id,
        name: p.name,
        hsn_code: p.hsn_code,
        reference: p.reference.unwrap_or_default(),
    })
}

async fn load_payments_for_posted(
    db: &DatabaseConnection,
    posted_id: i64,
    tz: &str,
    currency: &CurrencyFormat,
) -> Result<Vec<PdfPayment>, InvoicePdfError> {
    let rows = PaymentEntity::find()
        .filter(payment::Column::PostedInvoiceId.eq(posted_id))
        .order_by_asc(payment::Column::Datetime)
        .all(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|p| {
            let datetime_display = invoice_date_display(p.datetime, tz);
            PdfPayment {
                id: p.id,
                amount: money_str(p.amount, currency),
                datetime: p.datetime,
                datetime_display,
            }
        })
        .collect())
}

struct LineRow {
    id: i64,
    product_id: i64,
    rate: Decimal,
    quantity: Decimal,
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
) -> Result<Vec<i64>, InvoicePdfError> {
    let ids = match source {
        LineTaxSource::Draft => load_draft_line_tax_ids(db, line_id).await,
        LineTaxSource::Posted => load_posted_line_tax_ids(db, line_id).await,
        LineTaxSource::Cancelled => load_cancelled_line_tax_ids(db, line_id).await,
    }
    .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    Ok(ids)
}

async fn load_draft_lines(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<Vec<LineRow>, InvoicePdfError> {
    let rows = DraftInvoiceLineEntity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|l| LineRow {
            id: l.id,
            product_id: l.product_id,
            rate: l.rate,
            quantity: l.quantity,
        })
        .collect())
}

async fn load_posted_lines(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<Vec<LineRow>, InvoicePdfError> {
    let rows = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .all(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|l| LineRow {
            id: l.id,
            product_id: l.product_id,
            rate: l.rate,
            quantity: l.quantity,
        })
        .collect())
}

async fn load_cancelled_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Result<Vec<LineRow>, InvoicePdfError> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, product_id, rate, quantity FROM cancelled_invoice_lines \
             WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        let id: i64 = r
            .try_get("", "id")
            .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
        let product_id: i64 = r
            .try_get("", "product_id")
            .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
        let rate: Decimal = r
            .try_get("", "rate")
            .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
        let quantity: Decimal = r
            .try_get("", "quantity")
            .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
        out.push(LineRow {
            id,
            product_id,
            rate,
            quantity,
        });
    }
    Ok(out)
}

async fn build_pdf_lines(
    db: &DatabaseConnection,
    rows: &[LineRow],
    tax_source: LineTaxSource,
    currency: &CurrencyFormat,
) -> Result<Vec<PdfLine>, InvoicePdfError> {
    let mut lines = Vec::with_capacity(rows.len());
    for row in rows {
        let tax_ids = line_tax_ids(db, tax_source, row.id).await?;
        let taxes = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| InvoicePdfError::msg(e.to_string()))?
            .into_iter()
            .map(|t| tax_to_pdf(&t))
            .collect();
        let product = load_product_pdf(db, row.product_id).await?;
        lines.push(PdfLine {
            id: row.id,
            product_id: row.product_id,
            product,
            rate: money_str(row.rate, currency),
            quantity: dec_str(row.quantity),
            taxes,
        });
    }
    Ok(lines)
}

/// Company presentation fields from invoice preferences (Finance → Preferences).
fn company_fields_from_prefs(prefs: &preferences::Model) -> (
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
        prefs.company_address.clone().unwrap_or_default(),
        prefs.company_phone.clone().unwrap_or_default(),
        prefs.company_gstin.clone().unwrap_or_default(),
        prefs.place_of_supply.clone().unwrap_or_default(),
        prefs.invoice_logo_vnode_id.filter(|&id| id > 0),
        prefs.invoice_signature_vnode_id.filter(|&id| id > 0),
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
    datetime: DateTime<Utc>,
    customer_id: i64,
    header_tax_ids: Vec<i64>,
    line_rows: Vec<LineRow>,
    tax_source: LineTaxSource,
    payments: Vec<PdfPayment>,
    payment_term: PdfPaymentTerm,
    tz: &str,
    currency: &CurrencyFormat,
    prefs: &preferences::Model,
) -> Result<PdfRoot, InvoicePdfError> {
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .into_iter()
        .map(|t| tax_to_pdf(&t))
        .collect();
    let lines = build_pdf_lines(db, &line_rows, tax_source, currency).await?;
    let (datetime_display, datetime_year, datetime_month, datetime_day) =
        invoice_date_parts(datetime, tz);
    let (company_name, company_address, company_phone, company_gstin, place_of_supply, logo, signature) =
        company_fields_from_prefs(prefs);
    Ok(PdfRoot {
        id,
        number,
        reference,
        payment_reference,
        bank_account,
        datetime,
        datetime_display,
        datetime_year,
        datetime_month,
        datetime_day,
        customer_id,
        customer: load_customer(db, customer_id).await?,
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

pub async fn render_draft_invoice_pdf(
    fs: &FilesystemState,
    id: i64,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let db = &fs.db;
    let draft = DraftInvoiceEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    let header_tax_ids = load_draft_invoice_tax_ids(db, draft.id)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    let line_rows = load_draft_lines(db, draft.id).await?;
    let prefs = load_invoice_preferences(db).await;
    let currency = match prefs.journal_id.filter(|&id| id > 0) {
        Some(jid) => load_journal_currency_format(db, jid).await,
        None => load_default_currency_format(db).await,
    };
    let payment_term =
        load_draft_payment_term_pdf(db, draft.id, draft.datetime, tz, &currency).await?;
    let root = build_pdf_root(
        db,
        draft.id,
        draft.number.clone(),
        draft.reference.clone(),
        draft.payment_reference.clone(),
        draft.bank_account.clone(),
        draft.datetime,
        draft.customer_id,
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
        &format!("draft-invoice-{}", draft.id),
    );
    render_pdf_from_prefs(fs, &root, &base, Some(draft.id)).await
}

pub async fn render_posted_invoice_pdf(
    fs: &FilesystemState,
    posted: posted_invoice::Model,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let db = &fs.db;
    let header_tax_ids = load_posted_invoice_tax_ids(db, posted.id)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    let line_rows = load_posted_lines(db, posted.id).await?;
    let currency = load_journal_currency_format(db, posted.journal_id).await;
    let payments = load_payments_for_posted(db, posted.id, tz, &currency).await?;
    let payment_term =
        load_posted_payment_term_pdf(db, Some(posted.id), None, tz, &currency).await?;
    let prefs = load_invoice_preferences(db).await;
    let root = build_pdf_root(
        db,
        posted.id,
        Some(posted.number.clone()),
        posted.reference.clone(),
        posted.payment_reference.clone(),
        posted.bank_account.clone(),
        posted.datetime,
        posted.customer_id,
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
    let base = pdf_filename_base(Some(&posted.number), &format!("invoice-{}", posted.id));
    render_pdf_from_prefs(fs, &root, &base, Some(posted.draft_invoice_id)).await
}

pub async fn render_cancelled_invoice_pdf(
    fs: &FilesystemState,
    id: i64,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let db = &fs.db;
    let inv = CancelledInvoiceEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    let header_tax_ids = load_cancelled_invoice_tax_ids(db, inv.id)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?;
    let line_rows = load_cancelled_lines(db, inv.id).await?;
    let currency = load_journal_currency_format(db, inv.journal_id).await;
    let payments = load_payments_for_posted(db, inv.posted_invoice_id, tz, &currency).await?;
    let payment_term = load_posted_payment_term_pdf(db, None, Some(inv.id), tz, &currency).await?;
    let prefs = load_invoice_preferences(db).await;
    let root = build_pdf_root(
        db,
        inv.id,
        Some(inv.number.clone()),
        inv.reference.clone(),
        inv.payment_reference.clone(),
        inv.bank_account.clone(),
        inv.datetime,
        inv.customer_id,
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
    let draft_invoice_id = PostedInvoiceEntity::find_by_id(inv.posted_invoice_id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .map(|p| p.draft_invoice_id);
    let base = pdf_filename_base(Some(&inv.number), &format!("cancelled-invoice-{}", inv.id));
    render_pdf_from_prefs(fs, &root, &base, draft_invoice_id).await
}

pub async fn render_paid_invoice_pdf(
    fs: &FilesystemState,
    id: i64,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let db = &fs.db;
    let paid = PaidInvoiceEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    let posted = PostedInvoiceEntity::find_by_id(paid.posted_invoice_id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    render_posted_invoice_pdf(fs, posted, tz).await
}

pub async fn render_partially_paid_invoice_pdf(
    fs: &FilesystemState,
    id: i64,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let db = &fs.db;
    let partial = PartiallyPaidInvoiceEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    let posted = PostedInvoiceEntity::find_by_id(partial.posted_invoice_id)
        .one(db)
        .await
        .map_err(|e| InvoicePdfError::msg(e.to_string()))?
        .ok_or(InvoicePdfError::NotFound)?;
    render_posted_invoice_pdf(fs, posted, tz).await
}

/// Sample invoice data for PDF template preview (matches the built-in example layout).
fn sample_invoice_pdf_root(tz: &str) -> PdfRoot {
    let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
    let (datetime_display, datetime_year, datetime_month, datetime_day) =
        invoice_date_parts(dt, tz);
    PdfRoot {
        id: 1,
        number: Some("INV/2025-26/0042".into()),
        reference: Some("PO-1001".into()),
        payment_reference: Some("Payment ref: SAMPLE-001".into()),
        bank_account: Some("1234567890 - Sample Bank".into()),
        datetime: dt,
        datetime_display,
        datetime_year,
        datetime_month,
        datetime_day,
        customer_id: 1,
        customer: PdfCustomer {
            id: 1,
            name: "Acme Industries Pvt. Ltd.".into(),
            address: Some(
                "123 Example Street, \\ \
                 Business Park, \\ \
                 Mumbai 400001 \\ \
                 Maharashtra \\ \
                 India"
                    .into(),
            ),
            gstin: Some("27AAAAA0000A1Z5".into()),
            pan: Some("AAAAA0000A".into()),
            phone: Some("+91 98765 43210".into()),
            email: Some("billing@example.com".into()),
            website: None,
        },
        payment_term: PdfPaymentTerm {
            id: 1,
            summary: "23/02/2026: 38232; 10/03/2026: 25488".into(),
            lines: vec![
                PdfPaymentTermLine {
                    due_datetime: Utc.with_ymd_and_hms(2026, 2, 23, 0, 0, 0).unwrap(),
                    due_datetime_display: "23/02/2026".into(),
                    amount: "38232".into(),
                },
                PdfPaymentTermLine {
                    due_datetime: Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap(),
                    due_datetime_display: "10/03/2026".into(),
                    amount: "25488".into(),
                },
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
                hsn_code: 9983,
                reference: "Project: Alpha".into(),
            },
            rate: "27000".into(),
            quantity: "2".into(),
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

/// Render a sample invoice PDF using an optional template override (blank → built-in example).
pub async fn render_invoice_pdf_preview(
    fs: &FilesystemState,
    template_src: Option<&str>,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let tmpl_src = template_src
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_INVOICE_PDF_TEMPLATE);
    let mut root = sample_invoice_pdf_root(tz);
    let prefs = load_invoice_preferences(&fs.db).await;
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
        collect_invoice_pdf_sample_extras(),
        &work_dir,
        Some(vnode_ctx.as_ref()),
    )?;
    let pdf_bytes = typst::typst_compile_in(&work_dir, &typst_src)
        .await
        .map_err(InvoicePdfError::msg)?;
    if let Err(e) = std::fs::remove_dir_all(&work_dir) {
        tracing::warn!(error = %e, path = %work_dir.display(), "failed to remove invoice pdf preview work dir");
    }
    Ok(InvoicePdfResult {
        bytes: pdf_bytes,
        filename_base: "invoice-preview".to_string(),
    })
}

async fn render_pdf_from_prefs(
    fs: &FilesystemState,
    root: &PdfRoot,
    filename_base: &str,
    draft_invoice_id: Option<i64>,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    let prefs = load_invoice_preferences(&fs.db).await;
    let tmpl_src = prefs
        .invoice_pdf_template
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_INVOICE_PDF_TEMPLATE);
    let extras = match draft_invoice_id {
        Some(id) => collect_invoice_pdf_extras(&fs.db, id)
            .await
            .map_err(InvoicePdfError::msg)?,
        None => serde_json::json!({}),
    };
    let work_dir = typst::typst_work_dir();
    let vnode_ctx = VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.clone());
    let typst_src = render_template(tmpl_src, root, extras, &work_dir, Some(&vnode_ctx))?;
    let pdf_bytes = typst::typst_compile_in(&work_dir, &typst_src)
        .await
        .map_err(InvoicePdfError::msg)?;
    if let Err(e) = std::fs::remove_dir_all(&work_dir) {
        tracing::warn!(error = %e, path = %work_dir.display(), "failed to remove invoice pdf work dir");
    }
    Ok(InvoicePdfResult {
        bytes: pdf_bytes,
        filename_base: filename_base.to_string(),
    })
}

fn merge_pdf_context(
    root: &PdfRoot,
    extras: serde_json::Value,
) -> Result<serde_json::Value, InvoicePdfError> {
    let mut value = serde_json::to_value(root)
        .map_err(|e| InvoicePdfError::msg(format!("serialize invoice PDF context: {e}")))?;
    let Some(obj) = value.as_object_mut() else {
        return Err(InvoicePdfError::msg("invoice PDF context is not an object"));
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
) -> Result<String, InvoicePdfError> {
    let grand_words = invoice_amount_words_from_decimal(pdf_receivable_grand_total(root));
    let ctx = merge_pdf_context(root, extras)?;
    let mut env = Environment::new();
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
        "invoiceGrandTotalWords",
        move || -> Result<String, minijinja::Error> { Ok(grand_words.clone()) },
    );
    let tmpl = env
        .template_from_str(tmpl_src)
        .map_err(|e| InvoicePdfError::msg(format!("invalid invoice PDF template: {e}")))?;
    tmpl.render(ctx)
        .map_err(|e| InvoicePdfError::msg(format!("rendering invoice PDF template failed: {e}")))
}

fn num2words_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(num2words_cardinal(n))
}

fn num2words_and_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(num2words_and(n))
}

fn num2words_rupees_fn(n: i64) -> Result<String, minijinja::Error> {
    Ok(invoice_amount_words(n))
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
        "invoice".to_string()
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

fn title_invoice_words(s: &str) -> String {
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

pub fn invoice_amount_words(amount: i64) -> String {
    let words = if amount < 0 {
        title_invoice_words(&num2words_and(amount))
    } else {
        title_invoice_words(&num2words_and(amount))
    };
    format!("{words} Rupees")
}

pub fn invoice_amount_words_from_decimal(d: Decimal) -> String {
    let rounded = d.round().to_string().parse::<i64>().unwrap_or(0);
    invoice_amount_words(rounded)
}

fn pdf_receivable_grand_total(root: &PdfRoot) -> Decimal {
    let mut totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in &root.lines {
        let qty: Decimal = line.quantity.parse().unwrap_or(Decimal::ZERO);
        let rate: Decimal = line.rate.parse().unwrap_or(Decimal::ZERO);
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
        let (untaxed, levied, withholding, _) = invoice_line_amount_breakdown(qty, rate, &taxes);
        totals.untaxed_subtotal += untaxed;
        totals.lines_levied += levied;
        totals.lines_withholding += withholding;
        merge_invoice_line_tax_ids(&mut line_tax_ids, &taxes);
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
    invoice_receivable_grand_total(&totals, &header_taxes, &line_tax_ids)
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
    fn sample_example_invoice_root() -> PdfRoot {
        sample_invoice_pdf_root(crate::datetime::DEFAULT_TIMEZONE)
    }

    #[test]
    fn num2words_rupees_matches_go_sample() {
        assert_eq!(
            invoice_amount_words(63_720),
            "Sixty-Three Thousand Seven Hundred And Twenty Rupees"
        );
    }

    #[test]
    fn grand_total_words_for_sample_invoice() {
        let root = sample_example_invoice_root();
        let grand = pdf_receivable_grand_total(&root);
        assert_eq!(grand, Decimal::from(63_720));
        assert_eq!(
            invoice_amount_words_from_decimal(grand),
            "Sixty-Three Thousand Seven Hundred And Twenty Rupees"
        );
    }

    #[test]
    fn example_invoice_pdf_template_renders() {
        let root = sample_example_invoice_root();
        let asset_dir = std::env::temp_dir().join("lariv-invoice-pdf-test");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            DEFAULT_INVOICE_PDF_TEMPLATE,
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
        assert!(out.contains("GSTIN: 27AAAAA0000A1Z5"));
        assert!(out.contains("Place of supply: Maharashtra"));
        assert!(out.contains("dict-sum-prefix(tax-totals, \"SGST\")"));
    }

    #[test]
    fn example_invoice_pdf_template_renders_sites() {
        let root = sample_example_invoice_root();
        let extras = serde_json::json!({
            "Sites": [{
                "ID": 1,
                "Name": "North Yard",
                "Address": "Plot 12, Industrial Area",
            }]
        });
        let asset_dir = std::env::temp_dir().join("lariv-invoice-pdf-test-sites");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            DEFAULT_INVOICE_PDF_TEMPLATE,
            &root,
            extras,
            &asset_dir,
            None,
        )
        .expect("render");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(out.contains("North Yard"));
        assert!(out.contains("Plot 12, Industrial Area"));
    }

    #[test]
    fn minijinja_renders_simple_template() {
        let dt = Utc.with_ymd_and_hms(2026, 2, 8, 0, 0, 0).unwrap();
        let (datetime_display, datetime_year, datetime_month, datetime_day) =
            invoice_date_parts(dt, crate::datetime::DEFAULT_TIMEZONE);
        let root = PdfRoot {
            id: 1,
            number: Some("INV-1".into()),
            reference: None,
            payment_reference: None,
            bank_account: None,
            datetime: dt,
            datetime_display,
            datetime_year,
            datetime_month,
            datetime_day,
            customer_id: 1,
            customer: PdfCustomer {
                id: 1,
                name: "Acme".into(),
                address: None,
                gstin: None,
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
        let asset_dir = std::env::temp_dir().join("lariv-invoice-pdf-test-simple");
        let _ = std::fs::remove_dir_all(&asset_dir);
        let out = render_template(
            "#set page(paper: \"a4\")\n= {{ Customer.Name }}\n",
            &root,
            serde_json::json!({}),
            &asset_dir,
            None,
        )
        .expect("render");
        let _ = std::fs::remove_dir_all(&asset_dir);
        assert!(out.contains("Acme"));
    }
}
