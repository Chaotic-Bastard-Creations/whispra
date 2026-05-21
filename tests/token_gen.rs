use serde_json::json;
use whispra::protocol::crypto;

#[test]
fn generate_details() {
    let simulated_session_key: &[u8; 32] = &[55; 32];

    let linkan_padded = format!("{:<4096}", "l1nk4n_token");
    let linkan_handshake = crypto::encrypt_message(&linkan_padded, simulated_session_key).unwrap();
    println!("\n========================================");
    println!("-- TERMINAL 2: LINKAN HANDSHAKE HEX --");
    println!("{}", linkan_handshake);

    let bob_padded = format!("{:<4096}", "bob_token");
    let bob_handshake = crypto::encrypt_message(&bob_padded, simulated_session_key).unwrap();
    println!("\n========================================");
    println!("-- TERMINAL 1: BOB HANDSHAKE HEX --");
    println!("{}", bob_handshake);

    let routing_data = json!({
        "target_token": "bob_token",
        "payload": "Hello Whispra"
    })
    .to_string();
    let routing_padded = format!("{:<4096}", routing_data);
    let encrypted_routing =
        crypto::encrypt_message(&routing_padded, simulated_session_key).unwrap();
    println!("\n========================================");
    println!("-- TERMINAL 2: ROUTING MESSAGE HEX --");
    println!("{}", encrypted_routing);
    println!("========================================\n");
}
