use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use anyhow::Context;

pub fn encrypt_message(message: &str, key_bytes: &[u8; 32]) -> anyhow::Result<String> {
    let key: &Key<Aes256Gcm> = key_bytes.into();
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, message.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

    let mut combined_payload = nonce.to_vec();
    combined_payload.extend_from_slice(&ciphertext);

    let ciphertext_hex: String = combined_payload
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    Ok(ciphertext_hex)
}

pub fn decrypt_message(message_hex: &str, key_bytes: &[u8; 32]) -> anyhow::Result<String> {
    let key: &Key<Aes256Gcm> = key_bytes.into();
    let cipher = Aes256Gcm::new(key);

    let raw_payload = hex_decode(message_hex).context("Failed to decode hexadecimal payload")?;

    if raw_payload.len() < 12 {
        return Err(anyhow::anyhow!(
            "Payload too short to contain a valid cryptographic nonce"
        ));
    }

    let (nonce_bytes, ciphertext_bytes) = raw_payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext_bytes)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

    let plaintext_str = std::str::from_utf8(&plaintext_bytes)
        .context("Failed to parse decrypted bytes as valid UTF-8 string format")?;

    Ok(plaintext_str.to_string())
}

fn hex_decode(hex_str: &str) -> anyhow::Result<Vec<u8>> {
    if hex_str.len() % 2 != 0 {
        return Err(anyhow::anyhow!("Hex string must have an even length"));
    }
    (0..hex_str.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex_str[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex character sequence: {:?}", e))
        })
        .collect()
}
