use axum::{
    extract::{Path, Query},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    duration::format_duration,
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        filesystem::entities::VNodeEntity,
        meets::{
            cookies::anon_id_from_headers,
            entities::{
                ConferenceRoom, JoinedUser,
                meeting_recording::{self, Entity as MeetingRecordingEntity},
            },
            forms::CreateRoomForm,
            handlers::ModalNameQuery,
            keys::{MeetsEditModalKey, MeetsRoomChromeKey},
            logic::{
                join::{
                    display_name_for_joined, find_anonymous_join, find_registered_join,
                    join_registered, list_joined,
                },
                rooms::{
                    UpdateRoomInput, delete_room, find_room, room_is_live, set_joining_allowed,
                    start_room, stop_room, update_room,
                },
            },
            routes::{
                HubRouteTag, JoinGetRouteTag, RoomCallRouteTag, RoomLobbyRouteTag, RoomRouteTag,
            },
            state::MeetsState,
            templates::{
                MeetsCallPage, MeetsConfirmDeletePage, MeetsEditModalPage, MeetsLobbyPage,
                MeetsRecordingsPage, MeetsRoomPage, RecordingRow, RosterEntry,
            },
        },
        users::middleware::{OptionalAuth, RequireAuth},
    },
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, opt_or_log,
        respond_edit_modal_done,
    },
};

pub struct RoomAccess {
    pub room: ConferenceRoom,
    pub joined: Option<JoinedUser>,
    pub is_host: bool,
    pub can_manage: bool,
    pub authenticated: bool,
}

pub async fn resolve_access(
    state: &MeetsState,
    headers: &HeaderMap,
    auth: Option<&crate::plugins::users::state::AuthContext>,
    code: &str,
) -> Result<RoomAccess, String> {
    let room = find_room(&state.db, code)
        .await?
        .ok_or_else(|| "meeting not found".to_string())?;
    let is_host = auth.is_some_and(|a| a.user.id == room.created_by_id);
    let can_manage = auth.is_some_and(|a| can_manage_room(&room, a));
    let authenticated = auth.is_some();
    let joined = if let Some(ctx) = auth {
        find_registered_join(&state.db, &room.code, ctx.user.id).await?
    } else if let Some(anon_id) = anon_id_from_headers(headers, &state.anon_secret) {
        find_anonymous_join(&state.db, &room.code, anon_id).await?
    } else {
        None
    };
    Ok(RoomAccess {
        room,
        joined,
        is_host,
        can_manage,
        authenticated,
    })
}

fn datetime_label(dt: Option<DateTime<Utc>>, tz: &str) -> String {
    dt.map(|t| crate::datetime::DatetimeLabel::short(t, tz).into_string())
        .unwrap_or_default()
}

fn participant_duration_label(
    joined_at: DateTime<Utc>,
    now: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    live: bool,
) -> String {
    let end = if live { now } else { ended_at.unwrap_or(now) };
    let nanos = end
        .signed_duration_since(joined_at)
        .num_nanoseconds()
        .unwrap_or(0);
    if nanos <= 0 {
        return "—".to_string();
    }
    format_duration(nanos)
}

async fn roster_entries(
    state: &MeetsState,
    room: &ConferenceRoom,
    tz: &str,
    now: DateTime<Utc>,
) -> Vec<RosterEntry> {
    let live = room_is_live(room);
    let rows = list_joined(&state.db, &room.code).await.unwrap_or_default();
    let mut out = Vec::new();
    for row in rows {
        let name = display_name_for_joined(&state.db, &row)
            .await
            .unwrap_or_else(|_| format!("participant #{}", row.id));
        let ty = match row.user_type {
            crate::plugins::meets::entities::user_type::JoinedUserType::Registered => {
                "Registered".into()
            }
            crate::plugins::meets::entities::user_type::JoinedUserType::Anonymous => {
                "Anonymous".into()
            }
        };
        out.push(RosterEntry {
            joined_user_id: row.id,
            name,
            user_type: ty,
            joined_at: datetime_label(Some(row.joined_at), tz),
            duration: participant_duration_label(row.joined_at, now, room.ended_at, live),
        });
    }
    out
}

