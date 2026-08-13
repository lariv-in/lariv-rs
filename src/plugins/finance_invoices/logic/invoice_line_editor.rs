//! Invoice line editor preview JSON and form defaults.

use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use serde::Serialize;

use crate::plugins::customer::entities::customer::Entity as CustomerEntity;
use crate::plugins::finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_format,
};
use crate::plugins::finance_common::decimal;
use crate::plugins::finance_products::{
    entities::product::Entity as ProductEntity, preferences::load_product_tax_ids,
};
use crate::plugins::finance_taxes::{
    entities::tax::TaxKind,
    scope::{load_all_taxes, load_taxes_by_ids, tax_label},
};

use crate::plugins::finance_invoices::entities::{
    cancelled_invoice::Entity as CancelledInvoiceEntity,
    draft_invoice_line::{self, Entity as DraftInvoiceLineEntity},
    posted_invoice::Entity as PostedInvoiceEntity,
    posted_invoice_line::{self, Entity as PostedInvoiceLineEntity},
};
use crate::plugins::finance_invoices::logic::preferences::load_invoice_preferences;
use crate::plugins::finance_invoices::logic::tax_assoc::{
    load_cancelled_line_tax_ids, load_draft_line_tax_ids, load_posted_line_tax_ids,
};

pub fn default_lines_json() -> String {
    r#"[{"product_id":0,"quantity":"1","rate":"","product_label":"","fk_slot":"line-slot-0","tax_ids":[]}]"#
        .to_string()
}

#[derive(Serialize)]
struct InvoiceLineProductOpt {
    id: i64,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sales_price: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tax_ids: Vec<i64>,
}

#[derive(Serialize)]
struct InvoiceLineTaxMeta {
    id: i64,
    name: String,
    tax_kind: String,
}

#[derive(Serialize)]
struct InvoiceLineEditorPreview {
    products: Vec<InvoiceLineProductOpt>,
    tax_pct_by_id: std::collections::HashMap<String, String>,
    tax_kind_by_id: std::collections::HashMap<String, String>,
    all_taxes: Vec<InvoiceLineTaxMeta>,
}

pub async fn invoice_line_editor_preview_json(db: &DatabaseConnection) -> String {
    let products = ProductEntity::find().all(db).await.unwrap_or_default();

    let mut product_opts = Vec::with_capacity(products.len());
    for p in products {
        let tax_ids = load_product_tax_ids(db, p.id).await;
        let sales_price = if decimal::dec_is_zero(p.sales_price) {
            None
        } else {
            Some(decimal::decimal_display(p.sales_price))
        };
        product_opts.push(InvoiceLineProductOpt {
            id: p.id,
            name: p.name,
            sales_price,
            tax_ids,
        });
    }

    let taxes = load_all_taxes(db).await.unwrap_or_default();
    let mut tax_pct_by_id = std::collections::HashMap::new();
    let mut tax_kind_by_id = std::collections::HashMap::new();
    let mut all_taxes = Vec::with_capacity(taxes.len());
    for t in taxes {
        let id = t.id.to_string();
        tax_pct_by_id.insert(id.clone(), decimal::decimal_display(t.percentage));
        tax_kind_by_id.insert(
            id.clone(),
            match t.tax_type {
                TaxKind::Withholding => "withholding",
                TaxKind::Levied => "levied",
            }
            .to_string(),
        );
        all_taxes.push(InvoiceLineTaxMeta {
            id: t.id,
            name: tax_label(&t),
            tax_kind: tax_kind_by_id[&id].clone(),
        });
    }

    let preview = InvoiceLineEditorPreview {
        products: product_opts,
        tax_pct_by_id,
        tax_kind_by_id,
        all_taxes,
    };
    serde_json::to_string(&preview).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Serialize)]
struct DraftLineFormRow {
    product_id: i64,
    quantity: String,
    rate: String,
    product_label: String,
    fk_slot: String,
    tax_ids: Vec<i64>,
}

pub async fn draft_lines_form_json(db: &DatabaseConnection, draft_id: i64) -> String {
    let lines = DraftInvoiceLineEntity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .unwrap_or_default();

    if lines.is_empty() {
        return default_lines_json();
    }

    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let product = ProductEntity::find_by_id(ln.product_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let product_label = product.map(|p| p.name).unwrap_or_default();
        let tax_ids = load_draft_line_tax_ids(db, ln.id).await.unwrap_or_default();
        rows.push(DraftLineFormRow {
            product_id: ln.product_id,
            quantity: decimal::decimal_display(ln.quantity),
            rate: decimal::decimal_display(ln.rate),
            product_label,
            fk_slot: format!("InvoiceLineProduct_{draft_id}_{}", ln.id),
            tax_ids,
        });
    }

    serde_json::to_string(&rows).unwrap_or_else(|_| default_lines_json())
}

