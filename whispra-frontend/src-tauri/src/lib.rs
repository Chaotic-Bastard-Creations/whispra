use serde::Serialize;
use std::net::TcpListener as StdTcpListener;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::Emitter;
use whispra::bridge::conn::Connection;
use whispra::bridge::state::AppState;
use whispra::bridge::{epoch_loop, http};

const DEFAULT_EDGE_NAME: &str = "eu-edge-01";
const DEFAULT_UPSTREAM: &str = "199.91.221.109:3000";
const DEFAULT_UPSTREAM_KEY: &str =
    "b8137d2822342c9ebc54db75f03e271c2f0840b4a7ab04d9af17b96f0845ed45";

static BRIDGE_CONFIG: OnceLock<BridgeConfig> = OnceLock::new();

#[derive(Clone, Serialize)]
struct BridgeConfig {
    url: String,
    token: String,
}

#[tauri::command]
fn embedded_bridge_config() -> Result<BridgeConfig, String> {
    BRIDGE_CONFIG
        .get()
        .cloned()
        .ok_or_else(|| "embedded bridge has not started yet".to_string())
}

fn decode_server_key() -> anyhow::Result<[u8; 32]> {
    let bytes = hex::decode(DEFAULT_UPSTREAM_KEY)
        .map_err(|_| anyhow::anyhow!("embedded upstream key is not valid hex"))?;
    if bytes.len() != 32 {
        anyhow::bail!("embedded upstream key must be exactly 32 bytes");
    }

    let mut server_pk = [0u8; 32];
    server_pk.copy_from_slice(&bytes);
    Ok(server_pk)
}

async fn run_embedded_bridge(
    token: String,
    listener: StdTcpListener,
    app_handle: tauri::AppHandle,
) -> anyhow::Result<()> {
    let server_pk = decode_server_key()?;

    let conn = loop {
        match Connection::connect(DEFAULT_UPSTREAM.to_string(), server_pk).await {
            Ok(conn) => break conn,
            Err(e) => {
                eprintln!("embedded bridge upstream connection failed: {e}");
                let _ = app_handle.emit("embedded-bridge-error", e.to_string());
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };

    let state = AppState::new(conn, token, DEFAULT_EDGE_NAME.to_string());

    tokio::spawn(epoch_loop::run(state.clone()));

    let router = http::router(state);
    let listener = tokio::net::TcpListener::from_std(listener)?;
    axum::serve(listener, router).await?;
    Ok(())
}

fn start_embedded_bridge(app: &tauri::App) -> anyhow::Result<()> {
    let listener = StdTcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let local_addr = listener.local_addr()?;

    let token = hex::encode(rand::random::<[u8; 16]>());
    let config = BridgeConfig {
        url: format!("http://{}", local_addr),
        token: token.clone(),
    };
    let _ = BRIDGE_CONFIG.set(config);
    let app_handle = app.handle().clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_embedded_bridge(token, listener, app_handle.clone()).await {
            eprintln!("embedded bridge stopped: {e}");
            let _ = app_handle.emit("embedded-bridge-error", e.to_string());
        }
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            start_embedded_bridge(app).map_err(|e| e.to_string())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![embedded_bridge_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
