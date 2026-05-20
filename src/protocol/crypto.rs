use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key,
};
use anyhow::{Context, Result};

pub fn encrypt_message(message: &str) -> Result<()> {
    let _key = Aes256Gcm::generate_key(OsRng);

    let key_bytes: &[u8; 32] = &[42; 32];
    let key: &Key<Aes256Gcm> = key_bytes.into();

    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, message.as_bytes().as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;

    let ciphertext_hex: String = ciphertext.iter().map(|b| format!("{:02x}", b)).collect();

    let plaintext_str = std::str::from_utf8(&plaintext)
        .context("Failed to parse decrypted bytes as valid UTF-8")?;

    println!("ENCRYPTED: {}", ciphertext_hex);
    println!("DECRYPTED: {}", plaintext_str);

    Ok(())
}
