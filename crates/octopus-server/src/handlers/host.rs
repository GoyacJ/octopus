use super::*;

fn create_host_notifications_table(connection: &Connection) -> Result<(), AppError> {
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
        .map_err(|error| AppError::database(error.to_string()))
}

static HOST_NOTIFICATION_MIGRATIONS: &[Migration] = &[Migration {
    key: "0001-host-notifications",
    apply: create_host_notifications_table,
}];

pub(crate) fn load_host_preferences(state: &ServerState) -> Result<ShellPreferences, ApiError> {
    match fs::read_to_string(&state.host_preferences_path) {
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|error| ApiError::from(AppError::from(error)))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(state.host_default_preferences.clone())
        }
        Err(error) => Err(ApiError::from(AppError::from(error))),
    }
}

pub(crate) fn save_host_preferences(
    state: &ServerState,
    preferences: &ShellPreferences,
) -> Result<ShellPreferences, ApiError> {
    if let Some(parent) = state.host_preferences_path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::from(AppError::from(error)))?;
    }
    fs::write(
        &state.host_preferences_path,
        serde_json::to_vec_pretty(preferences)
            .map_err(|error| ApiError::from(AppError::from(error)))?,
    )
    .map_err(|error| ApiError::from(AppError::from(error)))?;
    Ok(preferences.clone())
}

pub(crate) fn normalize_host_update_channel(value: Option<&str>, fallback: &str) -> String {
    match value.map(str::trim) {
        Some("preview") => "preview".into(),
        Some("formal") => "formal".into(),
        _ => match fallback.trim() {
            "preview" => "preview".into(),
            _ => "formal".into(),
        },
    }
}

pub(crate) fn default_browser_host_update_status(
    state: &ServerState,
    channel: &str,
) -> HostUpdateStatus {
    let mut status = default_host_update_status(state.host_state.app_version.clone(), channel);
    let config = update_runtime_config(channel);
    status.capabilities.can_check = config.endpoint.is_some();
    status.capabilities.can_download = false;
    status.capabilities.can_install = false;
    status.capabilities.supports_channels = true;
    status
}

pub(crate) async fn load_host_update_status(
    state: &ServerState,
    requested_channel: Option<&str>,
) -> Result<HostUpdateStatus, ApiError> {
    let preferences = load_host_preferences(state)?;
    let channel = normalize_host_update_channel(requested_channel, &preferences.update_channel);
    refresh_browser_host_update_status(state, &channel).await
}

pub(crate) async fn check_host_update(
    state: &ServerState,
    requested_channel: Option<&str>,
) -> Result<HostUpdateStatus, ApiError> {
    let preferences = load_host_preferences(state)?;
    let channel = normalize_host_update_channel(requested_channel, &preferences.update_channel);
    refresh_browser_host_update_status(state, &channel).await
}

pub(crate) fn unsupported_host_update_action(
    state: &ServerState,
    requested_channel: Option<&str>,
    error_code: &str,
    error_message: &str,
) -> Result<HostUpdateStatus, ApiError> {
    let preferences = load_host_preferences(state)?;
    let channel = normalize_host_update_channel(requested_channel, &preferences.update_channel);
    let mut status = default_browser_host_update_status(state, &channel);
    status.state = "error".into();
    status.error_code = Some(error_code.into());
    status.error_message = Some(error_message.into());
    Ok(status)
}

pub(crate) async fn refresh_browser_host_update_status(
    state: &ServerState,
    channel: &str,
) -> Result<HostUpdateStatus, ApiError> {
    let runtime_config = update_runtime_config(channel);
    refresh_browser_host_update_status_with_endpoint(
        state,
        channel,
        runtime_config.endpoint.as_deref(),
    )
    .await
}

