use rand::distr::Alphanumeric;
use rand::Rng;
use rand::RngExt;
use serde_json::json;
use std::time::UNIX_EPOCH;
use std::time::{Duration, SystemTime};
//use whispra::protocol::crypto;

// set a timer, the user should get a new empheral token each 60 seconds
// for maximum anonymization and to make it very hard to track

fn gen_mailbox_token() -> String {
    let empheral_token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(18)
        .map(char::from)
        .collect();

    println!("SECURE_EMPHEREAL_TOKEN: {}", empheral_token);

    empheral_token
}

pub fn emp(payload: &str) {
    let time_to_live = Duration::from_secs(60);

    let expiration_date = SystemTime::now() + time_to_live;

    let exp_timestamp = expiration_date
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let token = gen_mailbox_token();

    let object = json!({
        "mailbox_token": token,
        "payload": payload,
        "expires_at": exp_timestamp
    });

    if let Some(mailbox_token) = object.get("mailbox_token") {
        println!("Token from JSON: {}", mailbox_token)
    }
    if let Some(payload) = object.get("payload") {
        println!("Payload from JSON: {}", payload);
    }
    if let Some(expires_at) = object.get("expires_at") {
        println!("Expiration Date: {}", expires_at);
    }
}
