//! Source document type registration for credit notes.

use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use lariv_plugin_finance_accounts::{
    SourceDocInstance, SourceDocJournalLine, SourceDocJournalSpec, SourceDocRegistrar,
    SourceDocRegistry, SourceDocType,
    entities::journal_entry_item::{self, Entity as JournalEntryItemEntity},
    scope::load_journal_entry_currency_format,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Statement,
};

use crate::{
    entities::credit_note::{CREDIT_NOTE_SOURCE_DOC_TYPE, Entity as CreditNoteEntity},
    routes::CreditNoteDetailRouteTag,
};

/// Recomputed journal for a credit note that reverses a cancelled invoice.
///
/// Invoices register this. Credit notes that do not reverse an invoice keep the
/// stored reversal.
#[async_trait]
pub trait CreditNoteJournalLines: Send + Sync {
    async fn lines_for_credit_note(
        &self,
        db: &DatabaseConnection,
        credit_note_id: i64,
    ) -> Result<Option<SourceDocJournalSpec>>;
}

static CREDIT_NOTE_JOURNAL_LINES: OnceLock<Arc<dyn CreditNoteJournalLines>> = OnceLock::new();

/// Install the cancelled-invoice recalculation. The first registration wins.
pub fn register_credit_note_journal_lines(provider: Arc<dyn CreditNoteJournalLines>) {
    let already_registered = CREDIT_NOTE_JOURNAL_LINES.set(provider).is_err();
    if already_registered {
        return;
    }
}

fn credit_note_journal_lines() -> Option<Arc<dyn CreditNoteJournalLines>> {
    CREDIT_NOTE_JOURNAL_LINES.get().cloned()
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl SourceDocRegistrar for Hook {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry.register(Arc::new(CreditNoteSourceDocType))
    }
}

struct CreditNoteSourceDocType;

struct CreditNoteInstance {
    id: i64,
    amount_display: String,
    datetime: chrono::DateTime<chrono::Utc>,
}

impl SourceDocInstance for CreditNoteInstance {
    fn source_doc_type(&self) -> &str {
        CREDIT_NOTE_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        format!("Credit Note of {}", self.amount_display)
    }

    fn detail_url(&self) -> String {
        CreditNoteDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }
}

#[async_trait]
impl SourceDocType for CreditNoteSourceDocType {
    fn source_doc_type(&self) -> &str {
        CREDIT_NOTE_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Credit Note"
    }

