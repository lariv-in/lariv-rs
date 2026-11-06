use axum::{
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::{
    entities::VNode,
    forms::VNodePermissionsForm,
    node,
    permissions::{AccessActor, NodePermissions},
    routes::{FilesystemRootPermissionsGetRouteTag, VNodeDetailRouteTag, VNodeListRouteTag},
    state::FilesystemState,
    templates::{FilesystemRootPermissionsPage, VNodePermissionsFormPage},
};
use lariv_core::components::SharedChromeFolder;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use super::{require_root_view, require_view, slot_ctx};

async fn principal_display(db: &DatabaseConnection, user_id: Option<i64>) -> String {
    match user_id {
        Some(id) => lariv_plugin_users::entities::UserEntity::find_by_id(id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|user| user.name)
            .unwrap_or_default(),
        None => String::new(),
    }
}

fn accepted_role(role: &Option<String>) -> Result<Option<String>, String> {
    let name = role.as_deref().unwrap_or("").trim();
    lariv_plugin_users::role_registry::current_role_registry().validate(name, false)?;
    if name.is_empty() {
        Ok(None)
    } else {
        Ok(Some(name.to_string()))
    }
}

fn role_name(role: &Option<String>) -> String {
    role.clone().unwrap_or_default()
}

fn permissions_page(
    node: &VNode,
    owner_id: i64,
    owner_display: String,
    role: String,
    permissions: NodePermissions,
    apply_inside: bool,
    error: String,
) -> VNodePermissionsFormPage {
    VNodePermissionsFormPage {
        id: node.id,
        name: node.name.clone(),
        is_directory: node.is_directory,
        owner_id,
        owner_display,
        role,
        permissions,
        apply_inside,
        error,
    }
}

/// HTTP handler: `permissions_get`.
pub async fn permissions_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(n) =
        lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    if let Err(response) = require_view(&n, &ctx) {
        return response;
    }
    let owner_display = principal_display(&state.db, n.owner_id).await;
    let page = permissions_page(
        &n,
        n.owner_id.unwrap_or(0),
        owner_display,
        role_name(&n.role),
        n.permissions,
        false,
        String::new(),
    );
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
}

