use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use octopus_core::{
    normalize_connection_base_url, normalize_notification_filter_scope,
    notification_list_response_from_records, AppError, CreateNotificationInput,
    HostWorkspaceConnectionRecord, NotificationFilter, NotificationListResponse,
    NotificationRecord, NotificationUnreadSummary, PreferencesPort, ShellPreferences,
    timestamp_now,
};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PreferencesService {
    path: PathBuf,
    defaults: ShellPreferences,
}

impl PreferencesService {
    pub fn new(path: PathBuf, defaults: ShellPreferences) -> Self {
        Self { path, defaults }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_parent_dir(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }
}

impl PreferencesPort for PreferencesService {
    fn load_preferences(&self) -> Result<ShellPreferences, AppError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => Ok(serde_json::from_str(&raw)?),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(self.defaults.clone()),
            Err(error) => Err(error.into()),
        }
    }

    fn save_preferences(
        &self,
        preferences: &ShellPreferences,
    ) -> Result<ShellPreferences, AppError> {
        self.ensure_parent_dir()?;
        fs::write(&self.path, serde_json::to_vec_pretty(preferences)?)?;
        Ok(preferences.clone())
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceConnectionRegistryService {
    path: PathBuf,
}

impl WorkspaceConnectionRegistryService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_parent_dir(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    pub fn load_connections(&self) -> Result<Vec<HostWorkspaceConnectionRecord>, AppError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => Ok(serde_json::from_str(&raw)?),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(vec![]),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save_connections(
        &self,
        connections: &[HostWorkspaceConnectionRecord],
    ) -> Result<Vec<HostWorkspaceConnectionRecord>, AppError> {
        self.ensure_parent_dir()?;
        fs::write(&self.path, serde_json::to_vec_pretty(connections)?)?;
        Ok(connections.to_vec())
    }

    pub fn delete_connection(
        &self,
        workspace_connection_id: &str,
    ) -> Result<Vec<HostWorkspaceConnectionRecord>, AppError> {
        let next_connections = self
            .load_connections()?
            .into_iter()
            .filter(|connection| connection.workspace_connection_id != workspace_connection_id)
            .collect::<Vec<_>>();

        self.save_connections(&next_connections)
    }

    pub fn upsert_connection(
        &self,
        connection: HostWorkspaceConnectionRecord,
    ) -> Result<HostWorkspaceConnectionRecord, AppError> {
        let normalized_base_url = normalize_connection_base_url(&connection.base_url);
        let mut existing_connections = self.load_connections()?;

        if let Some(existing) = existing_connections.iter_mut().find(|item| {
            normalize_connection_base_url(&item.base_url) == normalized_base_url
                && item.workspace_id == connection.workspace_id
        }) {
            existing.label = connection.label;
            existing.base_url = normalized_base_url;
            existing.transport_security = connection.transport_security;
            existing.auth_mode = connection.auth_mode;
            existing.last_used_at = connection.last_used_at;
            existing.status = connection.status;
            let persisted = existing.clone();
            self.save_connections(&existing_connections)?;
            return Ok(persisted);
        }

        existing_connections.push(HostWorkspaceConnectionRecord {
            base_url: normalized_base_url,
            ..connection
        });
        self.save_connections(&existing_connections)?;
        existing_connections
            .last()
            .cloned()
            .ok_or_else(|| AppError::runtime("workspace connection registry save failed"))
    }
}

#[derive(Debug, Clone)]
pub struct NotificationService {
    path: PathBuf,
}

impl NotificationService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_parent_dir(&self) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    fn open(&self) -> Result<Connection, AppError> {
        self.ensure_parent_dir()?;
        let connection = Connection::open(&self.path)
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS notifications (
                    id TEXT PRIMARY KEY NOT NULL,
                    scope_kind TEXT NOT NULL,
                    scope_owner_id TEXT,
                    level TEXT NOT NULL,
                    title TEXT NOT NULL,
                    body TEXT NOT NULL,
                    source TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    read_at INTEGER,
                    toast_visible_until INTEGER,
                    route_to TEXT,
                    action_label TEXT
                );",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(connection)
    }

    fn map_notification(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationRecord> {
        Ok(NotificationRecord {
            id: row.get("id")?,
            scope_kind: row.get("scope_kind")?,
            scope_owner_id: row.get("scope_owner_id")?,
            level: row.get("level")?,
            title: row.get("title")?,
            body: row.get("body")?,
            source: row.get("source")?,
            created_at: row.get::<_, i64>("created_at")? as u64,
            read_at: row.get::<_, Option<i64>>("read_at")?.map(|value| value as u64),
            toast_visible_until: row
                .get::<_, Option<i64>>("toast_visible_until")?
                .map(|value| value as u64),
            route_to: row.get("route_to")?,
            action_label: row.get("action_label")?,
        })
    }

    pub fn list_notifications(
        &self,
        filter: NotificationFilter,
    ) -> Result<NotificationListResponse, AppError> {
        let connection = self.open()?;
        let scope = normalize_notification_filter_scope(filter.scope.as_deref());
        let mut statement = if scope.is_some() {
            connection
                .prepare(
                    "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                     FROM notifications
                     WHERE scope_kind = ?1
                     ORDER BY created_at DESC, id DESC",
                )
                .map_err(|error| AppError::database(error.to_string()))?
        } else {
            connection
                .prepare(
                    "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                     FROM notifications
                     ORDER BY created_at DESC, id DESC",
                )
                .map_err(|error| AppError::database(error.to_string()))?
        };

        let notifications = if let Some(scope) = scope {
            statement
                .query_map(params![scope], Self::map_notification)
                .map_err(|error| AppError::database(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| AppError::database(error.to_string()))?
        } else {
            statement
                .query_map([], Self::map_notification)
                .map_err(|error| AppError::database(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| AppError::database(error.to_string()))?
        };

        Ok(notification_list_response_from_records(notifications))
    }

    pub fn unread_summary(&self) -> Result<NotificationUnreadSummary, AppError> {
        Ok(self.list_notifications(NotificationFilter { scope: None })?.unread)
    }

    pub fn create_notification(
        &self,
        input: CreateNotificationInput,
    ) -> Result<NotificationRecord, AppError> {
        let now = timestamp_now();
        let notification = NotificationRecord {
            id: format!("notif-{}", Uuid::new_v4()),
            scope_kind: input.scope_kind,
            scope_owner_id: input.scope_owner_id,
            level: if input.level.trim().is_empty() {
                "info".into()
            } else {
                input.level
            },
            title: if input.title.trim().is_empty() {
                "Notification".into()
            } else {
                input.title
            },
            body: input.body,
            source: if input.source.trim().is_empty() {
                "system".into()
            } else {
                input.source
            },
            created_at: now,
            read_at: None,
            toast_visible_until: input.toast_duration_ms.map(|duration| now + duration),
            route_to: input.route_to,
            action_label: input.action_label,
        };

        let connection = self.open()?;
        connection
            .execute(
                "INSERT INTO notifications (
                    id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    notification.id,
                    notification.scope_kind,
                    notification.scope_owner_id,
                    notification.level,
                    notification.title,
                    notification.body,
                    notification.source,
                    notification.created_at as i64,
                    notification.read_at.map(|value| value as i64),
                    notification.toast_visible_until.map(|value| value as i64),
                    notification.route_to,
                    notification.action_label,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        self.get_notification(&notification.id)
    }

    pub fn get_notification(&self, id: &str) -> Result<NotificationRecord, AppError> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                 FROM notifications
                 WHERE id = ?1",
                params![id],
                Self::map_notification,
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .ok_or_else(|| AppError::not_found(format!("notification {id} not found")))
    }

    pub fn mark_notification_read(&self, id: &str) -> Result<NotificationRecord, AppError> {
        let connection = self.open()?;
        connection
            .execute(
                "UPDATE notifications
                 SET read_at = COALESCE(read_at, ?2)
                 WHERE id = ?1",
                params![id, timestamp_now() as i64],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.get_notification(id)
    }

    pub fn mark_all_notifications_read(
        &self,
        filter: NotificationFilter,
    ) -> Result<NotificationUnreadSummary, AppError> {
        let connection = self.open()?;
        let scope = normalize_notification_filter_scope(filter.scope.as_deref());
        if let Some(scope) = scope {
            connection
                .execute(
                    "UPDATE notifications
                     SET read_at = COALESCE(read_at, ?2)
                     WHERE scope_kind = ?1",
                    params![scope, timestamp_now() as i64],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        } else {
            connection
                .execute(
                    "UPDATE notifications
                     SET read_at = COALESCE(read_at, ?1)",
                    params![timestamp_now() as i64],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        self.unread_summary()
    }

    pub fn dismiss_notification_toast(&self, id: &str) -> Result<NotificationRecord, AppError> {
        let connection = self.open()?;
        connection
            .execute(
                "UPDATE notifications
                 SET toast_visible_until = NULL
                 WHERE id = ?1",
                params![id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.get_notification(id)
    }
}
