use crate::bridge::conn::Connection;
use crate::bridge::contacts::ContactBook;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Message {
        from: String,
        counter: u64,
        payload: String,
    },
    ContactAdded {
        name: String,
    },
    Status {
        connected: bool,
    },
    Lagged,
}

pub struct OutgoingMessage {
    pub to: String,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct AppState {
    pub contacts: Arc<Mutex<ContactBook>>,
    pub outgoing: Arc<Mutex<VecDeque<OutgoingMessage>>>,
    pub events: broadcast::Sender<Event>,
    pub conn: Arc<Mutex<Connection>>,
    pub token: Arc<str>,
}

impl AppState {
    pub fn new(conn: Connection, token: String) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            contacts: Arc::new(Mutex::new(ContactBook::new())),
            outgoing: Arc::new(Mutex::new(VecDeque::new())),
            events,
            conn: Arc::new(Mutex::new(conn)),
            token: token.into(),
        }
    }
}
