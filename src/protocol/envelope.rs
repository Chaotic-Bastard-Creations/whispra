use serde_json::json;
use std::time::{Duration, SystemTime};

// set a timer, the user should get a new empheral token each 60 seconds
// for maximum anonymization and to make it very hard to track

fn gen_mailbox_token() {}

pub fn emp(payload: &str) {
    let time_to_live = Duration::from_secs(60);
    let start = SystemTime.now();
    match start.elapsed() {
        Ok(elapsed) => {
            println!("{}", elapsed.as_secs());
            if elapsed > time_to_live {
                let token = gen_mailbox_token();
            }
        }
        Err(e) => {
            println!("Error: {e:?}");
        }
    }
    let object = json!({ "mailbox_token": 65, "payload": payload, "ttl":  });
    assert_eq!(*object.get("A").unwrap(), json!(65));
}
