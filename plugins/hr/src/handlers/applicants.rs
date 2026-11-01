use axum::http::Uri;
use sea_orm::sea_query::Condition;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::UrlencodedFields;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use crate::{
    entities::{
        employee::Entity as EmployeeEntity, ex_employee::Entity as ExEmployeeEntity,
        job_form::Entity as JobFormEntity,
    },
    forms::{ApplicantForm, PersonForm},
    keys::ApplicantHubTableKey,
    logic::{
        applicant::{
            ApplicantInput, parse_optional_datetime, parse_optional_fk, parse_optional_gender,
            parse_optional_text,
        },
        person::PersonInput,
    },
    scope::{
        apply_employee_sort, apply_ex_employee_sort, employee_display_name,
        ex_employee_display_name, scope_employees, scope_ex_employees,
    },
    state::HrState,
    templates::{ApplicantFormValues, ApplicantHubPage, ApplicantRow},
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub sort: Option<String>,
}

fn hub_query_from_uri(uri: &Uri) -> HubQuery {
    let Some(query) = uri.query() else {
        return HubQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}
fn person_row(
    id: i64,
    name: String,
    mobile: String,
    email: String,
    employee_code: String,
    status: &str,
    detail_href: String,
) -> ApplicantRow {
    ApplicantRow {
        id,
        name,
        mobile,
        email,
        employee_code,
        status: status.to_string(),
        detail_href,
    }
}

pub(crate) async fn query_employees(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &HubQuery,
    page_size: u32,
    condition: Condition,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = scope_employees(EmployeeEntity::find(), auth);
    query = query.filter(condition);
    query = apply_employee_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|e| {
            person_row(
                e.id,
                employee_display_name(&e),
                e.mobile.unwrap_or_default(),
                e.email.unwrap_or_default(),
                e.employee_code,
                "Employee",
                crate::routes::EmployeeDetailRouteTag::new(e.id).url(),
            )
        })
        .collect();
    (rows, page_num, total)
}

pub(crate) async fn query_ex_employees(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &HubQuery,
    page_size: u32,
    condition: Condition,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = scope_ex_employees(ExEmployeeEntity::find(), auth);
    query = query.filter(condition);
    query = apply_ex_employee_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|x| {
            person_row(
                x.id,
                ex_employee_display_name(&x),
                x.mobile,
                x.email,
                String::new(),
                "Ex-employee",
                crate::routes::ExEmployeeDetailRouteTag::new(x.id).url(),
            )
        })
        .collect();
    (rows, page_num, total)
}

pub async fn hub(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let q = hub_query_from_uri(&uri);
    let tab = q.tab.as_deref().unwrap_or("employees").to_string();
    let fields = match tab.as_str() {
        "ex_employees" => crate::filters::ex_employee_fields(),
        _ => crate::filters::employee_fields(),
    };
    let prepared = crate::filters::prepare(
        uri.query().unwrap_or(""),
        fields,
        state.db.get_database_backend(),
    );
    let (rows, page, total) = match tab.as_str() {
        "ex_employees" => {
            query_ex_employees(&state.db, &ctx, &q, q.page_size.get(), prepared.condition).await
        }
        _ => query_employees(&state.db, &ctx, &q, q.page_size.get(), prepared.condition).await,
    };
    let people = ObjectList::from_page(rows, page, q.page_size.get(), total);
    let page = ApplicantHubPage {
        people,
        tab,
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<ApplicantHubTableKey>() {
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

pub(crate) fn person_input_from_form(form: &PersonForm) -> PersonInput {
    PersonInput {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
    }
}

pub(crate) fn applicant_input_from_form(
    form: &ApplicantForm,
    ctx: &AuthContext,
) -> Result<ApplicantInput, String> {
    Ok(ApplicantInput {
        person: PersonInput {
            name: form.name.clone(),
            mobile: form.mobile.clone(),
            email: form.email.clone(),
        },
        date_of_birth: parse_optional_datetime(&form.date_of_birth, &ctx.timezone)?,
        gender: parse_optional_gender(&form.gender)?,
        resume_vnode_id: parse_optional_fk(&form.resume_vnode_id),
        job_form_id: parse_optional_fk(&form.job_form_id),
        remarks: parse_optional_text(&form.remarks),
        address: parse_optional_text(&form.address),
    })
}

pub(crate) async fn form_values_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ApplicantForm,
) -> ApplicantFormValues {
    ApplicantFormValues {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
        date_of_birth: form.date_of_birth.clone(),
        gender: form.gender.clone(),
        address: form.address.clone(),
        remarks: form.remarks.clone(),
        job_form_id: form.job_form_id.clone(),
        job_form_display: job_form_display(db, parse_optional_fk(&form.job_form_id)).await,
        resume_vnode_id: form.resume_vnode_id.clone(),
        resume_display: resume_display(db, parse_optional_fk(&form.resume_vnode_id)).await,
    }
}

async fn job_form_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    lariv_core::web::opt_or_log(JobFormEntity::find_by_id(id).one(db).await, "find job form")
        .map(|j| j.job_title)
        .unwrap_or_else(|| format!("Job posting #{id}"))
}

async fn resume_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
    lariv_core::web::opt_or_log(
        VNodeEntity::find_by_id(id).one(db).await,
        "find resume vnode",
    )
    .map(|n| n.name)
    .unwrap_or_else(|| format!("File #{id}"))
}
