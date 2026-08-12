use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryOrder};
use serde::Deserialize;

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::customer::{customer_type::CustomerType, entities::customer::{self, Entity as CustomerEntity}, forms::CustomerForm, handlers::ModalNameQuery, keys::{CustomerCreateModalKey, CustomerEditModalKey, CustomerSelectModalKey, CustomerSelectTableKey, CustomerTableKey}, routes::CustomerDetailRouteTag, scope::{apply_customer_filters, find_customer_scoped, scope_customers}, state::CustomerState, templates::{
        CustomerCreateModalPage, CustomerDetailPage, CustomerEditModalPage, CustomerListPage,
        CustomerRow, CustomerSelectPage,
    }};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct CustomerListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "Email", alias = "email")]
    pub email: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct CustomerSelectQuery {
    #[serde(flatten)]
    pub filter: CustomerListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn customer_address_fields(customer: &customer::Model) -> (String, String, String, String, String) {
    (
        customer.address_line_1.clone().unwrap_or_default(),
        customer.address_line_2.clone().unwrap_or_default(),
        customer.city.clone().unwrap_or_default(),
        customer.pincode.clone().unwrap_or_default(),
        customer.state.clone().unwrap_or_default(),
    )
}

async fn query_customers(
    db: &sea_orm::DatabaseConnection,
    q: &CustomerListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<customer::Model>, u32, u64) {
    let mut query = CustomerEntity::find();
    query = apply_customer_filters(query, q.name.as_deref(), q.email.as_deref());
    query = scope_customers(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(customer::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(customer::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Type DESC") => {
            query.order_by_desc(customer::Column::CustomerType)
        }
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(customer::Column::CustomerType)
        }
        s if s.eq_ignore_ascii_case("Email DESC") => query.order_by_desc(customer::Column::Email),
        s if s.eq_ignore_ascii_case("Email ASC") || s.eq_ignore_ascii_case("Email") => {
            query.order_by_asc(customer::Column::Email)
        }
        s if s.eq_ignore_ascii_case("Phone DESC") => query.order_by_desc(customer::Column::Phone),
        s if s.eq_ignore_ascii_case("Phone ASC") || s.eq_ignore_ascii_case("Phone") => {
            query.order_by_asc(customer::Column::Phone)
        }
        _ => query
            .order_by_desc(customer::Column::CreatedAt)
            .order_by_desc(customer::Column::Id),
    };

    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

fn model_to_row(c: customer::Model) -> CustomerRow {
    CustomerRow {
        id: c.id,
        customer_type: c.customer_type.label().to_string(),
        name: c.name,
        email: c.email.unwrap_or_default(),
        phone: c.phone.unwrap_or_default(),
        gstin: c.gstin.unwrap_or_default(),
    }
}

fn parse_customer_type(raw: &str) -> CustomerType {
    CustomerType::parse(raw).unwrap_or_default()
}

async fn load_customer_rows(
    db: &sea_orm::DatabaseConnection,
    q: &CustomerListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> ObjectList<CustomerRow> {
    let (models, page, total) = query_customers(db, q, auth, page_size).await;
    let rows: Vec<CustomerRow> = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CustomerListQuery>,
) -> maud::Markup {
    let customers = load_customer_rows(&state.db, &q, &ctx, PAGE_SIZE).await;
    let page = CustomerListPage {
        customers,
        filter_name: q.name.clone().unwrap_or_default(),
        filter_email: q.email.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<CustomerTableKey>() {
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
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(customer) = find_customer_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/customers/").into_response();
    };
    let (address_line_1, address_line_2, city, pincode, state) = customer_address_fields(&customer);
    let page = CustomerDetailPage {
        id: customer.id,
        customer_type: customer.customer_type.label().to_string(),
        name: customer.name,
        address_line_1,
        address_line_2,
        city,
        pincode,
        state,
        gstin: customer.gstin.unwrap_or_default(),
        pan: customer.pan.unwrap_or_default(),
        phone: customer.phone.unwrap_or_default(),
        email: customer.email.unwrap_or_default(),
        website: customer.website.unwrap_or_default(),
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn customer_create_modal_page_from_form(
    form: &CustomerForm,
    form_name: String,
    refresh_table: String,
    error: String,
) -> CustomerCreateModalPage {
    CustomerCreateModalPage {
        form_name,
        refresh_table,
        customer_type: form.customer_type.clone(),
        name: form.name.clone(),
        address_line_1: form.address_line_1.clone(),
        address_line_2: form.address_line_2.clone(),
        city: form.city.clone(),
        pincode: form.pincode.clone(),
        state: form.state.clone(),
        gstin: form.gstin.clone(),
        pan: form.pan.clone(),
        phone: form.phone.clone(),
        email: form.email.clone(),
        website: form.website.clone(),
        error,
    }
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = CustomerCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        customer_type: CustomerType::default().as_str().to_string(),
        name: String::new(),
        address_line_1: String::new(),
        address_line_2: String::new(),
        city: String::new(),
        pincode: String::new(),
        state: String::new(),
        gstin: String::new(),
        pan: String::new(),
        phone: String::new(),
        email: String::new(),
        website: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<CustomerForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/customers/").into_response();
    }
    let now = Utc::now();
    let customer_type = parse_customer_type(&form.customer_type);
    let model = customer::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        customer_type: Set(customer_type),
        name: Set(form.name.clone()),
        address_line_1: Set(opt_string(form.address_line_1.clone())),
        address_line_2: Set(opt_string(form.address_line_2.clone())),
        city: Set(opt_string(form.city.clone())),
        pincode: Set(opt_string(form.pincode.clone())),
        state: Set(opt_string(form.state.clone())),
        gstin: Set(opt_string(form.gstin.clone())),
        pan: Set(opt_string(form.pan.clone())),
        phone: Set(opt_string(form.phone.clone())),
        email: Set(opt_string(form.email.clone())),
        website: Set(opt_string(form.website.clone())),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<CustomerCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &CustomerDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = customer_create_modal_page_from_form(
                &form,
                q.form_name(),
                q.refresh_table(),
                e.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/customers/").into_response();
    }
    let Some(customer) = find_customer_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/customers/").into_response();
    };
    let (address_line_1, address_line_2, city, pincode, state) = customer_address_fields(&customer);
    let page = CustomerEditModalPage {
        id: customer.id,
        form_name: q.form_name(),
        customer_type: customer.customer_type.as_str().to_string(),
        name: customer.name,
        address_line_1,
        address_line_2,
        city,
        pincode,
        state,
        gstin: customer.gstin.unwrap_or_default(),
        pan: customer.pan.unwrap_or_default(),
        phone: customer.phone.unwrap_or_default(),
        email: customer.email.unwrap_or_default(),
        website: customer.website.unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn customer_edit_modal_page_from_form(
    id: i64,
    form: &CustomerForm,
    form_name: String,
    error: String,
) -> CustomerEditModalPage {
    CustomerEditModalPage {
        id,
        form_name,
        customer_type: form.customer_type.clone(),
        name: form.name.clone(),
        address_line_1: form.address_line_1.clone(),
        address_line_2: form.address_line_2.clone(),
        city: form.city.clone(),
        pincode: form.pincode.clone(),
        state: form.state.clone(),
        gstin: form.gstin.clone(),
        pan: form.pan.clone(),
        phone: form.phone.clone(),
        email: form.email.clone(),
        website: form.website.clone(),
        error,
    }
}

pub async fn edit_post(
    Cap(state): Cap<CustomerState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<CustomerForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/customers/").into_response();
    }
    let Some(existing) = find_customer_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/customers/").into_response();
    };
    let now = Utc::now();
    let customer_type = parse_customer_type(&form.customer_type);
    let model = customer::ActiveModel {
        id: Set(existing.id),
        updated_at: Set(Some(now)),
        customer_type: Set(customer_type),
        name: Set(form.name.clone()),
        address_line_1: Set(opt_string(form.address_line_1.clone())),
        address_line_2: Set(opt_string(form.address_line_2.clone())),
        city: Set(opt_string(form.city.clone())),
        pincode: Set(opt_string(form.pincode.clone())),
        state: Set(opt_string(form.state.clone())),
        gstin: Set(opt_string(form.gstin.clone())),
        pan: Set(opt_string(form.pan.clone())),
        phone: Set(opt_string(form.phone.clone())),
        email: Set(opt_string(form.email.clone())),
        website: Set(opt_string(form.website.clone())),
        ..Default::default()
    };
    match model.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<CustomerEditModalKey>(
            &htmx,
            &CustomerDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = customer_edit_modal_page_from_form(id, &form, q.form_name(), e.to_string());
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<CustomerState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/customers/").into_response();
    }
    if find_customer_scoped(&state.db, id, &ctx).await.is_some() {
        let _ = customer::Entity::delete_by_id(id).exec(&state.db).await;
    }
    Redirect::to("/customers/").into_response()
}

pub async fn select(
    Cap(state): Cap<CustomerState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CustomerSelectQuery>,
) -> maud::Markup {
    let customers = load_customer_rows(&state.db, &q.filter, &ctx, PAGE_SIZE).await;
    let page = CustomerSelectPage {
        customers,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        filter_email: q.filter.email.clone().unwrap_or_default(),
        target_input: q
            .target_input
            .clone()
            .unwrap_or_else(|| "CustomerID".into()),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    respond_picker_select::<CustomerSelectTableKey, CustomerSelectModalKey, _>(&htmx, &page)
}
