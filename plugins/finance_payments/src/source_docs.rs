//! Source document type registration for payments.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use lariv_plugin_finance_accounts::{
    SourceDocInstance, SourceDocJournalLine, SourceDocJournalSpec, SourceDocRegistrar,
    SourceDocRegistry, SourceDocType, logic::journal::JournalLineSpec,
    scope::load_journal_entry_currency_format,
};
use sea_orm::{DatabaseConnection, DatabaseTransaction, EntityTrait};

use crate::{
    entities::payment::{Entity as PaymentEntity, PAYMENT_SOURCE_DOC_TYPE},
    logic::payment::payment_journal_lines,
    routes::PaymentDetailRouteTag,
};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl SourceDocRegistrar for Hook {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry.register(Arc::new(PaymentSourceDocType))
    }
}

fn journal_spec(
    datetime: chrono::DateTime<chrono::Utc>,
    lines: Vec<JournalLineSpec>,
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
        line_item_indexes: Vec::new(),
    }
}

struct PaymentSourceDocType;

struct PaymentInstance {
    id: i64,
    amount_display: String,
    datetime: chrono::DateTime<chrono::Utc>,
}

impl SourceDocInstance for PaymentInstance {
    fn source_doc_type(&self) -> &str {
        PAYMENT_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        format!("Payment of {}", self.amount_display)
    }

    fn detail_url(&self) -> String {
        PaymentDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }
}

#[async_trait]
impl SourceDocType for PaymentSourceDocType {
    fn source_doc_type(&self) -> &str {
        PAYMENT_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Payment"
    }

    fn detail_url(&self, id: i64) -> String {
        PaymentDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = PaymentEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("payment {id} not found"))?;
        let currency = load_journal_entry_currency_format(db, model.journal_entry_id).await;
        Ok(Arc::new(PaymentInstance {
            id: model.id,
            amount_display: currency.display(model.amount),
            datetime: model.datetime,
        }))
    }

    async fn journal_lines(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<SourceDocJournalSpec> {
        let (datetime, lines) = payment_journal_lines(db, id)
            .await
            .map_err(anyhow::Error::msg)?;
        Ok(journal_spec(datetime, lines))
    }

    async fn adopt_journal_entry(
        &self,
        txn: &DatabaseTransaction,
        backing_id: i64,
        _journal_id: i64,
        journal_entry_id: i64,
        _item_ids: &[i64],
        _line_item_indexes: &[usize],
    ) -> Result<()> {
        crate::logic::payment::adopt_payment_journal_entry(txn, backing_id, journal_entry_id)
            .await
            .map_err(anyhow::Error::msg)
    }
}
