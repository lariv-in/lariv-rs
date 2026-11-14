//! Invoice form, detail, and line-supply behavior for GST filing.

use async_trait::async_trait;
use lariv_core::html_form::UrlencodedFields;
use maud::Markup;
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DatabaseTransaction, Statement,
};

use lariv_plugin_finance_invoices::draft_form_addon::DraftInvoiceFormAddon;
use lariv_plugin_finance_invoices::line_editor_addon::InvoiceLineEditorAddon;

use crate::details::{
    GstFormValues, GstTable, copy_gst, copy_line_supply, gst_from_form, line_table, load_gst,
    name_is_gst_component, supply_for_line,
};
use crate::invoice_fields;

pub fn register() {
    lariv_plugin_finance_invoices::draft_form_addon::register_draft_invoice_form_addon(
        &GstInvoiceAddon,
    );
    lariv_plugin_finance_invoices::line_editor_addon::register_invoice_line_editor_addon(
        &GstInvoiceAddon,
    );
}

struct GstInvoiceAddon;

#[async_trait]
impl DraftInvoiceFormAddon for GstInvoiceAddon {
    fn id(&self) -> &'static str {
        "finance_gst_export"
    }

    async fn render_inputs(
        &self,
        db: &DatabaseConnection,
        draft_id: Option<i64>,
        posted: Option<&UrlencodedFields>,
    ) -> Markup {
        render_for(db, GstTable::Draft, draft_id, posted, false).await
    }

    async fn render_bulk_inputs(
        &self,
        _db: &DatabaseConnection,
        posted: Option<&UrlencodedFields>,
    ) -> Markup {
        let values = posted
            .map(values_from_fields)
            .unwrap_or_else(GstFormValues::blank_bulk);
        invoice_fields::render(&values, true)
    }

    async fn render_detail(&self, db: &DatabaseConnection, draft_id: i64) -> Markup {
        detail_for(db, GstTable::Draft, draft_id).await
    }

    async fn save(
        &self,
        db: &DatabaseConnection,
        draft_id: i64,
        fields: &UrlencodedFields,
    ) -> Result<(), String> {
        save_header(db, GstTable::Draft, draft_id, fields).await?;
        sync_line_supply(
            db,
            "draft_invoice_lines",
            "draft_invoice_id",
            draft_id,
            fields,
        )
        .await
    }

    fn bulk_has_values(&self, fields: &UrlencodedFields) -> bool {
        values_from_fields(fields).any_filled() || line_json_has_supply(fields)
    }

    async fn on_posted(
        &self,
        db: &DatabaseTransaction,
        draft_id: i64,
        posted_id: i64,
    ) -> Result<(), String> {
        copy_pair(db, GstTable::Draft, draft_id, GstTable::Posted, posted_id).await
    }

    async fn on_new_draft_from_posted(
        &self,
        db: &DatabaseTransaction,
        posted_id: i64,
        new_draft_id: i64,
    ) -> Result<(), String> {
        copy_pair(
            db,
            GstTable::Posted,
            posted_id,
            GstTable::Draft,
            new_draft_id,
        )
        .await
    }

    async fn render_quotation_inputs(
        &self,
        db: &DatabaseConnection,
        quotation_id: Option<i64>,
        posted: Option<&UrlencodedFields>,
    ) -> Markup {
        render_for(db, GstTable::Quotation, quotation_id, posted, false).await
    }

    async fn render_quotation_bulk_inputs(
        &self,
        _db: &DatabaseConnection,
        posted: Option<&UrlencodedFields>,
    ) -> Markup {
        let values = posted
            .map(values_from_fields)
            .unwrap_or_else(GstFormValues::blank_bulk);
        invoice_fields::render(&values, true)
    }

    async fn render_quotation_detail(&self, db: &DatabaseConnection, quotation_id: i64) -> Markup {
        detail_for(db, GstTable::Quotation, quotation_id).await
    }

    async fn save_quotation(
        &self,
        db: &DatabaseConnection,
        quotation_id: i64,
        fields: &UrlencodedFields,
    ) -> Result<(), String> {
        save_header(db, GstTable::Quotation, quotation_id, fields).await?;
        sync_line_supply(
            db,
            "quotation_invoice_lines",
            "quotation_invoice_id",
            quotation_id,
            fields,
        )
        .await
    }

    async fn render_posted_detail(&self, db: &DatabaseConnection, posted_id: i64) -> Markup {
        detail_for(db, GstTable::Posted, posted_id).await
    }

    async fn render_cancelled_detail(&self, db: &DatabaseConnection, cancelled_id: i64) -> Markup {
        detail_for(db, GstTable::Cancelled, cancelled_id).await
    }

    async fn on_cancelled(
        &self,
        db: &DatabaseTransaction,
        posted_id: i64,
        cancelled_id: i64,
    ) -> Result<(), String> {
        copy_pair(
            db,
            GstTable::Posted,
            posted_id,
            GstTable::Cancelled,
            cancelled_id,
        )
        .await
    }

    async fn on_quotation_from_posted(
        &self,
        db: &DatabaseTransaction,
        posted_id: i64,
        quotation_id: i64,
    ) -> Result<(), String> {
        copy_pair(
            db,
            GstTable::Posted,
            posted_id,
            GstTable::Quotation,
            quotation_id,
        )
        .await
    }

    async fn on_quotation_from_cancelled(
        &self,
        db: &DatabaseTransaction,
        cancelled_id: i64,
        quotation_id: i64,
    ) -> Result<(), String> {
        copy_pair(
            db,
            GstTable::Cancelled,
            cancelled_id,
            GstTable::Quotation,
            quotation_id,
        )
        .await
    }

    async fn on_draft_from_quotation(
        &self,
        db: &DatabaseTransaction,
        quotation_id: i64,
        draft_id: i64,
    ) -> Result<(), String> {
        copy_pair(
            db,
            GstTable::Quotation,
            quotation_id,
            GstTable::Draft,
            draft_id,
        )
        .await
    }
}

