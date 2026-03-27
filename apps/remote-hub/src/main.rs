use std::{env, net::SocketAddr, path::PathBuf};

use axum::serve;
use octopus_remote_hub::{app, AppState};
use octopus_runtime::Slice1Runtime;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = env::var("OCTOPUS_REMOTE_HUB_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/remote-hub.sqlite"));
    let bind_addr: SocketAddr = env::var("OCTOPUS_REMOTE_HUB_BIND")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse()?;

    let runtime = Slice1Runtime::open_at(&db_path).await?;
    let listener = TcpListener::bind(bind_addr).await?;
    serve(listener, app(AppState::new(runtime))).await?;
    Ok(())
}
