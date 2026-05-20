use axum::extract::ws::CloseFrame;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::protocol::crypto;
use crate::protocol::envelope::{self, SharedMailboxMap};

struct MailBoxGuard {
    token: String,
    map: SharedMailboxMap,
}

impl Drop for MailBoxGuard {
    fn drop(&mut self) {
        self.map.remove(&self.token);
    }
}

async fn websocket_handler(
    State(mailbox_map): State<SharedMailboxMap>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_failed_upgrade(|_| println!("Internal server error"))
        .on_upgrade(move |socket| handle_socket(socket, mailbox_map))
}

async fn handle_socket(socket: WebSocket, mailbox_map: SharedMailboxMap) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(32);

    let generated_token = envelope::gen_mailbox_token();
    mailbox_map.insert(generated_token.clone(), tx.clone());

    let _guard = MailBoxGuard {
        token: generated_token,
        map: mailbox_map.clone(),
    };

    let (mut socket_sender, mut socket_receiver) = socket.split();

    let mut outbound_task = tokio::spawn(async move {
        while let Some(forwarded_msg) = rx.recv().await {
            let is_close = matches!(forwarded_msg, Message::Close(_));
            if socket_sender.send(forwarded_msg).await.is_err() || is_close {
                break;
            }
        }
    });

    while let Some(msg_result) = socket_receiver.next().await {
        match msg_result {
            Ok(msg) => match msg {
                Message::Text(utf8_bytes) => match crypto::encrypt_message(&utf8_bytes) {
                    Ok(ciphertext_hex) => {
                        let response_msg =
                            Message::Text(axum::extract::ws::Utf8Bytes::from(ciphertext_hex));

                        if tx.send(response_msg).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                Message::Ping(payload) => {
                    if tx.send(Message::Pong(payload)).await.is_err() {
                        break;
                    }
                }
                Message::Close(frame) => {
                    let _ = tx.send(Message::Close(frame)).await;
                    break;
                }
                _ => {}
            },
            Err(_) => break,
        }
    }

    drop(tx);
    let _ = outbound_task.await;
}

async fn send_close_message(
    mut socket_sender: futures_util::stream::SplitSink<WebSocket, Message>,
    code: u16,
    reason: &str,
) {
    let _ = socket_sender
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

pub async fn start_server() -> anyhow::Result<()> {
    let global_mailbox_registry: SharedMailboxMap = Arc::new(DashMap::new());
    let app = Router::new()
        .route("/web_socket", get(websocket_handler))
        .with_state(global_mailbox_registry);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
