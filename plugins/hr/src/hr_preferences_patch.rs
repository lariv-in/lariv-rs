//! Preference addon types and render/save helpers for the unified `/hr/preferences` page.
//!
//! Plugins register addons via [`HrSidebarRegistrar::register_hr_preferences`](crate::hr_sidebar::HrSidebarRegistrar::register_hr_preferences)
//! on the shared HR sidebar capability hook.

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

use lariv_core::html_form::{FormError, UrlencodedFields, csrf::csrf_rejection, verify_form_csrf};

static ADDONS: OnceLock<Vec<&'static dyn HrPreferencesAddon>> = OnceLock::new();

/// Typed composite POST body for `/hr/preferences` (one parse, many sub-forms).
#[derive(Debug, Clone)]
pub struct HrPreferencesPost {
    fields: UrlencodedFields,
}

impl HrPreferencesPost {
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, FormError> {
        self.fields.deserialize()
    }
}

impl<S> FromRequest<S> for HrPreferencesPost
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

/// One labeled block of fields an addon contributes to `/hr/preferences`.
pub struct HrPreferenceGroup {
    pub title: &'static str,
    pub inputs: Markup,
}

/// Section heading plus the fields an addon added under it.
pub fn preference_section(title: &str, inputs: Markup) -> Markup {
    html! {
        section class="mt-6" {
            h2 class="text-lg font-semibold border-b border-base-300 pb-1 mb-2" { (title) }
            (inputs)
        }
    }
}

/// One plugin's extra fields on `/hr/preferences` (GET render + POST save).
#[async_trait]
pub trait HrPreferencesAddon: Send + Sync {
    fn id(&self) -> &'static str;
    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<HrPreferenceGroup>;
    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &HrPreferencesPost,
    ) -> Result<(), String>;
}

/// Registry of preference addons folded from plugin hooks.
#[derive(Clone, Default)]
pub struct HrPreferencesRegistry {
    addons: Vec<&'static dyn HrPreferencesAddon>,
}

impl HrPreferencesRegistry {
    pub fn new() -> Self {
        Self { addons: Vec::new() }
    }

    pub fn register_addon(mut self, addon: &'static dyn HrPreferencesAddon) -> Self {
        let id = addon.id();
        if !self.addons.iter().any(|a| a.id() == id) {
            self.addons.push(addon);
        }
        self
    }

    pub fn addons(&self) -> &[&'static dyn HrPreferencesAddon] {
        &self.addons
    }
}

pub(crate) fn store_hr_preferences_addons(registry: &HrPreferencesRegistry) {
    if ADDONS.set(registry.addons.clone()).is_err() {
        tracing::error!("hr preferences ADDONS already initialized");
    }
}

/// Registered preference addons (empty until app mount).
pub fn hr_preferences_addons() -> &'static [&'static dyn HrPreferencesAddon] {
    ADDONS.get().map(|v| v.as_slice()).unwrap_or(&[])
}

/// Render all patched preference form sections, each under the addon that added it.
pub async fn render_hr_preferences_addons(db: &DatabaseConnection) -> Markup {
    let mut out = Markup::default();
    for addon in hr_preferences_addons() {
        for group in addon.render_groups(db).await {
            let section = preference_section(group.title, group.inputs);
            out = html! { (out) (section) };
        }
    }
    out
}

/// Persist all patched preference sections from a urlencoded form body.
pub async fn save_hr_preferences_addons(
    db: &DatabaseConnection,
    post: &HrPreferencesPost,
) -> Result<(), String> {
    for addon in hr_preferences_addons() {
        addon.save_from_form(db, post).await?;
    }
    Ok(())
}
