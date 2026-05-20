use whispra::encryption::aes;

#[test]
fn execute_aes_code() {
    println!("--- 1. Test AES encrypt -> decrypt ---");
    aes::encrypt_message();
}
