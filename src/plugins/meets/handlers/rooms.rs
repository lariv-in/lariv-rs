use axum::{
    extract::Path,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        filesystem::entities::VNodeEntity,
        meets::{
            cookies::anon_id_from_headers,
            entities::{
                ConferenceRoom, JoinedUser,
                meeting_recording::{self, Entity as MeetingRecordingEntity},
                user_type::JoinedUserType,
            },
            keys::MeetsRoomChromeKey,
            logic::{
                join::{find_anonymous_join, get_anonymous_user, join_registered, list_joined},
                rooms::{find_room, room_is_live, set_joining_allowed, start_room},
            },
            routes::{HubRouteTag, JoinGetRouteTag, RoomRouteTag, SignalRouteTag},
            state::MeetsState,
            templates::{MeetsRecordingsPage, MeetsRoomPage, RecordingRow, RosterEntry},
        },
        users::{
            entities::user::Entity as UserEntity,
            middleware::{OptionalAuth, RequireAuth},
        },
    },
    web::{Htmx, html_built_page_or_app_layout, opt_or_log},
};

pub struct RoomAccess {
    pub room: ConferenceRoom,
    pub joined: Option<JoinedUser>,
    pub is_host: bool,
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
    let authenticated = auth.is_some();
    let joined = if let Some(ctx) = auth {
        Some(join_registered(&state.db, &room, ctx.user.id).await?)
    } else if let Some(anon_id) = anon_id_from_headers(headers, &state.anon_secret) {
        find_anonymous_join(&state.db, &room.code, anon_id).await?
    } else {
        None
    };
    Ok(RoomAccess {
        room,
        joined,
        is_host,
        authenticated,
    })
}

async fn roster_entries(state: &MeetsState, room_code: &str) -> Vec<RosterEntry> {
    let rows = list_joined(&state.db, room_code).await.unwrap_or_default();
    let mut out = Vec::new();
    for row in rows {
        let (name, ty) = match row.user_type {
            JoinedUserType::Registered => {
                let name = if let Some(uid) = row.user_id {
                    opt_or_log(
                        UserEntity::find_by_id(uid).one(&state.db).await,
                        "meets roster user",
                    )
                    .map(|u| u.name)
                    .unwrap_or_else(|| format!("user #{uid}"))
                } else {
                    "Registered".into()
                };
                (name, "Registered".into())
            }
            JoinedUserType::Anonymous => {
                let name = if let Some(aid) = row.anonymous_user_id {
                    get_anonymous_user(&state.db, aid)
                        .await
                        .ok()
                        .flatten()
                        .map(|a| a.name)
                        .unwrap_or_else(|| format!("guest #{aid}"))
                } else {
                    "Guest".into()
                };
                (name, "Anonymous".into())
            }
        };
        out.push(RosterEntry {
            joined_user_id: row.id,
            name,
            user_type: ty,
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
    let live = room_is_live(&access.room, now);
    let start_at_label = access
        .room
        .start_at
        .map(|t| crate::datetime::DatetimeLabel::short(t, tz).into_string())
        .unwrap_or_default();
    MeetsRoomPage {
        code: access.room.code.clone(),
        live,
        joining_allowed: access.room.joining_allowed,
        anonymous_allowed: access.room.anonymous_allowed,
        is_host: access.is_host,
        authenticated: access.authenticated,
        joined_user_id: access.joined.as_ref().map(|j| j.id),
        ice_servers_json: state.config.ice_servers_json(),
        signaling_url: SignalRouteTag::new(access.room.code.clone()).path(),
        roster: roster_entries(state, &access.room.code).await,
        recordings: recording_rows(state, &access.room.code, tz).await,
        start_at_label,
    }
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
            if access.joined.is_none() && !access.is_host {
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

pub async fn start(
    Cap(state): Cap<MeetsState>,
    RequireAuth(ctx): RequireAuth,
    Path(code): Path<String>,
    htmx: Htmx,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if room.created_by_id == ctx.user.id => {
            match start_room(&state.db, room).await {
                Ok(updated) => htmx.redirect(&RoomRouteTag::new(updated.code).url()),
                Err(_) => htmx.redirect(&RoomRouteTag::new(code).url()),
            }
        }
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
        Ok(Some(room)) if room.created_by_id == ctx.user.id => {
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
    htmx: Htmx,
    Path(code): Path<String>,
) -> Response {
    match find_room(&state.db, &code).await {
        Ok(Some(room)) if room.created_by_id == ctx.user.id || ctx.user.is_superuser => {
            let page = MeetsRecordingsPage {
                recordings: recording_rows(&state, &code, ctx.timezone.as_str()).await,
                code,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
        _ => htmx.redirect(&HubRouteTag.url()),
    }
}
