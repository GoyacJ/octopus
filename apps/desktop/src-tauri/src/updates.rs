use std::{env, sync::Arc};

use octopus_core::{
    default_host_update_status, timestamp_now, AppError, HostReleaseSummary,
    HostUpdateCapabilities, HostUpdateProgress, HostUpdateStatus, PreferencesPort,
};
use parking_lot::RwLock;
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{error::ShellResult, state::ShellState};

const UPDATE_ENDPOINT_FORMAL_ENV: &str = "OCTOPUS_UPDATE_ENDPOINT_FORMAL";
const UPDATE_ENDPOINT_PREVIEW_ENV: &str = "OCTOPUS_UPDATE_ENDPOINT_PREVIEW";
const UPDATE_PUBKEY_ENV: &str = "OCTOPUS_UPDATE_PUBKEY";
const BUILTIN_UPDATER_CONFIG: &str = include_str!("../updater.config.json");

#[derive(Default)]
struct AppUpdateSnapshot {
    status: Option<HostUpdateStatus>,
    pending_update: Option<Update>,
    downloaded_bytes: Option<Vec<u8>>,
}

#[derive(Clone, Default)]
pub struct AppUpdateService {
    snapshot: Arc<RwLock<AppUpdateSnapshot>>,
}

#[derive(Clone, Default)]
struct UpdateRuntimeConfig {
    endpoint: Option<String>,
    pubkey: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductUpdateConfig {
    formal_endpoint: Option<String>,
    preview_endpoint: Option<String>,
    pubkey: Option<String>,
    #[serde(rename = "releaseRepo")]
    release_repo: Option<String>,
}

impl AppUpdateService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_status(&self, current_version: &str, current_channel: &str) -> HostUpdateStatus {
        let capabilities = capabilities_for_channel(current_channel);
        let mut status = self
            .snapshot
            .read()
            .status
            .clone()
            .unwrap_or_else(|| default_host_update_status(current_version, current_channel));
        status.current_version = current_version.to_string();
        status.current_channel = normalize_update_channel(Some(current_channel), current_channel);
        status.capabilities = capabilities;
        status
    }

