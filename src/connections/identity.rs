use anyhow::{Context, Result};
use std::path::Path;
use x25519_dalek::{PublicKey, StaticSecret};

pub struct ServerIdentity {
    secret: [u8; 32],
    public_hex: String,
}

impl ServerIdentity {
    pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let secret = if path.exists() {
            let bytes = std::fs::read(path)
                .with_context(|| format!("reading key file {}", path.display()))?;
            if bytes.len() != 32 {
                anyhow::bail!(
                    "key file {} is corrupt: expected 32 bytes, got {}",
                    path.display(),
                    bytes.len()
                );
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        } else {
            let secret: [u8; 32] = rand::random();
            write_secret_file(path, &secret)
                .with_context(|| format!("writing key file {}", path.display()))?;
            secret
        };

        let static_secret = StaticSecret::from(secret);
        let public = PublicKey::from(&static_secret);
        let public_hex = hex::encode(public.as_bytes());

        Ok(Self { secret, public_hex })
    }

    pub fn secret_bytes(&self) -> &[u8; 32] {
        &self.secret
    }

    pub fn public_hex(&self) -> &str {
        &self.public_hex
    }
}

#[cfg(unix)]
fn write_secret_file(path: &Path, secret: &[u8; 32]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(secret)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_secret_file(path: &Path, secret: &[u8; 32]) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    file.write_all(secret)?;
    Ok(())
}
