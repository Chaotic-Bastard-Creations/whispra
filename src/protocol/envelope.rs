use dashmap::DashMap;
use rand::distr::Alphanumeric;
use rand::RngExt;
use std::sync::Arc;
//use whispra::protocol::crypto;

pub type SharedMailboxMap =
    Arc<DashMap<String, tokio::sync::mpsc::Sender<axum::extract::ws::Message>>>;

pub fn gen_mailbox_token() -> String {
    let empheral_token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(18)
        .map(char::from)
        .collect();
    empheral_token
}

pub fn emp(_payload: &str) {}