    pub async fn check<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        current_version: &str,
        current_channel: &str,
    ) -> ShellResult<HostUpdateStatus> {
        let config = update_runtime_config(current_channel);
        if config.endpoint.is_none() {
            let status = self.current_status(current_version, current_channel);
            self.replace_status(status.clone(), None, None);
            return Ok(status);
        }

        let endpoint = parse_update_endpoint(&config)?;
        let mut builder = app
            .updater_builder()
            .endpoints(vec![endpoint])
            .map_err(update_error)?;
        if let Some(pubkey) = config.pubkey {
            builder = builder.pubkey(pubkey);
        }

        let updater = builder.build().map_err(update_error)?;
        let checked_at = timestamp_now();
        if let Some(update) = updater.check().await.map_err(update_error)? {
            let status = HostUpdateStatus {
                current_version: current_version.to_string(),
                current_channel: normalize_update_channel(Some(current_channel), current_channel),
                state: "update_available".into(),
                latest_release: Some(release_summary_from_update(&update, current_channel)),
                last_checked_at: Some(checked_at),
                progress: None,
                capabilities: capabilities_for_channel(current_channel),
                error_code: None,
                error_message: None,
            };
            self.replace_status(status.clone(), Some(update), None);
            Ok(status)
        } else {
            let status = HostUpdateStatus {
                current_version: current_version.to_string(),
                current_channel: normalize_update_channel(Some(current_channel), current_channel),
                state: "up_to_date".into(),
                latest_release: None,
                last_checked_at: Some(checked_at),
                progress: None,
                capabilities: capabilities_for_channel(current_channel),
                error_code: None,
                error_message: None,
            };
            self.replace_status(status.clone(), None, None);
            Ok(status)
        }
    }

    pub async fn download(
        &self,
        current_version: &str,
        current_channel: &str,
    ) -> ShellResult<HostUpdateStatus> {
        let capabilities = capabilities_for_channel(current_channel);
        if !capabilities.can_download {
            let status = self.error_status(
                current_version,
                current_channel,
                "UPDATE_DOWNLOAD_UNSUPPORTED",
                "当前宿主环境未配置应用内下载更新能力。",
            );
            self.replace_status(status.clone(), None, None);
            return Ok(status);
        }

        let pending_update = self
            .snapshot
            .read()
            .pending_update
            .clone()
            .ok_or_else(|| AppError::runtime("no pending update is available for download"))?;

        let progress_snapshot = self.snapshot.clone();
        let progress_capabilities = capabilities.clone();
        let mut downloaded_bytes = 0_u64;
        let bytes = pending_update
            .download(
                move |chunk_length, total_bytes| {
                    downloaded_bytes += chunk_length as u64;
                    let total_bytes = total_bytes.unwrap_or(downloaded_bytes);
                    let percent = downloaded_bytes
                        .saturating_mul(100)
                        .checked_div(total_bytes)
                        .unwrap_or(0);
                    let mut snapshot = progress_snapshot.write();
                    let mut status = snapshot.status.clone().unwrap_or_else(|| {
                        default_host_update_status(current_version, current_channel)
                    });
                    status.state = "downloading".into();
                    status.progress = Some(HostUpdateProgress {
                        downloaded_bytes,
                        total_bytes,
                        percent,
                    });
                    status.error_code = None;
                    status.error_message = None;
                    status.capabilities = progress_capabilities.clone();
                    snapshot.status = Some(status);
                },
                || {},
            )
            .await
            .map_err(update_error)?;

        let total_bytes = bytes.len() as u64;
        let status = HostUpdateStatus {
            current_version: current_version.to_string(),
            current_channel: normalize_update_channel(Some(current_channel), current_channel),
            state: "downloaded".into(),
            latest_release: Some(release_summary_from_update(
                &pending_update,
                current_channel,
            )),
            last_checked_at: self
                .snapshot
                .read()
                .status
                .as_ref()
                .and_then(|status| status.last_checked_at),
            progress: Some(HostUpdateProgress {
                downloaded_bytes: total_bytes,
                total_bytes,
                percent: 100,
            }),
            capabilities,
            error_code: None,
            error_message: None,
        };
        self.replace_status(status.clone(), Some(pending_update), Some(bytes));
        Ok(status)
    }

    pub fn install(
        &self,
        current_version: &str,
        current_channel: &str,
    ) -> ShellResult<HostUpdateStatus> {
        let capabilities = capabilities_for_channel(current_channel);
        if !capabilities.can_install {
            let status = self.error_status(
                current_version,
                current_channel,
                "UPDATE_INSTALL_UNSUPPORTED",
                "当前宿主环境未配置应用内安装更新能力。",
            );
            self.replace_status(status.clone(), None, None);
            return Ok(status);
        }

        let (pending_update, downloaded_bytes, last_checked_at, latest_release) = {
            let snapshot = self.snapshot.read();
            let pending_update = snapshot
                .pending_update
                .clone()
                .ok_or_else(|| AppError::runtime("no pending update is ready to install"))?;
            let downloaded_bytes = snapshot
                .downloaded_bytes
                .clone()
                .ok_or_else(|| AppError::runtime("no downloaded update bytes are available"))?;
            let latest_release = snapshot
                .status
                .as_ref()
                .and_then(|status| status.latest_release.clone())
                .or_else(|| {
                    Some(release_summary_from_update(
                        &pending_update,
                        current_channel,
                    ))
                });

            (
                pending_update,
                downloaded_bytes,
                snapshot
                    .status
                    .as_ref()
                    .and_then(|status| status.last_checked_at),
                latest_release,
            )
        };

        pending_update
            .install(downloaded_bytes)
            .map_err(update_error)?;

        let status = HostUpdateStatus {
            current_version: current_version.to_string(),
            current_channel: normalize_update_channel(Some(current_channel), current_channel),
            state: "installing".into(),
            latest_release,
            last_checked_at,
            progress: None,
            capabilities,
            error_code: None,
            error_message: None,
        };
        self.replace_status(status.clone(), Some(pending_update), None);
        Ok(status)
    }

    fn replace_status(
        &self,
        status: HostUpdateStatus,
        pending_update: Option<Update>,
        downloaded_bytes: Option<Vec<u8>>,
    ) {
        let mut snapshot = self.snapshot.write();
        snapshot.status = Some(status);
        snapshot.pending_update = pending_update;
        snapshot.downloaded_bytes = downloaded_bytes;
    }

    fn error_status(
        &self,
        current_version: &str,
        current_channel: &str,
        error_code: &str,
        error_message: &str,
    ) -> HostUpdateStatus {
        HostUpdateStatus {
            current_version: current_version.to_string(),
            current_channel: normalize_update_channel(Some(current_channel), current_channel),
            state: "error".into(),
            latest_release: self
                .snapshot
                .read()
                .status
                .as_ref()
                .and_then(|status| status.latest_release.clone()),
            last_checked_at: self
                .snapshot
                .read()
                .status
                .as_ref()
                .and_then(|status| status.last_checked_at),
            progress: None,
            capabilities: capabilities_for_channel(current_channel),
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message.to_string()),
        }
    }
}

