//! Draft invoice create/update with line editor.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, TransactionTrait,
};
use serde::Deserialize;

use lariv_formula::decimal::{self, parse_decimal};
use lariv_formula::{
    Measure, VariableDef, VariableSchema, VariableType, VariableValues, format_value_lines,
    parse_values,
};
use lariv_plugin_finance_products::{
    entities::product::{Entity as ProductEntity, ProductKind},
    preferences::load_product_tax_ids,
    pricing,
};
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::entities::{
    DraftInvoiceEntity, DraftPaymentTermEntity, draft_invoice, draft_invoice_line,
};
use crate::logic::draft_payment_term::{DraftPaymentTermLineInput, upsert_draft_payment_term};
use crate::logic::tax_assoc::{set_draft_invoice_taxes, set_draft_line_taxes};

#[derive(Debug, Deserialize, Clone)]
pub struct DraftLinePending {
    pub product_id: i64,
    pub rate: Option<String>,
    /// Magnitude of the quantity measure. Multiplies the price of one product. Defaults to 1.
    #[serde(default = "default_line_quantity")]
    pub quantity: String,
    /// Measure kind (`length`, `weight`, `quantity`, …). Defaults to `quantity`.
    #[serde(default = "default_qty_kind")]
    pub qty_kind: String,
    /// Unit for [`Self::qty_kind`]. Empty uses that kind's default unit.
    #[serde(default)]
    pub qty_unit: String,
    /// Raw formula inputs, keyed by the product's variable names.
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub tax_ids: Option<Vec<i64>>,
    /// Optional note stored with this product line.
    #[serde(default)]
    pub remarks: Option<String>,
    /// Name printed for this line. Empty uses the catalog product name.
    #[serde(default)]
    pub product_name: Option<String>,
}

/// Format an invoice datetime as a calendar date (`DD/MM/YYYY`) in `tz`.
/// For preference-driven display (hub, details, PDF), use [`super::preferences::InvoiceDateFormats`].
pub fn format_invoice_date(dt: DateTime<Utc>, tz: &str) -> String {
    lariv_core::datetime::format_date_in_tz(dt, tz)
}

/// Parse an invoice date/datetime string into UTC.
///
/// Prefers a date-only `DD/MM/YYYY` (also ISO `YYYY-MM-DD`) as start-of-day in
/// `tz`, then a datetime text value, then a few legacy formats.
pub fn parse_invoice_datetime(s: &str, tz: &str) -> DateTime<Utc> {
    let s = s.trim();
    if s.is_empty() {
        return Utc::now();
    }
    if let Some(dt) = lariv_core::datetime::parse_date_start_in_tz(s, tz) {
        return dt;
    }
    lariv_core::datetime::DatetimeLocalInput::from_raw(s)
        .to_stored(tz)
        .unwrap_or_else(Utc::now)
}

/// Format an optional delivery date for form inputs (`DD/MM/YYYY`).
/// For preference-driven display, use [`super::preferences::InvoiceDateFormats`].
pub fn format_delivery_date(d: Option<NaiveDate>) -> String {
    d.map(lariv_core::datetime::format_date).unwrap_or_default()
}

/// Parse an optional delivery date from form input.
pub fn parse_delivery_date(s: &str) -> Result<Option<NaiveDate>, String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    lariv_core::datetime::parse_date(s)
        .map(Some)
        .ok_or_else(|| "invalid delivery date".to_string())
}

/// Hub lifecycle label for invoice PDFs and UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvoiceState {
    Draft,
    Quotation,
    Posted,
    PartiallyPaid,
    Paid,
    Cancelled,
}

impl InvoiceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Quotation => "quotation",
            Self::Posted => "posted",
            Self::PartiallyPaid => "partially paid",
            Self::Paid => "paid",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Draft rows are deleted on post, so any remaining draft is editable.
