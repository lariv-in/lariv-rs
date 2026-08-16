//! Source document type registrations for invoices, payments, and payment batches.

use std::sync::Arc;

use crate::plugins::finance_accounts::{
    SourceDocInstance, SourceDocRegistrar, SourceDocRegistry, SourceDocType,
    scope::load_journal_entry_currency_format,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::plugins::finance_invoices::{
    entities::{
        payment::{Entity as PaymentEntity, PAYMENT_SOURCE_DOC_TYPE},
        payment_batch::{Entity as PaymentBatchEntity, PAYMENT_BATCH_SOURCE_DOC_TYPE},
        posted_invoice::{Entity as PostedInvoiceEntity, POSTED_INVOICE_SOURCE_DOC_TYPE},
    },
    routes::{PaymentBatchDetailRouteTag, PaymentDetailRouteTag, PostedInvoiceDetailRouteTag},
};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl SourceDocRegistrar for Hook {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry
            .register(Arc::new(PostedInvoiceSourceDocType))
            .register(Arc::new(PaymentSourceDocType))
            .register(Arc::new(PaymentBatchSourceDocType))
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
}

struct PaymentBatchSourceDocType;

struct PaymentBatchInstance {
    id: i64,
    datetime: chrono::DateTime<chrono::Utc>,
}

impl SourceDocInstance for PaymentBatchInstance {
    fn source_doc_type(&self) -> &str {
        PAYMENT_BATCH_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        format!("Batch #{}", self.id)
    }

    fn detail_url(&self) -> String {
        PaymentBatchDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> chrono::DateTime<chrono::Utc> {
        self.datetime
    }
}

#[async_trait]
impl SourceDocType for PaymentBatchSourceDocType {
    fn source_doc_type(&self) -> &str {
        PAYMENT_BATCH_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Payment Batch"
    }

    fn detail_url(&self, id: i64) -> String {
        PaymentBatchDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = PaymentBatchEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("payment batch {id} not found"))?;
        Ok(Arc::new(PaymentBatchInstance {
            id: model.id,
            datetime: model.datetime,
        }))
    }
}
