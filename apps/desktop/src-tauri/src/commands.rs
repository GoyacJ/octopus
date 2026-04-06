use std::{fs, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use octopus_core::{
    AvatarUploadPayload, ConnectionProfile, DesktopBackendConnection, HostState, ShellPreferences,
};
use rfd::FileDialog;
use tauri::{AppHandle, State};

use crate::{
    bootstrap::{
        bootstrap_shell_payload, get_backend_connection_payload, get_host_state_payload,
        healthcheck_payload, list_connections_payload, load_shell_preferences,
        save_shell_preferences, HealthcheckStatusPayload, ShellBootstrapPayload,
    },
    error::into_command_error,
    state::ShellState,
};

#[tauri::command]
pub fn bootstrap_shell(state: State<'_, ShellState>) -> Result<ShellBootstrapPayload, String> {
    bootstrap_shell_payload(state.inner()).map_err(into_command_error)
}

#[tauri::command]
pub fn get_host_state(state: State<'_, ShellState>) -> HostState {
    get_host_state_payload(state.inner())
}

#[tauri::command]
pub fn load_preferences(state: State<'_, ShellState>) -> Result<ShellPreferences, String> {
    load_shell_preferences(state.inner()).map_err(into_command_error)
}

#[tauri::command]
pub fn save_preferences(
    state: State<'_, ShellState>,
    preferences: ShellPreferences,
) -> Result<ShellPreferences, String> {
    save_shell_preferences(state.inner(), preferences).map_err(into_command_error)
}

#[tauri::command]
pub fn list_connections_stub(state: State<'_, ShellState>) -> Vec<ConnectionProfile> {
    list_connections_payload(state.inner())
}

#[tauri::command]
pub fn get_backend_connection(state: State<'_, ShellState>) -> DesktopBackendConnection {
    get_backend_connection_payload(state.inner())
}

#[tauri::command]
pub fn healthcheck(state: State<'_, ShellState>) -> HealthcheckStatusPayload {
    healthcheck_payload(state.inner())
}

#[tauri::command]
pub async fn restart_desktop_backend(
    app: AppHandle,
    state: State<'_, ShellState>,
) -> Result<DesktopBackendConnection, String> {
    let preferences_path = state.preferences_service.path().to_path_buf();
    state
        .backend_supervisor
        .restart(&app, &state.host_state, &preferences_path)
        .await
        .map_err(into_command_error)
}

fn avatar_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[tauri::command]
pub fn pick_avatar_image() -> Result<Option<AvatarUploadPayload>, String> {
    let Some(path) = FileDialog::new()
        .add_filter("Avatar Image", &["png", "jpg", "jpeg", "webp"])
        .pick_file()
    else {
        return Ok(None);
    };

    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| String::from("Invalid avatar file name"))?;

    Ok(Some(AvatarUploadPayload {
        file_name: file_name.to_string(),
        content_type: avatar_content_type(&path).to_string(),
        data_base64: STANDARD.encode(bytes),
        byte_size: metadata.len(),
    }))
}
