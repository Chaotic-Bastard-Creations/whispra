pub const CELL_SIZE: usize = 4096;
pub const SLOT_ID_LEN: usize = 16;
pub const EPOCH_MS: u64 = 250;
pub const READ_BUCKET: usize = 32;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    InitiatorToResponder = 0,
    ResponderToInitiator = 1,
}

pub mod labels {
    pub const SLOT_KEY: &[u8] = b"whispra-slot-key-v1";
    pub const SLOT_RATCHET: &[u8] = b"whispra-slot-ratchet-v1";
    pub const SLOT_ID: &[u8] = b"whispra-slot-id-v1";
    pub const DECOY_KEY: &[u8] = b"whispra-decoy-key-v1";
    pub const CONTENT_KEY: &[u8] = b"whispra-content-key-v1";
}

#[path = "protocol/epoch.rs"]
pub mod epoch;
#[path = "protocol/cell.rs"]
pub mod cell;
#[path = "protocol/slot.rs"]
pub mod slot;
#[path = "protocol/wire.rs"]
pub mod wire;
#[path = "protocol/transport.rs"]
pub mod transport;

pub mod protocol {
    pub mod crypto;
    pub mod envelope;
    pub mod sealed_sender;
}

pub mod connections {
    pub mod identity;
    pub mod server;
}

pub mod bridge;
