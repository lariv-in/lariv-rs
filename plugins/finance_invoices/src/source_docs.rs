//! Source document type registration for posted invoices.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use lariv_plugin_finance_accounts::{
    SourceDocInstance, SourceDocJournalLine, SourceDocJournalSpec, SourceDocRegistrar,
    SourceDocRegistry, SourceDocType, logic::journal::JournalLineSpec,
};
use sea_orm::{DatabaseConnection, DatabaseTransaction, EntityTrait};

use crate::logic::invoice_posting;
use crate::{
    entities::posted_invoice::{Entity as PostedInvoiceEntity, POSTED_INVOICE_SOURCE_DOC_TYPE},
    routes::PostedInvoiceDetailRouteTag,
};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl SourceDocRegistrar for Hook {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry.register(Arc::new(PostedInvoiceSourceDocType))
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

fn invoice_instance_name(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

struct PostedInvoiceSourceDocType;

struct PostedInvoiceInstance {
    id: i64,
    number: String,
    datetime: chrono::DateTime<chrono::Utc>,
}

impl SourceDocInstance for PostedInvoiceInstance {
    fn source_doc_type(&self) -> &str {
        POSTED_INVOICE_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        invoice_instance_name(self.id, &self.number)
    }

    fn detail_url(&self) -> String {
        PostedInvoiceDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }
}

#[async_trait]
impl SourceDocType for PostedInvoiceSourceDocType {
    fn source_doc_type(&self) -> &str {
        POSTED_INVOICE_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Posted Invoice"
    }

    fn detail_url(&self, id: i64) -> String {
        PostedInvoiceDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = PostedInvoiceEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("posted invoice {id} not found"))?;
        Ok(Arc::new(PostedInvoiceInstance {
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
            invoice_posting::posted_invoice_journal_lines(db, id)
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
        invoice_posting::adopt_posted_invoice_journal_entry(
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
