use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use octopus_core::{
    AvatarUploadPayload, ConnectionProfile, CreateHostWorkspaceConnectionInput,
    CreateNotificationInput, DesktopBackendConnection, ExportWorkspaceAgentBundleResult, HostState,
    HostUpdateStatus, HostWorkspaceConnectionRecord, NotificationFilter, NotificationListResponse,
    NotificationRecord, NotificationUnreadSummary, ShellPreferences, WorkspaceDirectoryUploadEntry,
    WorkspaceFileUploadPayload,
};
use rfd::FileDialog;
use tauri::{AppHandle, State};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::{
    bootstrap::{
        bootstrap_shell_payload, create_notification as create_notification_payload,
        create_workspace_connection as create_workspace_connection_payload,
        delete_workspace_connection as delete_workspace_connection_payload,
        dismiss_notification_toast as dismiss_notification_toast_payload,
        get_backend_connection_payload, get_host_state_payload,
        get_notification_unread_summary as get_notification_unread_summary_payload,
        healthcheck_payload, list_connections_payload,
        list_notifications as list_notifications_payload,
        list_workspace_connections as list_workspace_connections_payload, load_shell_preferences,
        mark_all_notifications_read as mark_all_notifications_read_payload,
        mark_notification_read as mark_notification_read_payload, save_shell_preferences,
        HealthcheckStatusPayload, ShellBootstrapPayload,
    },
    error::into_command_error,
    state::ShellState,
    updates::{
        check_host_update as check_host_update_payload,
        download_host_update as download_host_update_payload,
        get_host_update_status as get_host_update_status_payload,
        install_host_update as install_host_update_payload,
    },
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
pub fn get_host_update_status(state: State<'_, ShellState>) -> Result<HostUpdateStatus, String> {
    get_host_update_status_payload(state.inner()).map_err(into_command_error)
}

#[tauri::command]
pub async fn check_host_update(
    app: AppHandle,
    state: State<'_, ShellState>,
    channel: Option<String>,
) -> Result<HostUpdateStatus, String> {
    check_host_update_payload(&app, state.inner(), channel.as_deref())
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn download_host_update(
    state: State<'_, ShellState>,
) -> Result<HostUpdateStatus, String> {
    download_host_update_payload(state.inner())
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub fn install_host_update(state: State<'_, ShellState>) -> Result<HostUpdateStatus, String> {
    install_host_update_payload(state.inner()).map_err(into_command_error)
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
pub fn list_workspace_connections(
    state: State<'_, ShellState>,
) -> Result<Vec<HostWorkspaceConnectionRecord>, String> {
    list_workspace_connections_payload(state.inner()).map_err(into_command_error)
}

#[tauri::command]
pub fn create_workspace_connection(
    state: State<'_, ShellState>,
    input: CreateHostWorkspaceConnectionInput,
) -> Result<HostWorkspaceConnectionRecord, String> {
    create_workspace_connection_payload(state.inner(), input).map_err(into_command_error)
}

#[tauri::command]
pub fn delete_workspace_connection(
    state: State<'_, ShellState>,
    workspace_connection_id: String,
) -> Result<(), String> {
    delete_workspace_connection_payload(state.inner(), &workspace_connection_id)
        .map_err(into_command_error)
}

#[tauri::command]
pub fn list_notifications(
    state: State<'_, ShellState>,
    filter: NotificationFilter,
) -> Result<NotificationListResponse, String> {
    list_notifications_payload(state.inner(), filter).map_err(into_command_error)
}

#[tauri::command]
pub fn create_notification(
    state: State<'_, ShellState>,
    input: CreateNotificationInput,
) -> Result<NotificationRecord, String> {
    create_notification_payload(state.inner(), input).map_err(into_command_error)
}

#[tauri::command]
pub fn mark_notification_read(
    state: State<'_, ShellState>,
    id: String,
) -> Result<NotificationRecord, String> {
    mark_notification_read_payload(state.inner(), &id).map_err(into_command_error)
}

#[tauri::command]
pub fn mark_all_notifications_read(
    state: State<'_, ShellState>,
    filter: NotificationFilter,
) -> Result<NotificationUnreadSummary, String> {
    mark_all_notifications_read_payload(state.inner(), filter).map_err(into_command_error)
}

#[tauri::command]
pub fn dismiss_notification_toast(
    state: State<'_, ShellState>,
    id: String,
) -> Result<NotificationRecord, String> {
    dismiss_notification_toast_payload(state.inner(), &id).map_err(into_command_error)
}

#[tauri::command]
pub fn get_notification_unread_summary(
    state: State<'_, ShellState>,
) -> Result<NotificationUnreadSummary, String> {
    get_notification_unread_summary_payload(state.inner()).map_err(into_command_error)
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

#[tauri::command]
pub fn pick_resource_directory() -> Result<Option<String>, String> {
    let Some(path) = FileDialog::new().pick_folder() else {
        return Ok(None);
    };

    Ok(Some(path.to_string_lossy().to_string()))
}

fn generic_file_payload(
    path: &Path,
    content_type: &str,
) -> Result<WorkspaceFileUploadPayload, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| String::from("Invalid file name"))?;

    Ok(WorkspaceFileUploadPayload {
        file_name: file_name.to_string(),
        content_type: content_type.to_string(),
        data_base64: STANDARD.encode(bytes),
        byte_size: metadata.len(),
    })
}

#[tauri::command]
pub fn pick_resource_file() -> Result<Option<WorkspaceFileUploadPayload>, String> {
    let Some(path) = FileDialog::new().pick_file() else {
        return Ok(None);
    };

    let path_label = path.to_string_lossy().to_string();
    Ok(Some(generic_file_payload(
        &path,
        content_type_for_path(&path_label),
    )?))
}

fn generic_directory_payload(
    file_name: &str,
    relative_path: String,
    content_type: &str,
    bytes: &[u8],
) -> WorkspaceDirectoryUploadEntry {
    WorkspaceDirectoryUploadEntry {
        relative_path,
        file_name: file_name.to_string(),
        content_type: content_type.to_string(),
        data_base64: STANDARD.encode(bytes),
        byte_size: bytes.len() as u64,
    }
}

fn content_type_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".md") {
        "text/markdown"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn trim_single_root_directory(
    entries: Vec<WorkspaceDirectoryUploadEntry>,
) -> Vec<WorkspaceDirectoryUploadEntry> {
    let top_level_names = entries
        .iter()
        .filter_map(|entry| entry.relative_path.split('/').next())
        .filter(|segment| !segment.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    if top_level_names.len() != 1 {
        return entries;
    }

    let root_prefix = format!(
        "{}/",
        top_level_names.into_iter().next().unwrap_or_default()
    );
    entries
        .into_iter()
        .map(|mut entry| {
            if entry.relative_path.starts_with(&root_prefix) {
                entry.relative_path = entry.relative_path[root_prefix.len()..].to_string();
            }
            entry
        })
        .collect()
}

#[tauri::command]
pub fn pick_skill_archive() -> Result<Option<Vec<WorkspaceFileUploadPayload>>, String> {
    let Some(paths) = FileDialog::new()
        .add_filter("Skill archive", &["zip"])
        .pick_files()
    else {
        return Ok(None);
    };

    Ok(Some(
        paths
            .iter()
            .map(|path| generic_file_payload(path, "application/zip"))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn read_folder_entries(
    root: &Path,
    current: &Path,
) -> Result<Vec<WorkspaceDirectoryUploadEntry>, String> {
    let mut entries = Vec::new();
    for child in fs::read_dir(current).map_err(|error| error.to_string())? {
        let child = child.map_err(|error| error.to_string())?;
        let path = child.path();
        if path.is_dir() {
            entries.extend(read_folder_entries(root, &path)?);
            continue;
        }

        let payload = generic_file_payload(&path, "application/octet-stream")?;
        let relative_path = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        entries.push(WorkspaceDirectoryUploadEntry {
            relative_path,
            file_name: payload.file_name,
            content_type: payload.content_type,
            data_base64: payload.data_base64,
            byte_size: payload.byte_size,
        });
    }
    Ok(entries)
}

#[tauri::command]
pub fn pick_resource_folder() -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, String> {
    let Some(path) = FileDialog::new().pick_folder() else {
        return Ok(None);
    };
    let root = path.parent().unwrap_or(path.as_path());
    Ok(Some(read_folder_entries(root, &path)?))
}

#[tauri::command]
pub fn pick_skill_folder() -> Result<Option<Vec<Vec<WorkspaceDirectoryUploadEntry>>>, String> {
    let Some(paths) = FileDialog::new().pick_folders() else {
        return Ok(None);
    };
    Ok(Some(
        paths
            .iter()
            .map(|path| read_folder_entries(path, path))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

#[tauri::command]
pub async fn pick_agent_bundle_folder() -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, String>
{
    let Some(path) = FileDialog::new().pick_folder() else {
        return Ok(None);
    };

    tauri::async_runtime::spawn_blocking(move || read_folder_entries(&path, &path))
        .await
        .map_err(|error| error.to_string())?
        .map(Some)
}

fn read_archive_entries(path: &Path) -> Result<Vec<WorkspaceDirectoryUploadEntry>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let mut item = archive.by_index(index).map_err(|error| error.to_string())?;
        if item.is_dir() {
            continue;
        }
        let enclosed = item
            .enclosed_name()
            .map(PathBuf::from)
            .ok_or_else(|| String::from("Archive contains an unsafe file path"))?;
        let relative_path = enclosed.to_string_lossy().replace('\\', "/");
        if relative_path.starts_with("__MACOSX/") {
            continue;
        }

        let mut content = Vec::new();
        item.read_to_end(&mut content)
            .map_err(|error| error.to_string())?;
        let file_name = enclosed
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| String::from("Archive contains a file with an invalid name"))?;
        entries.push(generic_directory_payload(
            file_name,
            relative_path,
            content_type_for_path(file_name),
            &content,
        ));
    }

    Ok(trim_single_root_directory(entries))
}

#[tauri::command]
pub async fn pick_agent_bundle_archive(
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, String> {
    let Some(path) = FileDialog::new()
        .add_filter("Agent bundle archive", &["zip"])
        .pick_file()
    else {
        return Ok(None);
    };

    tauri::async_runtime::spawn_blocking(move || read_archive_entries(&path))
        .await
        .map_err(|error| error.to_string())?
        .map(Some)
}

fn decode_directory_entry(entry: &WorkspaceDirectoryUploadEntry) -> Result<Vec<u8>, String> {
    STANDARD
        .decode(&entry.data_base64)
        .map_err(|error| error.to_string())
}

fn write_bundle_entries(
    root: &Path,
    entries: &[WorkspaceDirectoryUploadEntry],
) -> Result<(), String> {
    for entry in entries {
        let relative_path = Path::new(&entry.relative_path);
        let destination = root.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&destination, decode_directory_entry(entry)?)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn save_agent_bundle_folder(
    export_payload: ExportWorkspaceAgentBundleResult,
) -> Result<(), String> {
    let Some(root) = FileDialog::new().pick_folder() else {
        return Ok(());
    };
    tauri::async_runtime::spawn_blocking(move || write_bundle_entries(&root, &export_payload.files))
        .await
        .map_err(|error| error.to_string())?
}

fn build_bundle_archive_bytes(
    export_payload: &ExportWorkspaceAgentBundleResult,
) -> Result<Vec<u8>, String> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    for entry in &export_payload.files {
        writer
            .start_file(entry.relative_path.replace('\\', "/"), options)
            .map_err(|error| error.to_string())?;
        writer
            .write_all(&decode_directory_entry(entry)?)
            .map_err(|error| error.to_string())?;
    }

    writer
        .finish()
        .map_err(|error| error.to_string())
        .map(|cursor| cursor.into_inner())
}

#[tauri::command]
pub async fn save_agent_bundle_zip(
    export_payload: ExportWorkspaceAgentBundleResult,
) -> Result<(), String> {
    let suggested_name = format!("{}.zip", export_payload.root_dir_name);
    let Some(path) = FileDialog::new().set_file_name(&suggested_name).save_file() else {
        return Ok(());
    };
    tauri::async_runtime::spawn_blocking(move || {
        fs::write(path, build_bundle_archive_bytes(&export_payload)?)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
