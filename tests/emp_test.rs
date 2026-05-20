use whispra::protocol::crypto;
use whispra::protocol::envelope;

#[test]
fn execute_emp_code() {
    println!("--- 2. Test EMP anonym");
    if let Ok(encrypted_text) = crypto::encrypt_message("test123") {
        let _ = envelope::emp(&encrypted_text);
    } else {
        println!("Encryption failed!");
    }
}
