use whispra::connections::server;

#[test]
fn start_websocket() {
    println!("--- Starting Websocket ---");
    server::start_server();
}
