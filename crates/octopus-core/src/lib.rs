use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_WORKSPACE_ID: &str = "ws-local";
pub const DEFAULT_PROJECT_ID: &str = "proj-redesign";

#[derive(Debug, Error)]
pub enum AppError {
  #[error("failed to access filesystem: {0}")]
  Io(#[from] std::io::Error),
  #[error("failed to serialize shell payload: {0}")]
  Serde(#[from] serde_json::Error),
  #[error("host bootstrap failed: {0}")]
  Runtime(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostState {
  pub platform: String,
  pub mode: String,
  pub app_version: String,
  pub cargo_workspace: bool,
  pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellPreferences {
  pub theme: String,
  pub locale: String,
  pub compact_sidebar: bool,
  pub default_workspace_id: String,
  pub last_visited_route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
  pub id: String,
  pub mode: String,
  pub label: String,
  pub workspace_id: String,
  pub base_url: Option<String>,
  pub state: String,
  pub last_sync_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellBootstrap {
  pub host_state: HostState,
  pub preferences: ShellPreferences,
  pub connections: Vec<ConnectionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthcheckStatus {
  pub status: String,
  pub host: String,
  pub mode: String,
  #[serde(rename = "cargoWorkspace")]
  pub cargo_workspace: bool,
}

pub trait PreferencesPort {
  fn load_preferences(&self) -> Result<ShellPreferences, AppError>;
  fn save_preferences(&self, preferences: &ShellPreferences) -> Result<ShellPreferences, AppError>;
}

pub fn default_last_visited_route(workspace_id: &str, project_id: &str) -> String {
  format!("/workspaces/{workspace_id}/dashboard?project={project_id}")
}

pub fn default_preferences(workspace_id: &str, project_id: &str) -> ShellPreferences {
  ShellPreferences {
    theme: "system".into(),
    locale: "zh-CN".into(),
    compact_sidebar: false,
    default_workspace_id: workspace_id.into(),
    last_visited_route: default_last_visited_route(workspace_id, project_id),
  }
}

pub fn default_host_state(app_version: impl Into<String>, cargo_workspace: bool) -> HostState {
  HostState {
    platform: "tauri".into(),
    mode: "local".into(),
    app_version: app_version.into(),
    cargo_workspace,
    shell: "tauri2".into(),
  }
}

pub fn default_connection_stubs() -> Vec<ConnectionProfile> {
  vec![
    ConnectionProfile {
      id: "conn-local-shell".into(),
      mode: "local".into(),
      label: "Local Shell Runtime".into(),
      workspace_id: DEFAULT_WORKSPACE_ID.into(),
      base_url: None,
      state: "local-ready".into(),
      last_sync_at: None,
    },
    ConnectionProfile {
      id: "conn-enterprise-ops".into(),
      mode: "shared".into(),
      label: "Enterprise Ops Mirror".into(),
      workspace_id: "ws-enterprise".into(),
      base_url: Some("https://shared.stub.octopus.local".into()),
      state: "connected".into(),
      last_sync_at: Some(1_775_000_000_000),
    },
  ]
}
