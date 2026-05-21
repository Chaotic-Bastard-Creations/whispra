//! Noise NK transport.
//!
//! Noise pattern:
//!
//!     Noise_NK_25519_ChaChaPoly_SHA256
//!     <- s
//!     ...
//!     -> e, es
//!     <- e, ee
//!
//! The server has a static X25519 key pinned by the client. The
//! client has no static key — it is anonymous to the server at the
//! transport layer. Both messages are followed by transport-mode
//! encrypted frames in both directions.
//!
//! Why NK and not XK or IK?
//!
//! - XK would have the server learn the client's static key. We
//!   don't want any persistent client identity at the transport
//!   layer; identity lives entirely in pair keys, which the server
//!   never sees.
//! - NN (both anonymous) gives no authentication of the server,
//!   which means MITM is trivial.
//! - NK is the sweet spot: server identity is pinned, client is
//!   anonymous, mutual forward secrecy via ephemerals.
//!
//! Each transport-mode frame on the wire is length-prefixed:
//!
//!     [ length: u32 BE ][ noise-encrypted payload ]
//!
//! The length is the encrypted payload length (which is the plaintext
//! length + 16 bytes for the Poly1305 tag).

use anyhow::{anyhow, Context, Result};
use snow::{Builder, HandshakeState, TransportState};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const NOISE_PARAMS: &str = "Noise_NK_25519_ChaChaPoly_SHA256";

pub const MAX_NOISE_FRAME: usize = 65535;

pub struct NoiseTransport {
    state: TransportState,
}

impl NoiseTransport {
    pub async fn accept(stream: &mut TcpStream, server_static_secret: &[u8; 32]) -> Result<Self> {
        let builder = Builder::new(NOISE_PARAMS.parse()?);
        let mut hs: HandshakeState = builder
            .local_private_key(server_static_secret)
            .build_responder()
            .context("building Noise responder")?;

        let mut buf = [0u8; MAX_NOISE_FRAME];
        let msg1 = read_frame(stream, &mut buf).await?;
        let mut payload = [0u8; MAX_NOISE_FRAME];
        hs.read_message(msg1, &mut payload)
            .context("reading Noise message 1")?;

        let mut out = [0u8; MAX_NOISE_FRAME];
        let n = hs
            .write_message(&[], &mut out)
            .context("writing Noise message 2")?;
        write_frame(stream, &out[..n]).await?;

        let state = hs
            .into_transport_mode()
            .context("entering Noise transport mode")?;
        Ok(Self { state })
    }

    pub async fn initiate(stream: &mut TcpStream, server_static_public: &[u8; 32]) -> Result<Self> {
        let builder = Builder::new(NOISE_PARAMS.parse()?);
        let mut hs: HandshakeState = builder
            .remote_public_key(server_static_public)
            .build_initiator()
            .context("building Noise initiator")?;

        let mut out = [0u8; MAX_NOISE_FRAME];
        let n = hs
            .write_message(&[], &mut out)
            .context("writing Noise message 1")?;
        write_frame(stream, &out[..n]).await?;

        let mut buf = [0u8; MAX_NOISE_FRAME];
        let msg2 = read_frame(stream, &mut buf).await?;
        let mut payload = [0u8; MAX_NOISE_FRAME];
        hs.read_message(msg2, &mut payload)
            .context("reading Noise message 2")?;

        let state = hs
            .into_transport_mode()
            .context("entering Noise transport mode")?;
        Ok(Self { state })
    }

    pub async fn send(&mut self, stream: &mut TcpStream, plaintext: &[u8]) -> Result<()> {
        if plaintext.len() + 16 > MAX_NOISE_FRAME {
            return Err(anyhow!("plaintext too large for one Noise frame"));
        }
        let mut buf = [0u8; MAX_NOISE_FRAME];
        let n = self
            .state
            .write_message(plaintext, &mut buf)
            .context("Noise transport write")?;
        write_frame(stream, &buf[..n]).await
    }

    pub async fn recv(&mut self, stream: &mut TcpStream) -> Result<Vec<u8>> {
        let mut buf = [0u8; MAX_NOISE_FRAME];
        let ct = read_frame(stream, &mut buf).await?;
        let mut pt = vec![0u8; ct.len()];
        let n = self
            .state
            .read_message(ct, &mut pt)
            .context("Noise transport read")?;
        pt.truncate(n);
        Ok(pt)
    }
}

async fn read_frame<'a>(
    stream: &mut TcpStream,
    buf: &'a mut [u8; MAX_NOISE_FRAME],
) -> Result<&'a [u8]> {
    let mut len_bytes = [0u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .await
        .context("reading frame length")?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > MAX_NOISE_FRAME {
        return Err(anyhow!("incoming frame length {} exceeds max", len));
    }
    stream
        .read_exact(&mut buf[..len])
        .await
        .context("reading frame body")?;
    Ok(&buf[..len])
}

async fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<()> {
    let len = payload.len() as u32;
    stream
        .write_all(&len.to_be_bytes())
        .await
        .context("writing frame length")?;
    stream
        .write_all(payload)
        .await
        .context("writing frame body")?;
    Ok(())
}
