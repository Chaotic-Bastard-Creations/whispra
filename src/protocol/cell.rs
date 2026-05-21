use crate::CELL_SIZE;
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use thiserror::Error;

const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const PLAINTEXT_CELL_LEN: usize = CELL_SIZE - NONCE_LEN - TAG_LEN;
const COUNTER_LEN: usize = 8;
const LEN_FIELD_LEN: usize = 2;
const HEADER_LEN: usize = COUNTER_LEN + LEN_FIELD_LEN;

pub const MAX_PAYLOAD_LEN: usize = PLAINTEXT_CELL_LEN - HEADER_LEN;

#[derive(Debug, Error)]
pub enum CellError {
    #[error("payload too large for one cell ({0} > {})", MAX_PAYLOAD_LEN)]
    PayloadTooLarge(usize),
    #[error("AEAD operation failed")]
    Aead,
    #[error("declared payload length exceeds cell")]
    BadPayloadLen,
}

pub fn seal(
    key: &[u8; 32],
    counter: u64,
    payload: &[u8],
) -> Result<Box<[u8; CELL_SIZE]>, CellError> {
    if payload.len() > MAX_PAYLOAD_LEN {
        return Err(CellError::PayloadTooLarge(payload.len()));
    }

    let mut plaintext = vec![0u8; PLAINTEXT_CELL_LEN];
    plaintext[..COUNTER_LEN].copy_from_slice(&counter.to_be_bytes());
    plaintext[COUNTER_LEN..HEADER_LEN].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    plaintext[HEADER_LEN..HEADER_LEN + payload.len()].copy_from_slice(payload);
    plaintext[HEADER_LEN + payload.len()..].iter_mut().for_each(|b| *b = rand::random());

    let nonce_bytes: [u8; NONCE_LEN] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let ct = cipher
        .encrypt(
            nonce,
            Payload {
                msg: &plaintext,
                aad: &[],
            },
        )
        .map_err(|_| CellError::Aead)?;

    debug_assert_eq!(ct.len(), PLAINTEXT_CELL_LEN + TAG_LEN);

    let mut wire = Box::new([0u8; CELL_SIZE]);
    wire[..NONCE_LEN].copy_from_slice(&nonce_bytes);
    wire[NONCE_LEN..].copy_from_slice(&ct);
    Ok(wire)
}

#[derive(Debug)]
pub struct OpenedCell {
    pub counter: u64,
    pub payload: Vec<u8>,
}

pub fn open(key: &[u8; 32], wire: &[u8; CELL_SIZE]) -> Result<OpenedCell, CellError> {
    let nonce = Nonce::from_slice(&wire[..NONCE_LEN]);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: &wire[NONCE_LEN..],
                aad: &[],
            },
        )
        .map_err(|_| CellError::Aead)?;

    debug_assert_eq!(plaintext.len(), PLAINTEXT_CELL_LEN);

    let mut counter_bytes = [0u8; COUNTER_LEN];
    counter_bytes.copy_from_slice(&plaintext[..COUNTER_LEN]);
    let counter = u64::from_be_bytes(counter_bytes);

    let mut len_bytes = [0u8; LEN_FIELD_LEN];
    len_bytes.copy_from_slice(&plaintext[COUNTER_LEN..HEADER_LEN]);
    let payload_len = u16::from_be_bytes(len_bytes) as usize;

    if payload_len > MAX_PAYLOAD_LEN {
        return Err(CellError::BadPayloadLen);
    }

    let payload = plaintext[HEADER_LEN..HEADER_LEN + payload_len].to_vec();
    Ok(OpenedCell { counter, payload })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_short() {
        let key = [9u8; 32];
        let sealed = seal(&key, 42, b"hello").unwrap();
        let opened = open(&key, &sealed).unwrap();
        assert_eq!(opened.counter, 42);
        assert_eq!(opened.payload, b"hello");
    }

    #[test]
    fn roundtrip_empty() {
        let key = [9u8; 32];
        let sealed = seal(&key, 0, b"").unwrap();
        let opened = open(&key, &sealed).unwrap();
        assert_eq!(opened.payload, Vec::<u8>::new());
    }

    #[test]
    fn roundtrip_max() {
        let key = [9u8; 32];
        let payload = vec![0x77u8; MAX_PAYLOAD_LEN];
        let sealed = seal(&key, 1, &payload).unwrap();
        let opened = open(&key, &sealed).unwrap();
        assert_eq!(opened.payload, payload);
    }

    #[test]
    fn oversized_payload_rejected() {
        let key = [9u8; 32];
        let payload = vec![0u8; MAX_PAYLOAD_LEN + 1];
        assert!(matches!(
            seal(&key, 0, &payload),
            Err(CellError::PayloadTooLarge(_))
        ));
    }

    #[test]
    fn wrong_key_fails() {
        let sealed = seal(&[1u8; 32], 0, b"secret").unwrap();
        assert!(open(&[2u8; 32], &sealed).is_err());
    }

    #[test]
    fn ciphertext_is_full_cell_size() {
        let sealed = seal(&[0u8; 32], 0, b"x").unwrap();
        assert_eq!(sealed.len(), CELL_SIZE);
    }

    #[test]
    fn two_cells_with_same_key_have_different_ciphertexts() {
        let key = [0u8; 32];
        let a = seal(&key, 0, b"same").unwrap();
        let b = seal(&key, 0, b"same").unwrap();
        assert_ne!(&a[..], &b[..]);
    }
}
