use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use octopus_core::{AppError, DesktopBackendConnection, HealthcheckStatus, HostState};
use parking_lot::{Mutex, RwLock};
use tauri::AppHandle;
use tauri_plugin_shell::{process::CommandChild, ShellExt};
use uuid::Uuid;

use crate::error::ShellResult;

const BACKEND_HEALTH_TIMEOUT_ATTEMPTS: usize = 50;
const BACKEND_MONITOR_INTERVAL_MS: u64 = 1_000;

#[derive(Clone)]
pub struct BackendSupervisor {
    connection: Arc<RwLock<DesktopBackendConnection>>,
    child: Arc<Mutex<Option<ManagedBackendChild>>>,
    generation: Arc<AtomicU64>,
    workspace_root: PathBuf,
}

enum ManagedBackendChild {
    Dev(Child),
    Sidecar(CommandChild),
}

impl ManagedBackendChild {
    fn kill(self) -> ShellResult<()> {
        match self {
            Self::Dev(mut child) => {
                child.kill().map_err(AppError::from)?;
                let _ = child.wait();
                Ok(())
            }
            Self::Sidecar(child) => child
                .kill()
                .map_err(|error| AppError::Runtime(error.to_string())),
        }
    }
}

impl BackendSupervisor {
    pub fn new(connection: Arc<RwLock<DesktopBackendConnection>>, workspace_root: PathBuf) -> Self {
        Self {
            connection,
            child: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            workspace_root,
        }
    }

    pub fn connection(&self) -> DesktopBackendConnection {
        self.connection.read().clone()
    }

    fn mark_unavailable(&self) {
        *self.connection.write() = DesktopBackendConnection {
            base_url: None,
            auth_token: None,
            state: "unavailable".into(),
            transport: "http".into(),
        };
    }

    pub async fn start(
        &self,
        app: &AppHandle,
        host_state: &HostState,
        preferences_path: &Path,
    ) -> ShellResult<DesktopBackendConnection> {
        if self.child.lock().is_some() {
            return Ok(self.connection());
        }

        self.spawn_backend(app, host_state, preferences_path).await
    }

    pub async fn restart(
        &self,
        app: &AppHandle,
        host_state: &HostState,
        preferences_path: &Path,
    ) -> ShellResult<DesktopBackendConnection> {
        self.shutdown();
        self.spawn_backend(app, host_state, preferences_path).await
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    pub async fn start_dev(
        &self,
        host_state: &HostState,
        preferences_path: &Path,
    ) -> ShellResult<DesktopBackendConnection> {
        if self.child.lock().is_some() {
            return Ok(self.connection());
        }

        let port = find_available_port()?;
        let auth_token = Uuid::new_v4().to_string();
        let base_url = format!("http://127.0.0.1:{port}");
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        let child = match spawn_dev_backend(
            port,
            &auth_token,
            host_state,
            preferences_path,
            &self.workspace_root,
            &workspace_root(),
        ) {
            Ok(child) => child,
            Err(error) => {
                self.mark_unavailable();
                return Err(error);
            }
        };

        self.finish_spawn(child, &base_url, &auth_token, generation)
            .await
    }

    pub fn shutdown(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(child) = self.child.lock().take() {
            let _ = child.kill();
        }
        self.mark_unavailable();
    }

    async fn spawn_backend(
        &self,
        app: &AppHandle,
        host_state: &HostState,
        preferences_path: &Path,
    ) -> ShellResult<DesktopBackendConnection> {
        let port = find_available_port()?;
        let auth_token = Uuid::new_v4().to_string();
        let base_url = format!("http://127.0.0.1:{port}");
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        let child = match spawn_backend_process(
            app,
            port,
            &auth_token,
            host_state,
            preferences_path,
            &self.workspace_root,
        ) {
            Ok(child) => child,
            Err(error) => {
                self.mark_unavailable();
                return Err(error);
            }
        };
        self.finish_spawn(child, &base_url, &auth_token, generation)
            .await
    }

    async fn finish_spawn(
        &self,
        child: ManagedBackendChild,
        base_url: &str,
        auth_token: &str,
        generation: u64,
    ) -> ShellResult<DesktopBackendConnection> {
        *self.child.lock() = Some(child);

        if let Err(error) = wait_for_backend_ready(base_url, auth_token).await {
            if let Some(child) = self.child.lock().take() {
                let _ = child.kill();
            }
            self.mark_unavailable();
            return Err(error);
        }
        *self.connection.write() = DesktopBackendConnection {
            base_url: Some(base_url.into()),
            auth_token: Some(auth_token.into()),
            state: "ready".into(),
            transport: "http".into(),
        };
        self.spawn_health_monitor(base_url.into(), auth_token.into(), generation);

        Ok(self.connection())
    }

    fn spawn_health_monitor(&self, base_url: String, auth_token: String, generation: u64) {
        let connection = self.connection.clone();
        let current_generation = self.generation.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(BACKEND_MONITOR_INTERVAL_MS)).await;
                if current_generation.load(Ordering::SeqCst) != generation {
                    break;
                }

                let ready = ping_backend(&base_url, &auth_token).await.unwrap_or(false);
                if current_generation.load(Ordering::SeqCst) != generation {
                    break;
                }

                connection.write().state = if ready {
                    "ready".into()
                } else {
                    "unavailable".into()
                };
            }
        });
    }
}