#[async_trait]
impl InvoiceLineEditorAddon for GstInvoiceAddon {
    fn id(&self) -> &'static str {
        "finance_gst_export"
    }

    fn header_html(&self) -> &'static str {
        r#"<th class="whitespace-nowrap min-w-[8rem]">GST supply</th>"#
    }

    fn cell_html(&self) -> &'static str {
        r#"<td class="align-middle min-w-[8rem]"><select class="select select-bordered select-sm w-full" x-model="line.gst_supply" x-show="!(line.line_taxes||[]).some(t => ['IGST','CGST','SGST','UTGST','CESS'].some(p => String(t.Value||'').trim().toUpperCase().startsWith(p)))" x-cloak><option value="taxable">Taxable</option><option value="nil">Nil rated</option><option value="exempt">Exempt</option><option value="nongst">Non-GST</option></select><span class="text-sm" x-show="(line.line_taxes||[]).some(t => ['IGST','CGST','SGST','UTGST','CESS'].some(p => String(t.Value||'').trim().toUpperCase().startsWith(p)))" x-cloak>Taxable</span></td>"#
    }

    fn blank_js(&self) -> &'static str {
        ", gst_supply: 'taxable'"
    }

    fn hydrate_js(&self) -> &'static str {
        "if (!line.gst_supply) line.gst_supply = 'taxable';"
    }

    fn submit_js(&self) -> &'static str {
        "gst_supply: (l.line_taxes||[]).some(t => ['IGST','CGST','SGST','UTGST','CESS'].some(p => String(t.Value||'').trim().toUpperCase().startsWith(p))) ? 'taxable' : (l.gst_supply || 'taxable'),"
    }

    async fn fields_for_line(
        &self,
        db: &DatabaseConnection,
        line_table: &str,
        line_id: i64,
    ) -> Vec<(String, String)> {
        let Ok(rows) = db
            .query_all_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                format!("SELECT gst_supply FROM {line_table} WHERE id = $1"),
                [line_id.into()],
            ))
            .await
        else {
            return Vec::new();
        };
        let supply = rows
            .first()
            .and_then(|row| row.try_get::<String>("", "gst_supply").ok())
            .unwrap_or_else(|| "taxable".into());
        vec![("gst_supply".into(), supply)]
    }
}

impl GstInvoiceAddon {
    // Methods for the line editor live on the trait. Alpine needs `lineHasGstTax`.
}

async fn render_for(
    db: &DatabaseConnection,
    table: GstTable,
    id: Option<i64>,
    posted: Option<&UrlencodedFields>,
    bulk: bool,
) -> Markup {
    let values = if let Some(posted) = posted {
        values_from_fields(posted)
    } else if let Some(id) = id {
        load_gst(db, table, id)
            .await
            .map(|details| GstFormValues::from_details(&details))
            .unwrap_or_else(|_| GstFormValues::blank())
    } else {
        GstFormValues::blank()
    };
    invoice_fields::render(&values, bulk)
}