async fn recording_rows(state: &MeetsState, room_code: &str, tz: &str) -> Vec<RecordingRow> {
    let recs = MeetingRecordingEntity::find()
        .filter(meeting_recording::Column::ConferenceRoomCode.eq(room_code))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let mut out = Vec::new();
    for rec in recs {
        let vnode = opt_or_log(
            VNodeEntity::find_by_id(rec.video_recording_id)
                .one(&state.db)
                .await,
            "meets recording vnode",
        );
        let filename = vnode
            .as_ref()
            .map(|v| v.name.clone())
            .unwrap_or_else(|| format!("recording-{}", rec.id));
        out.push(RecordingRow {
            id: rec.id,
            started_at: crate::datetime::DatetimeLabel::short(rec.meeting_start_at, tz)
                .into_string(),
            vnode_id: rec.video_recording_id,
            filename,
        });
    }
    out
}

pub async fn room_page_from_access(
    state: &MeetsState,
    access: &RoomAccess,
    tz: &str,
) -> MeetsRoomPage {
    let now = Utc::now();
    let live = room_is_live(&access.room);
    MeetsRoomPage {
        code: access.room.code.clone(),
        live,
        joining_allowed: access.room.joining_allowed,
        anonymous_allowed: access.room.anonymous_allowed,
        is_host: access.is_host,
        can_manage: access.can_manage,
        authenticated: access.authenticated,
        scheduled_start_label: datetime_label(access.room.scheduled_start_at, tz),
        started_at_label: datetime_label(access.room.started_at, tz),
        ended_at_label: datetime_label(access.room.ended_at, tz),
        has_joined: access.joined.is_some(),
        roster: roster_entries(state, &access.room, tz, now).await,
    }
}

fn meeting_ended(access: &RoomAccess) -> bool {
    access.room.ended_at.is_some()
}

fn can_access_lobby(access: &RoomAccess) -> bool {
    if !access.authenticated || meeting_ended(access) {
        return false;
    }
    if access.can_manage {
        return true;
    }
    access.room.joining_allowed
}

fn lobby_page(state: &MeetsState, access: &RoomAccess) -> MeetsLobbyPage {
    MeetsLobbyPage {
        code: access.room.code.clone(),
        is_host: access.is_host,
        can_manage: access.can_manage,
        live: room_is_live(&access.room),
        relay_url: state.config.transport.room_relay_url(&access.room.code),
        moq_jwt: String::new(),
    }
}

fn call_page(
    state: &MeetsState,
    access: &RoomAccess,
    joined_user_id: i64,
) -> Result<MeetsCallPage, String> {
    let (relay_url, moq_jwt) = state
        .moq_auth_for_joined(&access.room.code, joined_user_id)
        .map_err(|e| e.to_string())?;
    Ok(MeetsCallPage {
        code: access.room.code.clone(),
        is_host: access.is_host,
        can_manage: access.can_manage,
        relay_url,
        moq_jwt,
        joined_user_id,
    })
}

fn slot_ctx(auth: Option<&crate::plugins::users::state::AuthContext>) -> SlotCtx {
    match auth {
        Some(ctx) => SlotCtx::from_auth(ctx),
        None => SlotCtx::default(),
    }
}

