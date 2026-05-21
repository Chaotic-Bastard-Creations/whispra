use crate::epoch::Epoch;
use crate::{labels, Direction, SLOT_ID_LEN};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SlotKey([u8; 32]);

impl SlotKey {
    pub fn from_pair_key(pair_key: &[u8; 32]) -> Self {
        let mut out = [0u8; 32];
        Hkdf::<Sha256>::new(None, pair_key)
            .expand(labels::SLOT_KEY, &mut out)
            .expect("HKDF expand of 32 bytes never fails");
        SlotKey(out)
    }

    pub fn ratchet(&mut self) {
        let mut next = [0u8; 32];
        Hkdf::<Sha256>::new(None, &self.0)
            .expand(labels::SLOT_RATCHET, &mut next)
            .expect("HKDF expand of 32 bytes never fails");
        self.0.zeroize();
        self.0 = next;
    }

    pub fn slot(&self, dir: Direction, epoch: Epoch) -> SlotId {
        let mut info = [0u8; 1 + 8];
        info[0] = dir as u8;
        info[1..].copy_from_slice(&epoch.to_be_bytes());

        let mut out = [0u8; SLOT_ID_LEN];
        Hkdf::<Sha256>::new(Some(labels::SLOT_ID), &self.0)
            .expand(&info, &mut out)
            .expect("HKDF expand of 16 bytes never fails");
        SlotId(out)
    }

    #[cfg(test)]
    fn from_raw(bytes: [u8; 32]) -> Self {
        SlotKey(bytes)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlotId(pub [u8; SLOT_ID_LEN]);

impl SlotId {
    pub fn as_bytes(&self) -> &[u8; SLOT_ID_LEN] {
        &self.0
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DecoyKey([u8; 32]);

impl DecoyKey {
    pub fn random() -> Self {
        let bytes: [u8; 32] = rand::random();
        DecoyKey(bytes)
    }

    pub fn slot(&self, epoch: Epoch, index: u32) -> SlotId {
        let mut info = [0u8; 8 + 4];
        info[..8].copy_from_slice(&epoch.to_be_bytes());
        info[8..].copy_from_slice(&index.to_be_bytes());

        let mut out = [0u8; SLOT_ID_LEN];
        Hkdf::<Sha256>::new(Some(labels::DECOY_KEY), &self.0)
            .expand(&info, &mut out)
            .expect("HKDF expand of 16 bytes never fails");
        SlotId(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_is_deterministic() {
        let key = SlotKey::from_raw([7u8; 32]);
        let e = Epoch(1234);
        let s1 = key.slot(Direction::InitiatorToResponder, e);
        let s2 = key.slot(Direction::InitiatorToResponder, e);
        assert_eq!(s1, s2);
    }

    #[test]
    fn slot_differs_by_direction() {
        let key = SlotKey::from_raw([7u8; 32]);
        let e = Epoch(1234);
        assert_ne!(
            key.slot(Direction::InitiatorToResponder, e),
            key.slot(Direction::ResponderToInitiator, e),
        );
    }

    #[test]
    fn slot_differs_by_epoch() {
        let key = SlotKey::from_raw([7u8; 32]);
        assert_ne!(
            key.slot(Direction::InitiatorToResponder, Epoch(1234)),
            key.slot(Direction::InitiatorToResponder, Epoch(1235)),
        );
    }

    #[test]
    fn ratchet_changes_future_slots() {
        let mut key = SlotKey::from_raw([7u8; 32]);
        let before = key.slot(Direction::InitiatorToResponder, Epoch(1234));
        key.ratchet();
        let after = key.slot(Direction::InitiatorToResponder, Epoch(1234));
        assert_ne!(before, after);
    }

    #[test]
    fn pair_keys_diverge() {
        let k1 = SlotKey::from_pair_key(&[0u8; 32]);
        let k2 = SlotKey::from_pair_key(&[1u8; 32]);
        assert_ne!(
            k1.slot(Direction::InitiatorToResponder, Epoch(0)),
            k2.slot(Direction::InitiatorToResponder, Epoch(0)),
        );
    }

    #[test]
    fn decoys_are_distinct_from_real_slots() {
        let real = SlotKey::from_raw([7u8; 32]);
        let decoy = DecoyKey([7u8; 32]);
        assert_ne!(
            real.slot(Direction::InitiatorToResponder, Epoch(0)).0,
            decoy.slot(Epoch(0), 0).0,
        );
    }
}
