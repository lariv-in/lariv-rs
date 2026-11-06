//! Quotation invoices. Same fields as a draft, stored separately, and never posted.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use lariv_formula::decimal::{self, parse_decimal};

use crate::entities::{
    quotation_invoice, quotation_invoice_line, quotation_payment_term, quotation_payment_term_line,
};
use crate::logic::draft::{
    CreateDraftInput, DraftLinePending, PatchDraftInput, UpdateDraftInput, occupied_lines,
    price_pending_line,
};
use crate::logic::draft_payment_term::{
    DraftPaymentTermLineInput, PaymentTermLineDisplayRow, format_due_date_input,
    load_posted_payment_term_by_id, validate_draft_payment_term_lines,
};
use crate::logic::preferences::format_pref_calendar_date;
use crate::logic::tax_assoc::{
    load_quotation_invoice_tax_ids, load_quotation_line_tax_ids, set_quotation_invoice_taxes,
    set_quotation_line_taxes,
};
use crate::payment_term_kind::{PaymentTermAmountKind, PaymentTermDateKind};

pub async fn create_quotation_invoice(
    db: &DatabaseConnection,
    input: CreateDraftInput,
    _tz: &str,
) -> Result<quotation_invoice::Model, String> {
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
    let row = quotation_invoice::ActiveModel {
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
    upsert_quotation_payment_term(&txn, row.id, &input.payment_term_lines).await?;
    set_quotation_invoice_taxes(&txn, row.id, &input.header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    for pending in &lines {
        insert_quotation_line(&txn, db, row.id, pending, &input.header_tax_ids).await?;
    }
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(row)
}

pub async fn update_quotation_invoice(
    db: &DatabaseConnection,
    quotation_id: i64,
    input: UpdateDraftInput,
    _tz: &str,
) -> Result<quotation_invoice::Model, String> {
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
    let mut am: quotation_invoice::ActiveModel =
        quotation_invoice::Entity::find_by_id(quotation_id)
            .one(&txn)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "quotation not found".to_string())?
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
    let row = am.update(&txn).await.map_err(|e| e.to_string())?;
    upsert_quotation_payment_term(&txn, row.id, &input.payment_term_lines).await?;
    set_quotation_invoice_taxes(&txn, row.id, &input.header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    quotation_invoice_line::Entity::delete_many()
        .filter(quotation_invoice_line::Column::QuotationInvoiceId.eq(quotation_id))
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    for pending in &lines {
        insert_quotation_line(&txn, db, row.id, pending, &input.header_tax_ids).await?;
    }
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(row)
}

pub async fn patch_quotation_invoice(
    db: &DatabaseConnection,
    quotation_id: i64,
    input: PatchDraftInput,
    _tz: &str,
) -> Result<quotation_invoice::Model, String> {
    if input.is_empty() {
        return Err("fill at least one field to update".to_string());
    }
    if let Some(lines) = &input.lines
        && occupied_lines(lines)?.is_empty()
    {
        return Err("add at least one line".to_string());
    }
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let mut am: quotation_invoice::ActiveModel =
        quotation_invoice::Entity::find_by_id(quotation_id)
            .one(&txn)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "quotation not found".to_string())?
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
    let row = am.update(&txn).await.map_err(|e| e.to_string())?;
    if let Some(ref payment_term_lines) = input.payment_term_lines {
        upsert_quotation_payment_term(&txn, row.id, payment_term_lines).await?;
    }
    if let Some(ref header_tax_ids) = input.header_tax_ids {
        set_quotation_invoice_taxes(&txn, row.id, header_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(ref lines) = input.lines {
        let header_tax_ids_for_lines = if let Some(ref header_tax_ids) = input.header_tax_ids {
            header_tax_ids.clone()
        } else {
            load_quotation_invoice_tax_ids(db, quotation_id)
                .await
                .unwrap_or_default()
        };
        quotation_invoice_line::Entity::delete_many()
            .filter(quotation_invoice_line::Column::QuotationInvoiceId.eq(quotation_id))
            .exec(&txn)
            .await
            .map_err(|e| e.to_string())?;
        for pending in &occupied_lines(lines)? {
            insert_quotation_line(&txn, db, row.id, pending, &header_tax_ids_for_lines).await?;
        }
    }
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(row)
}

pub async fn delete_quotation(db: &DatabaseConnection, quotation_id: i64) -> Result<(), String> {
    let term_id = quotation_invoice::Entity::find_by_id(quotation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|row| row.quotation_payment_term_id);
    quotation_invoice::Entity::delete_by_id(quotation_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(term_id) = term_id {
        quotation_payment_term::Entity::delete_by_id(term_id)
            .exec(db)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn insert_quotation_line<C: ConnectionTrait>(
    txn: &C,
    db: &DatabaseConnection,
    quotation_id: i64,
    pending: &DraftLinePending,
    header_tax_ids: &[i64],
) -> Result<(), String> {
    let priced = price_pending_line(db, txn, pending, header_tax_ids).await?;
    let now = Utc::now();
    let line = quotation_invoice_line::ActiveModel {
        quotation_invoice_id: Set(quotation_id),
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
    set_quotation_line_taxes(txn, line.id, &priced.tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn upsert_quotation_payment_term<C: ConnectionTrait>(
    conn: &C,
    quotation_id: i64,
    lines: &[DraftPaymentTermLineInput],
) -> Result<quotation_payment_term::Model, String> {
    validate_draft_payment_term_lines(lines)?;
    let now = Utc::now();
    let existing_id = quotation_invoice::Entity::find_by_id(quotation_id)
        .one(conn)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|row| row.quotation_payment_term_id);
    let term = if let Some(id) = existing_id {
        quotation_payment_term::Entity::find_by_id(id)
            .one(conn)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "quotation payment term not found".to_string())?
    } else {
        quotation_payment_term::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(conn)
        .await
        .map_err(|e| e.to_string())?
    };
    quotation_payment_term_line::Entity::delete_many()
        .filter(quotation_payment_term_line::Column::QuotationPaymentTermId.eq(term.id))
        .exec(conn)
        .await
        .map_err(|e| e.to_string())?;
    for (i, line) in lines.iter().enumerate() {
        quotation_line_input_to_active(term.id, i as i32, line, now)?
            .insert(conn)
            .await
            .map_err(|e| e.to_string())?;
    }
    if existing_id.is_none() {
        let mut am: quotation_invoice::ActiveModel =
            quotation_invoice::Entity::find_by_id(quotation_id)
                .one(conn)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "quotation not found".to_string())?
                .into();
        am.quotation_payment_term_id = Set(Some(term.id));
        am.update(conn).await.map_err(|e| e.to_string())?;
    }
    Ok(term)
}

fn quotation_line_input_to_active(
    quotation_payment_term_id: i64,
    line_order: i32,
    line: &DraftPaymentTermLineInput,
    now: DateTime<Utc>,
) -> Result<quotation_payment_term_line::ActiveModel, String> {
    let (due_date, due_duration) = match line.date_kind {
        PaymentTermDateKind::Absolute => (
            Some(crate::logic::draft_payment_term::parse_due_date_for_term(
                line.due_date.as_deref().unwrap_or(""),
            )?),
            None,
        ),
        PaymentTermDateKind::Relative | PaymentTermDateKind::RelativeDelivery => {
            let raw = line.due_duration.as_deref().unwrap_or("").trim();
            let dur = lariv_core::duration::parse_duration(raw)
                .map_err(|_| "invalid relative duration".to_string())?;
            (None, Some(dur))
        }
    };
    let parse_amount = |raw: &Option<String>| -> Decimal {
        parse_decimal(raw.as_deref().unwrap_or("")).unwrap_or(Decimal::ZERO)
    };
    let (amount, amount_percentage) = match line.amount_kind {
        PaymentTermAmountKind::Absolute => {
            (Some(decimal::normalize(parse_amount(&line.amount))), None)
        }
        PaymentTermAmountKind::Relative => (
            None,
            Some(decimal::normalize(parse_amount(&line.amount_percentage))),
        ),
    };
    Ok(quotation_payment_term_line::ActiveModel {
        quotation_payment_term_id: Set(quotation_payment_term_id),
        line_order: Set(line_order),
        date_kind: Set(line.date_kind),
        due_date: Set(due_date),
        due_duration: Set(due_duration),
        amount_kind: Set(line.amount_kind),
        amount: Set(amount),
        amount_percentage: Set(amount_percentage),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    })
}

pub async fn load_quotation_payment_term_lines(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> Result<Vec<quotation_payment_term_line::Model>, String> {
    let Some(term_id) = quotation_invoice::Entity::find_by_id(quotation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|row| row.quotation_payment_term_id)
    else {
        return Ok(Vec::new());
    };
    quotation_payment_term_line::Entity::find()
        .filter(quotation_payment_term_line::Column::QuotationPaymentTermId.eq(term_id))
        .order_by_asc(quotation_payment_term_line::Column::LineOrder)
        .order_by_asc(quotation_payment_term_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

pub fn quotation_payment_term_line_display(
    line: &quotation_payment_term_line::Model,
    minor_unit: i32,
    symbol: &str,
    date_fmt: &str,
) -> PaymentTermLineDisplayRow {
    let due_display = match line.date_kind {
        PaymentTermDateKind::Absolute => line
            .due_date
            .map(|d| format_pref_calendar_date(d, date_fmt))
            .unwrap_or_else(|| "—".to_string()),
        PaymentTermDateKind::Relative | PaymentTermDateKind::RelativeDelivery => line
            .due_duration
            .map(lariv_core::duration::format_duration)
            .unwrap_or_else(|| "—".to_string()),
    };
    let amount_display = match line.amount_kind {
        PaymentTermAmountKind::Absolute => line
            .amount
            .map(|amount| decimal::decimal_display_currency(amount, minor_unit, symbol))
            .unwrap_or_else(|| "—".to_string()),
        PaymentTermAmountKind::Relative => line
            .amount_percentage
            .map(|p| format!("{}%", decimal::decimal_display(p)))
            .unwrap_or_else(|| "—".to_string()),
    };
    PaymentTermLineDisplayRow {
        due_display,
        amount_display,
    }
}

pub async fn quotation_payment_term_display_rows(
    db: &DatabaseConnection,
    quotation_id: i64,
    date_fmt: &str,
) -> Vec<PaymentTermLineDisplayRow> {
    let currency =
        crate::logic::invoice_line_editor::quotation_invoice_currency_format(db, quotation_id)
            .await;
    load_quotation_payment_term_lines(db, quotation_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|line| {
            quotation_payment_term_line_display(
                line,
                currency.minor_unit,
                &currency.symbol,
                date_fmt,
            )
        })
        .collect()
}

pub async fn quotation_payment_term_lines_form_json(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> String {
    let lines = load_quotation_payment_term_lines(db, quotation_id)
        .await
        .unwrap_or_default();
    if lines.is_empty() {
        return crate::logic::draft_payment_term::default_payment_term_lines_json();
    }
    let out: Vec<serde_json::Value> = lines
        .iter()
        .map(|l| {
            serde_json::json!({
                "date_kind": l.date_kind,
                "due_date": l.due_date.map(format_due_date_input).unwrap_or_default(),
                "due_duration": l.due_duration.map(lariv_core::duration::format_duration).unwrap_or_default(),
                "amount_kind": l.amount_kind,
                "amount": l.amount.map(decimal::decimal_display).unwrap_or_default(),
                "amount_percentage": l.amount_percentage.map(decimal::decimal_display).unwrap_or_default(),
            })
        })
        .collect();
    serde_json::to_string(&out)
        .unwrap_or_else(|_| crate::logic::draft_payment_term::default_payment_term_lines_json())
}

pub fn resolve_quotation_due_date(
    line: &quotation_payment_term_line::Model,
    anchor: DateTime<Utc>,
    tz: &str,
) -> Result<NaiveDate, String> {
    match line.date_kind {
        PaymentTermDateKind::Absolute => {
            line.due_date.ok_or_else(|| "missing due date".to_string())
        }
        PaymentTermDateKind::Relative | PaymentTermDateKind::RelativeDelivery => {
            let dur = line
                .due_duration
                .ok_or_else(|| "missing relative duration".to_string())?;
            let resolved = anchor
                .checked_add_signed(chrono::Duration::nanoseconds(dur))
                .ok_or_else(|| "due date overflow".to_string())?;
            Ok(resolved
                .with_timezone(&lariv_core::datetime::parse_timezone(tz))
                .date_naive())
        }
    }
}

pub async fn copy_posted_payment_term_to_quotation<C: ConnectionTrait>(
    conn: &C,
    source_term_id: Option<i64>,
    quotation_id: i64,
) -> Result<(), String> {
    let Some(source_term_id) = source_term_id else {
        return Ok(());
    };
    let Some((_, lines)) = load_posted_payment_term_by_id(conn, source_term_id).await? else {
        return Ok(());
    };
    if lines.is_empty() {
        return Ok(());
    }
    let inputs: Vec<DraftPaymentTermLineInput> = lines
        .iter()
        .map(|l| DraftPaymentTermLineInput {
            date_kind: PaymentTermDateKind::Absolute,
            due_date: Some(format_due_date_input(l.due_date)),
            due_duration: None,
            amount_kind: PaymentTermAmountKind::Absolute,
            amount: Some(decimal::decimal_display(l.amount)),
            amount_percentage: None,
        })
        .collect();
    upsert_quotation_payment_term(conn, quotation_id, &inputs).await?;
    Ok(())
}

/// Copy a quotation into a new draft invoice. The quotation is left unchanged.
pub async fn quotation_to_draft(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> Result<crate::entities::draft_invoice::Model, String> {
    let quotation = quotation_invoice::Entity::find_by_id(quotation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "quotation not found".to_string())?;
    let header_tax_ids = load_quotation_invoice_tax_ids(db, quotation_id)
        .await
        .map_err(|e| e.to_string())?;
    let lines = quotation_invoice_line::Entity::find()
        .filter(quotation_invoice_line::Column::QuotationInvoiceId.eq(quotation_id))
        .order_by_asc(quotation_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() {
        return Err("quotation has no lines".to_string());
    }
    let term_lines = load_quotation_payment_term_lines(db, quotation_id).await?;
    let reference = quotation
        .reference
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| quotation.number.clone().filter(|s| !s.trim().is_empty()));
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let draft = crate::entities::draft_invoice::ActiveModel {
        number: Set(None),
        reference: Set(reference),
        eway_bill_number: Set(quotation.eway_bill_number.clone()),
        payment_reference: Set(quotation.payment_reference.clone()),
        bank_account: Set(quotation.bank_account.clone()),
        remarks: Set(quotation.remarks.clone()),
        datetime: Set(quotation.datetime),
        delivery_date: Set(quotation.delivery_date),
        customer_id: Set(quotation.customer_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;
    if !term_lines.is_empty() {
        let inputs: Vec<DraftPaymentTermLineInput> = term_lines
            .iter()
            .map(|l| DraftPaymentTermLineInput {
                date_kind: l.date_kind,
                due_date: l.due_date.map(format_due_date_input),
                due_duration: l.due_duration.map(lariv_core::duration::format_duration),
                amount_kind: l.amount_kind,
                amount: l.amount.map(decimal::decimal_display),
                amount_percentage: l.amount_percentage.map(decimal::decimal_display),
            })
            .collect();
        crate::logic::draft_payment_term::upsert_draft_payment_term(&txn, draft.id, &inputs)
            .await?;
    }
    crate::logic::tax_assoc::set_draft_invoice_taxes(&txn, draft.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    for line in lines {
        let tax_ids = load_quotation_line_tax_ids(&txn, line.id)
            .await
            .map_err(|e| e.to_string())?;
        let copied = crate::entities::draft_invoice_line::ActiveModel {
            draft_invoice_id: Set(draft.id),
            product_id: Set(line.product_id),
            product_name: Set(line.product_name),
            product_remarks: Set(line.product_remarks),
            hsn_code: Set(line.hsn_code),
            sac_code: Set(line.sac_code),
            variable_types: Set(line.variable_types),
            sales_price_formula: Set(line.sales_price_formula),
            rate: Set(line.rate),
            quantity: Set(line.quantity),
            qty_kind: Set(line.qty_kind),
            qty_unit: Set(line.qty_unit),
            variable_values: Set(line.variable_values),
            pre_tax_amount: Set(line.pre_tax_amount),
            remarks: Set(line.remarks),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        crate::logic::tax_assoc::set_draft_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}
