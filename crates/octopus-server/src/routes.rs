use super::*;
use crate::handlers::*;
use crate::workspace_runtime::*;

pub fn build_router(state: ServerState) -> Router {
    let cors_layer = build_cors_layer(&state.transport_security);

    Router::new()
        .route("/health", get(healthcheck))
        .route("/api/v1/host/bootstrap", get(host_bootstrap))
        .route("/api/v1/host/health", get(host_healthcheck))
        .route(
            "/api/v1/host/preferences",
            get(load_host_preferences_route).put(save_host_preferences_route),
        )
        .route(
            "/api/v1/host/update-status",
            get(get_host_update_status_route),
        )
        .route("/api/v1/host/update-check", post(check_host_update_route))
        .route(
            "/api/v1/host/update-download",
            post(download_host_update_route),
        )
        .route(
            "/api/v1/host/update-install",
            post(install_host_update_route),
        )
        .route(
            "/api/v1/host/workspace-connections",
            get(list_host_workspace_connections_route).post(create_host_workspace_connection_route),
        )
        .route(
            "/api/v1/host/workspace-connections/:connection_id",
            delete(delete_host_workspace_connection_route),
        )
        .route(
            "/api/v1/host/notifications",
            get(list_host_notifications_route).post(create_host_notification_route),
        )
        .route(
            "/api/v1/host/notifications/:notification_id/read",
            post(mark_host_notification_read_route),
        )
        .route(
            "/api/v1/host/notifications/read-all",
            post(mark_all_host_notifications_read_route),
        )
        .route(
            "/api/v1/host/notifications/:notification_id/dismiss-toast",
            post(dismiss_host_notification_toast_route),
        )
        .route(
            "/api/v1/host/notifications/unread-summary",
            get(get_host_notification_unread_summary_route),
        )
        .route("/api/v1/system/health", get(healthcheck))
        .route("/api/v1/system/bootstrap", get(system_bootstrap))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register-owner", post(register_owner))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/apps", get(list_apps).post(register_app))
        .route("/api/v1/workspace", get(workspace))
        .route("/api/v1/workspace/overview", get(workspace_overview))
        .route(
            "/api/v1/workspace/resources",
            get(workspace_resources).post(create_workspace_resource),
        )
        .route(
            "/api/v1/workspace/resources/:resource_id",
            patch(update_workspace_resource).delete(delete_workspace_resource),
        )
        .route("/api/v1/workspace/knowledge", get(workspace_knowledge))
        .route("/api/v1/workspace/pet", get(workspace_pet_snapshot))
        .route(
            "/api/v1/workspace/pet/presence",
            patch(save_workspace_pet_presence),
        )
        .route(
            "/api/v1/workspace/pet/conversation",
            put(bind_workspace_pet_conversation),
        )
        .route(
            "/api/v1/workspace/agents",
            get(list_agents).post(create_agent),
        )
        .route(
            "/api/v1/workspace/agents/import-preview",
            post(preview_import_agent_bundle_route),
        )
        .route(
            "/api/v1/workspace/agents/import",
            post(import_agent_bundle_route),
        )
        .route(
            "/api/v1/workspace/agents/export",
            post(export_agent_bundle_route),
        )
        .route(
            "/api/v1/workspace/agents/:agent_id",
            patch(update_agent).delete(delete_agent),
        )
        .route("/api/v1/workspace/teams", get(list_teams).post(create_team))
        .route(
            "/api/v1/workspace/teams/:team_id",
            patch(update_team).delete(delete_team),
        )
        .route(
            "/api/v1/workspace/catalog/models",
            get(workspace_catalog_models),
        )
        .route(
            "/api/v1/workspace/catalog/provider-credentials",
            get(workspace_provider_credentials),
        )
        .route(
            "/api/v1/workspace/catalog/tool-catalog",
            get(workspace_tool_catalog),
        )
        .route(
            "/api/v1/workspace/catalog/tool-catalog/disable",
            patch(workspace_tool_catalog_disable),
        )
        .route(
            "/api/v1/workspace/catalog/skills",
            post(create_workspace_skill_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id",
            get(get_workspace_skill_route)
                .patch(update_workspace_skill_route)
                .delete(delete_workspace_skill_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/import-archive",
            post(import_workspace_skill_archive_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/import-folder",
            post(import_workspace_skill_folder_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id/tree",
            get(get_workspace_skill_tree_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id/files/*relative_path",
            get(get_workspace_skill_file_route).patch(update_workspace_skill_file_route),
        )
        .route(
            "/api/v1/workspace/catalog/skills/:skill_id/copy-to-managed",
            post(copy_workspace_skill_to_managed_route),
        )
        .route(
            "/api/v1/workspace/catalog/mcp-servers",
            post(create_workspace_mcp_server_route),
        )
        .route(
            "/api/v1/workspace/catalog/mcp-servers/:server_name",
            get(get_workspace_mcp_server_route)
                .patch(update_workspace_mcp_server_route)
                .delete(delete_workspace_mcp_server_route),
        )
        .route(
            "/api/v1/workspace/catalog/tools",
            get(list_tools).post(create_tool),
        )
        .route(
            "/api/v1/workspace/catalog/tools/:tool_id",
            patch(update_tool).delete(delete_tool),
        )
        .route(
            "/api/v1/workspace/automations",
            get(list_automations).post(create_automation),
        )
        .route(
            "/api/v1/workspace/automations/:automation_id",
            patch(update_automation).delete(delete_automation),
        )
        .route(
            "/api/v1/workspace/permission-center/overview",
            get(permission_center_overview),
        )
        .route(
            "/api/v1/workspace/personal-center/profile/runtime-config",
            get(get_user_runtime_config_route).patch(save_user_runtime_config_route),
        )
        .route(
            "/api/v1/workspace/personal-center/profile/runtime-config/validate",
            post(validate_user_runtime_config_route),
        )
        .route(
            "/api/v1/workspace/personal-center/profile",
            patch(update_current_user_profile_route),
        )
        .route(
            "/api/v1/workspace/personal-center/profile/password",
            post(change_current_user_password_route),
        )
        .route(
            "/api/v1/workspace/rbac/users",
            get(list_users).post(create_user),
        )
        .route(
            "/api/v1/workspace/rbac/users/:user_id",
            patch(update_user).delete(delete_user),
        )
        .route(
            "/api/v1/workspace/rbac/roles",
            get(list_roles).post(create_role),
        )
        .route(
            "/api/v1/workspace/rbac/roles/:role_id",
            patch(update_role).delete(delete_role),
        )
        .route(
            "/api/v1/workspace/rbac/permissions",
            get(list_permissions).post(create_permission),
        )
        .route(
            "/api/v1/workspace/rbac/permissions/:permission_id",
            patch(update_permission).delete(delete_permission),
        )
        .route(
            "/api/v1/workspace/rbac/menus",
            get(list_menus).post(create_menu),
        )
        .route("/api/v1/workspace/rbac/menus/:menu_id", patch(update_menu))
        .route("/api/v1/projects", get(projects).post(create_project))
        .route("/api/v1/projects/:project_id", patch(update_project))
        .route(
            "/api/v1/projects/:project_id/dashboard",
            get(project_dashboard),
        )
        .route(
            "/api/v1/projects/:project_id/runtime-config",
            get(get_project_runtime_config_route).patch(save_project_runtime_config_route),
        )
        .route(
            "/api/v1/projects/:project_id/runtime-config/validate",
            post(validate_project_runtime_config_route),
        )
        .route(
            "/api/v1/projects/:project_id/resources",
            get(project_resources).post(create_project_resource),
        )
        .route(
            "/api/v1/projects/:project_id/resources/folder",
            post(create_project_resource_folder),
        )
        .route(
            "/api/v1/projects/:project_id/resources/:resource_id",
            patch(update_project_resource).delete(delete_project_resource),
        )
        .route(
            "/api/v1/projects/:project_id/knowledge",
            get(project_knowledge),
        )
        .route(
            "/api/v1/projects/:project_id/pet",
            get(project_pet_snapshot),
        )
        .route(
            "/api/v1/projects/:project_id/pet/presence",
            patch(save_project_pet_presence),
        )
        .route(
            "/api/v1/projects/:project_id/pet/conversation",
            put(bind_project_pet_conversation),
        )
        .route(
            "/api/v1/projects/:project_id/agent-links",
            get(list_project_agent_links).post(link_project_agent),
        )
        .route(
            "/api/v1/projects/:project_id/agents/import-preview",
            post(preview_import_project_agent_bundle_route),
        )
        .route(
            "/api/v1/projects/:project_id/agents/import",
            post(import_project_agent_bundle_route),
        )
        .route(
            "/api/v1/projects/:project_id/agents/export",
            post(export_project_agent_bundle_route),
        )
        .route(
            "/api/v1/projects/:project_id/agent-links/:agent_id",
            delete(unlink_project_agent),
        )
        .route(
            "/api/v1/projects/:project_id/team-links",
            get(list_project_team_links).post(link_project_team),
        )
        .route(
            "/api/v1/projects/:project_id/team-links/:team_id",
            delete(unlink_project_team),
        )
        .route("/api/v1/inbox", get(inbox))
        .route("/api/v1/artifacts", get(artifacts))
        .route("/api/v1/knowledge", get(knowledge))
        .route("/api/v1/audit", get(audit))
        .nest("/api/v1/runtime", runtime_routes())
        .layer(cors_layer)
        .with_state(state)
}

pub(crate) fn runtime_routes() -> Router<ServerState> {
    Router::new()
        .route("/bootstrap", get(runtime_bootstrap))
        .route("/config", get(get_runtime_config))
        .route("/config/validate", post(validate_runtime_config_route))
        .route(
            "/config/configured-models/probe",
            post(probe_runtime_configured_model_route),
        )
        .route("/config/scopes/:scope", patch(save_runtime_config_route))
        .route(
            "/sessions",
            get(list_runtime_sessions).post(create_runtime_session),
        )
        .route(
            "/sessions/:session_id",
            get(get_runtime_session).delete(delete_runtime_session),
        )
        .route("/sessions/:session_id/turns", post(submit_runtime_turn))
        .route(
            "/sessions/:session_id/approvals/:approval_id",
            post(resolve_runtime_approval),
        )
        .route("/sessions/:session_id/events", get(runtime_events))
}