pub async fn room(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Response {
    match resolve_access(&state, &headers, auth.as_ref(), &code).await {
        Ok(access) => {
            if !access.authenticated {
                if access.room.anonymous_allowed {
                    return Redirect::to(&JoinGetRouteTag::new(code).url()).into_response();
                }
                return Redirect::to("/users/login").into_response();
            }
            let tz = auth
                .as_ref()
                .map(|a| a.timezone.as_str())
                .unwrap_or("Asia/Kolkata");
            let page = room_page_from_access(&state, &access, tz).await;
            let ctx = slot_ctx(auth.as_ref());
            if htmx.targets::<MeetsRoomChromeKey>() {
                return page.render_chrome().into_response();
            }
            html_built_page_or_app_layout(&page, &htmx, &chrome, &ctx).into_response()
        }
        Err(_) => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn lobby(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Response {
    match resolve_access(&state, &headers, auth.as_ref(), &code).await {
        Ok(access) => {
            if !access.authenticated {
                if access.room.anonymous_allowed {
                    return Redirect::to(&JoinGetRouteTag::new(code).url()).into_response();
                }
                return Redirect::to("/users/login").into_response();
            }
            if !can_access_lobby(&access) {
                return Redirect::to(&RoomRouteTag::new(code).url()).into_response();
            }
            let page = lobby_page(&state, &access);
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(auth.as_ref()))
                .into_response()
        }
        Err(_) => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn enter(
    Cap(state): Cap<MeetsState>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Response {
    match resolve_access(&state, &headers, auth.as_ref(), &code).await {
        Ok(access) => {
            if !can_access_lobby(&access) {
                return htmx.redirect(&RoomRouteTag::new(code).url());
            }
            let room = if access.can_manage && !room_is_live(&access.room) {
                match start_room(&state.db, access.room.clone()).await {
                    Ok(updated) => updated,
                    Err(_) => return htmx.redirect(&RoomLobbyRouteTag::new(code).url()),
                }
            } else {
                access.room.clone()
            };
            if auth.is_some() {
                if let Some(ctx) = auth.as_ref() {
                    if join_registered(&state.db, &room, ctx.user.id, access.can_manage)
                        .await
                        .is_err()
                    {
                        return htmx.redirect(&RoomLobbyRouteTag::new(code).url());
                    }
                }
            } else if access.joined.is_none() {
                return htmx.redirect(&JoinGetRouteTag::new(code).url());
            }
            if !room_is_live(&room) {
                return htmx.redirect(&RoomLobbyRouteTag::new(code).url());
            }
            state
                .ensure_room_recorder(&room.code, room.created_by_id)
                .await;
            htmx.redirect(&RoomCallRouteTag::new(code).url())
        }
        Err(_) => htmx.redirect(&HubRouteTag.url()),
    }
}

pub async fn call(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    OptionalAuth(auth): OptionalAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Response {
    match resolve_access(&state, &headers, auth.as_ref(), &code).await {
        Ok(access) => {
            if !access.authenticated {
                if access.room.anonymous_allowed {
                    return Redirect::to(&JoinGetRouteTag::new(code).url()).into_response();
                }
                return Redirect::to("/users/login").into_response();
            }
            let Some(ref joined) = access.joined else {
                return Redirect::to(&RoomLobbyRouteTag::new(code).url()).into_response();
            };
            if !room_is_live(&access.room) {
                return Redirect::to(&RoomRouteTag::new(code).url()).into_response();
            }
            let page = match call_page(&state, &access, joined.id) {
                Ok(page) => page,
                Err(_) => return Redirect::to(&RoomLobbyRouteTag::new(code).url()).into_response(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(auth.as_ref()))
                .into_response()
        }
        Err(_) => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn start(
    Cap(_state): Cap<MeetsState>,
    RequireAuth(_ctx): RequireAuth,
    Path(code): Path<String>,
    htmx: Htmx,
) -> Response {
    htmx.redirect(&RoomLobbyRouteTag::new(code).url())
}

pub async fn stop(
    Cap(state): Cap<MeetsState>,
    RequireAuth(ctx): RequireAuth,
    Path(code): Path<String>,
    htmx: Htmx,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => match stop_room(&state.db, room).await {
            Ok(updated) => {
                state.stop_room_recorder(&updated.code).await;
                htmx.redirect(&RoomRouteTag::new(updated.code).url())
            }
            Err(_) => htmx.redirect(&RoomRouteTag::new(code).url()),
        },
        _ => htmx.redirect(&HubRouteTag.url()),
    }
}

pub async fn lock(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => {
            let joining = !room.joining_allowed;
            match set_joining_allowed(&state.db, room, joining).await {
                Ok(_) => match resolve_access(&state, &headers, Some(&ctx), &code).await {
                    Ok(access) => {
                        let page =
                            room_page_from_access(&state, &access, ctx.timezone.as_str()).await;
                        if htmx.targets::<MeetsRoomChromeKey>() {
                            return page.render_chrome().into_response();
                        }
                        html_built_page_or_app_layout(
                            &page,
                            &htmx,
                            &chrome,
                            &SlotCtx::from_auth(&ctx),
                        )
                        .into_response()
                    }
                    Err(_) => htmx.redirect(&HubRouteTag.url()),
                },
                Err(_) => htmx.redirect(&RoomRouteTag::new(code).url()),
            }
        }
        _ => htmx.redirect(&HubRouteTag.url()),
    }
}

pub async fn leave(htmx: Htmx) -> Response {
    htmx.redirect(&HubRouteTag.url())
}

pub async fn recordings(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(code): Path<String>,
) -> maud::Markup {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => {
            let page = MeetsRecordingsPage {
                recordings: recording_rows(&state, &code, ctx.timezone.as_str()).await,
                code,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
        }
        _ => MeetsRecordingsPage::access_denied(),
    }
}

fn can_manage_room(room: &ConferenceRoom, ctx: &crate::plugins::users::state::AuthContext) -> bool {
    room.created_by_id == ctx.user.id || ctx.user.is_superuser
}

async fn room_edit_modal_page(
    room: &ConferenceRoom,
    ctx: &crate::plugins::users::state::AuthContext,
    form_name: String,
    error: String,
) -> MeetsEditModalPage {
    let start_at = room
        .scheduled_start_at
        .map(|dt| ctx.datetime_local_input(dt).into_string())
        .unwrap_or_default();
    MeetsEditModalPage {
        code: room.code.clone(),
        form_name,
        anonymous_allowed: room.anonymous_allowed,
        joining_allowed: room.joining_allowed,
        start_at,
        error,
    }
}

pub async fn edit_get(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(code): Path<String>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => {
            let page = room_edit_modal_page(&room, &ctx, q.form_name(), String::new()).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
        _ => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn edit_post(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(code): Path<String>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CreateRoomForm>,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => {
            let scheduled_start_at = if form.start_at.trim().is_empty() {
                None
            } else {
                ctx.parse_datetime_local_input(&form.start_at)
            };
            match update_room(
                &state.db,
                room,
                UpdateRoomInput {
                    anonymous_allowed: form.anonymous_allowed,
                    joining_allowed: form.joining_allowed,
                    scheduled_start_at,
                },
            )
            .await
            {
                Ok(_) => respond_edit_modal_done::<MeetsEditModalKey>(
                    &htmx,
                    &RoomRouteTag::new(code).url(),
                ),
                Err(e) => {
                    let page = MeetsEditModalPage {
                        code,
                        form_name: q.form_name(),
                        anonymous_allowed: form.anonymous_allowed,
                        joining_allowed: form.joining_allowed,
                        start_at: form.start_at,
                        error: e,
                    };
                    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response()
                }
            }
        }
        _ => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn delete_get(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(code): Path<String>,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => {
            let page = MeetsConfirmDeletePage {
                code,
                form_name: q
                    .name
                    .clone()
                    .unwrap_or_else(|| "p_meets.RoomDeleteForm".into()),
                message: "Are you sure you want to delete this meeting?".into(),
                error: String::new(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
        _ => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}

pub async fn delete_post(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(code): Path<String>,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if can_manage_room(&room, &ctx) => match delete_room(&state.db, &code).await
        {
            Ok(_) => htmx.redirect(&HubRouteTag.url()),
            Err(e) => {
                tracing::error!(error = %e, code = %code, "failed to delete meeting");
                let page = MeetsConfirmDeletePage {
                    code,
                    form_name: "p_meets.RoomDeleteForm".into(),
                    message: "Are you sure you want to delete this meeting?".into(),
                    error: e,
                };
                html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response()
            }
        },
        _ => Redirect::to(&HubRouteTag.url()).into_response(),
    }
}