    fn detail_url(&self, id: i64) -> String {
        CreditNoteDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = CreditNoteEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("credit note {id} not found"))?;
        let amount = journal_entry_transfer_amount(db, model.reversed_journal_entry_id).await;
        let currency =
            load_journal_entry_currency_format(db, model.reversed_journal_entry_id).await;
        Ok(Arc::new(CreditNoteInstance {
            id: model.id,
            amount_display: currency.display(amount),
            datetime: model.datetime,
        }))
    }

    async fn journal_lines(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<SourceDocJournalSpec> {
        let model = CreditNoteEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("credit note {id} not found"))?;
        if model.journal_entry_id <= 0 {
            bail!("credit note {id} does not reverse a journal entry");
        }
        if let Some(provider) = credit_note_journal_lines()
            && let Some(mut spec) = provider.lines_for_credit_note(db, id).await?
        {
            spec.datetime = model.datetime;
            if spec.lines.is_empty() {
                bail!("credit note {id} produced no journal lines");
            }
            return Ok(spec);
        }
        let items = JournalEntryItemEntity::find()
            .filter(journal_entry_item::Column::JournalEntryId.eq(model.journal_entry_id))
            .order_by_asc(journal_entry_item::Column::Id)
            .all(db)
            .await?;
        if items.is_empty() {
            bail!(
                "credit note {id} reverses journal entry {} which has no lines",
                model.journal_entry_id
            );
        }
        Ok(SourceDocJournalSpec {
            datetime: model.datetime,
            lines: items
                .into_iter()
                .map(|item| SourceDocJournalLine {
                    account_id: item.account_id,
                    amount: -item.amount,
                })
                .collect(),
            line_item_indexes: Vec::new(),
        })
    }

    async fn adopt_journal_entry(
        &self,
        txn: &DatabaseTransaction,
        backing_id: i64,
        _journal_id: i64,
        journal_entry_id: i64,
        item_ids: &[i64],
        line_item_indexes: &[usize],
    ) -> Result<()> {
        let model = CreditNoteEntity::find_by_id(backing_id)
            .one(txn)
            .await?
            .with_context(|| format!("credit note {backing_id} not found"))?;
        if line_item_indexes.is_empty() {
            let old_items = JournalEntryItemEntity::find()
                .filter(
                    journal_entry_item::Column::JournalEntryId.eq(model.reversed_journal_entry_id),
                )
                .order_by_asc(journal_entry_item::Column::Id)
                .all(txn)
                .await?;
            if old_items.len() != item_ids.len() {
                bail!(
                    "credit note {backing_id} reversal has {} lines but the new entry has {}",
                    old_items.len(),
                    item_ids.len()
                );
            }
            for (old, new_id) in old_items.iter().zip(item_ids.iter().copied()) {
                txn.execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE cancelled_invoice_lines SET journal_entry_item_id = $1, updated_at = NOW() \
                     WHERE journal_entry_item_id = $2",
                    [new_id.into(), old.id.into()],
                ))
                .await?;
            }
        } else {
            let cancelled_id: Option<i64> = txn
                .query_one_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT id FROM cancelled_invoices WHERE credit_note_id = $1 LIMIT 1",
                    [backing_id.into()],
                ))
                .await?
                .map(|row| row.try_get("", "id"))
                .transpose()
                .context("read cancelled invoice id")?;
            let cancelled_id = cancelled_id.ok_or_else(|| {
                anyhow::anyhow!("credit note {backing_id} has no cancelled invoice to retarget")
            })?;
            let line_rows = txn
                .query_all_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT id FROM cancelled_invoice_lines WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
                    [cancelled_id.into()],
                ))
                .await?;
            let line_ids: Vec<i64> = line_rows
                .into_iter()
                .map(|row| row.try_get("", "id"))
                .collect::<Result<Vec<_>, _>>()
                .context("read cancelled invoice line ids")?;
            if line_ids.len() != line_item_indexes.len() {
                bail!(
                    "cancelled invoice {cancelled_id} line count {} does not match {} journal item indexes",
                    line_ids.len(),
                    line_item_indexes.len()
                );
            }
            for (line_id, index) in line_ids.into_iter().zip(line_item_indexes.iter().copied()) {
                let item_id = item_ids.get(index).copied().ok_or_else(|| {
                    anyhow::anyhow!(
                        "cancelled invoice {cancelled_id} journal item index {index} is out of range"
                    )
                })?;
                txn.execute_raw(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE cancelled_invoice_lines SET journal_entry_item_id = $1, updated_at = NOW() WHERE id = $2",
                    [item_id.into(), line_id.into()],
                ))
                .await?;
            }
        }
        let mut am: crate::entities::credit_note::ActiveModel = model.into();
        am.reversed_journal_entry_id = Set(journal_entry_id);
        am.updated_at = Set(Some(chrono::Utc::now()));
        am.update(txn).await?;
        Ok(())
    }
}

/// Transfer amount for a journal entry: sum of debit (positive) lines.
async fn journal_entry_transfer_amount(db: &DatabaseConnection, entry_id: i64) -> Decimal {
    if entry_id <= 0 {
        return Decimal::ZERO;
    }
    let items = JournalEntryItemEntity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(entry_id))
        .all(db)
        .await
        .unwrap_or_default();
    items
        .into_iter()
        .filter(|i| i.amount > Decimal::ZERO)
        .map(|i| i.amount)
        .sum()
}
