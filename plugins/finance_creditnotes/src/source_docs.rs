//! Source document type registration for credit notes.

use std::sync::Arc;

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
        _line_item_indexes: &[usize],
    ) -> Result<()> {
        let model = CreditNoteEntity::find_by_id(backing_id)
            .one(txn)
            .await?
            .with_context(|| format!("credit note {backing_id} not found"))?;
        let old_items = JournalEntryItemEntity::find()
            .filter(journal_entry_item::Column::JournalEntryId.eq(model.reversed_journal_entry_id))
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
