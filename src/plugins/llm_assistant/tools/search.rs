//! Database search tools for the assistant (`pg_trgm` on Postgres).

#[cfg(any(feature = "plugin-customer", feature = "plugin-finance-invoices"))]
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::trigram;
#[cfg(any(feature = "plugin-customer", feature = "plugin-finance-invoices"))]
use crate::{
    genai::FunctionDeclaration,
    llm_tools::{LlmTool, ToolCtx},
};

#[derive(Debug, Deserialize, Default)]
struct SearchArgs {
    #[serde(default)]
    query: String,
    #[serde(default)]
    limit: u64,
}

#[allow(dead_code)]
fn parse_query(args: Value) -> Result<(String, u64), String> {
    let parsed: SearchArgs = serde_json::from_value(args).unwrap_or_default();
    let query = parsed.query.trim().to_string();
    if query.is_empty() {
        return Err("query is required".into());
    }
    Ok((query, trigram::clamp_search_limit(parsed.limit)))
}

#[allow(dead_code)]
fn search_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Fuzzy search text (trigram / substring)" },
            "limit": { "type": "integer", "description": "Max results (default 20, max 50)" }
        },
        "required": ["query"]
    })
}

#[cfg(feature = "plugin-customer")]
pub struct SearchCustomersTool;

#[cfg(feature = "plugin-customer")]
#[async_trait]
impl LlmTool for SearchCustomersTool {
    fn name(&self) -> &str {
        "search_customers"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "search_customers".into(),
            description: "Search customers by name, email, GSTIN, or city using trigram fuzzy matching. Returns id and identifying fields.".into(),
            parameters: Some(search_params()),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        use crate::plugins::customer::entities::customer::{self, Entity as CustomerEntity};

        let (query, limit) = parse_query(args)?;
        let rows = trigram::search::<CustomerEntity, _>(
            ctx.db,
            &[
                customer::Column::Name,
                customer::Column::Email,
                customer::Column::Gstin,
                customer::Column::City,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        let results: Vec<Value> = rows
            .into_iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "name": c.name,
                    "email": c.email,
                    "gstin": c.gstin,
                    "city": c.city,
                    "phone": c.phone,
                })
            })
            .collect();
        Ok(json!({ "results": results }))
    }
}

#[cfg(feature = "plugin-finance-invoices")]
pub struct SearchInvoicesTool;

#[cfg(feature = "plugin-finance-invoices")]
#[async_trait]
impl LlmTool for SearchInvoicesTool {
    fn name(&self) -> &str {
        "search_invoices"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "search_invoices".into(),
            description: "Search draft and posted invoices by number or reference using trigram fuzzy matching.".into(),
            parameters: Some(search_params()),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        use crate::plugins::finance_invoices::entities::draft_invoice::{
            self, Entity as DraftInvoiceEntity,
        };
        use crate::plugins::finance_invoices::entities::posted_invoice::{
            self, Entity as PostedInvoiceEntity,
        };

        let (query, limit) = parse_query(args)?;
        let drafts = trigram::search::<DraftInvoiceEntity, _>(
            ctx.db,
            &[
                draft_invoice::Column::Number,
                draft_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        let posted = trigram::search::<PostedInvoiceEntity, _>(
            ctx.db,
            &[
                posted_invoice::Column::Number,
                posted_invoice::Column::Reference,
            ],
            &query,
            limit,
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(json!({
            "drafts": drafts.into_iter().map(|d| json!({
                "id": d.id,
                "number": d.number,
                "reference": d.reference,
                "customer_id": d.customer_id,
                "datetime": d.datetime,
            })).collect::<Vec<_>>(),
            "posted": posted.into_iter().map(|p| json!({
                "id": p.id,
                "draft_invoice_id": p.draft_invoice_id,
                "number": p.number,
                "reference": p.reference,
                "customer_id": p.customer_id,
                "datetime": p.datetime,
            })).collect::<Vec<_>>(),
        }))
    }
}
