use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::{
    components::{DEFAULT_PAGE_SIZE, ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx},
    html_form::{FormFieldKey, HtmlFormBody},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        ApplyQuery, Htmx, QueryI64, QueryPage, QueryStr, html_built_page_or_app_layout,
        html_built_page_with_slots, query_bool, respond_create_modal_done_fk,
        respond_edit_modal_done,
    },
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::{
    account_validation::{
        ACCOUNT_PARENT_UP_ROW_ID, account_descendant_ids, sync_account_children,
        validate_balance_type_change, validate_parent_balance_type_on_save,
        validate_parent_not_cycle,
    },
    balance_type::BalanceType,
    entities::account::{self, Entity as AccountEntity},
    forms::{AccountForm, AccountFormField},
    handlers::ModalNameQuery,
    keys::AccountCreateModalKey,
    keys::AccountEditModalKey,
    keys::AccountJournalEntriesTableKey,
    keys::AccountJournalEntryItemsTableKey,
    keys::AccountSelectModalKey,
    keys::AccountSelectTableKey,
    keys::AccountTableKey,
    routes::{AccountDetailRouteTag, FinanceDefaultRouteTag},
    scope::{
        CurrencyFormat, apply_account_filters, find_account_scoped, load_account_ancestors,
        load_account_parent_label, load_journal_entry_currency_formats,
        load_journal_entry_transfer_amounts, journal_entry_item_sort, journal_entry_sort,
        query_journal_entries_for_account_subtree,
        query_journal_entry_items_for_account_subtree, sum_account_subtree_balance,
    },
    source_doc_label::resolve_source_doc_display,
    source_doc_registry::SourceDocRegistry,
    state::AccountsState,
    templates::{
        AccountCreateModalPage, AccountDetailPage, AccountEditModalPage, AccountJournalEntriesPage,
        AccountJournalEntryItemRow, AccountJournalEntryItemsPage, AccountListPage, AccountRow,
        AccountSelectPage, JournalEntryRow,
    },
};

use super::util::{checkbox_on, parse_i32, parse_i64, path_and_query, query_param};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct AccountDetailQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct AccountCreateQuery {
    #[serde(default, rename = "ParentID", alias = "parent_id")]
    pub parent_id: QueryI64,
}

#[derive(Debug, Deserialize, Default)]
pub struct AccountListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: QueryStr,
    #[serde(default, rename = "Code", alias = "code")]
    pub code: QueryStr,
    #[serde(
        default,
        rename = "IsGroup",
        alias = "is_group",
        deserialize_with = "query_bool"
    )]
    pub is_group: Option<bool>,
    #[serde(default, rename = "BalanceType", alias = "balance_type")]
    pub balance_type: QueryStr,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct AccountSelectQuery {
    #[serde(flatten)]
    pub filter: AccountListQuery,
    #[serde(default, rename = "ParentID", alias = "parent_id")]
    pub parent_id: QueryI64,
    #[serde(default, rename = "balance_type_scope")]
    pub balance_type_scope: Option<String>,
    #[serde(default)]
    pub target_input: Option<String>,
    #[serde(default, rename = "exclude_account_id")]
    pub exclude_account_id: QueryI64,
}

impl ApplyQuery for AccountSelectQuery {
    fn apply_to<R: crate::http::RouteUrl>(
        &self,
        builder: crate::http::RouteQueryBuilder<R>,
    ) -> crate::http::RouteQueryBuilder<R> {
        let mut b = builder;
        if let Some(name) = self.filter.name.as_deref() {
            b = b.query("Name", name);
        }
        if let Some(code) = self.filter.code.as_deref() {
            b = b.query("Code", code);
        }
        if let Some(is_group) = self.filter.is_group {
            b = b.query("IsGroup", if is_group { "on" } else { "false" });
        }
        if let Some(bt) = self.filter.balance_type.as_deref() {
            b = b.query("BalanceType", bt);
        }
        if let Some(sort) = self.filter.sort.as_deref() {
            b = b.query("sort", sort);
        }
        if let Some(page) = self.filter.page.raw() {
            b = b.query("page", page);
        }
        if let Some(pid) = self.parent_id.positive() {
            b = b.query("ParentID", pid);
        }
        if let Some(scope) = self.balance_type_scope.as_deref() {
            b = b.query("balance_type_scope", scope);
        }
        if let Some(target) = self.target_input.as_deref() {
            b = b.query("target_input", target);
        }
        if let Some(excl) = self.exclude_account_id.positive() {
            b = b.query("exclude_account_id", excl);
        }
        b
    }
}

