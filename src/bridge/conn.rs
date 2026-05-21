use crate::slot::SlotId;
use crate::transport::NoiseTransport;
use crate::wire::{OwnedResponse, OP_GET, OP_PUT};
use crate::{CELL_SIZE, SLOT_ID_LEN};
use anyhow::{anyhow, Result};
use tokio::net::TcpStream;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoSample {
    pub bytes_up: u64,
    pub bytes_down: u64,
}

pub struct Connection {
    addr: String,
    server_pk: [u8; 32],
    stream: TcpStream,
    noise: NoiseTransport,
}

impl Connection {
    pub async fn connect(addr: impl Into<String>, server_pk: [u8; 32]) -> Result<Self> {
        let addr = addr.into();
        let mut stream = TcpStream::connect(&addr).await?;
        let noise = NoiseTransport::initiate(&mut stream, &server_pk).await?;
        Ok(Self {
            addr,
            server_pk,
            stream,
            noise,
        })
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        let mut stream = TcpStream::connect(&self.addr).await?;
        let noise = NoiseTransport::initiate(&mut stream, &self.server_pk).await?;
        self.stream = stream;
        self.noise = noise;
        Ok(())
    }

    pub fn server_public_key_hex(&self) -> String {
        hex::encode(self.server_pk)
    }

    pub fn upstream_addr(&self) -> &str {
        &self.addr
    }

    pub async fn put(&mut self, slot: SlotId, cell: Box<[u8; CELL_SIZE]>) -> Result<IoSample> {
        let mut buf = Vec::with_capacity(1 + SLOT_ID_LEN + CELL_SIZE);
        buf.push(OP_PUT);
        buf.extend_from_slice(slot.as_bytes());
        buf.extend_from_slice(cell.as_ref());
        let bytes_up = self.noise.send_counted(&mut self.stream, &buf).await? as u64;

        let (resp_bytes, bytes_down) = self.noise.recv_counted(&mut self.stream).await?;
        match OwnedResponse::parse(&resp_bytes)? {
            OwnedResponse::Ack => Ok(IoSample {
                bytes_up,
                bytes_down: bytes_down as u64,
            }),
            OwnedResponse::Err(code) => Err(anyhow!("server PUT error: {:#04x}", code)),
            _ => Err(anyhow!("unexpected response to PUT")),
        }
    }

    pub async fn get(&mut self, slot: SlotId) -> Result<(Option<Box<[u8; CELL_SIZE]>>, IoSample)> {
        let mut buf = Vec::with_capacity(1 + SLOT_ID_LEN);
        buf.push(OP_GET);
        buf.extend_from_slice(slot.as_bytes());
        let bytes_up = self.noise.send_counted(&mut self.stream, &buf).await? as u64;

        let (resp_bytes, bytes_down) = self.noise.recv_counted(&mut self.stream).await?;
        let sample = IoSample {
            bytes_up,
            bytes_down: bytes_down as u64,
        };
        match OwnedResponse::parse(&resp_bytes)? {
            OwnedResponse::Hit(cell) => Ok((Some(cell), sample)),
            OwnedResponse::Miss => Ok((None, sample)),
            OwnedResponse::Err(code) => Err(anyhow!("server GET error: {:#04x}", code)),
            _ => Err(anyhow!("unexpected response to GET")),
        }
    }
}