impl Drop for BackendSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_backend_process(
    app: &AppHandle,
    port: u16,
    auth_token: &str,
    host_state: &HostState,
    preferences_path: &Path,
    backend_workspace_root: &Path,
) -> ShellResult<ManagedBackendChild> {
    let repo_root = workspace_root();
    if should_spawn_dev_backend(host_state, &repo_root) {
        spawn_dev_backend(
            port,
            auth_token,
            host_state,
            preferences_path,
            backend_workspace_root,
            &repo_root,
        )
    } else {
        spawn_sidecar_backend(
            app,
            port,
            auth_token,
            host_state,
            preferences_path,
            backend_workspace_root,
        )
    }
}

fn spawn_dev_backend(
    port: u16,
    auth_token: &str,
    host_state: &HostState,
    preferences_path: &Path,
    backend_workspace_root: &Path,
    repo_root: &Path,
) -> ShellResult<ManagedBackendChild> {
    let backend_bin = debug_backend_binary_path(repo_root);
    if !backend_bin.exists() {
        return Err(AppError::Runtime(format!(
            "desktop backend binary is missing at {}",
            backend_bin.display()
        )));
    }

    let child = Command::new(&backend_bin)
        .current_dir(&repo_root)
        .arg("--port")
        .arg(port.to_string())
        .arg("--auth-token")
        .arg(auth_token)
        .arg("--app-version")
        .arg(&host_state.app_version)
        .arg("--cargo-workspace")
        .arg(host_state.cargo_workspace.to_string())
        .arg("--host-platform")
        .arg(&host_state.platform)
        .arg("--host-mode")
        .arg(&host_state.mode)
        .arg("--host-shell")
        .arg(&host_state.shell)
        .arg("--preferences-path")
        .arg(preferences_path)
        .arg("--workspace-root")
        .arg(backend_workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(AppError::from)?;

    Ok(ManagedBackendChild::Dev(child))
}

fn spawn_sidecar_backend(
    app: &AppHandle,
    port: u16,
    auth_token: &str,
    host_state: &HostState,
    preferences_path: &Path,
    workspace_root: &Path,
) -> ShellResult<ManagedBackendChild> {
    let (_rx, child) = app
        .shell()
        .sidecar("octopus-desktop-backend")
        .map_err(|error| AppError::Runtime(error.to_string()))?
        .args([
            "--port",
            &port.to_string(),
            "--auth-token",
            auth_token,
            "--app-version",
            &host_state.app_version,
            "--cargo-workspace",
            if host_state.cargo_workspace {
                "true"
            } else {
                "false"
            },
            "--host-platform",
            &host_state.platform,
            "--host-mode",
            &host_state.mode,
            "--host-shell",
            &host_state.shell,
            "--preferences-path",
            &preferences_path.display().to_string(),
            "--workspace-root",
            &workspace_root.display().to_string(),
        ])
        .spawn()
        .map_err(|error| AppError::Runtime(error.to_string()))?;

    Ok(ManagedBackendChild::Sidecar(child))
}

async fn wait_for_backend_ready(base_url: &str, auth_token: &str) -> ShellResult<()> {
    for _ in 0..BACKEND_HEALTH_TIMEOUT_ATTEMPTS {
        if ping_backend(base_url, auth_token).await.unwrap_or(false) {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(AppError::Runtime(
        "desktop backend healthcheck timed out".into(),
    ))
}

async fn ping_backend(base_url: &str, auth_token: &str) -> ShellResult<bool> {
    let response = reqwest::Client::new()
        .get(format!("{base_url}/health"))
        .bearer_auth(auth_token)
        .send()
        .await
        .map_err(|error| AppError::Runtime(error.to_string()))?;
    if !response.status().is_success() {
        return Ok(false);
    }

    let payload = response
        .json::<HealthcheckStatus>()
        .await
        .map_err(|error| AppError::Runtime(error.to_string()))?;
    Ok(payload.backend.state == "ready" && payload.backend.transport == "http")
}

fn find_available_port() -> ShellResult<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(AppError::from)?;
    let address = listener.local_addr().map_err(AppError::from)?;
    Ok(address.port())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn debug_backend_binary_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join("target")
        .join("debug")
        .join(executable_name("octopus-desktop-backend"))
}

fn should_spawn_dev_backend(host_state: &HostState, repo_root: &Path) -> bool {
    // Only use dev backend when actually running a debug build inside cargo workspace
    cfg!(debug_assertions)
        && host_state.cargo_workspace
        && debug_backend_binary_path(repo_root).exists()
}

fn executable_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.into()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use octopus_core::HostState;
    use tempfile::tempdir;

    use super::{debug_backend_binary_path, executable_name, should_spawn_dev_backend};

    fn host_state(cargo_workspace: bool) -> HostState {
        HostState {
            platform: "tauri".into(),
            mode: "local".into(),
            app_version: "0.1.0-test".into(),
            cargo_workspace,
            shell: "tauri2".into(),
        }
    }

    #[test]
    fn prefers_debug_backend_inside_cargo_workspace_when_binary_exists() {
        let temp = tempdir().expect("tempdir");
        let repo_root = temp.path();
        let backend_path = debug_backend_binary_path(repo_root);
        fs::create_dir_all(
            backend_path
                .parent()
                .expect("debug backend path should have parent"),
        )
        .expect("create debug backend directory");
        fs::write(&backend_path, []).expect("write debug backend placeholder");

        assert!(should_spawn_dev_backend(&host_state(true), repo_root));
    }

    #[test]
    fn does_not_use_debug_backend_outside_cargo_workspace() {
        let temp = tempdir().expect("tempdir");
        let repo_root = temp.path();
        let backend_path = debug_backend_binary_path(repo_root);
        fs::create_dir_all(
            backend_path
                .parent()
                .expect("debug backend path should have parent"),
        )
        .expect("create debug backend directory");
        fs::write(&backend_path, []).expect("write debug backend placeholder");

        assert!(!should_spawn_dev_backend(&host_state(false), repo_root));
    }

    #[test]
    fn does_not_use_debug_backend_when_binary_is_missing() {
        let temp = tempdir().expect("tempdir");

        assert!(!should_spawn_dev_backend(&host_state(true), temp.path()));
    }

    #[test]
    fn debug_backend_binary_path_points_to_target_debug_binary() {
        let repo_root = PathBuf::from("/tmp/octopus");

        assert_eq!(
            debug_backend_binary_path(&repo_root),
            repo_root
                .join("target")
                .join("debug")
                .join(executable_name("octopus-desktop-backend")),
        );
    }
}
