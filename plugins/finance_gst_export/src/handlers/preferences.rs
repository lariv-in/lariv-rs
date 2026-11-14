use std::collections::HashMap;

use axum::response::{IntoResponse, Redirect, Response};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait};

use lariv_core::components::{ManyToManyItem, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};
use lariv_plugin_users::middleware::RequireAuth;

use crate::forms::GstPreferencesForm;
use crate::routes::GstPreferencesRouteTag;
use crate::state::GstExportState;
use crate::templates::GstPreferencesPage;

pub async fn page(
    Cap(state): Cap<GstExportState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let page = match load_page(&state.db, GstPreferencesForm::empty(), String::new()).await {
        Ok(page) => page,
        Err(error) => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error).into_response();
        }
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn save(
    Cap(state): Cap<GstExportState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<GstPreferencesForm>,
) -> Response {
    if let Err(error) = replace_components(&state.db, &form).await {
        let page = load_page(&state.db, form, error)
            .await
            .unwrap_or_else(|load_error| GstPreferencesPage {
                igst: Vec::new(),
                cgst: Vec::new(),
                sgst: Vec::new(),
                cess: Vec::new(),
                error: load_error,
            });
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    Redirect::to(&GstPreferencesRouteTag.url()).into_response()
}

async fn load_page(
    db: &DatabaseConnection,
    mut form: GstPreferencesForm,
    error: String,
) -> Result<GstPreferencesPage, String> {
    if error.is_empty() {
        form = load_form(db).await?;
    }
    Ok(GstPreferencesPage {
        igst: items_for(db, &form.igst).await?,
        cgst: items_for(db, &form.cgst).await?,
        sgst: items_for(db, &form.sgst).await?,
        cess: items_for(db, &form.cess).await?,
        error,
    })
}

async fn load_form(db: &DatabaseConnection) -> Result<GstPreferencesForm, String> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT tax_id, component FROM gst_tax_components ORDER BY tax_id".to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;
    let mut form = GstPreferencesForm::empty();
    for row in rows {
        let id: i64 = row.try_get("", "tax_id").map_err(|e| e.to_string())?;
        let component: String = row.try_get("", "component").map_err(|e| e.to_string())?;
        match component.as_str() {
            "igst" => form.igst.push(id),
            "cgst" => form.cgst.push(id),
            "sgst" => form.sgst.push(id),
            "cess" => form.cess.push(id),
            _ => {}
        }
    }
    Ok(form)
}

async fn items_for(db: &DatabaseConnection, ids: &[i64]) -> Result<Vec<ManyToManyItem>, String> {
    let taxes = load_taxes_by_ids(db, ids)
        .await
        .map_err(|e| e.to_string())?;
    let labels: HashMap<i64, String> = taxes
        .into_iter()
        .map(|tax| (tax.id, tax_label(&tax)))
        .collect();
    Ok(ids
        .iter()
        .filter(|id| **id > 0)
        .map(|id| {
            let label = labels.get(id).cloned().unwrap_or_else(|| format!("#{id}"));
            ManyToManyItem::new(id.to_string(), label)
        })
        .collect())
}

async fn replace_components(
    db: &DatabaseConnection,
    form: &GstPreferencesForm,
) -> Result<(), String> {
    let mut seen = HashMap::new();
    let mut rows = Vec::new();
    assign(&form.igst, "igst", &mut seen, &mut rows)?;
    assign(&form.cgst, "cgst", &mut seen, &mut rows)?;
    assign(&form.sgst, "sgst", &mut seen, &mut rows)?;
    assign(&form.cess, "cess", &mut seen, &mut rows)?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    txn.execute_raw(Statement::from_string(
        DatabaseBackend::Postgres,
        "DELETE FROM gst_tax_components".to_string(),
    ))
    .await
    .map_err(|e| e.to_string())?;
    for (id, component) in rows {
        txn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO gst_tax_components (tax_id, component) VALUES ($1, $2)",
            [id.into(), component.to_string().into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    }
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

fn assign(
    ids: &[i64],
    component: &'static str,
    seen: &mut HashMap<i64, &'static str>,
    rows: &mut Vec<(i64, &'static str)>,
) -> Result<(), String> {
    for id in ids {
        if *id <= 0 {
            continue;
        }
        if let Some(other) = seen.insert(*id, component) {
            if other != component {
                return Err(
                    "each tax can belong to only one of IGST, CGST, SGST/UTGST, or Cess"
                        .to_string(),
                );
            }
            continue;
        }
        rows.push((*id, component));
    }
    Ok(())
}
