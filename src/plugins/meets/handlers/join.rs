use axum::{
    extract::Path,
    http::{HeaderMap, header},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        meets::{
            cookies::{anon_id_from_headers, set_anon_cookie_header},
            forms::AnonymousJoinForm,
            logic::{join::join_anonymous, rooms::find_room},
            routes::{HubRouteTag, RoomLobbyRouteTag},
            state::MeetsState,
            templates::MeetsJoinPage,
        },
        users::middleware::OptionalAuth,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

fn slot_ctx(auth: Option<&crate::plugins::users::state::AuthContext>) -> SlotCtx {
    match auth {
        Some(ctx) => SlotCtx::from_auth(ctx),
        None => SlotCtx::default(),
    }
}

pub async fn join_get(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    Path(code): Path<String>,
) -> Response {
    if auth.is_some() {
        return Redirect::to(&RoomLobbyRouteTag::new(code).url()).into_response();
    }
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if room.anonymous_allowed => {
            let page = MeetsJoinPage {
                code,
                name: String::new(),
                email: String::new(),
                error: String::new(),
                authenticated: false,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(None)).into_response()
        }
        _ => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn join_post(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(code): Path<String>,
    HtmlFormBody(form): HtmlFormBody<AnonymousJoinForm>,
) -> Response {
    if auth.is_some() {
        return htmx.redirect(&RoomLobbyRouteTag::new(code).url());
    }
    let room = match find_room(&state.db, &code).await {
        Ok(Some(room)) => room,
        _ => return htmx.redirect(&HubRouteTag.url()),
    };
    let existing = anon_id_from_headers(&headers, &state.anon_secret);
    match join_anonymous(&state.db, &room, &form.name, &form.email, existing).await {
        Ok((anon, _)) => {
            let mut response = htmx.redirect(&RoomLobbyRouteTag::new(code).url());
            let cookie = set_anon_cookie_header(&state.anon_secret, anon.id, &headers);
            response.headers_mut().append(header::SET_COOKIE, cookie);
            response
        }
        Err(e) => {
            let page = MeetsJoinPage {
                code,
                name: form.name,
                email: form.email,
                error: e,
                authenticated: false,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(None)).into_response()
        }
    }
}
