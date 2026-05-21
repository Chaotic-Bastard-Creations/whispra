use crate::bridge::contacts::Role;
use crate::bridge::pairing;
use crate::bridge::state::{AppState, Event, OutgoingMessage};
use crate::bridge::ws::ws_handler;
use axum::{
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Deserialize)]
pub struct PairRequest {
    name: String,
    role: String,
    secret_hex: String,
}

#[derive(Deserialize)]
pub struct SendRequest {
    to: String,
    message: String,
}

#[derive(Serialize)]
struct ContactInfo {
    name: String,
    role: String,
    send_counter: u64,
    recv_counter: u64,
}

#[derive(Serialize)]
struct StatusResponse {
    connected: bool,
    contact_count: usize,
}

pub async fn bearer_auth(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let expected = format!("Bearer {}", state.token);
    let ok = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == expected)
        .unwrap_or(false);

    if ok {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "missing or invalid Authorization header"})),
        )
            .into_response()
    }
}

async fn pair(
    State(state): State<AppState>,
    Json(req): Json<PairRequest>,
) -> impl IntoResponse {
    let role = match req.role.as_str() {
        "initiator" => Role::Initiator,
        "responder" => Role::Responder,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "role must be 'initiator' or 'responder'"})),
            )
                .into_response();
        }
    };

    let contact = match pairing::from_secret(req.name.clone(), role, &req.secret_hex) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    state.contacts.lock().await.insert(contact);
    let _ = state.events.send(Event::ContactAdded { name: req.name });
    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

async fn send_message(
    State(state): State<AppState>,
    Json(req): Json<SendRequest>,
) -> impl IntoResponse {
    state.outgoing.lock().await.push_back(OutgoingMessage {
        to: req.to,
        payload: req.message.into_bytes(),
    });
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}

async fn contacts(State(state): State<AppState>) -> impl IntoResponse {
    let book = state.contacts.lock().await;
    let list: Vec<ContactInfo> = book
        .iter()
        .map(|c| ContactInfo {
            name: c.name.clone(),
            role: format!("{:?}", c.role).to_lowercase(),
            send_counter: c.send_counter,
            recv_counter: c.recv_counter,
        })
        .collect();
    Json(list)
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let count = state.contacts.lock().await.len();
    Json(StatusResponse {
        connected: true,
        contact_count: count,
    })
}

async fn quit() -> impl IntoResponse {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}

fn localhost_only() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            |origin: &HeaderValue, _| -> bool {
                let Ok(s) = std::str::from_utf8(origin.as_bytes()) else {
                    return false;
                };
                s == "http://localhost"
                    || s.starts_with("http://localhost:")
                    || s == "http://127.0.0.1"
                    || s.starts_with("http://127.0.0.1:")
            },
        ))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/pair", post(pair))
        .route("/send", post(send_message))
        .route("/contacts", get(contacts))
        .route("/status", get(status))
        .route("/quit", post(quit))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            bearer_auth,
        ));

    let ws = Router::new().route("/events", get(ws_handler));

    Router::new()
        .merge(api)
        .merge(ws)
        .with_state(state)
        .layer(localhost_only())
}
