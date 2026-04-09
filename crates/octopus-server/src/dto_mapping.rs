use super::*;

pub(crate) fn build_healthcheck_status(state: &ServerState) -> HealthcheckStatus {
    HealthcheckStatus {
        status: "ok".into(),
        host: state.host_state.platform.clone(),
        mode: state.host_state.mode.clone(),
        cargo_workspace: state.host_state.cargo_workspace,
        backend: HealthcheckBackendStatus {
            state: state.backend_connection.state.clone(),
            transport: state.backend_connection.transport.clone(),
        },
    }
}
pub(crate) fn map_notification(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationRecord> {
    Ok(NotificationRecord {
        id: row.get(0)?,
        scope_kind: row.get(1)?,
        scope_owner_id: row.get(2)?,
        level: row.get(3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        source: row.get(6)?,
        created_at: row.get::<_, i64>(7)? as u64,
        read_at: row.get::<_, Option<i64>>(8)?.map(|value| value as u64),
        toast_visible_until: row.get::<_, Option<i64>>(9)?.map(|value| value as u64),
        route_to: row.get(10)?,
        action_label: row.get(11)?,
    })
}
pub(crate) fn metric_record(id: &str, label: &str, value: usize) -> WorkspaceMetricRecord {
    WorkspaceMetricRecord {
        id: id.into(),
        label: label.into(),
        value: value.to_string(),
        helper: None,
        tone: None,
    }
}
pub(crate) fn build_user_center_alerts(
    session: &SessionRecord,
    permissions: &[PermissionRecord],
) -> Vec<UserCenterAlertRecord> {
    let mut alerts = Vec::new();
    if session.scope_project_ids.is_empty() {
        alerts.push(UserCenterAlertRecord {
            id: "alert-workspace-scope".into(),
            title: "Workspace scope active".into(),
            description: "Current session can access the full workspace scope.".into(),
            severity: "low".into(),
        });
    }
    if permissions.is_empty() {
        alerts.push(UserCenterAlertRecord {
            id: "alert-missing-permissions".into(),
            title: "RBAC not configured".into(),
            description: "No permissions are available for the current workspace.".into(),
            severity: "medium".into(),
        });
    }
    alerts
}