pub(crate) async fn refresh_browser_host_update_status_with_endpoint(
    state: &ServerState,
    channel: &str,
    endpoint: Option<&str>,
) -> Result<HostUpdateStatus, ApiError> {
    let mut status = default_browser_host_update_status(state, channel);
    let Some(endpoint) = endpoint else {
        return Ok(status);
    };

    status.last_checked_at = Some(timestamp_now());

    match fetch_remote_update_manifest(endpoint).await {
        Ok(manifest) => {
            let latest_version = manifest
                .version
                .clone()
                .unwrap_or_else(|| state.host_state.app_version.clone());
            let latest_channel = manifest
                .channel
                .clone()
                .unwrap_or_else(|| normalize_host_update_channel(Some(channel), channel));
            status.latest_release = Some(HostReleaseSummary {
                version: latest_version.clone(),
                channel: latest_channel,
                published_at: manifest
                    .pub_date
                    .unwrap_or_else(|| "1970-01-01T00:00:00Z".into()),
                notes: manifest.notes,
                notes_url: manifest.notes_url,
            });
            status.state = if latest_version == state.host_state.app_version {
                "up_to_date".into()
            } else {
                "update_available".into()
            };
            Ok(status)
        }
        Err(error) => {
            status.state = "error".into();
            status.error_code = Some("UPDATE_CHECK_FAILED".into());
            status.error_message = Some(format!("无法连接更新服务，请稍后重试。{error}"));
            Ok(status)
        }
    }
}

pub(crate) async fn fetch_remote_update_manifest(
    endpoint: &str,
) -> Result<RemoteUpdateManifest, AppError> {
    let response = Client::new()
        .get(endpoint)
        .header(reqwest::header::USER_AGENT, "octopus-browser-host")
        .send()
        .await
        .map_err(|error| AppError::runtime(format!("failed to fetch update manifest: {error}")))?;
    let response = response
        .error_for_status()
        .map_err(|error| AppError::runtime(format!("update manifest request failed: {error}")))?;
    response
        .json::<RemoteUpdateManifest>()
        .await
        .map_err(|error| AppError::runtime(format!("failed to parse update manifest: {error}")))
}

pub(crate) fn update_runtime_config(channel: &str) -> UpdateRuntimeConfig {
    let built_in = load_product_update_config();
    UpdateRuntimeConfig {
        endpoint: env_var(update_endpoint_env(channel))
            .or_else(|| built_in.endpoint_for_channel(channel)),
        _pubkey: env_var(UPDATE_PUBKEY_ENV).or_else(|| built_in.pubkey()),
    }
}

#[derive(Clone, Default)]
pub(crate) struct UpdateRuntimeConfig {
    endpoint: Option<String>,
    _pubkey: Option<String>,
}

pub(crate) fn update_endpoint_env(channel: &str) -> &'static str {
    match normalize_host_update_channel(Some(channel), "formal").as_str() {
        "preview" => UPDATE_ENDPOINT_PREVIEW_ENV,
        _ => UPDATE_ENDPOINT_FORMAL_ENV,
    }
}

pub(crate) fn env_var(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn load_product_update_config() -> ProductUpdateConfig {
    serde_json::from_str::<ProductUpdateConfig>(BUILTIN_UPDATER_CONFIG)
        .unwrap_or_default()
        .normalized()
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
        match normalize_host_update_channel(Some(channel), "formal").as_str() {
            "preview" => self.preview_endpoint.clone(),
            _ => self.formal_endpoint.clone(),
        }
    }

    fn pubkey(&self) -> Option<String> {
        self.pubkey.clone()
    }
}

pub(crate) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(crate) fn load_remote_host_workspace_connections(
    state: &ServerState,
) -> Result<Vec<HostWorkspaceConnectionRecord>, ApiError> {
    match fs::read_to_string(&state.host_workspace_connections_path) {
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|error| ApiError::from(AppError::from(error)))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(error) => Err(ApiError::from(AppError::from(error))),
    }
}