/// HTTP handler: `permissions_post`.
pub async fn permissions_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<VNodePermissionsForm>,
) -> Response {
    let Some(n) =
        lariv_core::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return Redirect::to(&VNodeListRouteTag.url()).into_response();
    };
    let owner_id = form.owner_id.filter(|owner| *owner > 0);
    let mut permissions = NodePermissions::empty();
    permissions.set(NodePermissions::USER_READ, form.owner_view);
    permissions.set(NodePermissions::USER_WRITE, form.owner_change);
    permissions.set(NodePermissions::USER_EXECUTE, form.owner_open);
    permissions.set(NodePermissions::ROLE_READ, form.role_view);
    permissions.set(NodePermissions::ROLE_WRITE, form.role_change);
    permissions.set(NodePermissions::ROLE_EXECUTE, form.role_open);
    permissions.set(NodePermissions::OTHER_READ, form.other_view);
    permissions.set(NodePermissions::OTHER_WRITE, form.other_change);
    permissions.set(NodePermissions::OTHER_EXECUTE, form.other_open);
    permissions.set(NodePermissions::ALL_READ, form.anyone_view);
    permissions.set(NodePermissions::ALL_WRITE, form.anyone_change);
    permissions.set(NodePermissions::ALL_EXECUTE, form.anyone_open);
    let role = match accepted_role(&form.role) {
        Ok(role) => role,
        Err(error) => {
            let owner_display = principal_display(&state.db, owner_id).await;
            let page = permissions_page(
                &n,
                owner_id.unwrap_or(0),
                owner_display,
                role_name(&form.role),
                permissions,
                form.apply_inside,
                error,
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
                .into_response();
        }
    };
    if let Err(e) = node::authorize_set_access(&n, &AccessActor::from_auth(&ctx)) {
        let owner_display = principal_display(&state.db, owner_id).await;
        let page = permissions_page(
            &n,
            owner_id.unwrap_or(0),
            owner_display,
            role_name(&role),
            permissions,
            form.apply_inside,
            e.to_string(),
        );
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
            .into_response();
    }
    let recursive = n.is_directory && form.apply_inside;
    let name = n.name.clone();
    let is_directory = n.is_directory;
    match node::set_access(&state.db, n, owner_id, role.clone(), permissions, recursive).await {
        Ok(()) => htmx.redirect(&VNodeDetailRouteTag::new(id).url()),
        Err(e) => {
            let owner_display = principal_display(&state.db, owner_id).await;
            let page = VNodePermissionsFormPage {
                id,
                name,
                is_directory,
                owner_id: owner_id.unwrap_or(0),
                owner_display,
                role: role_name(&role),
                permissions,
                apply_inside: form.apply_inside,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
        }
    }
}

fn root_permissions_page(
    owner_id: i64,
    owner_display: String,
    role: String,
    permissions: NodePermissions,
    apply_all: bool,
    error: String,
) -> FilesystemRootPermissionsPage {
    FilesystemRootPermissionsPage {
        owner_id,
        owner_display,
        role,
        permissions,
        apply_all,
        error,
    }
}

/// HTTP handler: filesystem root permissions.
pub async fn root_permissions_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    if let Err(response) = require_root_view(&state.db, &ctx).await {
        return response;
    }
    let root = match crate::preferences::load(&state.db).await {
        Ok(root) => root,
        Err(err) => {
            let page = root_permissions_page(
                0,
                String::new(),
                String::new(),
                NodePermissions::empty(),
                false,
                err.to_string(),
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
                .into_response();
        }
    };
    let owner_display = principal_display(&state.db, root.owner_id).await;
    let page = root_permissions_page(
        root.owner_id.unwrap_or(0),
        owner_display,
        role_name(&root.role),
        root.permissions,
        false,
        String::new(),
    );
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
}

/// HTTP handler: save filesystem root permissions.
pub async fn root_permissions_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<VNodePermissionsForm>,
) -> Response {
    let owner_id = form.owner_id.filter(|owner| *owner > 0);
    let mut permissions = NodePermissions::empty();
    permissions.set(NodePermissions::USER_READ, form.owner_view);
    permissions.set(NodePermissions::USER_WRITE, form.owner_change);
    permissions.set(NodePermissions::USER_EXECUTE, form.owner_open);
    permissions.set(NodePermissions::ROLE_READ, form.role_view);
    permissions.set(NodePermissions::ROLE_WRITE, form.role_change);
    permissions.set(NodePermissions::ROLE_EXECUTE, form.role_open);
    permissions.set(NodePermissions::OTHER_READ, form.other_view);
    permissions.set(NodePermissions::OTHER_WRITE, form.other_change);
    permissions.set(NodePermissions::OTHER_EXECUTE, form.other_open);
    permissions.set(NodePermissions::ALL_READ, form.anyone_view);
    permissions.set(NodePermissions::ALL_WRITE, form.anyone_change);
    permissions.set(NodePermissions::ALL_EXECUTE, form.anyone_open);
    let role = match accepted_role(&form.role) {
        Ok(role) => role,
        Err(error) => {
            let owner_display = principal_display(&state.db, owner_id).await;
            let page = root_permissions_page(
                owner_id.unwrap_or(0),
                owner_display,
                role_name(&form.role),
                permissions,
                form.apply_all,
                error,
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx))
                .into_response();
        }
    };
    let show = |owner_display: String, error: String| {
        let page = root_permissions_page(
            owner_id.unwrap_or(0),
            owner_display,
            role_name(&role),
            permissions,
            form.apply_all,
            error,
        );
        html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
    };
    if let Err(e) = node::authorize_set_root_access(&state.db, &AccessActor::from_auth(&ctx)).await
    {
        let owner_display = principal_display(&state.db, owner_id).await;
        return show(owner_display, e.to_string());
    }
    if let Err(e) = crate::preferences::save(&state.db, owner_id, role.clone(), permissions).await {
        let owner_display = principal_display(&state.db, owner_id).await;
        return show(owner_display, e.to_string());
    }
    if form.apply_all
        && let Err(e) = node::set_access_all(&state.db, owner_id, role.clone(), permissions).await
    {
        let owner_display = principal_display(&state.db, owner_id).await;
        return show(owner_display, e.to_string());
    }
    htmx.redirect(&FilesystemRootPermissionsGetRouteTag.url())
}
