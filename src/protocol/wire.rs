use crate::slot::SlotId;
use crate::{CELL_SIZE, SLOT_ID_LEN};
use thiserror::Error;

pub const OP_PUT: u8 = 0x01;
pub const OP_GET: u8 = 0x02;
pub const OP_TIME: u8 = 0x03;

pub const RSP_ACK: u8 = 0x81;
pub const RSP_HIT: u8 = 0x82;
pub const RSP_MISS: u8 = 0x83;
pub const RSP_TIME: u8 = 0x84;
pub const RSP_ERR: u8 = 0xFF;

pub const ERR_MALFORMED: u8 = 0x01;
pub const ERR_UNKNOWN_OP: u8 = 0x02;
pub const ERR_RATE_LIMIT: u8 = 0x03;
pub const ERR_INTERNAL: u8 = 0x04;

#[derive(Debug, Error)]
pub enum WireError {
    #[error("frame too short")]
    TooShort,
    #[error("frame too long")]
    TooLong,
    #[error("unknown opcode {0:#04x}")]
    UnknownOp(u8),
}

#[derive(Debug)]
pub enum Request {
    Put {
        slot: SlotId,
        cell: Box<[u8; CELL_SIZE]>,
    },
    Get {
        slot: SlotId,
    },
    Time,
}

impl Request {
    pub fn parse(buf: &[u8]) -> Result<Self, WireError> {
        if buf.is_empty() {
            return Err(WireError::TooShort);
        }
        match buf[0] {
            OP_PUT => {
                const EXPECTED: usize = 1 + SLOT_ID_LEN + CELL_SIZE;
                if buf.len() < EXPECTED {
                    return Err(WireError::TooShort);
                }
                if buf.len() > EXPECTED {
                    return Err(WireError::TooLong);
                }
                let mut slot = [0u8; SLOT_ID_LEN];
                slot.copy_from_slice(&buf[1..1 + SLOT_ID_LEN]);
                let mut cell = Box::new([0u8; CELL_SIZE]);
                cell.copy_from_slice(&buf[1 + SLOT_ID_LEN..]);
                Ok(Request::Put {
                    slot: SlotId(slot),
                    cell,
                })
            }
            OP_GET => {
                const EXPECTED: usize = 1 + SLOT_ID_LEN;
                if buf.len() < EXPECTED {
                    return Err(WireError::TooShort);
                }
                if buf.len() > EXPECTED {
                    return Err(WireError::TooLong);
                }
                let mut slot = [0u8; SLOT_ID_LEN];
                slot.copy_from_slice(&buf[1..]);
                Ok(Request::Get { slot: SlotId(slot) })
            }
            OP_TIME => {
                if buf.len() != 1 {
                    return Err(WireError::TooLong);
                }
                Ok(Request::Time)
            }
            op => Err(WireError::UnknownOp(op)),
        }
    }
}

pub enum Response<'a> {
    Ack,
    Hit { cell: &'a [u8; CELL_SIZE] },
    Miss,
    Time { epoch: u64 },
    Err { code: u8 },
}

impl<'a> Response<'a> {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Response::Ack => vec![RSP_ACK],
            Response::Hit { cell } => {
                let mut out = Vec::with_capacity(1 + CELL_SIZE);
                out.push(RSP_HIT);
                out.extend_from_slice(cell.as_slice());
                out
            }
            Response::Miss => vec![RSP_MISS],
            Response::Time { epoch } => {
                let mut out = Vec::with_capacity(1 + 8);
                out.push(RSP_TIME);
                out.extend_from_slice(&epoch.to_be_bytes());
                out
            }
            Response::Err { code } => vec![RSP_ERR, *code],
        }
    }
}

#[derive(Debug)]
pub enum OwnedResponse {
    Ack,
    Hit(Box<[u8; CELL_SIZE]>),
    Miss,
    Time(u64),
    Err(u8),
}

impl OwnedResponse {
    pub fn parse(buf: &[u8]) -> Result<Self, WireError> {
        if buf.is_empty() {
            return Err(WireError::TooShort);
        }
        match buf[0] {
            RSP_ACK => {
                if buf.len() != 1 {
                    return Err(WireError::TooLong);
                }
                Ok(OwnedResponse::Ack)
            }
            RSP_HIT => {
                const EXPECTED: usize = 1 + CELL_SIZE;
                if buf.len() < EXPECTED {
                    return Err(WireError::TooShort);
                }
                if buf.len() > EXPECTED {
                    return Err(WireError::TooLong);
                }
                let mut cell = Box::new([0u8; CELL_SIZE]);
                cell.copy_from_slice(&buf[1..]);
                Ok(OwnedResponse::Hit(cell))
            }
            RSP_MISS => {
                if buf.len() != 1 {
                    return Err(WireError::TooLong);
                }
                Ok(OwnedResponse::Miss)
            }
            RSP_TIME => {
                if buf.len() != 9 {
                    return Err(if buf.len() < 9 {
                        WireError::TooShort
                    } else {
                        WireError::TooLong
                    });
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&buf[1..]);
                Ok(OwnedResponse::Time(u64::from_be_bytes(bytes)))
            }
            RSP_ERR => {
                if buf.len() != 2 {
                    return Err(if buf.len() < 2 {
                        WireError::TooShort
                    } else {
                        WireError::TooLong
                    });
                }
                Ok(OwnedResponse::Err(buf[1]))
            }
            op => Err(WireError::UnknownOp(op)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_roundtrip() {
        let slot = SlotId([0xABu8; SLOT_ID_LEN]);
        let cell = Box::new([0x42u8; CELL_SIZE]);
        let mut buf = Vec::new();
        buf.push(OP_PUT);
        buf.extend_from_slice(&slot.0);
        buf.extend_from_slice(cell.as_slice());

        let parsed = Request::parse(&buf).unwrap();
        match parsed {
            Request::Put { slot: s, cell: c } => {
                assert_eq!(s, slot);
                assert_eq!(&c[..], &cell[..]);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn get_too_short() {
        let buf = [OP_GET, 0, 0, 0];
        assert!(matches!(Request::parse(&buf), Err(WireError::TooShort)));
    }

    #[test]
    fn put_too_long_rejected() {
        let mut buf = vec![OP_PUT];
        buf.extend_from_slice(&[0u8; SLOT_ID_LEN + CELL_SIZE + 1]);
        assert!(matches!(Request::parse(&buf), Err(WireError::TooLong)));
    }

    #[test]
    fn unknown_op() {
        assert!(matches!(
            Request::parse(&[0x77]),
            Err(WireError::UnknownOp(0x77))
        ));
    }

    #[test]
    fn response_roundtrip_hit() {
        let cell = [0x99u8; CELL_SIZE];
        let r = Response::Hit { cell: &cell };
        let encoded = r.encode();
        let parsed = OwnedResponse::parse(&encoded).unwrap();
        match parsed {
            OwnedResponse::Hit(c) => assert_eq!(&c[..], &cell[..]),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_roundtrip_time() {
        let r = Response::Time {
            epoch: 0xDEAD_BEEF_CAFE_F00D,
        };
        let parsed = OwnedResponse::parse(&r.encode()).unwrap();
        match parsed {
            OwnedResponse::Time(e) => assert_eq!(e, 0xDEAD_BEEF_CAFE_F00D),
            _ => panic!("wrong variant"),
        }
    }
}