pub(crate) fn save_remote_host_workspace_connections(
    state: &ServerState,
    connections: &[HostWorkspaceConnectionRecord],
) -> Result<Vec<HostWorkspaceConnectionRecord>, ApiError> {
    if let Some(parent) = state.host_workspace_connections_path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::from(AppError::from(error)))?;
    }
    fs::write(
        &state.host_workspace_connections_path,
        serde_json::to_vec_pretty(connections)
            .map_err(|error| ApiError::from(AppError::from(error)))?,
    )
    .map_err(|error| ApiError::from(AppError::from(error)))?;
    Ok(connections.to_vec())
}

pub(crate) fn list_host_workspace_connections(
    state: &ServerState,
) -> Result<Vec<HostWorkspaceConnectionRecord>, ApiError> {
    let mut connections = state
        .host_connections
        .iter()
        .map(|connection| {
            host_workspace_connection_record_from_profile(
                connection,
                Some(&state.backend_connection),
            )
        })
        .collect::<Vec<_>>();
    connections.extend(load_remote_host_workspace_connections(state)?);
    Ok(connections)
}

pub(crate) fn create_host_workspace_connection(
    state: &ServerState,
    input: CreateHostWorkspaceConnectionInput,
) -> Result<HostWorkspaceConnectionRecord, ApiError> {
    let mut existing_connections = load_remote_host_workspace_connections(state)?;
    let normalized_base_url = normalize_connection_base_url(&input.base_url);

    if let Some(existing) = existing_connections.iter_mut().find(|connection| {
        normalize_connection_base_url(&connection.base_url) == normalized_base_url
            && connection.workspace_id == input.workspace_id
    }) {
        existing.label = input.label;
        existing.base_url = normalized_base_url;
        existing.transport_security = input.transport_security;
        existing.auth_mode = input.auth_mode;
        existing.last_used_at = Some(timestamp_now());
        existing.status = "connected".into();
        let persisted = existing.clone();
        save_remote_host_workspace_connections(state, &existing_connections)?;
        return Ok(persisted);
    }

    let created = HostWorkspaceConnectionRecord {
        workspace_connection_id: format!("conn-remote-{}-{}", input.workspace_id, timestamp_now()),
        workspace_id: input.workspace_id,
        label: input.label,
        base_url: normalized_base_url,
        transport_security: input.transport_security,
        auth_mode: input.auth_mode,
        last_used_at: Some(timestamp_now()),
        status: "connected".into(),
    };
    existing_connections.push(created.clone());
    save_remote_host_workspace_connections(state, &existing_connections)?;
    Ok(created)
}

pub(crate) fn delete_host_workspace_connection(
    state: &ServerState,
    connection_id: &str,
) -> Result<(), ApiError> {
    if state
        .host_connections
        .iter()
        .any(|connection| connection.id == connection_id)
    {
        return Err(ApiError::from(AppError::invalid_input(
            "local workspace connection cannot be deleted",
        )));
    }

    let next_connections = load_remote_host_workspace_connections(state)?
        .into_iter()
        .filter(|connection| connection.workspace_connection_id != connection_id)
        .collect::<Vec<_>>();
    save_remote_host_workspace_connections(state, &next_connections)?;
    Ok(())
}

pub(crate) fn host_notifications_db_path(state: &ServerState) -> PathBuf {
    state
        .host_preferences_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("data/main.db")
}

pub(crate) fn open_host_notifications_db(state: &ServerState) -> Result<Connection, ApiError> {
    let database = Database::open(host_notifications_db_path(state))
        .map(|database| database.with_migrations(HOST_NOTIFICATION_MIGRATIONS))
        .map_err(ApiError::from)?;
    database.run_migrations().map_err(ApiError::from)?;
    database.acquire().map_err(ApiError::from)
}

