use crate::slot::SlotId;
use crate::transport::NoiseTransport;
use crate::wire::{OwnedResponse, OP_GET, OP_PUT};
use crate::{CELL_SIZE, SLOT_ID_LEN};
use anyhow::{anyhow, Result};
use tokio::net::TcpStream;

pub struct Connection {
    stream: TcpStream,
    noise: NoiseTransport,
}

impl Connection {
    pub async fn connect(addr: &str, server_pk: &[u8; 32]) -> Result<Self> {
        let mut stream = TcpStream::connect(addr).await?;
        let noise = NoiseTransport::initiate(&mut stream, server_pk).await?;
        Ok(Self { stream, noise })
    }

    pub async fn put(&mut self, slot: SlotId, cell: Box<[u8; CELL_SIZE]>) -> Result<()> {
        let mut buf = Vec::with_capacity(1 + SLOT_ID_LEN + CELL_SIZE);
        buf.push(OP_PUT);
        buf.extend_from_slice(slot.as_bytes());
        buf.extend_from_slice(cell.as_ref());
        self.noise.send(&mut self.stream, &buf).await?;

        let resp_bytes = self.noise.recv(&mut self.stream).await?;
        match OwnedResponse::parse(&resp_bytes)? {
            OwnedResponse::Ack => Ok(()),
            OwnedResponse::Err(code) => Err(anyhow!("server PUT error: {:#04x}", code)),
            _ => Err(anyhow!("unexpected response to PUT")),
        }
    }

    pub async fn get(&mut self, slot: SlotId) -> Result<Option<Box<[u8; CELL_SIZE]>>> {
        let mut buf = Vec::with_capacity(1 + SLOT_ID_LEN);
        buf.push(OP_GET);
        buf.extend_from_slice(slot.as_bytes());
        self.noise.send(&mut self.stream, &buf).await?;

        let resp_bytes = self.noise.recv(&mut self.stream).await?;
        match OwnedResponse::parse(&resp_bytes)? {
            OwnedResponse::Hit(cell) => Ok(Some(cell)),
            OwnedResponse::Miss => Ok(None),
            OwnedResponse::Err(code) => Err(anyhow!("server GET error: {:#04x}", code)),
            _ => Err(anyhow!("unexpected response to GET")),
        }
    }
}
