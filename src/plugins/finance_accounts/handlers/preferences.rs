use axum::response::{IntoResponse, Redirect, Response};

use crate::html_form::{FormCtx, HtmlForm};
use maud::html;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::{
    accounting_preferences_patch::{save_accounting_preferences_addons, AccountingPreferencesPost},
    forms::AccountingPreferencesForm,
    preferences::{load_accounting_preferences, save_default_currency_id},
    routes::AccountingPreferencesRouteTag,
    scope::{currency_summary, load_currency_by_id},
    state::AccountsState,
    templates::AccountingPreferencesPage,
};

async fn render_accounts_inputs(db: &sea_orm::DatabaseConnection) -> maud::Markup {
    use crate::plugins::finance_accounts::forms::AccountingPreferencesFormField;

    let prefs = load_accounting_preferences(db).await;
    let currency_id = prefs.default_currency_id.unwrap_or(0);
    let currency_display = if currency_id > 0 {
        load_currency_by_id(db, currency_id)
            .await
            .map(|c| currency_summary(&c))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let id_value = if currency_id > 0 {
        currency_id.to_string()
    } else {
        String::new()
    };
    html! {
        (AccountingPreferencesForm::render_inputs(
            &FormCtx::form::<AccountingPreferencesForm>()
                .value(AccountingPreferencesFormField::DefaultCurrencyId, id_value)
                .display(
                    AccountingPreferencesFormField::DefaultCurrencyId,
                    &currency_display,
                ),
        ))
    }
}

pub async fn get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance").into_response();
    }
    let accounts_inputs = render_accounts_inputs(&state.db).await;
    let addon_inputs =
        crate::plugins::finance_accounts::accounting_preferences_patch::render_accounting_preferences_addons(&state.db).await;
    let page = AccountingPreferencesPage {
        accounts_inputs,
        addon_inputs,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn post(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    post: AccountingPreferencesPost,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance").into_response();
    }
    match post.accounts() {
        Ok(form) => {
            if let Err(e) = save_default_currency_id(
                &state.db,
                form.default_currency_id.filter(|id| *id > 0),
            )
            .await
            {
                tracing::error!("accounting preferences default currency save: {e}");
            }
        }
        Err(e) => tracing::error!("accounting preferences: invalid form body: {e}"),
    }
    if let Err(e) = save_accounting_preferences_addons(&state.db, &post).await {
        tracing::error!("accounting preferences addon save: {e}");
    }
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}
