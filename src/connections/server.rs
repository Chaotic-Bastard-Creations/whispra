use axum::extract::ws::CloseFrame;
use axum::Router;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};

use crate::protocol::crypto;

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_failed_upgrade(|_| println!("Internal server error"))
        .on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(utf8_bytes) => match crypto::encrypt_message(&utf8_bytes) {
                    Ok(ciphertext_hex) => {
                        let response_msg = Message::Text(ciphertext_hex.into());

                        if let Err(_) = socket.send(response_msg).await {
                            send_close_message(socket, 1011, "Internal server error").await;
                            break;
                        }
                    }
                    Err(_) => {
                        send_close_message(socket, 1011, "Internal cryptographic error").await;
                        break;
                    }
                },
                _ => {}
            }
        } else {
            let _ = msg.err().unwrap();
            send_close_message(socket, 1011, "Internal server error").await;
            break;
        }
    }
}

async fn send_close_message(mut socket: WebSocket, code: u16, reason: &str) {
    _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: code,
            reason: reason.into(),
        })))
        .await;
}

pub async fn start_server() -> anyhow::Result<()> {
    let app = Router::new().route("/web_socket", get(websocket_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}
