//! Authenticated WebSocket for todo sync.

use axum::extract::Path;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sea_orm::TransactionTrait;
use serde::Serialize;
use tokio::sync::broadcast;

use crate::state::TodoState;
use crate::sync::{self, ClientOp, Incoming, ServerEvent};
use lariv_core::http::Cap;
use lariv_plugin_users::middleware::RequireAuth;

/// `GET /dashboard/todo/{id}/ws` — upgrade after cookie auth, for one owned page.
pub async fn upgrade(
    Path(page_id): Path<i64>,
    Cap(state): Cap<TodoState>,
    RequireAuth(ctx): RequireAuth,
    ws: WebSocketUpgrade,
) -> Response {
    let user_id = ctx.user.id;
    let owned = sync::find_visible_page(&state.db, user_id, page_id)
        .await
        .map(|page| page.is_some());
    match owned {
        Ok(true) => ws
            .on_upgrade(move |socket| handle_socket(socket, state, user_id, page_id))
            .into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!(error = %err, user_id, page_id, "todo: ws page lookup");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn handle_socket(mut socket: WebSocket, state: TodoState, user_id: i64, page_id: i64) {
    let mut rx = state.hub.subscribe(page_id);
    let mut synced = false;
    loop {
        if !synced {
            match socket.recv().await {
                Some(Ok(Message::Text(text))) => match sync::parse_incoming(&text) {
                    Ok(Incoming::Sync(ops)) => {
                        if handshake(&mut socket, &state, user_id, page_id, &ops, &mut rx)
                            .await
                            .is_err()
                        {
                            break;
                        }
                        synced = true;
                    }
                    Ok(Incoming::Op(op)) => {
                        if handle_op(&mut socket, &state, user_id, page_id, &[op])
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(err) => {
                        tracing::debug!(error = %err, "todo: ignored ws frame");
                        if send_error(&mut socket, "Could not read that change.")
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                },
                Some(Ok(Message::Ping(payload))) => {
                    if socket.send(Message::Pong(payload)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    tracing::debug!(error = %err, "todo: ws recv");
                    break;
                }
            }
            continue;
        }

        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => match sync::parse_incoming(&text) {
                        Ok(Incoming::Sync(ops)) => {
                            if handshake(&mut socket, &state, user_id, page_id, &ops, &mut rx)
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(Incoming::Op(op)) => {
                            if handle_op(&mut socket, &state, user_id, page_id, &[op])
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::debug!(error = %err, "todo: ignored ws frame");
                            if send_error(&mut socket, "Could not read that change.")
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    },
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        tracing::debug!(error = %err, "todo: ws recv");
                        break;
                    }
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        if send_json(&mut socket, &event).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if resnapshot(&mut socket, &state, user_id, page_id, &mut rx)
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn handshake(
    socket: &mut WebSocket,
    state: &TodoState,
    user_id: i64,
    page_id: i64,
    ops: &[ClientOp],
    rx: &mut broadcast::Receiver<ServerEvent>,
) -> Result<(), ()> {
    let _guard = state.hub.lock_page(page_id).await;
    let txn = state.db.begin().await.map_err(|err| {
        tracing::error!(error = %err, user_id, page_id, "todo: begin sync");
    })?;
    let applied = sync::apply_ops(&txn, user_id, page_id, ops)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, user_id, page_id, "todo: apply sync");
        })?;
    let todos = sync::load_snapshot(&txn, user_id, page_id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, user_id, page_id, "todo: snapshot");
        })?;
    txn.commit().await.map_err(|err| {
        tracing::error!(error = %err, user_id, page_id, "todo: commit sync");
    })?;
    // Stale splices stay in `applied.rejects`. The snapshot lets this socket rebase them.
    for event in applied.events {
        state.hub.publish(page_id, event);
    }
    drain(rx);
    drop(_guard);
    send_snapshot(socket, &todos).await
}

async fn handle_op(
    socket: &mut WebSocket,
    state: &TodoState,
    user_id: i64,
    page_id: i64,
    ops: &[ClientOp],
) -> Result<(), ()> {
    match apply_and_publish(state, user_id, page_id, ops).await {
        Ok(rejects) => {
            for reject in &rejects {
                send_reject(socket, reject).await?;
            }
            Ok(())
        }
        Err(()) => send_error(socket, "Could not save changes.").await,
    }
}

async fn apply_and_publish(
    state: &TodoState,
    user_id: i64,
    page_id: i64,
    ops: &[ClientOp],
) -> Result<Vec<sync::SpliceReject>, ()> {
    let _guard = state.hub.lock_page(page_id).await;
    let txn = state.db.begin().await.map_err(|err| {
        tracing::error!(error = %err, user_id, page_id, "todo: begin op");
    })?;
    let applied = sync::apply_ops(&txn, user_id, page_id, ops)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, user_id, page_id, "todo: apply op");
        })?;
    txn.commit().await.map_err(|err| {
        tracing::error!(error = %err, user_id, page_id, "todo: commit op");
    })?;
    for event in applied.events {
        state.hub.publish(page_id, event);
    }
    Ok(applied.rejects)
}

async fn resnapshot(
    socket: &mut WebSocket,
    state: &TodoState,
    user_id: i64,
    page_id: i64,
    rx: &mut broadcast::Receiver<ServerEvent>,
) -> Result<(), ()> {
    let _guard = state.hub.lock_page(page_id).await;
    let todos = sync::load_snapshot(&state.db, user_id, page_id)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, user_id, page_id, "todo: resnapshot");
        })?;
    drain(rx);
    drop(_guard);
    send_snapshot(socket, &todos).await
}

fn drain(rx: &mut broadcast::Receiver<ServerEvent>) {
    loop {
        match rx.try_recv() {
            Ok(_) => {}
            Err(broadcast::error::TryRecvError::Empty)
            | Err(broadcast::error::TryRecvError::Closed) => break,
            Err(broadcast::error::TryRecvError::Lagged(_)) => {}
        }
    }
}

async fn send_snapshot(socket: &mut WebSocket, todos: &[sync::TodoJson]) -> Result<(), ()> {
    #[derive(Serialize)]
    struct Snapshot<'a> {
        #[serde(rename = "type")]
        kind: &'static str,
        todos: &'a [sync::TodoJson],
    }
    send_json(
        socket,
        &Snapshot {
            kind: "snapshot",
            todos,
        },
    )
    .await
}

async fn send_reject(socket: &mut WebSocket, reject: &sync::SpliceReject) -> Result<(), ()> {
    #[derive(Serialize)]
    struct Msg<'a> {
        #[serde(rename = "type")]
        kind: &'static str,
        op_id: &'a str,
        todo: &'a sync::TodoJson,
    }
    send_json(
        socket,
        &Msg {
            kind: "reject",
            op_id: &reject.op_id,
            todo: &reject.todo,
        },
    )
    .await
}

async fn send_error(socket: &mut WebSocket, message: &str) -> Result<(), ()> {
    #[derive(Serialize)]
    struct ErrorMsg<'a> {
        #[serde(rename = "type")]
        kind: &'static str,
        message: &'a str,
    }
    send_json(
        socket,
        &ErrorMsg {
            kind: "error",
            message,
        },
    )
    .await
}

async fn send_json(socket: &mut WebSocket, value: &impl Serialize) -> Result<(), ()> {
    let text = serde_json::to_string(value).map_err(|err| {
        tracing::error!(error = %err, "todo: encode ws message");
    })?;
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|err| {
            tracing::debug!(error = %err, "todo: ws send closed");
        })
}
