use whispra::connections::server;
use whispra::protocol::crypto;
use whispra::protocol::envelope;

fn main() {
    println!("Start server.");
    //server::start_sever();
    //let _ = crypto::encrypt_message("test");
    //let _ = envelope::emp(crypto::encrypt_message("test123"));
    //
    if let Ok(encrypted_text) = crypto::encrypt_message("test123") {
        let _ = envelope::emp(&encrypted_text);
    } else {
        println!("Failed to encrypt");
    }
}
