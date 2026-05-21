use whispra::connections::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting whispra-server on 127.0.0.1:3000");
    server::start_server().await?;
    Ok(())
}
