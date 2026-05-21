use clap::Parser;
use whispra::connections::server;

const DEFAULT_LISTEN: &str = "0.0.0.0:3000";

#[derive(Parser)]
#[command(about = "Whispra edge server")]
struct Args {
    #[arg(long, default_value = DEFAULT_LISTEN)]
    listen: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("Starting whispra-server on {}", args.listen);
    server::start_server_on(&args.listen).await?;
    Ok(())
}
