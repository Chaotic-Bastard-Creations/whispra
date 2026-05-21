use crate::bridge::state::{AppState, Event};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

#[derive(Deserialize)]
pub struct WsAuth {
    token: Option<String>,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(auth): Query<WsAuth>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if auth.token.as_deref() != Some(state.token.as_ref()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "missing or invalid ?token= query parameter"})),
        )
            .into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut rx = state.events.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };
                    if sender
                        .send(Message::Text(axum::extract::ws::Utf8Bytes::from(json)))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    let lagged = serde_json::to_string(&Event::Lagged).unwrap_or_default();
                    if sender
                        .send(Message::Text(axum::extract::ws::Utf8Bytes::from(lagged)))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    });

    while receiver.next().await.is_some() {}

    send_task.abort();
}