pub(crate) fn list_host_notifications(
    state: &ServerState,
    filter: NotificationFilter,
) -> Result<NotificationListResponse, ApiError> {
    let connection = open_host_notifications_db(state)?;
    let scope = normalize_notification_filter_scope(filter.scope.as_deref());
    let mut statement = if scope.is_some() {
        connection
            .prepare(
                "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                 FROM notifications
                 WHERE scope_kind = ?1
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?
    } else {
        connection
            .prepare(
                "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
                 FROM notifications
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?
    };

    let mapped = if let Some(scope) = scope {
        statement.query_map(params![scope], map_notification)
    } else {
        statement.query_map([], map_notification)
    }
    .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    let notifications = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    Ok(notification_list_response_from_records(notifications))
}

pub(crate) fn create_host_notification(
    state: &ServerState,
    input: CreateNotificationInput,
) -> Result<NotificationRecord, ApiError> {
    let now = timestamp_now();
    let scope_kind = match input.scope_kind.trim() {
        "workspace" => "workspace",
        "user" => "user",
        _ => "app",
    };
    let notification = NotificationRecord {
        id: format!("notif-{}", Uuid::new_v4()),
        scope_kind: scope_kind.into(),
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

    let connection = open_host_notifications_db(state)?;
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
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    get_host_notification(state, &notification.id)
}

pub(crate) fn get_host_notification(
    state: &ServerState,
    notification_id: &str,
) -> Result<NotificationRecord, ApiError> {
    let connection = open_host_notifications_db(state)?;
    connection
        .query_row(
            "SELECT id, scope_kind, scope_owner_id, level, title, body, source, created_at, read_at, toast_visible_until, route_to, action_label
             FROM notifications
             WHERE id = ?1",
            params![notification_id],
            map_notification,
        )
        .optional()
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?
        .ok_or_else(|| ApiError::from(AppError::not_found(format!("notification {notification_id} not found"))))
}

pub(crate) fn get_host_notification_unread_summary(
    state: &ServerState,
) -> Result<NotificationUnreadSummary, ApiError> {
    let connection = open_host_notifications_db(state)?;
    let mut statement = connection
        .prepare("SELECT scope_kind, COUNT(*) FROM notifications WHERE read_at IS NULL GROUP BY scope_kind")
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    let counts = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;

    let mut summary = create_default_notification_unread_summary();
    for item in counts {
        let (scope, count) =
            item.map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
        let count = count.max(0) as u64;
        summary.total += count;
        match scope.as_str() {
            "workspace" => summary.by_scope.workspace += count,
            "user" => summary.by_scope.user += count,
            _ => summary.by_scope.app += count,
        }
    }

    Ok(summary)
}

pub(crate) fn mark_host_notification_read(
    state: &ServerState,
    notification_id: &str,
) -> Result<NotificationRecord, ApiError> {
    let connection = open_host_notifications_db(state)?;
    connection
        .execute(
            "UPDATE notifications
             SET read_at = COALESCE(read_at, ?2)
             WHERE id = ?1",
            params![notification_id, timestamp_now() as i64],
        )
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    get_host_notification(state, notification_id)
}

pub(crate) fn mark_all_host_notifications_read(
    state: &ServerState,
    filter: NotificationFilter,
) -> Result<NotificationUnreadSummary, ApiError> {
    let connection = open_host_notifications_db(state)?;
    let scope = normalize_notification_filter_scope(filter.scope.as_deref());
    if let Some(scope) = scope {
        connection
            .execute(
                "UPDATE notifications
                 SET read_at = COALESCE(read_at, ?2)
                 WHERE scope_kind = ?1",
                params![scope, timestamp_now() as i64],
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    } else {
        connection
            .execute(
                "UPDATE notifications
                 SET read_at = COALESCE(read_at, ?1)",
                params![timestamp_now() as i64],
            )
            .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    }

    get_host_notification_unread_summary(state)
}

pub(crate) fn dismiss_host_notification_toast(
    state: &ServerState,
    notification_id: &str,
) -> Result<NotificationRecord, ApiError> {
    let connection = open_host_notifications_db(state)?;
    connection
        .execute(
            "UPDATE notifications
             SET toast_visible_until = NULL
             WHERE id = ?1",
            params![notification_id],
        )
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    get_host_notification(state, notification_id)
}

pub(crate) fn ensure_host_authorized(
    state: &ServerState,
    headers: &HeaderMap,
    request_id: &str,
) -> Result<(), ApiError> {
    let token = extract_bearer(headers)
        .ok_or_else(|| ApiError::new(AppError::auth("missing bearer token"), request_id))?;
    if token != state.host_auth_token {
        return Err(ApiError::new(
            AppError::auth("invalid bearer token"),
            request_id,
        ));
    }
    Ok(())
}

pub(crate) async fn host_bootstrap(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ShellBootstrap>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    let connections = list_host_workspace_connections(&state)?
        .iter()
        .map(connection_profile_from_host_workspace_connection)
        .collect::<Vec<_>>();

    Ok(Json(ShellBootstrap {
        host_state: state.host_state.clone(),
        preferences: load_host_preferences(&state)?,
        connections,
        backend: Some(state.backend_connection.clone()),
    }))
}

pub(crate) async fn host_healthcheck(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HealthcheckStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(build_healthcheck_status(&state)))
}

pub(crate) async fn load_host_preferences_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<ShellPreferences>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(load_host_preferences(&state)?))
}

pub(crate) async fn save_host_preferences_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(preferences): Json<ShellPreferences>,
) -> Result<Json<ShellPreferences>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(save_host_preferences(&state, &preferences)?))
}

