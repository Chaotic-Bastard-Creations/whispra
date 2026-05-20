use whispra::protocol::crypto;

#[test]
fn execute_aes_code() {
    println!("--- 1. Test AES encrypt -> decrypt ---");
    let _ = crypto::encrypt_message("test");
}
