//! Source document type registration for credit notes.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use crate::plugins::finance_accounts::{
    entities::{
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
    },
    scope::load_journal_entry_currency_format,
    SourceDocInstance, SourceDocRegistrar, SourceDocRegistry, SourceDocType,
};

use crate::plugins::finance_creditnotes::{
    entities::credit_note::{Entity as CreditNoteEntity, CREDIT_NOTE_SOURCE_DOC_TYPE},
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
        }))
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