pub async fn draft_invoice_state(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<InvoiceState, String> {
    let exists = DraftInvoiceEntity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if exists {
        Ok(InvoiceState::Draft)
    } else {
        Err("draft invoice not found".to_string())
    }
}

pub async fn err_if_draft_sealed(db: &DatabaseConnection, draft_id: i64) -> Result<(), String> {
    if draft_id == 0 {
        return Ok(());
    }
    draft_invoice_state(db, draft_id).await?;
    Ok(())
}

fn default_line_quantity() -> String {
    "1".to_string()
}

fn default_qty_kind() -> String {
    VariableType::Quantity.as_str().to_string()
}

/// Parse a line quantity the same way a formula variable is parsed.
///
/// `kind` is the measure kind and `unit` is the unit for that kind. An empty
/// kind is a count (`quantity` / `Number`). The returned magnitude is what
/// multiplies the unit price.
pub(crate) fn parse_qty_measure(
    raw_value: &str,
    kind: &str,
    unit: &str,
) -> Result<(Decimal, String, String), String> {
    let kind = kind.trim();
    let ty = if kind.is_empty() {
        VariableType::Quantity
    } else {
        VariableType::parse_name(kind).ok_or_else(|| format!("unknown quantity kind `{kind}`"))?
    };
    let mut schema = VariableSchema::new();
    schema.insert("quantity".to_string(), VariableDef::new(ty));
    let raw = serde_json::json!({
        "quantity": {
            "kind": ty.as_str(),
            "value": raw_value.trim(),
            "unit": unit.trim(),
        }
    });
    let values = parse_values(&schema, &raw).map_err(|e| e.to_string())?;
    let measure = values
        .get("quantity")
        .ok_or_else(|| "quantity is required".to_string())?;
    if measure.value <= Decimal::ZERO {
        return Err("quantity must be positive".to_string());
    }
    Ok((
        decimal::normalize(measure.value),
        measure.kind.as_str().to_string(),
        measure.unit.clone(),
    ))
}

/// Display a stored quantity measure (`5 No.`, `2 cm`, `18%`).
pub(crate) fn format_qty_measure(value: Decimal, kind: &str, unit: &str) -> String {
    let ty = VariableType::parse_name(kind).unwrap_or(VariableType::Quantity);
    let unit = {
        let trimmed = unit.trim();
        if trimmed.is_empty() {
            ty.default_accepted_unit().to_string()
        } else {
            trimmed.to_string()
        }
    };
    let mut schema = VariableSchema::new();
    schema.insert(
        "quantity".to_string(),
        VariableDef {
            ty,
            accepted_unit: unit.clone(),
        },
    );
    let mut values = VariableValues::new();
    values.insert(
        "quantity".to_string(),
        Measure {
            kind: ty,
            value,
            unit,
        },
    );
    format_value_lines(&schema, &values)
        .into_iter()
        .next()
        .unwrap_or_default()
        .trim_start_matches("Quantity: ")
        .to_string()
}

/// A typed unit price overrides the sales formula. An empty rate leaves pricing to the formula.
pub(crate) fn optional_non_negative_rate(raw: &Option<String>) -> Result<Option<Decimal>, String> {
    let Some(raw) = raw.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let rate = parse_decimal(raw).ok_or_else(|| "invalid rate".to_string())?;
    if rate < Decimal::ZERO {
        return Err("rate must be non-negative".to_string());
    }
    Ok(Some(rate))
}

pub(crate) fn merge_tax_ids(header: &[i64], product: &[i64], line: Option<&[i64]>) -> Vec<i64> {
    if let Some(ids) = line {
        return ids.to_vec();
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for id in header.iter().chain(product.iter()) {
        if *id != 0 && seen.insert(*id) {
            out.push(*id);
        }
    }
    out
}

pub(crate) struct PricedLine {
    pub product_id: i64,
    pub product_name: String,
    pub product_remarks: Option<String>,
    pub hsn_code: Option<i64>,
    pub sac_code: Option<i64>,
    pub variable_types: String,
    pub sales_price_formula: String,
    pub rate: Decimal,
    pub quantity: Decimal,
    pub qty_kind: String,
    pub qty_unit: String,
    pub variable_values: String,
    pub pre_tax_amount: Decimal,
    pub remarks: Option<String>,
    pub tax_ids: Vec<i64>,
}

pub(crate) async fn price_pending_line(
    db: &DatabaseConnection,
    txn: &impl ConnectionTrait,
    row: &DraftLinePending,
    header_tax_ids: &[i64],
) -> Result<PricedLine, String> {
    if row.product_id == 0 {
        return Err("choose a product for each line".to_string());
    }
    let prod = ProductEntity::find_by_id(row.product_id)
        .one(txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown product #{}", row.product_id))?;
    let (quantity, qty_kind, qty_unit) =
        parse_qty_measure(&row.quantity, &row.qty_kind, &row.qty_unit)?;
    let explicit_rate = optional_non_negative_rate(&row.rate)?;
    let priced = if pricing::has_variable_pricing(&prod.variables) {
        let raw = row
            .variables
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        let unit = pricing::price_line(&prod.variables, &prod.sales_price_formula, &raw)?;
        let mut priced = pricing::with_quantity(unit, quantity)?;
        if let Some(rate) = explicit_rate {
            priced.rate = decimal::normalize(rate);
            priced.pre_tax = decimal::dec_mul(priced.quantity, priced.rate);
        }
        priced
    } else {
        let rate = match explicit_rate {
            Some(rate) => rate,
            None => pricing::unit_amount(&prod.sales_price_formula)?,
        };
        pricing::from_quantity_rate(quantity, rate)?
    };
    let product_tax_ids = load_product_tax_ids(db, prod.id).await;
    let tax_ids = merge_tax_ids(header_tax_ids, &product_tax_ids, row.tax_ids.as_deref());
    if !tax_ids.is_empty() {
        let loaded = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        if loaded.len() != tax_ids.len() {
            return Err("one or more line tax ids are invalid".to_string());
        }
    }
    let remarks = resolved_line_remarks(&row.remarks, prod.remarks.as_deref());
    let product_name = resolved_line_product_name(&row.product_name, &prod.name)?;
    let (hsn_code, sac_code) = match prod.product_kind {
        ProductKind::Goods => (prod.hsn_code, None),
        ProductKind::Services => (None, prod.hsn_code),
    };
    Ok(PricedLine {
        product_id: row.product_id,
        product_name,
        product_remarks: prod.remarks.clone(),
        hsn_code,
        sac_code,
        variable_types: prod.variables.clone(),
        sales_price_formula: prod.sales_price_formula.clone(),
        rate: priced.rate,
        quantity: priced.quantity,
        qty_kind,
        qty_unit,
        variable_values: priced.variable_values,
        pre_tax_amount: priced.pre_tax,
        remarks,
        tax_ids,
    })
}

async fn build_line<C: ConnectionTrait>(
    db: &DatabaseConnection,
    txn: &C,
    draft_id: i64,
    row: &DraftLinePending,
    header_tax_ids: &[i64],
) -> Result<(draft_invoice_line::Model, Vec<i64>), String> {
    let priced = price_pending_line(db, txn, row, header_tax_ids).await?;
    let now = Utc::now();
    let line = draft_invoice_line::ActiveModel {
        draft_invoice_id: Set(draft_id),
        product_id: Set(priced.product_id),
        product_name: Set(priced.product_name),
        product_remarks: Set(priced.product_remarks),
        hsn_code: Set(priced.hsn_code),
        sac_code: Set(priced.sac_code),
        variable_types: Set(priced.variable_types),
        sales_price_formula: Set(priced.sales_price_formula),
        rate: Set(priced.rate),
        quantity: Set(priced.quantity),
        qty_kind: Set(priced.qty_kind),
        qty_unit: Set(priced.qty_unit),
        variable_values: Set(priced.variable_values),
        pre_tax_amount: Set(priced.pre_tax_amount),
        remarks: Set(priced.remarks),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(txn)
    .await
    .map_err(|e| e.to_string())?;
    Ok((line, priced.tax_ids))
}

/// A typed line name wins. An empty name uses the catalog product name.
pub(crate) fn resolved_line_product_name(
    explicit: &Option<String>,
    catalog_name: &str,
) -> Result<String, String> {
    if let Some(name) = explicit.as_deref().and_then(optional_trimmed_text) {
        return Ok(name);
    }
    optional_trimmed_text(catalog_name).ok_or_else(|| "product name is required".to_string())
}

/// Line remarks from the form win. When the caller omits remarks, use the product's.
fn resolved_line_remarks(
    explicit: &Option<String>,
    product_remarks: Option<&str>,
) -> Option<String> {
    match explicit {
        Some(s) => optional_trimmed_text(s),
        None => product_remarks.and_then(optional_trimmed_text),
    }
}

pub fn optional_trimmed_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn optional_display(opt: &Option<String>) -> String {
    opt.as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("—")
        .to_string()
}

pub struct CreateDraftInput {
    pub number: Option<String>,
    pub reference: Option<String>,
    pub eway_bill_number: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub remarks: Option<String>,
    pub datetime: DateTime<Utc>,
    pub delivery_date: Option<NaiveDate>,
    pub customer_id: i64,
    pub payment_term_lines: Vec<DraftPaymentTermLineInput>,
    pub header_tax_ids: Vec<i64>,
    pub lines: Vec<DraftLinePending>,
}

impl CreateDraftInput {
    pub fn into_update(self) -> UpdateDraftInput {
        UpdateDraftInput {
            number: self.number,
            reference: self.reference,
            eway_bill_number: self.eway_bill_number,
            payment_reference: self.payment_reference,
            bank_account: self.bank_account,
            remarks: self.remarks,
            datetime: self.datetime,
            delivery_date: self.delivery_date,
            customer_id: self.customer_id,
            payment_term_lines: self.payment_term_lines,
            header_tax_ids: self.header_tax_ids,
            lines: self.lines,
        }
    }
}

pub async fn create_draft_invoice(
    db: &DatabaseConnection,
    input: CreateDraftInput,
    _tz: &str,
) -> Result<draft_invoice::Model, String> {
    let lines = occupied_lines(&input.lines)?;
    if lines.is_empty() {
        return Err("add at least one line".to_string());
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let number = input
        .number
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let draft = draft_invoice::ActiveModel {
        number: Set(number),
        reference: Set(input.reference),
        eway_bill_number: Set(input.eway_bill_number),
        payment_reference: Set(input.payment_reference),
        bank_account: Set(input.bank_account),
        remarks: Set(input.remarks),
        datetime: Set(input.datetime),
        delivery_date: Set(input.delivery_date),
        customer_id: Set(
            crate::logic::bill_to::require_customer_id(&txn, input.customer_id).await?,
        ),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    upsert_draft_payment_term(&txn, draft.id, &input.payment_term_lines).await?;

    set_draft_invoice_taxes(&txn, draft.id, &input.header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for row in &lines {
        let (line, tax_ids) = build_line(db, &txn, draft.id, row, &input.header_tax_ids).await?;
        set_draft_line_taxes(&txn, line.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

pub struct UpdateDraftInput {
    pub number: Option<String>,
    pub reference: Option<String>,
    pub eway_bill_number: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub remarks: Option<String>,
    pub datetime: DateTime<Utc>,
    pub delivery_date: Option<NaiveDate>,
    pub customer_id: i64,
    pub payment_term_lines: Vec<DraftPaymentTermLineInput>,
    pub header_tax_ids: Vec<i64>,
    pub lines: Vec<DraftLinePending>,
}

pub async fn update_draft_invoice(
    db: &DatabaseConnection,
    draft_id: i64,
    input: UpdateDraftInput,
    _tz: &str,
) -> Result<draft_invoice::Model, String> {
    err_if_draft_sealed(db, draft_id).await?;
    let lines = occupied_lines(&input.lines)?;
    if lines.is_empty() {
        return Err("add at least one line".to_string());
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let number = input
        .number
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    let mut am: draft_invoice::ActiveModel = draft_invoice::Entity::find_by_id(draft_id)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "draft invoice not found".to_string())?
        .into();
    am.number = Set(number);
    am.reference = Set(input.reference);
    am.eway_bill_number = Set(input.eway_bill_number);
    am.payment_reference = Set(input.payment_reference);
    am.bank_account = Set(input.bank_account);
    am.remarks = Set(input.remarks);
    am.datetime = Set(input.datetime);
    am.delivery_date = Set(input.delivery_date);
    am.customer_id =
        Set(crate::logic::bill_to::require_customer_id(&txn, input.customer_id).await?);
    am.updated_at = Set(Some(now));
    let draft = am.update(&txn).await.map_err(|e| e.to_string())?;

    upsert_draft_payment_term(&txn, draft.id, &input.payment_term_lines).await?;

    set_draft_invoice_taxes(&txn, draft.id, &input.header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    draft_invoice_line::Entity::delete_many()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;

    for row in &lines {
        let (line, tax_ids) = build_line(db, &txn, draft.id, row, &input.header_tax_ids).await?;
        set_draft_line_taxes(&txn, line.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

/// Partial draft update: only `Some` fields are written; `None` leaves the existing value.
#[derive(Clone)]
pub struct PatchDraftInput {
    pub number: Option<String>,
    pub reference: Option<String>,
    pub eway_bill_number: Option<String>,
    pub payment_reference: Option<String>,
    pub bank_account: Option<String>,
    pub remarks: Option<String>,
    pub datetime: Option<DateTime<Utc>>,
    pub delivery_date: Option<NaiveDate>,
    pub customer_id: Option<i64>,
    pub payment_term_lines: Option<Vec<DraftPaymentTermLineInput>>,
    pub header_tax_ids: Option<Vec<i64>>,
    pub lines: Option<Vec<DraftLinePending>>,
}

impl PatchDraftInput {
    pub fn is_empty(&self) -> bool {
        self.number.is_none()
            && self.reference.is_none()
            && self.eway_bill_number.is_none()
            && self.payment_reference.is_none()
            && self.bank_account.is_none()
            && self.remarks.is_none()
            && self.datetime.is_none()
            && self.delivery_date.is_none()
            && self.customer_id.is_none()
            && self.payment_term_lines.is_none()
            && self.header_tax_ids.is_none()
            && self.lines.is_none()
    }
}

pub async fn patch_draft_invoice(
    db: &DatabaseConnection,
    draft_id: i64,
    input: PatchDraftInput,
    _tz: &str,
) -> Result<draft_invoice::Model, String> {
    err_if_draft_sealed(db, draft_id).await?;
    if input.is_empty() {
        return Err("fill at least one field to update".to_string());
    }
    if let Some(lines) = &input.lines {
        if occupied_lines(lines)?.is_empty() {
            return Err("add at least one line".to_string());
        }
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();

    let mut am: draft_invoice::ActiveModel = draft_invoice::Entity::find_by_id(draft_id)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "draft invoice not found".to_string())?
        .into();

    if let Some(number) = &input.number {
        let number = number.trim();
        am.number = Set(if number.is_empty() {
            None
        } else {
            Some(number.to_string())
        });
    }
    if let Some(reference) = &input.reference {
        am.reference = Set(Some(reference.clone()));
    }
    if let Some(eway_bill_number) = &input.eway_bill_number {
        am.eway_bill_number = Set(Some(eway_bill_number.clone()));
    }
    if let Some(payment_reference) = &input.payment_reference {
        am.payment_reference = Set(Some(payment_reference.clone()));
    }
    if let Some(bank_account) = &input.bank_account {
        am.bank_account = Set(Some(bank_account.clone()));
    }
    if let Some(remarks) = &input.remarks {
        am.remarks = Set(Some(remarks.clone()));
    }
    if let Some(datetime) = input.datetime {
        am.datetime = Set(datetime);
    }
    if let Some(delivery_date) = input.delivery_date {
        am.delivery_date = Set(Some(delivery_date));
    }
    if let Some(customer_id) = input.customer_id {
        am.customer_id = Set(crate::logic::bill_to::require_customer_id(&txn, customer_id).await?);
    }
    am.updated_at = Set(Some(now));
    let draft = am.update(&txn).await.map_err(|e| e.to_string())?;

    if let Some(ref payment_term_lines) = input.payment_term_lines {
        upsert_draft_payment_term(&txn, draft.id, payment_term_lines).await?;
    }

    if let Some(ref header_tax_ids) = input.header_tax_ids {
        set_draft_invoice_taxes(&txn, draft.id, header_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(ref lines) = input.lines {
        let header_tax_ids_for_lines = if let Some(ref header_tax_ids) = input.header_tax_ids {
            header_tax_ids.clone()
        } else {
            crate::logic::tax_assoc::load_draft_invoice_tax_ids(db, draft_id)
                .await
                .unwrap_or_default()
        };

        draft_invoice_line::Entity::delete_many()
            .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
            .exec(&txn)
            .await
            .map_err(|e| e.to_string())?;

        for row in &occupied_lines(lines)? {
            let (line, tax_ids) =
                build_line(db, &txn, draft.id, row, &header_tax_ids_for_lines).await?;
            set_draft_line_taxes(&txn, line.id, &tax_ids)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

pub async fn delete_draft(db: &DatabaseConnection, draft_id: i64) -> Result<(), String> {
    err_if_draft_sealed(db, draft_id).await?;
    let term_id = draft_invoice::Entity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|d| d.draft_payment_term_id);
    draft_invoice::Entity::delete_by_id(draft_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    // Trigger `trg_draft_invoices_delete_payment_term` also deletes the term;
    // keep this in case the migration has not been applied yet.
    if let Some(term_id) = term_id {
        DraftPaymentTermEntity::delete_by_id(term_id)
            .exec(db)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn parse_header_tax_ids(s: &str) -> Vec<i64> {
    s.split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect()
}

pub fn parse_lines_json(raw: &str) -> Result<Vec<DraftLinePending>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw).map_err(|e| format!("invalid goods lines data: {e}"))
}

pub(crate) fn line_is_blank(id: i64, rate: &Option<String>, remarks: &Option<String>) -> bool {
    id <= 0
        && rate.as_deref().unwrap_or("").trim().is_empty()
        && remarks.as_deref().unwrap_or("").trim().is_empty()
}

pub(crate) fn occupied_lines(lines: &[DraftLinePending]) -> Result<Vec<DraftLinePending>, String> {
    let mut out = Vec::new();
    for row in lines {
        if line_is_blank(row.product_id, &row.rate, &row.remarks) {
            continue;
        }
        if row.product_id <= 0 {
            return Err("choose a product for each line".to_string());
        }
        out.push(row.clone());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lines_json_keeps_line_remarks() {
        let lines =
            parse_lines_json(r#"[{"product_id":1,"quantity":"2","remarks":"  dock delivery  "}]"#)
                .unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].remarks.as_deref(), Some("  dock delivery  "));
        assert_eq!(lines[0].quantity, "2");
        assert_eq!(lines[0].qty_kind, "quantity");
        assert!(lines[0].qty_unit.is_empty());

        let omitted = parse_lines_json(r#"[{"product_id":1,"quantity":"1"}]"#).unwrap();
        assert!(omitted[0].remarks.is_none());
        assert!(omitted[0].product_name.is_none());

        let named = parse_lines_json(
            r#"[{"product_id":1,"quantity":"1","product_name":"  Dock widget  "}]"#,
        )
        .unwrap();
        assert_eq!(named[0].product_name.as_deref(), Some("  Dock widget  "));
    }

    #[test]
    fn typed_product_name_wins_over_catalog_name() {
        assert_eq!(
            resolved_line_product_name(&Some("  Dock widget  ".into()), "Widget").unwrap(),
            "Dock widget"
        );
        assert_eq!(
            resolved_line_product_name(&None, "Widget").unwrap(),
            "Widget"
        );
        assert_eq!(
            resolved_line_product_name(&Some("   ".into()), "Widget").unwrap(),
            "Widget"
        );
        assert_eq!(
            resolved_line_product_name(&None, "  ").unwrap_err(),
            "product name is required"
        );
    }

    #[test]
    fn optional_rate_overrides_formula_only_when_typed() {
        assert_eq!(optional_non_negative_rate(&None).unwrap(), None);
        assert_eq!(
            optional_non_negative_rate(&Some("  ".into())).unwrap(),
            None
        );
        assert_eq!(
            optional_non_negative_rate(&Some("12.5".into())).unwrap(),
            Some(Decimal::new(125, 1))
        );
        assert_eq!(
            optional_non_negative_rate(&Some("-1".into())).unwrap_err(),
            "rate must be non-negative"
        );
        assert_eq!(
            optional_non_negative_rate(&Some("nope".into())).unwrap_err(),
            "invalid rate"
        );
    }

    #[test]
    fn omitted_line_remarks_use_product_remarks() {
        assert_eq!(
            resolved_line_remarks(&None, Some("  dock delivery  ")).as_deref(),
            Some("dock delivery")
        );
        assert_eq!(
            resolved_line_remarks(&Some("  custom  ".into()), Some("product")).as_deref(),
            Some("custom")
        );
        assert!(resolved_line_remarks(&Some("   ".into()), Some("product")).is_none());
        assert!(resolved_line_remarks(&None, Some("   ")).is_none());
    }

    #[test]
    fn qty_measure_follows_variable_kinds() {
        let (value, kind, unit) = parse_qty_measure("2", "length", "cm").unwrap();
        assert_eq!(value, Decimal::from(2));
        assert_eq!(kind, "length");
        assert_eq!(unit, "cm");
        assert_eq!(format_qty_measure(value, &kind, &unit), "2 cm");

        let (value, kind, unit) = parse_qty_measure("3", "", "").unwrap();
        assert_eq!(kind, "quantity");
        assert_eq!(unit, "Number");
        assert_eq!(format_qty_measure(value, &kind, &unit), "3 No.");

        let (value, kind, unit) = parse_qty_measure("1.5", "weight", "kg").unwrap();
        assert_eq!(value, Decimal::new(15, 1));
        assert_eq!(kind, "weight");
        assert_eq!(unit, "kg");
        assert_eq!(format_qty_measure(value, &kind, &unit), "1.5 kg");

        assert!(parse_qty_measure("1.5", "quantity", "Number").is_err());
        assert!(parse_qty_measure("2", "length", "furlong").is_err());
        assert!(parse_qty_measure("1", "area", "m").is_err());
    }

    #[test]
    fn invoice_state_labels() {
        assert_eq!(InvoiceState::Draft.as_str(), "draft");
        assert_eq!(InvoiceState::Quotation.as_str(), "quotation");
        assert_eq!(InvoiceState::Posted.as_str(), "posted");
        assert_eq!(InvoiceState::PartiallyPaid.as_str(), "partially paid");
        assert_eq!(InvoiceState::Paid.as_str(), "paid");
        assert_eq!(InvoiceState::Cancelled.as_str(), "cancelled");
    }
}
