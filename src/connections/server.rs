use aes_gcm::aead::rand_core::OsRng;
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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::protocol::crypto;
use crate::protocol::envelope::SharedMailboxMap;

const UNIFORM_PAYLOAD_SIZE: usize = 4096;

#[derive(Serialize, Deserialize)]
struct RoutingEnvelope {
    target_token: String,
    payload: String,
}

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
    let (mut socket_sender, mut socket_receiver) = socket.split();

    let client_public_raw = match socket_receiver.next().await {
        Some(Ok(Message::Text(hex_pk))) => match hex::decode(hex_pk.trim()) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut array = [0u8; 32];
                array.copy_from_slice(&bytes);
                PublicKey::from(array)
            }
            _ => return,
        },
        _ => return,
    };

    let server_secret = EphemeralSecret::random_from_rng(OsRng);
    let server_public = PublicKey::from(&server_secret);
    let shared_secret = server_secret.diffie_hellman(&client_public_raw);
    let session_key: [u8; 32] = *shared_secret.as_bytes();

    let server_pk_hex = hex::encode(server_public.as_bytes());
    if socket_sender
        .send(Message::Text(axum::extract::ws::Utf8Bytes::from(
            server_pk_hex,
        )))
        .await
        .is_err()
    {
        return;
    }

    let client_token = match socket_receiver.next().await {
        Some(Ok(Message::Text(encrypted_handshake))) => {
            match crypto::decrypt_message(&encrypted_handshake, &session_key) {
                Ok(decrypted_payload) => {
                    let token = decrypted_payload.trim_end_matches(' ').to_string();
                    if token.is_empty() {
                        return;
                    }
                    token
                }
                Err(_) => return,
            }
        }
        _ => return,
    };

    mailbox_map.insert(client_token.clone(), tx.clone());

    let _guard = MailBoxGuard {
        token: client_token,
        map: mailbox_map.clone(),
    };

    let outbound_task = tokio::spawn(async move {
        while let Some(forwarded_msg) = rx.recv().await {
            let is_close = matches!(forwarded_msg, Message::Close(_));
            let jitter_ms = rand::random_range(10..40);
            tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

            if socket_sender.send(forwarded_msg).await.is_err() || is_close {
                break;
            }
        }
    });

    while let Some(msg_result) = socket_receiver.next().await {
        match msg_result {
            Ok(msg) => match msg {
                Message::Text(utf8_bytes) => {
                    match crypto::decrypt_message(&utf8_bytes, &session_key) {
                        Ok(decrypted_payload) => {
                            let cleaned_json = decrypted_payload.trim_end_matches(' ');

                            if let Ok(envelope) =
                                serde_json::from_str::<RoutingEnvelope>(cleaned_json)
                            {
                                let padded_payload =
                                    apply_uniform_padding(&envelope.payload, UNIFORM_PAYLOAD_SIZE);

                                // Re-encrypt message payload using the session key!
                                if let Ok(ciphertext_hex) =
                                    crypto::encrypt_message(&padded_payload, &session_key)
                                {
                                    let outbound_msg = Message::Text(
                                        axum::extract::ws::Utf8Bytes::from(ciphertext_hex),
                                    );

                                    if let Some(recipient_tx) =
                                        mailbox_map.get(&envelope.target_token)
                                    {
                                        if recipient_tx.send(outbound_msg).await.is_err() {
                                            // Silent drop
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
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

fn apply_uniform_padding(input: &str, target_len: usize) -> String {
    let input_bytes = input.as_bytes();
    if input_bytes.len() >= target_len {
        return input.to_string();
    }

    let mut padded = input_bytes.to_vec();
    padded.resize(target_len, 0x20);

    String::from_utf8(padded).unwrap_or_else(|_| input.to_string())
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
