use serde_json::json;
use whispra::protocol::crypto;

#[test]
fn generate_details() {
    let simulated_session_key: &[u8; 32] = &[55; 32];

    let token_plain = "l1nk4n_token";
    let token_padded = format!("{:<4096}", token_plain);

    let encrypted_handshake =
        crypto::encrypt_message(&token_padded, simulated_session_key).unwrap();
    println!("-- HANDSHAKE HEX --");
    println!("{}", encrypted_handshake);

    let routing_data = json!({
        "target_token": "bob_token",
        "payload": "Hello Whispra"
    })
    .to_string();

    let routing_padded = format!("{:<4096}", routing_data);

    let encrypted_routing =
        crypto::encrypt_message(&routing_padded, simulated_session_key).unwrap();
    println!("\n--- ROUTING MESSAGE HEX ---");
    println!("{}", encrypted_routing);
}
