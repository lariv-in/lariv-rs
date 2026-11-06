use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx, sort_desc};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done,
};
use lariv_plugin_hr::{
    routes::EmployeeDetailRouteTag,
    scope::{employee_display_name, find_employee_scoped},
};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::leaves::{
        leave_journal::{self, Entity as LeaveJournalEntity},
        leave_type::LeaveType,
    },
    forms::GiveLeaveForm,
    handlers::ModalNameQuery,
    keys::{GIVE_LEAVE_MODAL, LEAVE_JOURNAL_TABLE},
    logic::leave::{
        format_journal_amount, give_leave, leave_journal_balances, parse_give_leave_amount,
    },
    routes::EmployeeLeaveJournalRouteTag,
    state::LeavesState,
    templates::leaves::{
        EmployeeLeaveJournalPage, GiveLeaveModalPage, LeaveJournalBalance, LeaveJournalRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
struct LeaveJournalQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
}

fn journal_path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn journal_query_from_uri(uri: &Uri) -> LeaveJournalQuery {
    let Some(query) = uri.query() else {
        return LeaveJournalQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

pub async fn leave_journal(
    Cap(state): Cap<LeavesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    uri: Uri,
) -> Response {
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&EmployeeDetailRouteTag::new(id).url()).into_response();
    };
    let q = journal_query_from_uri(&uri);
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let sort = q
        .sort
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Datetime DESC".to_string());
    let desc = sort_desc(&sort);
    let mut query =
        LeaveJournalEntity::find().filter(leave_journal::Column::UserId.eq(employee.user_id));
    query = match sort.split_whitespace().next().unwrap_or("") {
        column if column.eq_ignore_ascii_case("LeaveType") => {
            if desc {
                query.order_by_desc(leave_journal::Column::LeaveType)
            } else {
                query.order_by_asc(leave_journal::Column::LeaveType)
            }
        }
        column if column.eq_ignore_ascii_case("Amount") => {
            if desc {
                query.order_by_desc(leave_journal::Column::Amount)
            } else {
                query.order_by_asc(leave_journal::Column::Amount)
            }
        }
        _ => {
            if desc {
                query.order_by_desc(leave_journal::Column::Datetime)
            } else {
                query.order_by_asc(leave_journal::Column::Datetime)
            }
        }
    };
    query = query.order_by_desc(leave_journal::Column::Id);
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|row| LeaveJournalRow {
            datetime: ctx.format_datetime(row.datetime).into_string(),
            leave_type: row.leave_type.to_string(),
            amount: format_journal_amount(row.amount),
        })
        .collect();
    let totals = leave_journal_balances(&state.db, employee.user_id).await;
    let balances = [LeaveType::Casual, LeaveType::Sick, LeaveType::Privilege]
        .into_iter()
        .zip(totals)
        .map(|(kind, amount)| LeaveJournalBalance {
            leave_type: kind.label().to_string(),
            amount: format_journal_amount(amount),
        })
        .collect();
    let page = EmployeeLeaveJournalPage {
        employee_id: employee.id,
        display_name: employee_display_name(&employee),
        balances,
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        sort,
        path_and_query: journal_path_and_query(&uri),
    };
    if htmx.targets(LEAVE_JOURNAL_TABLE) {
        return page.render_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn give_leave_get(
    Cap(state): Cap<LeavesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_employee_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&EmployeeDetailRouteTag::new(id).url()).into_response();
    }
    let page = GiveLeaveModalPage::new(id, q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn give_leave_post(
    Cap(state): Cap<LeavesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<GiveLeaveForm>,
) -> Response {
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&EmployeeDetailRouteTag::new(id).url()).into_response();
    };
    let error = match give_leave_from_form(&form) {
        Ok((leave_type, amount)) => {
            match give_leave(&state.db, employee.user_id, leave_type, amount).await {
                Ok(()) => {
                    return respond_create_modal_done(
                        &htmx,
                        GIVE_LEAVE_MODAL,
                        &q.refresh_table(),
                        &EmployeeLeaveJournalRouteTag::new(id).url(),
                    );
                }
                Err(error) => error,
            }
        }
        Err(error) => error,
    };
    let page = GiveLeaveModalPage {
        employee_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        leave_type: form.leave_type,
        amount: form.amount,
        error,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn give_leave_from_form(form: &GiveLeaveForm) -> Result<(LeaveType, i64), String> {
    let leave_type = LeaveType::parse(form.leave_type.trim())
        .ok_or_else(|| "leave type is required".to_string())?;
    let amount = parse_give_leave_amount(&form.amount)?;
    Ok((leave_type, amount))
}
