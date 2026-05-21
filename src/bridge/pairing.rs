use crate::bridge::contacts::{Contact, Role};
use crate::labels;
use crate::slot::SlotKey;
use hkdf::Hkdf;
use sha2::Sha256;

/// Derive a Contact from a 32-byte shared secret (hex-encoded).
///
/// Key schedule:
///   pair_secret (32 raw bytes, the user-supplied hex) →
///     slot_key   = HKDF(pair_secret, "whispra-slot-key-v1")     [via SlotKey::from_pair_key]
///     content_key = HKDF(pair_secret, "whispra-content-key-v1")
pub fn from_secret(name: String, role: Role, secret_hex: &str) -> anyhow::Result<Contact> {
    let raw = hex::decode(secret_hex)
        .map_err(|_| anyhow::anyhow!("secret_hex is not valid hex"))?;
    if raw.len() != 32 {
        anyhow::bail!("secret must be exactly 32 bytes (64 hex chars), got {}", raw.len());
    }
    let mut pair_secret = [0u8; 32];
    pair_secret.copy_from_slice(&raw);

    let slot_key = SlotKey::from_pair_key(&pair_secret);

    let mut content_key = [0u8; 32];
    Hkdf::<Sha256>::new(None, &pair_secret)
        .expand(labels::CONTENT_KEY, &mut content_key)
        .map_err(|_| anyhow::anyhow!("HKDF expand failed for content key"))?;

    Ok(Contact {
        name,
        role,
        slot_key,
        content_key,
        send_counter: 0,
        recv_counter: 0,
    })
}