#[derive(Clone, Debug)]
pub struct InvoiceLineDisplayRow {
    pub product: String,
    pub quantity: String,
    pub rate: String,
    pub line_taxes: String,
    pub untaxed_amount: String,
    pub levied_tax_amount: String,
    pub withholding_amount: String,
    pub line_total: String,
}

async fn currency_for_journal_or_default(
    db: &DatabaseConnection,
    journal_id: Option<i64>,
) -> CurrencyFormat {
    match journal_id.filter(|&id| id > 0) {
        Some(id) => load_journal_currency_format(db, id).await,
        None => load_default_currency_format(db).await,
    }
}

pub async fn invoice_customer_name(db: &DatabaseConnection, customer_id: i64) -> String {
    CustomerEntity::find_by_id(customer_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_else(|| format!("#{customer_id}"))
}

pub async fn invoice_header_tax_labels(db: &DatabaseConnection, tax_ids: &[i64]) -> String {
    if tax_ids.is_empty() {
        return "—".to_string();
    }
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    }
}

async fn product_display_name(db: &DatabaseConnection, product_id: i64) -> String {
    ProductEntity::find_by_id(product_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|p| p.name)
        .unwrap_or_else(|| format!("#{product_id}"))
}

async fn build_line_display_row(
    db: &DatabaseConnection,
    product_id: i64,
    quantity: Decimal,
    rate: Decimal,
    tax_ids: &[i64],
    currency: &CurrencyFormat,
) -> InvoiceLineDisplayRow {
    let product_name = product_display_name(db, product_id).await;
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let line_taxes = if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    };
    let (untaxed, levied, withholding, net) =
        crate::plugins::finance_invoices::logic::tax_calculations::invoice_line_amount_breakdown(
            quantity, rate, &taxes,
        );
    InvoiceLineDisplayRow {
        product: product_name,
        quantity: decimal::decimal_display(quantity),
        rate: currency.display(rate),
        line_taxes,
        untaxed_amount: currency.display(untaxed),
        levied_tax_amount: currency.display(levied),
        withholding_amount: decimal::decimal_display_withholding(
            withholding,
            currency.minor_unit,
            &currency.symbol,
        ),
        line_total: currency.display(net),
    }
}

pub async fn draft_invoice_line_display_rows(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Vec<InvoiceLineDisplayRow> {
    let lines = DraftInvoiceLineEntity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .unwrap_or_default();
    let prefs = load_invoice_preferences(db).await;
    let currency = currency_for_journal_or_default(db, prefs.journal_id).await;

    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let tax_ids = load_draft_line_tax_ids(db, ln.id).await.unwrap_or_default();
        rows.push(
            build_line_display_row(db, ln.product_id, ln.quantity, ln.rate, &tax_ids, &currency)
                .await,
        );
    }
    rows
}

pub async fn posted_invoice_line_display_rows(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Vec<InvoiceLineDisplayRow> {
    let lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .all(db)
        .await
        .unwrap_or_default();
    let journal_id = PostedInvoiceEntity::find_by_id(posted_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|p| p.journal_id);
    let currency = currency_for_journal_or_default(db, journal_id).await;

    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let tax_ids = load_posted_line_tax_ids(db, ln.id)
            .await
            .unwrap_or_default();
        rows.push(
            build_line_display_row(db, ln.product_id, ln.quantity, ln.rate, &tax_ids, &currency)
                .await,
        );
    }
    rows
}

struct CancelledLineRow {
    id: i64,
    product_id: i64,
    rate: Decimal,
    quantity: Decimal,
}

async fn load_cancelled_invoice_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Vec<CancelledLineRow> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, product_id, rate, quantity FROM cancelled_invoice_lines \
             WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .unwrap_or_default();

    rows.into_iter()
        .filter_map(|r| {
            Some(CancelledLineRow {
                id: r.try_get("", "id").ok()?,
                product_id: r.try_get("", "product_id").ok()?,
                rate: r.try_get("", "rate").ok()?,
                quantity: r.try_get("", "quantity").ok()?,
            })
        })
        .collect()
}

pub async fn cancelled_invoice_line_display_rows(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Vec<InvoiceLineDisplayRow> {
    let lines = load_cancelled_invoice_lines(db, cancelled_id).await;
    let journal_id = CancelledInvoiceEntity::find_by_id(cancelled_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|c| c.journal_id);
    let currency = currency_for_journal_or_default(db, journal_id).await;
    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let tax_ids = load_cancelled_line_tax_ids(db, ln.id)
            .await
            .unwrap_or_default();
        rows.push(
            build_line_display_row(db, ln.product_id, ln.quantity, ln.rate, &tax_ids, &currency)
                .await,
        );
    }
    rows
}