pub(crate) async fn get_host_update_status_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(load_host_update_status(&state, None).await?))
}

pub(crate) async fn check_host_update_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<HostUpdateCheckRequestPayload>,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(
        check_host_update(&state, request.channel.as_deref()).await?,
    ))
}

pub(crate) async fn download_host_update_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(unsupported_host_update_action(
        &state,
        None,
        "UPDATE_DOWNLOAD_UNSUPPORTED",
        "当前环境不支持应用内下载安装更新。",
    )?))
}

pub(crate) async fn install_host_update_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<HostUpdateStatus>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(unsupported_host_update_action(
        &state,
        None,
        "UPDATE_INSTALL_UNSUPPORTED",
        "当前环境不支持应用内安装更新。",
    )?))
}

pub(crate) async fn list_host_workspace_connections_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<HostWorkspaceConnectionRecord>>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(list_host_workspace_connections(&state)?))
}

pub(crate) async fn create_host_workspace_connection_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateHostWorkspaceConnectionInput>,
) -> Result<Json<HostWorkspaceConnectionRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(create_host_workspace_connection(&state, input)?))
}

pub(crate) async fn delete_host_workspace_connection_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(connection_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    delete_host_workspace_connection(&state, &connection_id)?;
    Ok(Json(()))
}

pub(crate) async fn list_host_notifications_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(filter): Query<NotificationFilter>,
) -> Result<Json<NotificationListResponse>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(list_host_notifications(&state, filter)?))
}

pub(crate) async fn create_host_notification_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreateNotificationInput>,
) -> Result<Json<NotificationRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(create_host_notification(&state, input)?))
}

pub(crate) async fn mark_host_notification_read_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
) -> Result<Json<NotificationRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(mark_host_notification_read(&state, &notification_id)?))
}

pub(crate) async fn mark_all_host_notifications_read_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(filter): Json<NotificationFilter>,
) -> Result<Json<NotificationUnreadSummary>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(mark_all_host_notifications_read(&state, filter)?))
}

pub(crate) async fn dismiss_host_notification_toast_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
) -> Result<Json<NotificationRecord>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(dismiss_host_notification_toast(
        &state,
        &notification_id,
    )?))
}

pub(crate) async fn get_host_notification_unread_summary_route(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<NotificationUnreadSummary>, ApiError> {
    let request_id = request_id(&headers);
    ensure_host_authorized(&state, &headers, &request_id)?;
    Ok(Json(get_host_notification_unread_summary(&state)?))
}
