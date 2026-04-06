use std::{collections::HashMap, env, net::SocketAddr, path::PathBuf, sync::{Arc, Mutex}};

use octopus_core::{
    default_connection_stubs, default_preferences, AppError, HostState,
    DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use octopus_infra::build_infra_bundle;
use octopus_platform::PlatformServices;
use octopus_runtime_adapter::RuntimeAdapter;
use octopus_server::{build_router, ServerState};

#[derive(Debug, Clone)]
struct BackendArgs {
    port: u16,
    auth_token: String,
    app_version: String,
    cargo_workspace: bool,
    host_platform: String,
    host_mode: String,
    host_shell: String,
    preferences_path: PathBuf,
    runtime_root: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = BackendArgs::parse(env::args().skip(1).collect())?;
    let workspace_root = args
        .runtime_root
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| args.runtime_root.clone());

    let infra = build_infra_bundle(&workspace_root)?;
    let runtime = std::sync::Arc::new(RuntimeAdapter::new(
        DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
    ));
    let services = PlatformServices {
        workspace: infra.workspace.clone(),
        auth: infra.auth.clone(),
        app_registry: infra.app_registry.clone(),
        rbac: infra.rbac.clone(),
        runtime_session: runtime.clone(),
        runtime_execution: runtime.clone(),
        runtime_config: runtime,
        artifact: infra.artifact.clone(),
        inbox: infra.inbox.clone(),
        knowledge: infra.knowledge.clone(),
        observation: infra.observation.clone(),
    };
    let router = build_router(ServerState {
        services,
        host_auth_token: args.auth_token.clone(),
        transport_security: "loopback".into(),
        idempotency_cache: Arc::new(Mutex::new(HashMap::new())),
        host_state: HostState {
            platform: args.host_platform.clone(),
            mode: args.host_mode.clone(),
            app_version: args.app_version.clone(),
            cargo_workspace: args.cargo_workspace,
            shell: args.host_shell.clone(),
        },
        host_connections: default_connection_stubs(),
        host_preferences_path: args.preferences_path.clone(),
        host_default_preferences: default_preferences(DEFAULT_WORKSPACE_ID, DEFAULT_PROJECT_ID),
        backend_connection: octopus_core::DesktopBackendConnection {
            base_url: Some(format!("http://127.0.0.1:{}", args.port)),
            auth_token: Some(args.auth_token.clone()),
            state: "ready".into(),
            transport: "http".into(),
        },
    });

    let address = SocketAddr::from(([127, 0, 0, 1], args.port));
    log::info!(
        "starting octopus desktop backend version={} cargo_workspace={} workspace_root={} preferences_path={}",
        args.app_version,
        args.cargo_workspace,
        workspace_root.display(),
        args.preferences_path.display(),
    );
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| AppError::runtime(error.to_string()))?;
    axum::serve(listener, router)
        .await
        .map_err(|error| AppError::runtime(error.to_string()))
}

impl BackendArgs {
    fn parse(args: Vec<String>) -> Result<Self, AppError> {
        let mut port = None;
        let mut auth_token = None;
        let mut app_version = None;
        let mut cargo_workspace = None;
        let mut host_platform = None;
        let mut host_mode = None;
        let mut host_shell = None;
        let mut preferences_path = None;
        let mut runtime_root = None;

        let mut iter = args.into_iter();
        while let Some(flag) = iter.next() {
            let value = iter
                .next()
                .ok_or_else(|| AppError::invalid_input(format!("missing value for {flag}")))?;
            match flag.as_str() {
                "--port" => {
                    port = Some(value.parse::<u16>().map_err(|error| {
                        AppError::invalid_input(format!("invalid --port value: {error}"))
                    })?);
                }
                "--auth-token" => auth_token = Some(value),
                "--app-version" => app_version = Some(value),
                "--cargo-workspace" => {
                    cargo_workspace = Some(matches!(value.as_str(), "true" | "1" | "yes"))
                }
                "--host-platform" => host_platform = Some(value),
                "--host-mode" => host_mode = Some(value),
                "--host-shell" => host_shell = Some(value),
                "--preferences-path" => preferences_path = Some(PathBuf::from(value)),
                "--runtime-root" => runtime_root = Some(PathBuf::from(value)),
                _ => return Err(AppError::invalid_input(format!("unknown argument {flag}"))),
            }
        }

        Ok(Self {
            port: port.ok_or_else(|| AppError::invalid_input("missing --port"))?,
            auth_token: auth_token
                .ok_or_else(|| AppError::invalid_input("missing --auth-token"))?,
            app_version: app_version
                .ok_or_else(|| AppError::invalid_input("missing --app-version"))?,
            cargo_workspace: cargo_workspace
                .ok_or_else(|| AppError::invalid_input("missing --cargo-workspace"))?,
            host_platform: host_platform
                .ok_or_else(|| AppError::invalid_input("missing --host-platform"))?,
            host_mode: host_mode
                .ok_or_else(|| AppError::invalid_input("missing --host-mode"))?,
            host_shell: host_shell
                .ok_or_else(|| AppError::invalid_input("missing --host-shell"))?,
            preferences_path: preferences_path
                .ok_or_else(|| AppError::invalid_input("missing --preferences-path"))?,
            runtime_root: runtime_root
                .ok_or_else(|| AppError::invalid_input("missing --runtime-root"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::BackendArgs;

    #[test]
    fn parses_supervisor_arguments() {
        let args = BackendArgs::parse(vec![
            "--port".into(),
            "43127".into(),
            "--auth-token".into(),
            "desktop-test-token".into(),
            "--app-version".into(),
            "0.1.0".into(),
            "--cargo-workspace".into(),
            "true".into(),
            "--host-platform".into(),
            "web".into(),
            "--host-mode".into(),
            "local".into(),
            "--host-shell".into(),
            "browser".into(),
            "--preferences-path".into(),
            "/tmp/preferences.json".into(),
            "--runtime-root".into(),
            "/tmp/runtime".into(),
        ])
        .expect("args parse");

        assert_eq!(args.port, 43127);
        assert!(args.cargo_workspace);
        assert_eq!(args.host_platform, "web");
        assert_eq!(args.host_shell, "browser");
        assert_eq!(args.runtime_root, PathBuf::from("/tmp/runtime"));
    }
}
