use whispra::connections::server;
use whispra::protocol::crypto;
use whispra::protocol::envelope;

fn main() {
    println!("Start server.");
    //server::start_sever();
    let _ = crypto::encrypt_message("test");
    let _ = envelope::emp();
}
