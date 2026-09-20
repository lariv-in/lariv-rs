use axum::{
    extract::{
        Path,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use rtc::peer_connection::transport::RTCIceCandidateInit;

use crate::{
    http::Cap,
    plugins::{
        meets::{
            cookies::anon_id_from_headers,
            logic::{
                join::{find_anonymous_join, find_registered_join},
                rooms::find_room,
            },
            signaling::{ClientMsg, ServerMsg},
            state::MeetsState,
        },
        users::middleware::OptionalAuth,
    },
};

/// `GET /meets/{code}/signal` — WebRTC JSON signaling.
pub async fn upgrade(
    Cap(state): Cap<MeetsState>,
    OptionalAuth(auth): OptionalAuth,
    headers: HeaderMap,
    Path(code): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, auth, headers, code))
}

async fn handle_socket(
    socket: WebSocket,
    state: MeetsState,
    auth: Option<crate::plugins::users::state::AuthContext>,
    headers: HeaderMap,
    code: String,
) {
    let Ok(Some(room_row)) = find_room(&state.db, &code).await else {
        return;
    };
    let joined = if let Some(ctx) = &auth {
        find_registered_join(&state.db, &code, ctx.user.id)
            .await
            .ok()
            .flatten()
    } else if let Some(anon_id) = anon_id_from_headers(&headers, &state.anon_secret) {
        find_anonymous_join(&state.db, &code, anon_id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    let Some(joined) = joined else {
        return;
    };
    let joined_user_id = joined.id;
    let sfu = state.room(&code).await;
    sfu.set_created_by(room_row.created_by_id);

    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMsg>();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink
                .send(Message::Text(msg.to_text().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };
        let parsed: ClientMsg = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.send(ServerMsg::Error {
                    message: e.to_string(),
                });
                continue;
            }
        };
        match parsed {
            ClientMsg::Join {
                joined_user_id: claimed,
                video_codecs,
            } => {
                if claimed != joined_user_id {
                    let _ = tx.send(ServerMsg::Error {
                        message: "joined_user_id mismatch".into(),
                    });
                    continue;
                }
                if let Err(e) = sfu.add_peer(joined_user_id, video_codecs, tx.clone()).await {
                    let _ = tx.send(ServerMsg::Error { message: e });
                }
            }
            ClientMsg::Answer { sdp } => {
                if let Err(e) = sfu.handle_answer(joined_user_id, sdp).await {
                    let _ = tx.send(ServerMsg::Error { message: e });
                }
            }
            ClientMsg::Ice {
                candidate,
                sdp_mid,
                sdp_mline_index,
            } => {
                let init = RTCIceCandidateInit {
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                    username_fragment: None,
                    url: None,
                };
                if let Err(e) = sfu.handle_ice(joined_user_id, init).await {
                    let _ = tx.send(ServerMsg::Error { message: e });
                }
            }
            ClientMsg::Leave => break,
        }
    }

    sfu.remove_peer(joined_user_id).await;
    send_task.abort();
}
