use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

use axum::serve;
use chrono::Utc;
use octopus_access_auth::{RemoteAccessConfig, RemoteAccessService};
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
    let cron_tick_interval_seconds = env::var("OCTOPUS_REMOTE_HUB_CRON_TICK_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(60);
    let mut access_config = RemoteAccessConfig::default();
    if let Ok(value) = env::var("OCTOPUS_REMOTE_HUB_JWT_SECRET") {
        access_config.jwt_secret = value;
    }
    if let Ok(value) = env::var("OCTOPUS_REMOTE_HUB_SESSION_TTL_SECONDS") {
        if let Ok(ttl_seconds) = value.parse::<i64>() {
            access_config.session_ttl_seconds = ttl_seconds;
        }
    }
    if let Ok(value) = env::var("OCTOPUS_REMOTE_HUB_BOOTSTRAP_EMAIL") {
        access_config.bootstrap_email = value;
    }
    if let Ok(value) = env::var("OCTOPUS_REMOTE_HUB_BOOTSTRAP_PASSWORD") {
        access_config.bootstrap_password = value;
    }

    let runtime = Slice1Runtime::open_at(&db_path).await?;
    let auth = RemoteAccessService::open_at_with_config(&db_path, access_config).await?;
    if cron_tick_interval_seconds > 0 {
        let ticker_runtime = runtime.clone();
        tokio::spawn(async move {
            loop {
                let now = Utc::now().to_rfc3339();
                if let Err(error) = ticker_runtime.tick_due_triggers(&now).await {
                    eprintln!("cron tick failed at {now}: {error}");
                }
                tokio::time::sleep(Duration::from_secs(cron_tick_interval_seconds)).await;
            }
        });
    }
    let listener = TcpListener::bind(bind_addr).await?;
    serve(listener, app(AppState::new(runtime, auth))).await?;
    Ok(())
}
