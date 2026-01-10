use axum::{
    Json, Router,
    routing::{get, get_service},
};
use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::services::ServeFile;
use tracing_subscriber::EnvFilter;

/// A simple server to serve WASM files
#[derive(Parser, Debug)]
struct Args {
    /// List of .wasm files to serve
    #[arg(required = true, num_args = 1..)]
    files: Vec<PathBuf>,

    #[arg(short, long, default_value_t = 0)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env(/* RUST_LOG env var sets logging level */))
        .init();

    let args = Args::parse();
    let mut router = Router::new();

    let mut route_paths = vec![];

    for path in args.files {
        if path.is_file()
            && let Some(unicode_path) = path.to_str()
        {
            let route_path = format!("/{}", unicode_path);
            let service = get_service(ServeFile::new(&path));
            router = router.route(&route_path, service);

            tracing::info!("serving file {:?} at route {}", path, route_path);
            route_paths.push(route_path);
        }
    }

    router = router.route("/", get(Json(route_paths)));

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!("listening on {}", local_addr);
    axum::serve(listener, router).await?;

    Ok(())
}
