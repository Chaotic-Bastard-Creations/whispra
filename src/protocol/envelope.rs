use dashmap::DashMap;
use rand::distr::Alphanumeric;
use rand::Rng;
use rand::RngExt;
use serde_json::json;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use std::time::{Duration, SystemTime};
//use whispra::protocol::crypto;

// set a timer, the user should get a new empheral token each 60 seconds
// for maximum anonymization and to make it very hard to track
//
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

pub fn emp(payload: &str) {}