fn model_to_row(a: account::Model, parent_label: String) -> AccountRow {
    AccountRow {
        id: a.id,
        name: a.name,
        code: a.code,
        is_group: a.is_group,
        balance_type: a.balance_type.to_string(),
        parent_label,
    }
}

async fn query_accounts(
    db: &sea_orm::DatabaseConnection,
    q: &AccountListQuery,
    auth: &AuthContext,
    parent_id: Option<i64>,
    balance_type_scope: Option<&str>,
    root_only: bool,
    page_size: u32,
) -> (Vec<AccountRow>, u32, u64) {
    let mut query = AccountEntity::find();
    query = apply_account_filters(
        query,
        q.name.as_deref(),
        q.code.as_deref(),
        q.is_group,
        q.balance_type.as_deref(),
        parent_id,
        balance_type_scope,
        root_only,
    );
    query = crate::plugins::finance_accounts::scope::scope_superuser(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Code DESC") => query.order_by_desc(account::Column::Code),
        s if s.eq_ignore_ascii_case("Code ASC") || s.eq_ignore_ascii_case("Code") => {
            query.order_by_asc(account::Column::Code)
        }
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(account::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(account::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Type DESC") => query.order_by_desc(account::Column::IsGroup),
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(account::Column::IsGroup)
        }
        s if s.eq_ignore_ascii_case("Balance DESC") => {
            query.order_by_desc(account::Column::BalanceType)
        }
        s if s.eq_ignore_ascii_case("Balance ASC") || s.eq_ignore_ascii_case("Balance") => {
            query.order_by_asc(account::Column::BalanceType)
        }
        _ => query.order_by_asc(account::Column::Code),
    };
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for m in models {
        let parent_label = load_account_parent_label(db, m.parent_id).await;
        rows.push(model_to_row(m, parent_label));
    }
    (rows, page, total)
}

async fn load_account_rows(
    db: &sea_orm::DatabaseConnection,
    q: &AccountListQuery,
    auth: &AuthContext,
    parent_id: Option<i64>,
    balance_type_scope: Option<&str>,
    root_only: bool,
    page_size: u32,
) -> ObjectList<AccountRow> {
    let (rows, page, total) = query_accounts(
        db,
        q,
        auth,
        parent_id,
        balance_type_scope,
        root_only,
        page_size,
    )
    .await;
    ObjectList::from_page(rows, page, page_size, total)
}

async fn filter_excluded_account_rows(
    db: &sea_orm::DatabaseConnection,
    exclude_account_id: Option<i64>,
    mut accounts: ObjectList<AccountRow>,
) -> ObjectList<AccountRow> {
    let Some(exclude_id) = exclude_account_id.filter(|&id| id > 0) else {
        return accounts;
    };
    let forbidden: std::collections::HashSet<i64> = account_descendant_ids(db, exclude_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();
    if forbidden.is_empty() {
        return accounts;
    }
    let removed = accounts
        .items
        .iter()
        .filter(|r| r.id != ACCOUNT_PARENT_UP_ROW_ID && forbidden.contains(&r.id))
        .count();
    accounts
        .items
        .retain(|r| r.id == ACCOUNT_PARENT_UP_ROW_ID || !forbidden.contains(&r.id));
    accounts.total = accounts.total.saturating_sub(removed as u64);
    accounts
}

fn prepend_parent_up_row(mut accounts: ObjectList<AccountRow>) -> ObjectList<AccountRow> {
    if accounts.number != 1 {
        return accounts;
    }
    let mut items = vec![AccountRow {
        id: ACCOUNT_PARENT_UP_ROW_ID,
        name: "..".into(),
        code: 0,
        is_group: true,
        balance_type: String::new(),
        parent_label: String::new(),
    }];
    items.append(&mut accounts.items);
    accounts.items = items;
    accounts.total += 1;
    accounts
}

async fn load_child_items_for_account(
    db: &sea_orm::DatabaseConnection,
    parent_id: i64,
) -> Vec<ManyToManyItem> {
    let children = AccountEntity::find()
        .filter(account::Column::ParentId.eq(parent_id))
        .all(db)
        .await
        .unwrap_or_default();
    children
        .into_iter()
        .map(|a| ManyToManyItem::new(a.id.to_string(), format!("{} — {}", a.code, a.name)))
        .collect()
}

pub async fn list(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<AccountListQuery>,
) -> maud::Markup {
    let accounts = load_account_rows(&state.db, &q, &ctx, None, None, true, PAGE_SIZE).await;
    let page = AccountListPage {
        accounts,
        filter_name: q.name.or_empty(),
        filter_code: q.code.or_empty(),
        filter_is_group: q.is_group.unwrap_or(false),
        filter_balance_type: q.balance_type.or_empty(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<AccountTableKey>() {
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
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<AccountDetailQuery>,
) -> Response {
    let Some(a) = find_account_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    };
    let parent_label = load_account_parent_label(&state.db, a.parent_id).await;
    let ancestors = load_account_ancestors(&state.db, a.parent_id).await;
    let balance_total = sum_account_subtree_balance(&state.db, a.id).await;
    let mut children = ObjectList::from_page(vec![], 1, PAGE_SIZE, 0);
    if a.is_group {
        let child_q = AccountListQuery {
            sort: q.sort.clone(),
            ..Default::default()
        };
        children = load_account_rows(&state.db, &child_q, &ctx, Some(a.id), None, false, 100).await;
        children = prepend_parent_up_row(children);
    }
    let page = AccountDetailPage {
        id: a.id,
        name: a.name,
        code: a.code,
        is_group: a.is_group,
        balance_type: a.balance_type.to_string(),
        parent_label,
        parent_id: a.parent_id.unwrap_or(0),
        ancestors,
        balance_total,
        children,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    if htmx.targets::<AccountTableKey>() {
        return page.render_children_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn journal_entries(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<AccountDetailQuery>,
) -> Response {
    let Some(a) = find_account_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    };
    let ancestors = load_account_ancestors(&state.db, a.parent_id).await;
    let page_num = q.page.get();
    let (entry_models, entry_total) = query_journal_entries_for_account_subtree(
        &state.db,
        &ctx,
        a.id,
        page_num,
        PAGE_SIZE,
        q.sort.as_deref(),
    )
    .await;
    let entry_ids: Vec<i64> = entry_models.iter().map(|(e, _)| e.id).collect();
    let amounts = load_journal_entry_transfer_amounts(&state.db, &entry_ids).await;
    let mut entry_rows = Vec::with_capacity(entry_models.len());
    for (e, journal_name) in entry_models {
        let source_doc = resolve_source_doc_display(&state.db, &source_docs, e.source_doc_id).await;
        entry_rows.push(JournalEntryRow {
            id: e.id,
            datetime: ctx.format_datetime_seconds(e.datetime).into_string(),
            source_doc_label: source_doc.type_label,
            source_doc_instance_name: source_doc.instance_name,
            source_doc_url: source_doc.detail_url,
            journal_name,
            amount: amounts.get(&e.id).cloned().unwrap_or_else(|| "—".into()),
            label: String::new(),
        });
    }
    let entries = ObjectList::from_page(entry_rows, page_num, PAGE_SIZE, entry_total);
    let page = AccountJournalEntriesPage {
        id: a.id,
        name: a.name,
        ancestors,
        entries,
        sort: journal_entry_sort(q.sort.as_deref()).to_string(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    if htmx.targets::<AccountJournalEntriesTableKey>() {
        return page.render_entries_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn journal_entry_items(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<AccountDetailQuery>,
) -> Response {
    let Some(a) = find_account_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    };
    let ancestors = load_account_ancestors(&state.db, a.parent_id).await;
    let page_num = q.page.get();
    let (item_models, item_total) = query_journal_entry_items_for_account_subtree(
        &state.db,
        &ctx,
        a.id,
        page_num,
        PAGE_SIZE,
        q.sort.as_deref(),
    )
    .await;
    let entry_ids: Vec<i64> = item_models
        .iter()
        .map(|(item, _)| item.journal_entry_id)
        .collect();
    let currency_fmts = load_journal_entry_currency_formats(&state.db, &entry_ids).await;
    let fallback = CurrencyFormat::fallback();
    let mut item_rows = Vec::with_capacity(item_models.len());
    for (item, source_doc_id) in item_models {
        let source_doc = resolve_source_doc_display(&state.db, &source_docs, source_doc_id).await;
        let fmt = currency_fmts
            .get(&item.journal_entry_id)
            .unwrap_or(&fallback);
        item_rows.push(AccountJournalEntryItemRow {
            datetime: ctx.format_datetime_seconds(item.datetime).into_string(),
            amount: fmt.display(item.amount),
            source_doc_instance_name: source_doc.instance_name,
            source_doc_url: source_doc.detail_url,
        });
    }
    let items = ObjectList::from_page(item_rows, page_num, PAGE_SIZE, item_total);
    let page = AccountJournalEntryItemsPage {
        id: a.id,
        name: a.name,
        ancestors,
        items,
        sort: journal_entry_item_sort(q.sort.as_deref()).to_string(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    if htmx.targets::<AccountJournalEntryItemsTableKey>() {
        return page.render_items_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Query(create_q): Query<AccountCreateQuery>,
) -> maud::Markup {
    if !require_superuser(&ctx) {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let mut page = AccountCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        code: String::new(),
        is_group: false,
        balance_type: String::new(),
        parent_id: String::new(),
        parent_display: String::new(),
        error: String::new(),
    };
    if let Some(pid) = create_q.parent_id.positive() {
        if let Some(parent) = find_account_scoped(&state.db, pid, &ctx).await {
            page.parent_id = pid.to_string();
            page.parent_display = format!("{} — {}", parent.code, parent.name);
            page.balance_type = parent.balance_type.to_string();
        }
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

fn account_create_modal_page_from_form(
    form: &AccountForm,
    form_name: String,
    refresh_table: String,
    target_input: String,
    parent_display: String,
    error: String,
) -> AccountCreateModalPage {
    AccountCreateModalPage {
        form_name,
        refresh_table,
        target_input,
        name: form.name.clone(),
        code: form.code.clone(),
        is_group: checkbox_on(&form.is_group),
        balance_type: form.balance_type.clone(),
        parent_id: form.parent_id.clone(),
        parent_display,
        error,
    }
}

async fn save_account_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &AccountForm,
    existing: Option<account::Model>,
) -> Result<account::Model, String> {
    let balance_type =
        BalanceType::parse(&form.balance_type).ok_or_else(|| "invalid balance type".to_string())?;
    let parent_id = parse_i64(&form.parent_id);
    validate_parent_balance_type_on_save(db, parent_id, balance_type).await?;
    validate_parent_not_cycle(db, existing.as_ref().map(|a| a.id), parent_id).await?;
    if let Some(ref old) = existing {
        validate_balance_type_change(db, old.id, old.balance_type, balance_type).await?;
    }
    let code = parse_i32(&form.code).ok_or_else(|| "code is required".to_string())?;
    let now = Utc::now();
    if let Some(old) = existing {
        let model = account::ActiveModel {
            id: Set(old.id),
            updated_at: Set(Some(now)),
            name: Set(form.name.clone()),
            code: Set(code),
            is_group: Set(checkbox_on(&form.is_group)),
            balance_type: Set(balance_type),
            parent_id: Set(parent_id),
            ..Default::default()
        };
        model.update(db).await.map_err(|e| e.to_string())
    } else {
        let model = account::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set(form.name.clone()),
            code: Set(code),
            is_group: Set(checkbox_on(&form.is_group)),
            balance_type: Set(balance_type),
            parent_id: Set(parent_id),
            ..Default::default()
        };
        model.insert(db).await.map_err(|e| e.to_string())
    }
}

pub async fn create_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<AccountForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    }
    match save_account_from_form(&state.db, &form, None).await {
        Ok(saved) => respond_create_modal_done_fk::<AccountCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &AccountDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let parent_display = if !form.parent_id.is_empty() {
                load_account_parent_label(&state.db, parse_i64(&form.parent_id)).await
            } else {
                String::new()
            };
            let page = account_create_modal_page_from_form(
                &form,
                q.form_name(),
                q.refresh_table(),
                q.target_input(),
                parent_display,
                e,
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    }
    let Some(a) = find_account_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    };
    let parent_display = load_account_parent_label(&state.db, a.parent_id).await;
    let child_items = load_child_items_for_account(&state.db, a.id).await;
    let page = AccountEditModalPage::from_model(&a, q.form_name(), parent_display, child_items);
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn account_edit_modal_from_form(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form: &AccountForm,
    form_name: String,
    error: String,
) -> AccountEditModalPage {
    let parent_display = if !form.parent_id.is_empty() {
        load_account_parent_label(db, parse_i64(&form.parent_id)).await
    } else {
        String::new()
    };
    let child_items = {
        let mut items = Vec::new();
        for cid in &form.child_ids {
            if let Some(child) = AccountEntity::find_by_id(*cid).one(db).await.ok().flatten() {
                items.push(ManyToManyItem::new(
                    child.id.to_string(),
                    format!("{} — {}", child.code, child.name),
                ));
            }
        }
        items
    };
    AccountEditModalPage {
        id,
        form_name,
        name: form.name.clone(),
        code: form.code.clone(),
        is_group: checkbox_on(&form.is_group),
        balance_type: form.balance_type.clone(),
        parent_id: form.parent_id.clone(),
        parent_display,
        child_items,
        error,
    }
}

pub async fn edit_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<AccountForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    }
    let Some(existing) = find_account_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    };
    match save_account_from_form(&state.db, &form, Some(existing.clone())).await {
        Ok(saved) => {
            if checkbox_on(&form.is_group) {
                if let Err(e) =
                    sync_account_children(&state.db, saved.id, saved.balance_type, &form.child_ids)
                        .await
                {
                    let page =
                        account_edit_modal_from_form(&state.db, id, &form, q.form_name(), e).await;
                    return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response();
                }
            }
            respond_edit_modal_done::<AccountEditModalKey>(
                &htmx,
                &AccountDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => {
            let page =
                account_edit_modal_from_form(&state.db, id, &form, q.form_name(), e.to_string())
                    .await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    }
    if find_account_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&FinanceDefaultRouteTag.url()).into_response();
    }
    let _ = account::Entity::delete_by_id(id).exec(&state.db).await;
    Redirect::to(&FinanceDefaultRouteTag.url()).into_response()
}

pub async fn select(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<AccountSelectQuery>,
) -> maud::Markup {
    let parent_id = q.parent_id.positive();
    let mut accounts = load_account_rows(
        &state.db,
        &q.filter,
        &ctx,
        parent_id,
        q.balance_type_scope.as_deref(),
        false,
        PAGE_SIZE,
    )
    .await;
    accounts = filter_excluded_account_rows(&state.db, q.exclude_account_id.get(), accounts).await;
    let grandparent_id = if let Some(pid) = parent_id {
        AccountEntity::find_by_id(pid)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .and_then(|a| a.parent_id)
    } else {
        None
    };
    if parent_id.is_some() && accounts.number == 1 {
        let mut items = vec![AccountRow {
            id: ACCOUNT_PARENT_UP_ROW_ID,
            name: "..".into(),
            code: 0,
            is_group: true,
            balance_type: String::new(),
            parent_label: String::new(),
        }];
        items.extend(accounts.items.drain(..));
        accounts.items = items;
        accounts.total += 1;
    }
    let target_input = q
        .target_input
        .clone()
        .or_else(|| query_param(&path_and_query(&uri), "target_input"))
        .unwrap_or_else(|| AccountFormField::ParentId.target_input().into());
    let page = AccountSelectPage {
        accounts,
        filter_name: q.filter.name.or_empty(),
        filter_code: q.filter.code.or_empty(),
        filter_balance_type: q.filter.balance_type.or_empty(),
        balance_type_scope: q.balance_type_scope.clone().unwrap_or_default(),
        parent_id: parent_id.unwrap_or(0),
        grandparent_id,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input,
        exclude_account_id: q.exclude_account_id.or_zero(),
        can_edit: require_superuser(&ctx),
    };
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    respond_picker_select::<AccountSelectTableKey, AccountSelectModalKey, _>(&htmx, &page)
}
