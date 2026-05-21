use whispra::protocol::crypto;

#[test]
fn execute_aes_code() {
    println!("--- 1. Test AES encrypt -> decrypt ---");
    let key = [0u8; 32];
    let _ = crypto::encrypt_message("test", &key);
}
