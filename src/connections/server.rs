use crate::connections::identity::ServerIdentity;
use crate::epoch::Epoch;
use crate::transport::NoiseTransport;
use crate::wire::{ERR_MALFORMED, RSP_ACK, RSP_ERR, RSP_HIT, RSP_MISS, RSP_TIME};
use crate::{CELL_SIZE, SLOT_ID_LEN};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

type Board = Arc<DashMap<[u8; SLOT_ID_LEN], Box<[u8; CELL_SIZE]>>>;

pub async fn start_server() -> Result<()> {
    start_server_on("0.0.0.0:3000").await
}

pub async fn start_server_on(listen: &str) -> Result<()> {
    let key_path = std::env::var("WHISPRA_KEY").unwrap_or_else(|_| "whispra_server_key".into());
    let identity = ServerIdentity::load_or_generate(&key_path)?;
    println!("public key (pin this in clients): {}", identity.public_hex());

    let board: Board = Arc::new(DashMap::new());
    let listener = TcpListener::bind(listen).await?;
    println!("whispra-server listening on {listen}");

    loop {
        let (stream, addr) = listener.accept().await?;
        let board = board.clone();
        let secret = *identity.secret_bytes();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, board, secret).await {
                eprintln!("connection {addr} closed: {e}");
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream, board: Board, secret: [u8; 32]) -> Result<()> {
    use crate::wire::Request;

    let mut noise = NoiseTransport::accept(&mut stream, &secret).await?;

    loop {
        let msg = noise.recv(&mut stream).await?;

        let response = match Request::parse(&msg) {
            Err(_) => vec![RSP_ERR, ERR_MALFORMED],

            Ok(Request::Put { slot, cell }) => {
                board.insert(slot.0, cell);
                vec![RSP_ACK]
            }

            Ok(Request::Get { slot }) => match board.get(&slot.0) {
                Some(entry) => {
                    let mut out = Vec::with_capacity(1 + CELL_SIZE);
                    out.push(RSP_HIT);
                    out.extend_from_slice(entry.value().as_ref());
                    out
                }
                None => vec![RSP_MISS],
            },

            Ok(Request::Time) => {
                let mut out = Vec::with_capacity(9);
                out.push(RSP_TIME);
                out.extend_from_slice(&Epoch::now().to_be_bytes());
                out
            }
        };

        noise.send(&mut stream, &response).await?;
    }
}
