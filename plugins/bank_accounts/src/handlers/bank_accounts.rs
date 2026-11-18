use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter};

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, opt_string};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    FilterField, apply_filters, clauses_from_query, like_column, text_column,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    path_and_query, respond_create_modal_done_fk, respond_edit_modal_done,
};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use crate::{
    entities::bank_account::{self, Entity as BankAccountEntity},
    forms::BankAccountForm,
    handlers::ModalNameQuery,
    keys::{
        BANK_ACCOUNT_CREATE_MODAL, BANK_ACCOUNT_DELETE_MODAL, BANK_ACCOUNT_EDIT_MODAL,
        BANK_ACCOUNT_TABLE,
    },
    logic::{self, BankAccountWrite},
    routes::{BankAccountDefaultRouteTag, BankAccountDetailRouteTag},
    scope::{
        apply_bank_account_sort, bank_display_label, find_bank_account_scoped, find_bank_by_ifsc,
    },
    state::BankAccountsState,
    templates::{
        BankAccountCreateModalPage, BankAccountDetailPage, BankAccountEditModalPage,
        BankAccountListPage, BankAccountRow, ConfirmDeletePage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct BankAccountListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn bank_accounts_list_url() -> String {
    BankAccountDefaultRouteTag.url()
}

fn bank_account_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", bank_account::Column::Name),
        text_column(
            "account_number",
            "Account Number",
            bank_account::Column::AccountNumber,
        ),
        like_column("ifsc_code", "IFSC", bank_account::Column::IfscCode),
    ]
}

async fn query_bank_accounts(
    db: &sea_orm::DatabaseConnection,
    q: &BankAccountListQuery,
    _auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> (Vec<bank_account::Model>, u32, u64) {
    let mut query =
        scope_allowed::<super::super::routes::BankAccountsView, _>(BankAccountEntity::find());
    query = query.filter(condition);
    query = apply_bank_account_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

fn model_to_row(a: bank_account::Model) -> BankAccountRow {
    BankAccountRow {
        id: a.id,
        name: a.name,
        account_number: a.account_number,
        ifsc_code: a.ifsc_code,
        micr_code: a.micr_code.unwrap_or_default(),
    }
}

async fn load_bank_account_rows(
    db: &sea_orm::DatabaseConnection,
    q: &BankAccountListQuery,
    auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> ObjectList<BankAccountRow> {
    let (models, page, total) = query_bank_accounts(db, q, auth, page_size, condition).await;
    let rows = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

fn account_write(form: &BankAccountForm) -> BankAccountWrite {
    BankAccountWrite {
        name: form.name.clone(),
        account_number: form.account_number.clone(),
        ifsc_code: form.ifsc_code.clone(),
        micr_code: opt_string(form.micr_code.clone()),
    }
}

pub async fn list(
    Cap(state): Cap<BankAccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<BankAccountListQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), None);
    let fields = bank_account_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let accounts =
        load_bank_account_rows(&state.db, &q, &ctx, q.page_size.get(), applied.condition).await;
    let page = BankAccountListPage {
        accounts,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(BANK_ACCOUNT_TABLE) {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn detail(
    Cap(state): Cap<BankAccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(account) = find_bank_account_scoped(&state.db, id).await else {
        return Redirect::to(&bank_accounts_list_url()).into_response();
    };
    let bank = find_bank_by_ifsc(&state.db, &account.ifsc_code).await;
    let page = BankAccountDetailPage {
        id: account.id,
        name: account.name,
        account_number: account.account_number,
        ifsc_code: account.ifsc_code,
        bank_name: bank
            .as_ref()
            .and_then(|b| b.bank_name.clone())
            .unwrap_or_default(),
        branch_name: bank
            .as_ref()
            .and_then(|b| b.branch_name.clone())
            .unwrap_or_default(),
        micr_code: account.micr_code.unwrap_or_default(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = BankAccountCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        account_number: String::new(),
        ifsc_code: String::new(),
        ifsc_display: String::new(),
        micr_code: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<BankAccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<BankAccountForm>,
) -> Response {
    match logic::create_bank_account(&state.db, account_write(&form)).await {
        Ok(saved) => respond_create_modal_done_fk(
            &htmx,
            BANK_ACCOUNT_CREATE_MODAL,
            &q.refresh_table(),
            &BankAccountDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let ifsc_display = bank_display_label(&state.db, &form.ifsc_code).await;
            let page = BankAccountCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                account_number: form.account_number,
                ifsc_code: form.ifsc_code,
                ifsc_display,
                micr_code: form.micr_code,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<BankAccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(account) = find_bank_account_scoped(&state.db, id).await else {
        return Redirect::to(&bank_accounts_list_url()).into_response();
    };
    let ifsc_display = bank_display_label(&state.db, &account.ifsc_code).await;
    let page = BankAccountEditModalPage {
        id: account.id,
        form_name: q.form_name(),
        name: account.name,
        account_number: account.account_number,
        ifsc_code: account.ifsc_code,
        ifsc_display,
        micr_code: account.micr_code.unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<BankAccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<BankAccountForm>,
) -> Response {
    if find_bank_account_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&bank_accounts_list_url()).into_response();
    }
    match logic::update_bank_account(&state.db, id, account_write(&form)).await {
        Ok(_) => respond_edit_modal_done(
            &htmx,
            BANK_ACCOUNT_EDIT_MODAL,
            &BankAccountDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let ifsc_display = bank_display_label(&state.db, &form.ifsc_code).await;
            let page = BankAccountEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                account_number: form.account_number,
                ifsc_code: form.ifsc_code,
                ifsc_display,
                micr_code: form.micr_code,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: BANK_ACCOUNT_DELETE_MODAL.id().to_string(),
        message: "Are you sure you want to delete this bank account?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_bank_accounts.BankAccountDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<BankAccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match BankAccountEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&bank_accounts_list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete bank account");
            let page = ConfirmDeletePage {
                modal_uid: BANK_ACCOUNT_DELETE_MODAL.id().to_string(),
                message: "Are you sure you want to delete this bank account?".into(),
                form_name: "p_bank_accounts.BankAccountDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
