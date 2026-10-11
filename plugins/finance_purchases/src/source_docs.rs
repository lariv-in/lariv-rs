//! Source document type registrations for purchases.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use lariv_plugin_finance_accounts::{
    SourceDocInstance, SourceDocJournalLine, SourceDocJournalSpec, SourceDocRegistrar,
    SourceDocRegistry,     SourceDocType, logic::journal::JournalLineSpec,
};
use sea_orm::{DatabaseConnection, DatabaseTransaction, EntityTrait};

use crate::logic::purchase_posting;
use crate::{
    entities::{
        cancelled_purchase::{Entity as CancelledPurchaseEntity, PURCHASE_REVERSAL_SOURCE_DOC_TYPE},
        posted_purchase::{Entity as PostedPurchaseEntity, POSTED_PURCHASE_SOURCE_DOC_TYPE},
    },
    routes::{CancelledPurchaseDetailRouteTag, PostedPurchaseDetailRouteTag},
};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl SourceDocRegistrar for Hook {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry
            .register(Arc::new(PostedPurchaseSourceDocType))
            .register(Arc::new(PurchaseReversalSourceDocType))
    }
}

fn journal_spec_indexed(
    datetime: chrono::DateTime<chrono::Utc>,
    lines: Vec<JournalLineSpec>,
    line_item_indexes: Vec<usize>,
) -> SourceDocJournalSpec {
    SourceDocJournalSpec {
        datetime,
        lines: lines
            .into_iter()
            .map(|line| SourceDocJournalLine {
                account_id: line.account_id,
                amount: line.amount,
            })
            .collect(),
        line_item_indexes,
    }
}

fn purchase_instance_name(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

struct PostedPurchaseSourceDocType;

struct PostedPurchaseInstance {
    id: i64,
    number: String,
    datetime: chrono::DateTime<chrono::Utc>,
}

impl SourceDocInstance for PostedPurchaseInstance {
    fn source_doc_type(&self) -> &str {
        POSTED_PURCHASE_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        purchase_instance_name(self.id, &self.number)
    }

    fn detail_url(&self) -> String {
        PostedPurchaseDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }
}

#[async_trait]
impl SourceDocType for PostedPurchaseSourceDocType {
    fn source_doc_type(&self) -> &str {
        POSTED_PURCHASE_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Posted Purchase"
    }

    fn detail_url(&self, id: i64) -> String {
        PostedPurchaseDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = PostedPurchaseEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("posted purchase {id} not found"))?;
        Ok(Arc::new(PostedPurchaseInstance {
            id: model.id,
            number: model.number,
            datetime: model.datetime,
        }))
    }

    async fn journal_lines(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<SourceDocJournalSpec> {
        let (datetime, lines, line_item_indexes) =
            purchase_posting::posted_purchase_journal_lines(db, id)
                .await
                .map_err(anyhow::Error::msg)?;
        Ok(journal_spec_indexed(datetime, lines, line_item_indexes))
    }

    async fn adopt_journal_entry(
        &self,
        txn: &DatabaseTransaction,
        backing_id: i64,
        journal_id: i64,
        journal_entry_id: i64,
        item_ids: &[i64],
        line_item_indexes: &[usize],
    ) -> Result<()> {
        purchase_posting::adopt_posted_purchase_journal_entry(
            txn,
            backing_id,
            journal_id,
            journal_entry_id,
            item_ids,
            line_item_indexes,
        )
        .await
        .map_err(anyhow::Error::msg)
    }
}

struct PurchaseReversalSourceDocType;

struct PurchaseReversalInstance {
    id: i64,
    number: String,
    datetime: chrono::DateTime<chrono::Utc>,
}

impl SourceDocInstance for PurchaseReversalInstance {
    fn source_doc_type(&self) -> &str {
        PURCHASE_REVERSAL_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        purchase_instance_name(self.id, &self.number)
    }

    fn detail_url(&self) -> String {
        CancelledPurchaseDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }
}

#[async_trait]
impl SourceDocType for PurchaseReversalSourceDocType {
    fn source_doc_type(&self) -> &str {
        PURCHASE_REVERSAL_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Purchase reversal"
    }

    fn detail_url(&self, id: i64) -> String {
        CancelledPurchaseDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = CancelledPurchaseEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("cancelled purchase {id} not found"))?;
        let datetime = model.cancelled_at.unwrap_or(model.datetime);
        Ok(Arc::new(PurchaseReversalInstance {
            id: model.id,
            number: model.number,
            datetime,
        }))
    }

    async fn journal_lines(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<SourceDocJournalSpec> {
        let (datetime, lines, line_item_indexes) =
            purchase_posting::cancelled_reversal_journal_lines(db, id)
                .await
                .map_err(anyhow::Error::msg)?;
        Ok(journal_spec_indexed(datetime, lines, line_item_indexes))
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
        purchase_posting::adopt_cancelled_purchase_journal_entry(
            txn,
            backing_id,
            journal_entry_id,
            item_ids,
            line_item_indexes,
        )
        .await
        .map_err(anyhow::Error::msg)
    }
}

