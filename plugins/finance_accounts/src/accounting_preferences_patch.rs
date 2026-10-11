//! Preference addon types and render/save helpers for the unified `/finance/preferences` page.
//!
//! Plugins register addons via [`AccountingSidebarRegistrar::register_accounting_preferences`]
//! on the shared accounting sidebar capability hook.

use std::sync::OnceLock;

use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::{StatusCode, header},
};
use maud::{Markup, html};
use sea_orm::DatabaseConnection;
use serde::de::DeserializeOwned;

use crate::forms::AccountingPreferencesForm;
use lariv_core::html_form::{FormError, UrlencodedFields, csrf::csrf_rejection, verify_form_csrf};

static ADDONS: OnceLock<Vec<&'static dyn AccountingPreferencesAddon>> = OnceLock::new();

/// Parse an optional FK/text field from a form string (empty → `None`).
pub fn str_to_opt_i64(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() { None } else { s.parse().ok() }
}

/// Parse an optional text field from a form string (empty → `None`).
pub fn str_to_opt_string(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Typed composite POST body for `/finance/preferences` (one parse, many sub-forms).
#[derive(Debug, Clone)]
pub struct AccountingPreferencesPost {
    fields: UrlencodedFields,
}

impl AccountingPreferencesPost {
    pub fn accounts(&self) -> Result<AccountingPreferencesForm, FormError> {
        self.fields.deserialize()
    }

    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, FormError> {
        self.fields.deserialize()
    }
}

impl<S> FromRequest<S> for AccountingPreferencesPost
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

        let headers = req.headers().clone();
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

        let fields = UrlencodedFields::parse(&bytes).map_err(form_rejection)?;
        verify_form_csrf(&headers, &fields).map_err(form_rejection)?;

        Ok(Self { fields })
    }
}

fn form_rejection(err: FormError) -> (StatusCode, String) {
    if let Some(rej) = csrf_rejection(&err) {
        return rej;
    }
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to parse form body: {err}"),
    )
}

/// One labeled block of fields an app contributes to `/finance/preferences`.
pub struct AccountingPreferenceGroup {
    pub title: &'static str,
    pub inputs: Markup,
}

/// Section heading plus the fields an app added under it.
pub fn preference_section(title: &str, inputs: Markup) -> Markup {
    html! {
        section class="mt-6" {
            h2 class="text-lg font-semibold border-b border-base-300 pb-1 mb-2" { (title) }
            (inputs)
        }
    }
}

/// One plugin's extra fields on `/finance/preferences` (GET render + POST save).
#[async_trait]
pub trait AccountingPreferencesAddon: Send + Sync {
    fn id(&self) -> &'static str;
    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<AccountingPreferenceGroup>;
    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &AccountingPreferencesPost,
    ) -> Result<(), String>;
}

/// Registry of preference addons folded from plugin hooks.
#[derive(Clone, Default)]
pub struct AccountingPreferencesRegistry {
    addons: Vec<&'static dyn AccountingPreferencesAddon>,
}

impl AccountingPreferencesRegistry {
    pub fn new() -> Self {
        Self { addons: Vec::new() }
    }

    pub fn register_addon(mut self, addon: &'static dyn AccountingPreferencesAddon) -> Self {
        let id = addon.id();
        if !self.addons.iter().any(|a| a.id() == id) {
            self.addons.push(addon);
        }
        self
    }

    pub fn addons(&self) -> &[&'static dyn AccountingPreferencesAddon] {
        &self.addons
    }
}

pub(crate) fn store_accounting_preferences_addons(registry: &AccountingPreferencesRegistry) {
    if ADDONS.set(registry.addons.clone()).is_err() {
        tracing::error!("accounting preferences ADDONS already initialized");
    }
}

/// Registered preference addons (empty until app mount).
pub fn accounting_preferences_addons() -> &'static [&'static dyn AccountingPreferencesAddon] {
    ADDONS.get().map(|v| v.as_slice()).unwrap_or(&[])
}

/// Render all patched preference form sections, each under the app that added it.
pub async fn render_accounting_preferences_addons(db: &DatabaseConnection) -> Markup {
    let mut out = Markup::default();
    for addon in accounting_preferences_addons() {
        for group in addon.render_groups(db).await {
            let section = preference_section(group.title, group.inputs);
            out = html! { (out) (section) };
        }
    }
    out
}

/// Persist all patched preference sections from a urlencoded form body.
pub async fn save_accounting_preferences_addons(
    db: &DatabaseConnection,
    post: &AccountingPreferencesPost,
) -> Result<(), String> {
    for addon in accounting_preferences_addons() {
        addon.save_from_form(db, post).await?;
    }
    Ok(())
}
