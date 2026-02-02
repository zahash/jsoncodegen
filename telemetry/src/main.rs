use std::{net::SocketAddr, path::PathBuf};

use axum::{Router, routing::post};
use clap::Parser;
use jsoncodegen_utils::default_runtime_dir;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, env("JSONCODEGEN_RUNTIME"), default_value_os_t = default_runtime_dir())]
    runtime_dir: PathBuf,

    #[arg(short, long, default_value_t = 0)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env(/* RUST_LOG env var sets logging level */))
        .init();

    let args = Args::parse();
    let router = Router::new().route("/", post(async || {}));

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!("listening on {}", local_addr);
    axum::serve(listener, router).await?;

    Ok(())
}
