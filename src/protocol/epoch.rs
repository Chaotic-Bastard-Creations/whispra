use crate::EPOCH_MS;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

impl Epoch {
    pub fn from_unix_millis(ms: u64) -> Self {
        Epoch(ms / EPOCH_MS)
    }

    pub fn now() -> Self {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self::from_unix_millis(ms)
    }

    pub fn to_be_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    pub fn offset(self, delta: i64) -> Self {
        Epoch(self.0.wrapping_add(delta as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_from_known_timestamp() {
        assert_eq!(Epoch::from_unix_millis(1000).0, 4);
        assert_eq!(Epoch::from_unix_millis(1249).0, 4);
        assert_eq!(Epoch::from_unix_millis(1250).0, 5);
    }

    #[test]
    fn epoch_offset_roundtrip() {
        let e = Epoch(1_000_000);
        assert_eq!(e.offset(5).offset(-5), e);
    }
}
