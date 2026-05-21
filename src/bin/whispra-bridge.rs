use clap::Parser;
use whispra::bridge::conn::Connection;
use whispra::bridge::state::AppState;
use whispra::bridge::{epoch_loop, http};

const DEFAULT_UPSTREAM: &str = "199.91.221.109:3000";

#[derive(Parser)]
#[command(about = "Local HTTP/WebSocket bridge between the browser and a Whispra server")]
struct Args {
    #[arg(long, default_value = DEFAULT_UPSTREAM)]
    upstream: String,

    #[arg(long)]
    upstream_key: String,

    #[arg(long)]
    token: Option<String>,

    #[arg(long, default_value = "127.0.0.1:7000")]
    listen: String,

    #[arg(long, default_value = "eu-edge-01")]
    edge_name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let pk_bytes = hex::decode(&args.upstream_key)
        .map_err(|_| anyhow::anyhow!("--upstream-key is not valid hex"))?;
    if pk_bytes.len() != 32 {
        anyhow::bail!("--upstream-key must be exactly 32 bytes (64 hex chars)");
    }
    let mut server_pk = [0u8; 32];
    server_pk.copy_from_slice(&pk_bytes);

    println!("Connecting to upstream {}...", args.upstream);
    let conn = Connection::connect(args.upstream.clone(), server_pk).await?;
    println!("Noise NK handshake complete.");

    let token = args
        .token
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| hex::encode(rand::random::<[u8; 16]>()));
    println!("bridge auth token: {token}");

    let state = AppState::new(conn, token, args.edge_name);

    tokio::spawn(epoch_loop::run(state.clone()));

    let router = http::router(state);
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    println!("whispra-bridge listening on http://{}", args.listen);
    axum::serve(listener, router).await?;

    Ok(())
}
