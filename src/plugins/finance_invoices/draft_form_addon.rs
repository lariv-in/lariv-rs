//! Extra fields on draft invoice create/edit/detail, registered by other plugins.
//!
//! Uniquity (and other deployments) can attach many-to-many fields such as sites
//! without the core invoice plugin knowing those entities.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{StatusCode, header},
};
use maud::Markup;
use sea_orm::DatabaseConnection;

use crate::html_form::{FormError, UrlencodedFields};

use super::forms::{DraftInvoiceBulkEditForm, DraftInvoiceForm};

static ADDONS: OnceLock<Mutex<Vec<&'static dyn DraftInvoiceFormAddon>>> = OnceLock::new();

fn addon_list() -> &'static Mutex<Vec<&'static dyn DraftInvoiceFormAddon>> {
    ADDONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// One plugin's extra fields on draft invoice forms and detail.
#[async_trait]
pub trait DraftInvoiceFormAddon: Send + Sync {
    fn id(&self) -> &'static str;

    async fn render_inputs(
        &self,
        db: &DatabaseConnection,
        draft_id: Option<i64>,
        posted: Option<&UrlencodedFields>,
    ) -> Markup;

    async fn render_detail(&self, db: &DatabaseConnection, draft_id: i64) -> Markup;

    async fn save(
        &self,
        db: &DatabaseConnection,
        draft_id: i64,
        fields: &UrlencodedFields,
    ) -> Result<(), String>;

    /// Whether posted addon values should be applied during bulk edit.
    ///
    /// Default is `false` so empty bulk forms do not clear existing addon data.
    fn bulk_has_values(&self, _fields: &UrlencodedFields) -> bool {
        false
    }
}

/// Register a draft-invoice form addon (idempotent by [`DraftInvoiceFormAddon::id`]).
pub fn register_draft_invoice_form_addon(addon: &'static dyn DraftInvoiceFormAddon) {
    let mut list = addon_list().lock().unwrap_or_else(|e| e.into_inner());
    if !list.iter().any(|a| a.id() == addon.id()) {
        list.push(addon);
    }
}

fn addons() -> Vec<&'static dyn DraftInvoiceFormAddon> {
    addon_list()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Render extra create/edit inputs from all registered addons.
pub async fn render_draft_invoice_form_extras(
    db: &DatabaseConnection,
    draft_id: Option<i64>,
    posted: Option<&UrlencodedFields>,
) -> String {
    let mut out = String::new();
    for addon in addons() {
        out.push_str(
            &addon
                .render_inputs(db, draft_id, posted)
                .await
                .into_string(),
        );
    }
    out
}

/// Render extra detail markup from all registered addons.
pub async fn render_draft_invoice_detail_extras(db: &DatabaseConnection, draft_id: i64) -> String {
    let mut out = String::new();
    for addon in addons() {
        out.push_str(&addon.render_detail(db, draft_id).await.into_string());
    }
    out
}

/// Persist extra fields after a draft is created or updated.
pub async fn save_draft_invoice_form_extras(
    db: &DatabaseConnection,
    draft_id: i64,
    fields: &UrlencodedFields,
) -> Result<(), String> {
    for addon in addons() {
        addon.save(db, draft_id, fields).await?;
    }
    Ok(())
}

/// Persist addon fields during bulk edit only when [`DraftInvoiceFormAddon::bulk_has_values`].
pub async fn save_draft_invoice_form_extras_bulk(
    db: &DatabaseConnection,
    draft_id: i64,
    fields: &UrlencodedFields,
) -> Result<(), String> {
    for addon in addons() {
        if addon.bulk_has_values(fields) {
            addon.save(db, draft_id, fields).await?;
        }
    }
    Ok(())
}

/// True when any registered addon has bulk values to apply.
pub fn addons_bulk_has_values(fields: &UrlencodedFields) -> bool {
    addons().iter().any(|addon| addon.bulk_has_values(fields))
}

/// Typed draft invoice POST plus raw fields for addons.
#[derive(Debug)]
pub struct DraftInvoiceFormPost {
    pub form: DraftInvoiceForm,
    pub fields: UrlencodedFields,
}

impl<S> FromRequest<S> for DraftInvoiceFormPost
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("application/x-www-form-urlencoded") {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected `application/x-www-form-urlencoded` request body".into(),
            ));
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

        let fields = UrlencodedFields::parse(&bytes).map_err(form_rejection)?;
        let form = fields.deserialize().map_err(form_rejection)?;
        Ok(Self { form, fields })
    }
}

/// Bulk-edit draft invoice POST plus raw fields for addons.
#[derive(Debug)]
pub struct DraftInvoiceBulkEditFormPost {
    pub form: DraftInvoiceBulkEditForm,
    pub fields: UrlencodedFields,
    pub ids: String,
}

impl<S> FromRequest<S> for DraftInvoiceBulkEditFormPost
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("application/x-www-form-urlencoded") {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected `application/x-www-form-urlencoded` request body".into(),
            ));
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

        let fields = UrlencodedFields::parse(&bytes).map_err(form_rejection)?;
        let form = fields.deserialize().map_err(form_rejection)?;
        let ids = fields
            .get_first("ids")
            .map(|s| s.to_string())
            .unwrap_or_default();
        Ok(Self { form, fields, ids })
    }
}

fn form_rejection(err: FormError) -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to deserialize form body: {err}"),
    )
}