pub fn get_host_update_status(state: &ShellState) -> ShellResult<HostUpdateStatus> {
    let preferences = state.preferences_service.load_preferences()?;
    Ok(state
        .app_update_service
        .current_status(&state.host_state.app_version, &preferences.update_channel))
}

pub async fn check_host_update<R: Runtime>(
    app: &AppHandle<R>,
    state: &ShellState,
    requested_channel: Option<&str>,
) -> ShellResult<HostUpdateStatus> {
    let preferences = state.preferences_service.load_preferences()?;
    let channel = normalize_update_channel(requested_channel, &preferences.update_channel);
    state
        .app_update_service
        .check(app, &state.host_state.app_version, &channel)
        .await
}

pub async fn download_host_update(state: &ShellState) -> ShellResult<HostUpdateStatus> {
    let preferences = state.preferences_service.load_preferences()?;
    state
        .app_update_service
        .download(&state.host_state.app_version, &preferences.update_channel)
        .await
}

pub fn install_host_update(state: &ShellState) -> ShellResult<HostUpdateStatus> {
    let preferences = state.preferences_service.load_preferences()?;
    state
        .app_update_service
        .install(&state.host_state.app_version, &preferences.update_channel)
}

fn capabilities_for_channel(channel: &str) -> HostUpdateCapabilities {
    let config = update_runtime_config(channel);
    HostUpdateCapabilities {
        can_check: config.endpoint.is_some(),
        can_download: config.endpoint.is_some() && config.pubkey.is_some(),
        can_install: config.endpoint.is_some() && config.pubkey.is_some(),
        supports_channels: true,
    }
}

fn update_runtime_config(channel: &str) -> UpdateRuntimeConfig {
    update_runtime_config_with(channel, env_var)
}

fn update_runtime_config_with(
    channel: &str,
    env_lookup: impl Fn(&str) -> Option<String>,
) -> UpdateRuntimeConfig {
    let built_in = built_in_update_runtime_config(channel);
    UpdateRuntimeConfig {
        endpoint: env_lookup(update_endpoint_env(channel)).or(built_in.endpoint),
        pubkey: env_lookup(UPDATE_PUBKEY_ENV).or(built_in.pubkey),
    }
}

fn built_in_update_runtime_config(channel: &str) -> UpdateRuntimeConfig {
    if cfg!(debug_assertions) {
        return UpdateRuntimeConfig::default();
    }

    let built_in = load_product_update_config();
    UpdateRuntimeConfig {
        endpoint: built_in.endpoint_for_channel(channel),
        pubkey: built_in.pubkey(),
    }
}

