use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use octopus_api_http::serve;
use octopus_infra_sqlite::SqlitePhase3Store;
use octopus_runtime::Phase3Service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let target_dir = std::env::current_dir()?.join("target");
    std::fs::create_dir_all(&target_dir)?;
    let database_path = target_dir.join("octopus-phase3.db");
    let connection_string = format!("sqlite://{}?mode=rwc", database_path.display());

    let store = SqlitePhase3Store::connect(&connection_string).await?;
    let service = Phase3Service::new(store);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4040);

    println!("Phase 3 HTTP server listening on http://{addr}");
    serve(service, addr).await
}