async fn detail_for(db: &DatabaseConnection, table: GstTable, id: i64) -> Markup {
    let summary = load_gst(db, table, id)
        .await
        .map(|details| details.summary())
        .unwrap_or_default();
    invoice_fields::detail(&summary)
}

async fn save_header<C: ConnectionTrait>(
    db: &C,
    table: GstTable,
    id: i64,
    fields: &UrlencodedFields,
) -> Result<(), String> {
    let details = gst_from_form(&values_from_fields(fields))?;
    crate::details::upsert_gst(db, table, id, &details).await
}

fn values_from_fields(fields: &UrlencodedFields) -> GstFormValues {
    GstFormValues {
        gst_section: field(fields, "gst_section"),
        gst_inv_typ: field(fields, "gst_inv_typ"),
        reverse_charge: field(fields, "gst_reverse_charge"),
        etin: field(fields, "gst_etin"),
        supplier_gstin: field(fields, "gst_supplier_gstin"),
        exp_typ: field(fields, "gst_exp_typ"),
        shipping_bill_number: field(fields, "gst_shipping_bill_number"),
        shipping_bill_date: field(fields, "gst_shipping_bill_date"),
        port_code: field(fields, "gst_port_code"),
        amendment_of_number: field(fields, "gst_amendment_of_number"),
        amendment_of_date: field(fields, "gst_amendment_of_date"),
    }
}

fn field(fields: &UrlencodedFields, name: &str) -> String {
    fields.get_first(name).unwrap_or("").to_string()
}

fn line_json_has_supply(fields: &UrlencodedFields) -> bool {
    fields
        .get_first("invoice_lines_json")
        .is_some_and(|json| json.contains("gst_supply"))
}

async fn sync_line_supply<C: ConnectionTrait>(
    db: &C,
    table: &str,
    parent_col: &str,
    parent: i64,
    fields: &UrlencodedFields,
) -> Result<(), String> {
    let Some(json) = fields.get_first("invoice_lines_json") else {
        return Ok(());
    };
    let rows: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    let mut supplies = Vec::new();
    for row in rows {
        let product_id = row
            .get("product_id")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        if product_id <= 0 {
            continue;
        }
        let raw = row
            .get("gst_supply")
            .and_then(|value| value.as_str())
            .unwrap_or("taxable");
        let tax_ids = row
            .get("tax_ids")
            .and_then(|value| value.as_array())
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_i64())
                    .filter(|id| *id > 0)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let has_gst = line_has_gst_tax(db, &tax_ids).await?;
        supplies.push(supply_for_line(raw, has_gst));
    }
    if supplies.is_empty() {
        return Ok(());
    }
    let ids = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT id FROM {table} WHERE {parent_col} = $1 ORDER BY id"),
            [parent.into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    if ids.len() != supplies.len() {
        return Ok(());
    }
    for (row, supply) in ids.iter().zip(supplies) {
        let id: i64 = row.try_get("", "id").map_err(|error| error.to_string())?;
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("UPDATE {table} SET gst_supply = $1 WHERE id = $2"),
            [supply.into(), id.into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn line_has_gst_tax<C: ConnectionTrait>(db: &C, tax_ids: &[i64]) -> Result<bool, String> {
    if tax_ids.is_empty() {
        return Ok(false);
    }
    let list = tax_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT name, tax_type::text AS tax_type FROM taxes WHERE id IN ({list})"),
        ))
        .await
        .map_err(|error| error.to_string())?;
    Ok(rows.iter().any(|row| {
        let levied = row.try_get::<String>("", "tax_type").unwrap_or_default() == "levied";
        let name = row.try_get::<String>("", "name").unwrap_or_default();
        levied && name_is_gst_component(&name)
    }))
}

async fn copy_pair<C: ConnectionTrait>(
    db: &C,
    from: GstTable,
    from_id: i64,
    to: GstTable,
    to_id: i64,
) -> Result<(), String> {
    copy_gst(db, from, from_id, to, to_id).await?;
    let (from_lines, from_col) = line_table(from);
    let (to_lines, to_col) = line_table(to);
    copy_line_supply(db, from_lines, from_col, from_id, to_lines, to_col, to_id).await
}

impl GstFormValues {
    fn blank_bulk() -> Self {
        Self {
            gst_section: String::new(),
            gst_inv_typ: String::new(),
            reverse_charge: String::new(),
            etin: String::new(),
            supplier_gstin: String::new(),
            exp_typ: String::new(),
            shipping_bill_number: String::new(),
            shipping_bill_date: String::new(),
            port_code: String::new(),
            amendment_of_number: String::new(),
            amendment_of_date: String::new(),
        }
    }
}