fn update_endpoint_env(channel: &str) -> &'static str {
    match normalize_update_channel(Some(channel), "formal").as_str() {
        "preview" => UPDATE_ENDPOINT_PREVIEW_ENV,
        _ => UPDATE_ENDPOINT_FORMAL_ENV,
    }
}

fn env_var(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn load_product_update_config() -> ProductUpdateConfig {
    serde_json::from_str::<ProductUpdateConfig>(BUILTIN_UPDATER_CONFIG)
        .unwrap_or_default()
        .normalized()
}

fn normalize_update_channel(value: Option<&str>, fallback: &str) -> String {
    match value.map(str::trim) {
        Some("preview") => "preview".into(),
        Some("formal") => "formal".into(),
        _ => match fallback.trim() {
            "preview" => "preview".into(),
            _ => "formal".into(),
        },
    }
}

fn parse_update_endpoint(config: &UpdateRuntimeConfig) -> ShellResult<Url> {
    config
        .endpoint
        .as_deref()
        .ok_or_else(|| AppError::runtime("update endpoint is not configured"))?
        .parse::<Url>()
        .map_err(|error| AppError::runtime(format!("invalid update endpoint: {error}")))
}

fn release_summary_from_update(update: &Update, current_channel: &str) -> HostReleaseSummary {
    HostReleaseSummary {
        version: update.version.clone(),
        channel: extract_string_field(&update.raw_json, &["channel"])
            .unwrap_or_else(|| normalize_update_channel(Some(current_channel), current_channel)),
        published_at: update
            .date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".into()),
        notes: update.body.clone(),
        notes_url: extract_string_field(
            &update.raw_json,
            &[
                "notesUrl",
                "notes_url",
                "releaseNotesUrl",
                "release_notes_url",
                "url",
            ],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_build_disables_built_in_update_endpoints_by_default() {
        let config = update_runtime_config_with("formal", |_| None);
        let capabilities = capabilities_for_channel("formal");

        assert!(cfg!(debug_assertions));
        assert_eq!(config.endpoint, None);
        assert_eq!(config.pubkey, None);
        assert!(!capabilities.can_check);
        assert!(!capabilities.can_download);
        assert!(!capabilities.can_install);
    }

    #[test]
    fn explicit_update_env_overrides_reenable_dev_update_checking() {
        let config = update_runtime_config_with("preview", |key| match key {
            UPDATE_ENDPOINT_PREVIEW_ENV => {
                Some(String::from("https://updates.example.test/latest.json"))
            }
            UPDATE_PUBKEY_ENV => Some(String::from("test-pubkey")),
            _ => None,
        });
        let capabilities = HostUpdateCapabilities {
            can_check: config.endpoint.is_some(),
            can_download: config.endpoint.is_some() && config.pubkey.is_some(),
            can_install: config.endpoint.is_some() && config.pubkey.is_some(),
            supports_channels: true,
        };

        assert_eq!(
            config.endpoint.as_deref(),
            Some("https://updates.example.test/latest.json"),
        );
        assert_eq!(config.pubkey.as_deref(), Some("test-pubkey"));
        assert!(capabilities.can_check);
        assert!(capabilities.can_download);
        assert!(capabilities.can_install);
    }
}

fn extract_string_field(source: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        source
            .get(key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn update_error(error: impl std::fmt::Display) -> AppError {
    AppError::runtime(error.to_string())
}

impl ProductUpdateConfig {
    fn normalized(mut self) -> Self {
        self.formal_endpoint = normalize_optional_string(self.formal_endpoint);
        self.preview_endpoint = normalize_optional_string(self.preview_endpoint);
        self.pubkey = normalize_optional_string(self.pubkey);
        self.release_repo = normalize_optional_string(self.release_repo);
        self
    }

    fn endpoint_for_channel(&self, channel: &str) -> Option<String> {
        match normalize_update_channel(Some(channel), "formal").as_str() {
            "preview" => self.preview_endpoint.clone(),
            _ => self.formal_endpoint.clone(),
        }
    }

    fn pubkey(&self) -> Option<String> {
        self.pubkey.clone()
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}
