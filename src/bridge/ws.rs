use crate::bridge::state::{AppState, Event};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
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
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let query_ok = auth.token.as_deref() == Some(state.token.as_ref());
    let bearer_ok = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == format!("Bearer {}", state.token.as_ref()))
        .unwrap_or(false);

    if !query_ok && !bearer_ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "missing or invalid bearer token"})),
        )
            .into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut rx = state.events.subscribe();
    let (mut sender, mut receiver) = socket.split();
    let initial_status = Event::Status {
        connected: state.connected(),
    };

    let send_task = tokio::spawn(async move {
        if let Ok(json) = serde_json::to_string(&initial_status) {
            if sender
                .send(Message::Text(axum::extract::ws::Utf8Bytes::from(json)))
                .await
                .is_err()
            {
                return;
            }
        }

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
