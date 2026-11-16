//! Copy header and line-item audit rows onto the document created by a state change.

use async_trait::async_trait;
use maud::Markup;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Order, Query};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use lariv_core::html_form::UrlencodedFields;
use lariv_plugin_finance_invoices::draft_form_addon::{
    DraftInvoiceFormAddon, register_draft_invoice_form_addon,
};
use lariv_plugin_finance_invoices::invoice_detail_menu::InvoiceDetailKind;
use lariv_plugin_logging::entities::log::{ActiveModel, Column, Entity, Model};

/// Register the addon that copies header logs across invoice state changes.
pub fn register() {
    register_draft_invoice_form_addon(&InvoiceLogAddon);
}

struct InvoiceLogAddon;

#[async_trait]
impl DraftInvoiceFormAddon for InvoiceLogAddon {
    fn id(&self) -> &'static str {
        "finance_invoices_log"
    }

    async fn render_inputs(
        &self,
        _db: &sea_orm::DatabaseConnection,
        _draft_id: Option<i64>,
        _posted: Option<&UrlencodedFields>,
    ) -> Markup {
        Markup::default()
    }

    async fn render_detail(&self, _db: &sea_orm::DatabaseConnection, _draft_id: i64) -> Markup {
        Markup::default()
    }

    async fn save(
        &self,
        _db: &sea_orm::DatabaseConnection,
        _draft_id: i64,
        _fields: &UrlencodedFields,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn on_posted(
        &self,
        db: &sea_orm::DatabaseTransaction,
        draft_id: i64,
        posted_id: i64,
    ) -> Result<(), String> {
        copy_document_logs(
            db,
            InvoiceDetailKind::Draft,
            draft_id,
            InvoiceDetailKind::Posted,
            posted_id,
        )
        .await
    }

    async fn on_cancelled(
        &self,
        db: &sea_orm::DatabaseTransaction,
        posted_id: i64,
        cancelled_id: i64,
    ) -> Result<(), String> {
        copy_document_logs(
            db,
            InvoiceDetailKind::Posted,
            posted_id,
            InvoiceDetailKind::Cancelled,
            cancelled_id,
        )
        .await
    }

    async fn on_quotation_from_posted(
        &self,
        db: &sea_orm::DatabaseTransaction,
        posted_id: i64,
        quotation_id: i64,
    ) -> Result<(), String> {
        copy_document_logs(
            db,
            InvoiceDetailKind::Posted,
            posted_id,
            InvoiceDetailKind::Quotation,
            quotation_id,
        )
        .await
    }

    async fn on_quotation_from_cancelled(
        &self,
        db: &sea_orm::DatabaseTransaction,
        cancelled_id: i64,
        quotation_id: i64,
    ) -> Result<(), String> {
        copy_document_logs(
            db,
            InvoiceDetailKind::Cancelled,
            cancelled_id,
            InvoiceDetailKind::Quotation,
            quotation_id,
        )
        .await
    }

    async fn on_draft_from_quotation(
        &self,
        db: &sea_orm::DatabaseTransaction,
        quotation_id: i64,
        draft_id: i64,
    ) -> Result<(), String> {
        copy_document_logs(
            db,
            InvoiceDetailKind::Quotation,
            quotation_id,
            InvoiceDetailKind::Draft,
            draft_id,
        )
        .await
    }

    async fn on_new_draft_from_posted(
        &self,
        db: &sea_orm::DatabaseTransaction,
        posted_id: i64,
        new_draft_id: i64,
    ) -> Result<(), String> {
        copy_document_logs(
            db,
            InvoiceDetailKind::Posted,
            posted_id,
            InvoiceDetailKind::Draft,
            new_draft_id,
        )
        .await
    }
}

/// Line ids for one document, oldest id first.
///
/// State changes insert destination lines in that same order, so pairing by
/// position maps a source line onto the line copied from it.
pub(crate) async fn document_line_ids(
    db: &impl ConnectionTrait,
    kind: InvoiceDetailKind,
    document_id: i64,
) -> Result<Vec<i64>, String> {
    let statement = Query::select()
        .column(Alias::new("id"))
        .from(Alias::new(kind.line_table_name()))
        .and_where(Expr::col(Alias::new(kind.line_parent_column())).eq(document_id))
        .order_by(Alias::new("id"), Order::Asc)
        .to_owned();
    let rows = db
        .query_all(&statement)
        .await
        .map_err(|error| error.to_string())?;
    rows.into_iter()
        .map(|row| row.try_get("", "id").map_err(|error| error.to_string()))
        .collect()
}

/// Copy the source header's logs, then each source line's logs onto the
/// destination line in the same position.
async fn copy_document_logs(
    db: &impl ConnectionTrait,
    from: InvoiceDetailKind,
    from_id: i64,
    to: InvoiceDetailKind,
    to_id: i64,
) -> Result<(), String> {
    copy_row_logs(db, from.table_name(), from_id, to.table_name(), to_id).await?;
    let source_lines = document_line_ids(db, from, from_id).await?;
    let destination_lines = document_line_ids(db, to, to_id).await?;
    for (source_line_id, destination_line_id) in source_lines.into_iter().zip(destination_lines) {
        copy_row_logs(
            db,
            from.line_table_name(),
            source_line_id,
            to.line_table_name(),
            destination_line_id,
        )
        .await?;
    }
    Ok(())
}

/// Insert a copy of each log for one row, retargeted at another row.
///
/// A row is skipped when the destination already has the same timestamp, kind,
/// user, and description, so a transition that copies the same history twice
/// does not duplicate it.
async fn copy_row_logs(
    db: &impl ConnectionTrait,
    from_table: &str,
    from_id: i64,
    to_table: &str,
    to_id: i64,
) -> Result<(), String> {
    let rows = Entity::find()
        .filter(Column::TableName.eq(from_table))
        .filter(Column::RowId.eq(from_id))
        .all(db)
        .await
        .map_err(|error| error.to_string())?;
    for row in rows {
        if same_log_exists(db, to_table, to_id, &row).await? {
            continue;
        }
        let model = ActiveModel {
            created_at: Set(row.created_at),
            table_name: Set(to_table.to_string()),
            row_id: Set(to_id),
            log_kind: Set(row.log_kind),
            user_id: Set(row.user_id),
            description: Set(row.description),
            ..Default::default()
        };
        model.insert(db).await.map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn same_log_exists(
    db: &impl ConnectionTrait,
    to_table: &str,
    to_id: i64,
    row: &Model,
) -> Result<bool, String> {
    let mut query = Entity::find()
        .filter(Column::TableName.eq(to_table))
        .filter(Column::RowId.eq(to_id))
        .filter(Column::CreatedAt.eq(row.created_at))
        .filter(Column::LogKind.eq(row.log_kind))
        .filter(Column::Description.eq(row.description.clone()));
    query = match row.user_id {
        Some(user_id) => query.filter(Column::UserId.eq(user_id)),
        None => query.filter(Column::UserId.is_null()),
    };
    query
        .one(db)
        .await
        .map(|found| found.is_some())
        .map_err(|error| error.to_string())
}
