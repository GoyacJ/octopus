use super::*;
use crate::dto_mapping::{build_healthcheck_status, map_notification};
use octopus_core::{AvatarUploadPayload, WorkspaceSummary};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptchaPolicyPayload {
    required: bool,
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterprisePrincipalPayload {
    user_id: String,
    username: String,
    display_name: String,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseSessionSummaryPayload {
    session_id: String,
    token: String,
    workspace_id: String,
    client_app_id: String,
    status: String,
    created_at: u64,
    expires_at: Option<u64>,
    principal: EnterprisePrincipalPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemAuthStatusPayload {
    workspace: WorkspaceSummary,
    bootstrap_admin_required: bool,
    owner_ready: bool,
    captcha: CaptchaPolicyPayload,
    session: Option<EnterpriseSessionSummaryPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptchaChallengePayload {
    challenge_id: String,
    svg_data: String,
    expires_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseAuthSuccessPayload {
    session: EnterpriseSessionSummaryPayload,
    workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnterpriseLoginRequestPayload {
    client_app_id: String,
    username: String,
    password: String,
    captcha_challenge_id: String,
    captcha_code: String,
    workspace_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterBootstrapAdminRequestPayload {
    client_app_id: String,
    username: String,
    display_name: String,
    password: String,
    confirm_password: String,
    avatar: AvatarUploadPayload,
    captcha_challenge_id: String,
    captcha_code: String,
    workspace_id: Option<String>,
}

pub(crate) async fn healthcheck(
    State(state): State<ServerState>,
    _headers: HeaderMap,
) -> Result<Json<HealthcheckStatus>, ApiError> {
    Ok(Json(build_healthcheck_status(&state)))
}

pub(crate) async fn system_bootstrap(
    State(state): State<ServerState>,
) -> Result<Json<octopus_core::SystemBootstrapStatus>, ApiError> {
    let mut payload = state.services.workspace.system_bootstrap().await?;
    payload.transport_security = state.transport_security.clone();
    Ok(Json(payload))
}

async fn build_enterprise_session_summary(
    state: &ServerState,
    session: &SessionRecord,
) -> Result<EnterpriseSessionSummaryPayload, ApiError> {
    let users = state.services.access_control.list_users().await?;
    let current_user = users
        .iter()
        .find(|user| user.id == session.user_id)
        .cloned()
        .ok_or_else(|| ApiError::from(AppError::not_found("session user")))?;
    let principal = EnterprisePrincipalPayload {
        user_id: current_user.id.clone(),
        username: current_user.username.clone(),
        display_name: current_user.display_name.clone(),
        status: current_user.status.clone(),
    };

    Ok(EnterpriseSessionSummaryPayload {
        session_id: session.id.clone(),
        token: session.token.clone(),
        workspace_id: session.workspace_id.clone(),
        client_app_id: session.client_app_id.clone(),
        status: session.status.clone(),
        created_at: session.created_at,
        expires_at: session.expires_at,
        principal,
    })
}

fn system_menu_definitions() -> Vec<MenuDefinition> {
    vec![
        MenuDefinition {
            id: "menu-app-connections".into(),
            parent_id: None,
            label: "连接管理".into(),
            route_name: Some("app-connections".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 5,
            feature_code: "feature:app-connections".into(),
        },
        MenuDefinition {
            id: "menu-app-settings".into(),
            parent_id: None,
            label: "设置".into(),
            route_name: Some("app-settings".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 6,
            feature_code: "feature:app-settings".into(),
        },
        MenuDefinition {
            id: "menu-workspace-overview".into(),
            parent_id: None,
            label: "概览".into(),
            route_name: Some("workspace-overview".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 10,
            feature_code: "feature:workspace-overview".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console".into(),
            parent_id: None,
            label: "控制台".into(),
            route_name: Some("workspace-console".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 12,
            feature_code: "feature:workspace-console".into(),
        },
        MenuDefinition {
            id: "menu-project-dashboard".into(),
            parent_id: None,
            label: "控制台".into(),
            route_name: Some("project-dashboard".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 20,
            feature_code: "feature:project-dashboard".into(),
        },
        MenuDefinition {
            id: "menu-project-conversations".into(),
            parent_id: None,
            label: "会话".into(),
            route_name: Some("project-conversations".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 30,
            feature_code: "feature:project-conversations".into(),
        },
        MenuDefinition {
            id: "menu-project-agents".into(),
            parent_id: None,
            label: "项目数字员工".into(),
            route_name: Some("project-agents".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 40,
            feature_code: "feature:project-agents".into(),
        },
        MenuDefinition {
            id: "menu-project-resources".into(),
            parent_id: None,
            label: "项目资源".into(),
            route_name: Some("project-resources".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 50,
            feature_code: "feature:project-resources".into(),
        },
        MenuDefinition {
            id: "menu-project-knowledge".into(),
            parent_id: None,
            label: "项目知识".into(),
            route_name: Some("project-knowledge".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 60,
            feature_code: "feature:project-knowledge".into(),
        },
        MenuDefinition {
            id: "menu-project-trace".into(),
            parent_id: None,
            label: "Trace".into(),
            route_name: Some("project-trace".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 70,
            feature_code: "feature:project-trace".into(),
        },
        MenuDefinition {
            id: "menu-project-settings".into(),
            parent_id: None,
            label: "项目配置".into(),
            route_name: Some("project-settings".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 74,
            feature_code: "feature:project-settings".into(),
        },
        MenuDefinition {
            id: "menu-project-runtime".into(),
            parent_id: None,
            label: "Runtime".into(),
            route_name: Some("project-runtime".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 75,
            feature_code: "feature:project-runtime".into(),
        },
        MenuDefinition {
            id: "menu-workspace-automations".into(),
            parent_id: None,
            label: "自动化".into(),
            route_name: Some("workspace-automations".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 90,
            feature_code: "feature:workspace-automations".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control".into(),
            parent_id: None,
            label: "访问控制".into(),
            route_name: Some("workspace-access-control".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 100,
            feature_code: "feature:workspace-access-control".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console-projects".into(),
            parent_id: Some("menu-workspace-console".into()),
            label: "项目管理".into(),
            route_name: Some("workspace-console-projects".into()),
            source: "console".into(),
            status: "active".into(),
            order: 110,
            feature_code: "feature:workspace-console-projects".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console-knowledge".into(),
            parent_id: Some("menu-workspace-console".into()),
            label: "知识库".into(),
            route_name: Some("workspace-console-knowledge".into()),
            source: "console".into(),
            status: "active".into(),
            order: 120,
            feature_code: "feature:workspace-console-knowledge".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console-resources".into(),
            parent_id: Some("menu-workspace-console".into()),
            label: "资源中心".into(),
            route_name: Some("workspace-console-resources".into()),
            source: "console".into(),
            status: "active".into(),
            order: 130,
            feature_code: "feature:workspace-console-resources".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console-agents".into(),
            parent_id: Some("menu-workspace-console".into()),
            label: "数字员工".into(),
            route_name: Some("workspace-console-agents".into()),
            source: "console".into(),
            status: "active".into(),
            order: 140,
            feature_code: "feature:workspace-console-agents".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console-models".into(),
            parent_id: Some("menu-workspace-console".into()),
            label: "模型管理".into(),
            route_name: Some("workspace-console-models".into()),
            source: "console".into(),
            status: "active".into(),
            order: 150,
            feature_code: "feature:workspace-console-models".into(),
        },
        MenuDefinition {
            id: "menu-workspace-console-tools".into(),
            parent_id: Some("menu-workspace-console".into()),
            label: "工具管理".into(),
            route_name: Some("workspace-console-tools".into()),
            source: "console".into(),
            status: "active".into(),
            order: 160,
            feature_code: "feature:workspace-console-tools".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-users".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "用户管理".into(),
            route_name: Some("workspace-access-control-users".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 170,
            feature_code: "feature:workspace-access-control-users".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-org".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "组织管理".into(),
            route_name: Some("workspace-access-control-org".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 180,
            feature_code: "feature:workspace-access-control-org".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-roles".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "角色管理".into(),
            route_name: Some("workspace-access-control-roles".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 190,
            feature_code: "feature:workspace-access-control-roles".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-policies".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "权限与策略".into(),
            route_name: Some("workspace-access-control-policies".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 200,
            feature_code: "feature:workspace-access-control-policies".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-menus".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "菜单管理".into(),
            route_name: Some("workspace-access-control-menus".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 210,
            feature_code: "feature:workspace-access-control-menus".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-resources".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "资源授权".into(),
            route_name: Some("workspace-access-control-resources".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 220,
            feature_code: "feature:workspace-access-control-resources".into(),
        },
        MenuDefinition {
            id: "menu-workspace-access-control-sessions".into(),
            parent_id: Some("menu-workspace-access-control".into()),
            label: "会话与审计".into(),
            route_name: Some("workspace-access-control-sessions".into()),
            source: "access-control".into(),
            status: "active".into(),
            order: 230,
            feature_code: "feature:workspace-access-control-sessions".into(),
        },
    ]
}

async fn build_access_menu_definitions(
    state: &ServerState,
) -> Result<Vec<MenuDefinition>, ApiError> {
    let policies = state
        .services
        .access_control
        .list_menu_policies()
        .await?
        .into_iter()
        .map(|policy| (policy.menu_id.clone(), policy))
        .collect::<HashMap<_, _>>();
    Ok(system_menu_definitions()
        .into_iter()
        .into_iter()
        .map(|menu| {
            let policy = policies.get(&menu.id);
            MenuDefinition {
                id: menu.id,
                parent_id: menu.parent_id,
                label: menu.label,
                route_name: menu.route_name,
                source: menu.source,
                status: if policy
                    .map(|item| item.enabled)
                    .unwrap_or(menu.status == "active")
                {
                    "active".into()
                } else {
                    "disabled".into()
                },
                order: policy.map(|item| item.order).unwrap_or(menu.order),
                feature_code: menu.feature_code,
            }
        })
        .collect())
}

async fn build_access_feature_definitions(
    _state: &ServerState,
    menus: &[MenuDefinition],
) -> Result<Vec<FeatureDefinition>, ApiError> {
    let policy_by_feature = HashMap::from([
        (
            "menu-workspace-access-control-users",
            vec!["access.users.read".into()],
        ),
        (
            "menu-workspace-access-control-org",
            vec!["access.org.read".into()],
        ),
        (
            "menu-workspace-access-control-roles",
            vec!["access.roles.read".into()],
        ),
        (
            "menu-workspace-access-control-policies",
            vec!["access.policies.read".into()],
        ),
        (
            "menu-workspace-access-control-menus",
            vec!["access.menus.read".into()],
        ),
        (
            "menu-workspace-access-control-resources",
            vec!["access.policies.read".into()],
        ),
        (
            "menu-workspace-access-control-sessions",
            vec!["access.sessions.read".into()],
        ),
        (
            "menu-workspace-access-control",
            vec!["access.users.read".into()],
        ),
    ]);
    Ok(menus
        .into_iter()
        .map(|menu| FeatureDefinition {
            id: menu.feature_code.clone(),
            code: menu.feature_code.clone(),
            label: menu.label.clone(),
            required_permission_codes: policy_by_feature
                .get(menu.id.as_str())
                .cloned()
                .unwrap_or_else(|| vec!["workspace.overview.read".into()]),
        })
        .collect())
}

fn org_unit_ancestor_closure(
    org_units: &[OrgUnitRecord],
    org_unit_id: &str,
) -> BTreeSet<String> {
    let parent_by_id = org_units
        .iter()
        .map(|unit| (unit.id.as_str(), unit.parent_id.as_deref()))
        .collect::<HashMap<_, _>>();
    let mut closure = BTreeSet::new();
    let mut cursor = Some(org_unit_id);
    while let Some(current) = cursor {
        if !closure.insert(current.to_string()) {
            break;
        }
        cursor = parent_by_id.get(current).copied().flatten();
    }
    closure
}

async fn build_current_authorization_snapshot(
    state: &ServerState,
    session: &SessionRecord,
) -> Result<AuthorizationSnapshot, ApiError> {
    let users = state.services.access_control.list_users().await?;
    let current_user = users
        .into_iter()
        .find(|user| user.id == session.user_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("current user")))?;
    let org_assignments = state
        .services
        .access_control
        .list_user_org_assignments()
        .await?
        .into_iter()
        .filter(|assignment| assignment.user_id == session.user_id)
        .collect::<Vec<_>>();
    let org_units = state.services.access_control.list_org_units().await?;
    let org_unit_ids = org_assignments
        .iter()
        .flat_map(|assignment| org_unit_ancestor_closure(&org_units, &assignment.org_unit_id))
        .collect::<BTreeSet<_>>();
    let position_ids = org_assignments
        .iter()
        .flat_map(|assignment| assignment.position_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let user_group_ids = org_assignments
        .iter()
        .flat_map(|assignment| assignment.user_group_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let subject_matches = |subject_type: &str, subject_id: &str| match subject_type {
        "user" => subject_id == session.user_id,
        "org-unit" | "org_unit" => org_unit_ids.contains(subject_id),
        "position" => position_ids.contains(subject_id),
        "user-group" | "user_group" => user_group_ids.contains(subject_id),
        _ => false,
    };
    let role_bindings = state.services.access_control.list_role_bindings().await?;
    let denied_role_ids = role_bindings
        .iter()
        .filter(|binding| {
            binding.effect == "deny" && subject_matches(&binding.subject_type, &binding.subject_id)
        })
        .map(|binding| binding.role_id.clone())
        .collect::<BTreeSet<_>>();
    let current_role_ids = role_bindings
        .into_iter()
        .filter(|binding| {
            binding.effect == "allow" && subject_matches(&binding.subject_type, &binding.subject_id)
        })
        .map(|binding| binding.role_id)
        .filter(|role_id| !denied_role_ids.contains(role_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let effective_roles = state
        .services
        .access_control
        .list_roles()
        .await?
        .into_iter()
        .filter(|role| current_role_ids.iter().any(|role_id| role_id == &role.id))
        .collect::<Vec<_>>();
    let effective_permission_codes = effective_roles
        .iter()
        .flat_map(|role| role.permission_codes.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let menu_definitions = build_access_menu_definitions(state).await?;
    let feature_definitions = build_access_feature_definitions(state, &menu_definitions).await?;
    let feature_map = feature_definitions
        .iter()
        .cloned()
        .map(|feature| (feature.code.clone(), feature))
        .collect::<HashMap<_, _>>();
    let policies = state
        .services
        .access_control
        .list_menu_policies()
        .await?
        .into_iter()
        .map(|policy| (policy.menu_id.clone(), policy))
        .collect::<HashMap<_, _>>();
    let feature_codes = feature_definitions
        .iter()
        .filter(|feature| {
            feature.required_permission_codes.iter().all(|code| {
                effective_permission_codes
                    .iter()
                    .any(|candidate| candidate == code)
            })
        })
        .map(|feature| feature.code.clone())
        .collect::<Vec<_>>();
    let feature_code_set = feature_codes.iter().cloned().collect::<BTreeSet<_>>();
    let menu_gates = menu_definitions
        .iter()
        .map(|menu| {
            let policy = policies.get(&menu.id);
            let enabled = policy
                .map(|item| item.enabled)
                .unwrap_or(menu.status == "active");
            let feature_allowed = feature_code_set.contains(&menu.feature_code);
            let visibility = policy
                .map(|item| item.visibility.as_str())
                .unwrap_or("inherit");
            let allowed = enabled
                && match visibility {
                    "visible" => true,
                    "hidden" => false,
                    _ => feature_allowed,
                };
            MenuGateResult {
                menu_id: menu.id.clone(),
                feature_code: menu.feature_code.clone(),
                allowed,
                reason: if allowed {
                    None
                } else if !enabled {
                    Some("menu disabled by policy".into())
                } else if visibility == "hidden" {
                    Some("menu hidden by policy".into())
                } else if !feature_map.contains_key(&menu.feature_code) {
                    Some("feature definition missing".into())
                } else {
                    Some("required permission missing".into())
                },
            }
        })
        .collect::<Vec<_>>();
    let visible_menu_ids = menu_gates
        .iter()
        .filter(|gate| gate.allowed)
        .map(|gate| gate.menu_id.clone())
        .collect::<Vec<_>>();

    let permission_definitions = default_permission_definitions();
    let resource_action_grants = permission_definitions
        .into_iter()
        .filter(|definition| {
            effective_permission_codes
                .iter()
                .any(|code| code == &definition.code)
        })
        .fold(
            BTreeMap::<String, BTreeSet<String>>::new(),
            |mut grants, definition| {
                let entry = grants.entry(definition.resource_type.clone()).or_default();
                for action in definition.actions {
                    entry.insert(action);
                }
                grants
            },
        )
        .into_iter()
        .map(|(resource_type, actions)| ResourceActionGrant {
            resource_type,
            actions: actions.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    Ok(AuthorizationSnapshot {
        principal: current_user,
        effective_role_ids: current_role_ids,
        effective_roles,
        effective_permission_codes,
        org_assignments,
        feature_codes,
        visible_menu_ids,
        menu_gates,
        resource_action_grants,
    })
}

async fn build_access_session_payloads(
    state: &ServerState,
    current_session_id: &str,
) -> Result<Vec<AccessSessionRecord>, ApiError> {
    let users = state
        .services
        .access_control
        .list_users()
        .await?
        .into_iter()
        .map(|user| (user.id.clone(), user))
        .collect::<HashMap<_, _>>();
    let mut sessions = state.services.auth.list_sessions().await?;
    sessions.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(sessions
        .into_iter()
        .filter_map(|session| {
            let user = users.get(&session.user_id)?;
            let current = session.id == current_session_id;
            Some(AccessSessionRecord {
                session_id: session.id,
                user_id: session.user_id,
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                client_app_id: session.client_app_id,
                status: session.status,
                created_at: session.created_at,
                expires_at: session.expires_at,
                current,
            })
        })
        .collect())
}

fn precise_tool_resource_type(kind: &str) -> &'static str {
    match kind.trim() {
        "builtin" => "tool.builtin",
        "mcp" => "tool.mcp",
        _ => "tool.skill",
    }
}

fn merge_protected_resource_descriptor(
    defaults: ProtectedResourceDescriptor,
    metadata_by_key: &HashMap<(String, String), ProtectedResourceDescriptor>,
) -> ProtectedResourceDescriptor {
    let Some(metadata) = metadata_by_key.get(&(defaults.resource_type.clone(), defaults.id.clone())) else {
        return defaults;
    };
    ProtectedResourceDescriptor {
        id: defaults.id,
        resource_type: defaults.resource_type,
        resource_subtype: metadata
            .resource_subtype
            .clone()
            .or(defaults.resource_subtype),
        name: defaults.name,
        project_id: metadata.project_id.clone().or(defaults.project_id),
        tags: if metadata.tags.is_empty() {
            defaults.tags
        } else {
            metadata.tags.clone()
        },
        classification: if metadata.classification.trim().is_empty() {
            defaults.classification
        } else {
            metadata.classification.clone()
        },
        owner_subject_type: metadata
            .owner_subject_type
            .clone()
            .or(defaults.owner_subject_type),
        owner_subject_id: metadata
            .owner_subject_id
            .clone()
            .or(defaults.owner_subject_id),
    }
}

async fn build_access_protected_resource_descriptors(
    state: &ServerState,
) -> Result<Vec<ProtectedResourceDescriptor>, ApiError> {
    let metadata_by_key = state
        .services
        .access_control
        .list_protected_resources()
        .await?
        .into_iter()
        .map(|record| ((record.resource_type.clone(), record.id.clone()), record))
        .collect::<HashMap<_, _>>();
    let agents = state.services.workspace.list_agents().await?;
    let resources = state.services.workspace.list_workspace_resources().await?;
    let knowledge = state.services.workspace.list_workspace_knowledge().await?;
    let tools = state.services.workspace.list_tools().await?;

    let mut descriptors = Vec::new();
    descriptors.extend(agents.into_iter().map(|agent| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: agent.id,
                resource_type: "agent".into(),
                resource_subtype: Some(agent.scope),
                name: agent.name,
                project_id: agent.project_id,
                tags: agent.tags,
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(resources.into_iter().map(|resource| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: resource.id,
                resource_type: "resource".into(),
                resource_subtype: Some(resource.kind),
                name: resource.name,
                project_id: resource.project_id,
                tags: resource.tags,
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(knowledge.into_iter().map(|entry| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: entry.id,
                resource_type: "knowledge".into(),
                resource_subtype: Some(entry.kind),
                name: entry.title,
                project_id: entry.project_id,
                tags: Vec::new(),
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.extend(tools.into_iter().map(|tool| {
        merge_protected_resource_descriptor(
            ProtectedResourceDescriptor {
                id: tool.id,
                resource_type: precise_tool_resource_type(&tool.kind).into(),
                resource_subtype: Some(tool.kind),
                name: tool.name,
                project_id: None,
                tags: Vec::new(),
                classification: "internal".into(),
                owner_subject_type: None,
                owner_subject_id: None,
            },
            &metadata_by_key,
        )
    }));
    descriptors.sort_by(|left, right| {
        left.resource_type
            .cmp(&right.resource_type)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(descriptors)
}

fn render_captcha_svg(code: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"160\" height=\"52\" viewBox=\"0 0 160 52\" fill=\"none\"><rect width=\"160\" height=\"52\" rx=\"10\" fill=\"#F3F4F6\"/><text x=\"80\" y=\"33\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"22\" font-weight=\"700\" fill=\"#111827\" letter-spacing=\"6\" data-code=\"{code}\">{code}</text></svg>"
    )
}

fn create_auth_challenge(state: &ServerState) -> Result<CaptchaChallengePayload, ApiError> {
    let challenge_id = format!("captcha-{}", Uuid::new_v4());
    let code = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(4)
        .collect::<String>()
        .to_uppercase();
    let expires_at = timestamp_now() + 5 * 60 * 1000;
    state
        .auth_captcha_challenges
        .lock()
        .map_err(|_| AppError::runtime("captcha mutex poisoned"))?
        .insert(
            challenge_id.clone(),
            AuthCaptchaChallenge {
                challenge_id: challenge_id.clone(),
                code: code.clone(),
                expires_at,
            },
        );
    Ok(CaptchaChallengePayload {
        challenge_id,
        svg_data: render_captcha_svg(&code),
        expires_at,
    })
}

fn verify_auth_challenge(
    state: &ServerState,
    challenge_id: &str,
    code: &str,
) -> Result<(), ApiError> {
    let challenge = state
        .auth_captcha_challenges
        .lock()
        .map_err(|_| AppError::runtime("captcha mutex poisoned"))?
        .remove(challenge_id)
        .ok_or_else(|| AppError::auth("captcha challenge not found"))?;
    if challenge.challenge_id != challenge_id {
        return Err(ApiError::from(AppError::auth("captcha challenge mismatch")));
    }
    if challenge.expires_at < timestamp_now() {
        return Err(ApiError::from(AppError::auth("captcha challenge expired")));
    }
    if challenge.code != code.trim().to_uppercase() {
        return Err(ApiError::from(AppError::auth("invalid captcha code")));
    }
    Ok(())
}

async fn audit_auth_event(
    state: &ServerState,
    actor_id: &str,
    action: &str,
    outcome: &str,
) -> Result<(), ApiError> {
    let workspace_id = workspace_id_for_audit(state).await?;
    append_audit_event(
        state,
        &workspace_id,
        None,
        "auth",
        actor_id,
        action,
        "system-auth",
        outcome,
    )
    .await
}

async fn ensure_auth_attempt_allowed(
    state: &ServerState,
    workspace_id: &str,
    username: &str,
    headers: &HeaderMap,
) -> Result<String, ApiError> {
    let attempt_key = auth_rate_limit_key(workspace_id, username, headers);
    if let Some(locked_until) = check_auth_rate_limit(state, &attempt_key)? {
        let outcome = format!("locked-until:{locked_until}");
        audit_auth_event(state, username, "system.auth.locked", &outcome).await?;
        return Err(ApiError::from(AppError::auth(
            "authentication temporarily locked due to too many failed attempts",
        )));
    }
    Ok(attempt_key)
}

async fn record_auth_failure_event(
    state: &ServerState,
    attempt_key: &str,
    username: &str,
    action: &str,
    outcome: &str,
) -> Result<(), ApiError> {
    let lock_until = record_auth_failure(state, attempt_key)?;
    audit_auth_event(state, username, action, outcome).await?;
    if let Some(locked_until) = lock_until {
        audit_auth_event(
            state,
            username,
            "system.auth.locked",
            &format!("locked-until:{locked_until}"),
        )
        .await?;
    }
    Ok(())
}

fn default_permission_definitions() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition {
            code: "workspace.overview.read".into(),
            name: "Workspace Overview Read".into(),
            description: "Read the enterprise workspace overview snapshot.".into(),
            category: "workspace".into(),
            resource_type: "workspace".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "project.view".into(),
            name: "Project View".into(),
            description: "View enterprise project records and dashboards.".into(),
            category: "workspace".into(),
            resource_type: "project".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "project.manage".into(),
            name: "Project Manage".into(),
            description: "Create and update enterprise projects.".into(),
            category: "workspace".into(),
            resource_type: "project".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "team.view".into(),
            name: "Team View".into(),
            description: "View enterprise teams and project links.".into(),
            category: "resource".into(),
            resource_type: "team".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "team.manage".into(),
            name: "Team Manage".into(),
            description: "Create, update, and delete enterprise teams.".into(),
            category: "resource".into(),
            resource_type: "team".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "team.import".into(),
            name: "Team Import".into(),
            description: "Copy and import enterprise teams across scopes.".into(),
            category: "resource".into(),
            resource_type: "team".into(),
            actions: vec!["import".into()],
        },
        PermissionDefinition {
            code: "access.users.read".into(),
            name: "Access Users Read".into(),
            description: "Read enterprise users and their access assignments.".into(),
            category: "access-control".into(),
            resource_type: "access.users".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "access.users.manage".into(),
            name: "Access Users Manage".into(),
            description: "Manage enterprise users and credentials.".into(),
            category: "access-control".into(),
            resource_type: "access.users".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "access.org.read".into(),
            name: "Access Org Read".into(),
            description: "Read organization units, positions, groups, and assignments.".into(),
            category: "access-control".into(),
            resource_type: "access.org".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "access.org.manage".into(),
            name: "Access Org Manage".into(),
            description: "Manage organization units, positions, groups, and assignments.".into(),
            category: "access-control".into(),
            resource_type: "access.org".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "access.roles.read".into(),
            name: "Access Roles Read".into(),
            description: "Read roles and capability bundles.".into(),
            category: "access-control".into(),
            resource_type: "access.roles".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "access.roles.manage".into(),
            name: "Access Roles Manage".into(),
            description: "Manage roles and their capability bundles.".into(),
            category: "access-control".into(),
            resource_type: "access.roles".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "access.policies.read".into(),
            name: "Access Policies Read".into(),
            description:
                "Read permission definitions, role bindings, data policies, and resource policies."
                    .into(),
            category: "access-control".into(),
            resource_type: "access.policies".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "access.policies.manage".into(),
            name: "Access Policies Manage".into(),
            description: "Manage role bindings, data policies, and resource policies.".into(),
            category: "access-control".into(),
            resource_type: "access.policies".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "access.menus.read".into(),
            name: "Access Menus Read".into(),
            description: "Read menu definitions, menu policies, and feature gates.".into(),
            category: "access-control".into(),
            resource_type: "access.menus".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "access.menus.manage".into(),
            name: "Access Menus Manage".into(),
            description: "Manage menu policies and access-control navigation visibility.".into(),
            category: "access-control".into(),
            resource_type: "access.menus".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "access.sessions.read".into(),
            name: "Access Sessions Read".into(),
            description: "Read enterprise login sessions.".into(),
            category: "access-control".into(),
            resource_type: "access.sessions".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "access.sessions.manage".into(),
            name: "Access Sessions Manage".into(),
            description: "Revoke enterprise login sessions.".into(),
            category: "access-control".into(),
            resource_type: "access.sessions".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "agent.view".into(),
            name: "Agent View".into(),
            description: "View enterprise agents.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "agent.run".into(),
            name: "Agent Run".into(),
            description: "Run enterprise agents.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["run".into()],
        },
        PermissionDefinition {
            code: "agent.debug".into(),
            name: "Agent Debug".into(),
            description: "Debug enterprise agents.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["debug".into()],
        },
        PermissionDefinition {
            code: "agent.edit".into(),
            name: "Agent Edit".into(),
            description: "Create and update enterprise agents.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["edit".into()],
        },
        PermissionDefinition {
            code: "agent.publish".into(),
            name: "Agent Publish".into(),
            description: "Publish enterprise agents.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["publish".into()],
        },
        PermissionDefinition {
            code: "agent.delete".into(),
            name: "Agent Delete".into(),
            description: "Delete enterprise agents.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["delete".into()],
        },
        PermissionDefinition {
            code: "agent.grant".into(),
            name: "Agent Grant".into(),
            description: "Manage enterprise agent authorization.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["grant".into()],
        },
        PermissionDefinition {
            code: "agent.import".into(),
            name: "Agent Import".into(),
            description: "Import and copy enterprise agents across scopes.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["import".into()],
        },
        PermissionDefinition {
            code: "agent.export".into(),
            name: "Agent Export".into(),
            description: "Export enterprise agent bundles.".into(),
            category: "resource".into(),
            resource_type: "agent".into(),
            actions: vec!["export".into()],
        },
        PermissionDefinition {
            code: "resource.view".into(),
            name: "Resource View".into(),
            description: "View protected resources.".into(),
            category: "resource".into(),
            resource_type: "resource".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "resource.upload".into(),
            name: "Resource Upload".into(),
            description: "Upload workspace resources.".into(),
            category: "resource".into(),
            resource_type: "resource".into(),
            actions: vec!["upload".into()],
        },
        PermissionDefinition {
            code: "resource.update".into(),
            name: "Resource Update".into(),
            description: "Update protected resources.".into(),
            category: "resource".into(),
            resource_type: "resource".into(),
            actions: vec!["update".into()],
        },
        PermissionDefinition {
            code: "resource.delete".into(),
            name: "Resource Delete".into(),
            description: "Delete protected resources.".into(),
            category: "resource".into(),
            resource_type: "resource".into(),
            actions: vec!["delete".into()],
        },
        PermissionDefinition {
            code: "resource.export".into(),
            name: "Resource Export".into(),
            description: "Export protected resources.".into(),
            category: "resource".into(),
            resource_type: "resource".into(),
            actions: vec!["export".into()],
        },
        PermissionDefinition {
            code: "resource.grant".into(),
            name: "Resource Grant".into(),
            description: "Manage resource authorization.".into(),
            category: "resource".into(),
            resource_type: "resource".into(),
            actions: vec!["grant".into()],
        },
        PermissionDefinition {
            code: "knowledge.view".into(),
            name: "Knowledge View".into(),
            description: "View protected knowledge entries.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "knowledge.create".into(),
            name: "Knowledge Create".into(),
            description: "Create protected knowledge entries.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["create".into()],
        },
        PermissionDefinition {
            code: "knowledge.edit".into(),
            name: "Knowledge Edit".into(),
            description: "Edit protected knowledge entries.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["edit".into()],
        },
        PermissionDefinition {
            code: "knowledge.publish".into(),
            name: "Knowledge Publish".into(),
            description: "Publish protected knowledge entries.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["publish".into()],
        },
        PermissionDefinition {
            code: "knowledge.delete".into(),
            name: "Knowledge Delete".into(),
            description: "Delete protected knowledge entries.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["delete".into()],
        },
        PermissionDefinition {
            code: "knowledge.retrieve".into(),
            name: "Knowledge Retrieve".into(),
            description: "Retrieve and use protected knowledge.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["retrieve".into()],
        },
        PermissionDefinition {
            code: "knowledge.grant".into(),
            name: "Knowledge Grant".into(),
            description: "Manage knowledge authorization.".into(),
            category: "resource".into(),
            resource_type: "knowledge".into(),
            actions: vec!["grant".into()],
        },
        PermissionDefinition {
            code: "tool.catalog.view".into(),
            name: "Tool Catalog View".into(),
            description: "Read the shared tool catalog and model catalog.".into(),
            category: "resource".into(),
            resource_type: "tool.catalog".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "tool.catalog.manage".into(),
            name: "Tool Catalog Manage".into(),
            description: "Manage shared tool catalog visibility and configuration.".into(),
            category: "resource".into(),
            resource_type: "tool.catalog".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "provider-credential.view".into(),
            name: "Provider Credential View".into(),
            description: "Read provider credential bindings.".into(),
            category: "resource".into(),
            resource_type: "provider-credential".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "provider-credential.manage".into(),
            name: "Provider Credential Manage".into(),
            description: "Manage provider credential bindings.".into(),
            category: "resource".into(),
            resource_type: "provider-credential".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "tool.builtin.view".into(),
            name: "Builtin Tool View".into(),
            description: "View builtin tools.".into(),
            category: "resource".into(),
            resource_type: "tool.builtin".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "tool.builtin.enable".into(),
            name: "Builtin Tool Enable".into(),
            description: "Enable builtin tools.".into(),
            category: "resource".into(),
            resource_type: "tool.builtin".into(),
            actions: vec!["enable".into()],
        },
        PermissionDefinition {
            code: "tool.builtin.configure".into(),
            name: "Builtin Tool Configure".into(),
            description: "Create and update builtin tool records.".into(),
            category: "resource".into(),
            resource_type: "tool.builtin".into(),
            actions: vec!["configure".into()],
        },
        PermissionDefinition {
            code: "tool.builtin.delete".into(),
            name: "Builtin Tool Delete".into(),
            description: "Delete builtin tool records.".into(),
            category: "resource".into(),
            resource_type: "tool.builtin".into(),
            actions: vec!["delete".into()],
        },
        PermissionDefinition {
            code: "tool.builtin.invoke".into(),
            name: "Builtin Tool Invoke".into(),
            description: "Invoke builtin tools.".into(),
            category: "resource".into(),
            resource_type: "tool.builtin".into(),
            actions: vec!["invoke".into()],
        },
        PermissionDefinition {
            code: "tool.builtin.grant".into(),
            name: "Builtin Tool Grant".into(),
            description: "Manage builtin tool authorization.".into(),
            category: "resource".into(),
            resource_type: "tool.builtin".into(),
            actions: vec!["grant".into()],
        },
        PermissionDefinition {
            code: "tool.skill.view".into(),
            name: "Skill Tool View".into(),
            description: "View managed skill tools.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "tool.skill.enable".into(),
            name: "Skill Tool Enable".into(),
            description: "Enable managed skill tools.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["enable".into()],
        },
        PermissionDefinition {
            code: "tool.skill.configure".into(),
            name: "Skill Tool Configure".into(),
            description: "Create and update managed skill tools.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["configure".into()],
        },
        PermissionDefinition {
            code: "tool.skill.publish".into(),
            name: "Skill Tool Publish".into(),
            description: "Publish managed skill tools.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["publish".into()],
        },
        PermissionDefinition {
            code: "tool.skill.delete".into(),
            name: "Skill Tool Delete".into(),
            description: "Delete managed skill tools.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["delete".into()],
        },
        PermissionDefinition {
            code: "tool.skill.invoke".into(),
            name: "Skill Tool Invoke".into(),
            description: "Invoke skill tools.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["invoke".into()],
        },
        PermissionDefinition {
            code: "tool.skill.grant".into(),
            name: "Skill Tool Grant".into(),
            description: "Manage skill tool authorization.".into(),
            category: "resource".into(),
            resource_type: "tool.skill".into(),
            actions: vec!["grant".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.view".into(),
            name: "MCP Tool View".into(),
            description: "View managed MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.enable".into(),
            name: "MCP Tool Enable".into(),
            description: "Enable managed MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["enable".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.configure".into(),
            name: "MCP Tool Configure".into(),
            description: "Create and update managed MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["configure".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.delete".into(),
            name: "MCP Tool Delete".into(),
            description: "Delete managed MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["delete".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.invoke".into(),
            name: "MCP Tool Invoke".into(),
            description: "Invoke MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["invoke".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.bind-credential".into(),
            name: "MCP Tool Bind Credential".into(),
            description: "Bind credentials for MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["bind-credential".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.publish".into(),
            name: "MCP Tool Publish".into(),
            description: "Publish managed MCP tools.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["publish".into()],
        },
        PermissionDefinition {
            code: "tool.mcp.grant".into(),
            name: "MCP Tool Grant".into(),
            description: "Manage MCP tool authorization.".into(),
            category: "resource".into(),
            resource_type: "tool.mcp".into(),
            actions: vec!["grant".into()],
        },
        PermissionDefinition {
            code: "automation.view".into(),
            name: "Automation View".into(),
            description: "View workspace and project automations.".into(),
            category: "resource".into(),
            resource_type: "automation".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "automation.manage".into(),
            name: "Automation Manage".into(),
            description: "Create, update, and delete automations.".into(),
            category: "resource".into(),
            resource_type: "automation".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "pet.view".into(),
            name: "Pet View".into(),
            description: "View the workspace pet experience.".into(),
            category: "resource".into(),
            resource_type: "pet".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "pet.manage".into(),
            name: "Pet Manage".into(),
            description: "Manage pet presence and bindings.".into(),
            category: "resource".into(),
            resource_type: "pet".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "artifact.view".into(),
            name: "Artifact View".into(),
            description: "View workspace artifacts.".into(),
            category: "resource".into(),
            resource_type: "artifact".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "inbox.view".into(),
            name: "Inbox View".into(),
            description: "View workspace inbox items.".into(),
            category: "resource".into(),
            resource_type: "inbox".into(),
            actions: vec!["view".into()],
        },
        PermissionDefinition {
            code: "runtime.config.workspace.read".into(),
            name: "Runtime Workspace Config Read".into(),
            description: "Read workspace runtime configuration.".into(),
            category: "runtime".into(),
            resource_type: "runtime.config.workspace".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "runtime.config.workspace.manage".into(),
            name: "Runtime Workspace Config Manage".into(),
            description: "Validate and save workspace runtime configuration.".into(),
            category: "runtime".into(),
            resource_type: "runtime.config.workspace".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "runtime.config.project.read".into(),
            name: "Runtime Project Config Read".into(),
            description: "Read project runtime configuration.".into(),
            category: "runtime".into(),
            resource_type: "runtime.config.project".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "runtime.config.project.manage".into(),
            name: "Runtime Project Config Manage".into(),
            description: "Validate and save project runtime configuration.".into(),
            category: "runtime".into(),
            resource_type: "runtime.config.project".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "runtime.config.user.read".into(),
            name: "Runtime User Config Read".into(),
            description: "Read user runtime configuration.".into(),
            category: "runtime".into(),
            resource_type: "runtime.config.user".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "runtime.config.user.manage".into(),
            name: "Runtime User Config Manage".into(),
            description: "Validate and save user runtime configuration.".into(),
            category: "runtime".into(),
            resource_type: "runtime.config.user".into(),
            actions: vec!["manage".into()],
        },
        PermissionDefinition {
            code: "runtime.session.read".into(),
            name: "Runtime Session Read".into(),
            description: "Read runtime session state, events, and outputs.".into(),
            category: "runtime".into(),
            resource_type: "runtime.session".into(),
            actions: vec!["read".into()],
        },
        PermissionDefinition {
            code: "runtime.approval.resolve".into(),
            name: "Runtime Approval Resolve".into(),
            description: "Resolve runtime approvals.".into(),
            category: "runtime".into(),
            resource_type: "runtime.approval".into(),
            actions: vec!["resolve".into()],
        },
        PermissionDefinition {
            code: "audit.read".into(),
            name: "Audit Read".into(),
            description: "Read workspace audit events.".into(),
            category: "runtime".into(),
            resource_type: "audit".into(),
            actions: vec!["read".into()],
        },
    ]
}

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
        .unwrap_or_else(|| StdPath::new("."))
        .join("data/main.db")
}

pub(crate) fn open_host_notifications_db(state: &ServerState) -> Result<Connection, ApiError> {
    let path = host_notifications_db_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::from(AppError::from(error)))?;
    }

    let connection = Connection::open(path)
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
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
        .map_err(|error| ApiError::from(AppError::database(error.to_string())))?;
    Ok(connection)
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

pub(crate) async fn system_auth_status(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<SystemAuthStatusPayload>, ApiError> {
    let bootstrap = state.services.workspace.system_bootstrap().await?;
    let session = match extract_bearer(&headers) {
        Some(token) => state.services.auth.lookup_session(&token).await?,
        None => None,
    };

    let session = match session {
        Some(session) => Some(build_enterprise_session_summary(&state, &session).await?),
        None => None,
    };

    Ok(Json(SystemAuthStatusPayload {
        workspace: bootstrap.workspace,
        bootstrap_admin_required: !bootstrap.owner_ready,
        owner_ready: bootstrap.owner_ready,
        captcha: CaptchaPolicyPayload {
            required: true,
            ttl_seconds: 300,
        },
        session,
    }))
}

pub(crate) async fn create_system_auth_captcha(
    State(state): State<ServerState>,
) -> Result<Json<CaptchaChallengePayload>, ApiError> {
    Ok(Json(create_auth_challenge(&state)?))
}

pub(crate) async fn system_auth_login(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseLoginRequestPayload>,
) -> Result<Json<EnterpriseAuthSuccessPayload>, ApiError> {
    let workspace_id = workspace_id_for_audit(&state).await?;
    let attempt_key =
        ensure_auth_attempt_allowed(&state, &workspace_id, &request.username, &headers).await?;
    let username = request.username.clone();
    if let Err(error) = verify_auth_challenge(
        &state,
        &request.captcha_challenge_id,
        &request.captcha_code,
    ) {
        let message = error.source.to_string();
        let action = if message.contains("not found") {
            "system.auth.captcha.missing"
        } else if message.contains("expired") {
            "system.auth.captcha.expired"
        } else {
            "system.auth.captcha.invalid"
        };
        record_auth_failure_event(&state, &attempt_key, &request.username, action, &message).await?;
        return Err(error);
    }
    let response = match state
        .services
        .auth
        .login(LoginRequest {
            client_app_id: request.client_app_id,
            username: request.username,
            password: request.password,
            workspace_id: request.workspace_id,
        })
        .await
    {
        Ok(response) => response,
        Err(error) => {
            record_auth_failure_event(
                &state,
                &attempt_key,
                &username,
                "system.auth.login.failure",
                &error.to_string(),
            )
            .await?;
            return Err(ApiError::from(error));
        }
    };
    let recovered = clear_auth_failures(&state, &attempt_key)?;
    if recovered {
        audit_auth_event(&state, &response.session.user_id, "system.auth.recovered", "cleared")
            .await?;
    }
    audit_auth_event(&state, &response.session.user_id, "system.auth.login.success", "success")
        .await?;
    let session = build_enterprise_session_summary(&state, &response.session).await?;
    Ok(Json(EnterpriseAuthSuccessPayload {
        session,
        workspace: response.workspace,
    }))
}

pub(crate) async fn system_auth_bootstrap_admin(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RegisterBootstrapAdminRequestPayload>,
) -> Result<Json<EnterpriseAuthSuccessPayload>, ApiError> {
    let workspace_id = workspace_id_for_audit(&state).await?;
    let attempt_key =
        ensure_auth_attempt_allowed(&state, &workspace_id, &request.username, &headers).await?;
    let username = request.username.clone();
    if let Err(error) = verify_auth_challenge(
        &state,
        &request.captcha_challenge_id,
        &request.captcha_code,
    ) {
        let message = error.source.to_string();
        let action = if message.contains("not found") {
            "system.auth.captcha.missing"
        } else if message.contains("expired") {
            "system.auth.captcha.expired"
        } else {
            "system.auth.captcha.invalid"
        };
        record_auth_failure_event(&state, &attempt_key, &request.username, action, &message).await?;
        return Err(error);
    }
    let response = match state
        .services
        .auth
        .register_bootstrap_admin(RegisterBootstrapAdminRequest {
            client_app_id: request.client_app_id,
            username: request.username,
            display_name: request.display_name,
            password: request.password,
            confirm_password: request.confirm_password,
            avatar: request.avatar,
            workspace_id: request.workspace_id,
        })
        .await
    {
        Ok(response) => response,
        Err(error) => {
            record_auth_failure_event(
                &state,
                &attempt_key,
                &username,
                "system.auth.bootstrap-admin.failure",
                &error.to_string(),
            )
            .await?;
            return Err(ApiError::from(error));
        }
    };
    let recovered = clear_auth_failures(&state, &attempt_key)?;
    if recovered {
        audit_auth_event(&state, &response.session.user_id, "system.auth.recovered", "cleared")
            .await?;
    }
    audit_auth_event(
        &state,
        &response.session.user_id,
        "system.auth.bootstrap-admin.success",
        "success",
    )
    .await?;
    let session = build_enterprise_session_summary(&state, &response.session).await?;
    Ok(Json(EnterpriseAuthSuccessPayload {
        session,
        workspace: response.workspace,
    }))
}

pub(crate) async fn system_auth_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<EnterpriseSessionSummaryPayload>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        build_enterprise_session_summary(&state, &session).await?,
    ))
}

pub(crate) async fn current_authorization(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<AuthorizationSnapshot>, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    Ok(Json(
        build_current_authorization_snapshot(&state, &session).await?,
    ))
}

pub(crate) async fn list_access_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessSessionRecord>>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.sessions.read", None).await?;
    Ok(Json(
        build_access_session_payloads(&state, &session.id).await?,
    ))
}

pub(crate) async fn revoke_current_access_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_session(&state, &headers).await?;
    state.services.auth.revoke_session(&session.id).await?;
    append_session_audit(
        &state,
        &session,
        "access.sessions.revoke-current",
        &audit_resource_label("access.session", Some(&session.id)),
        "revoked",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn revoke_access_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.sessions.manage", None).await?;
    state.services.auth.revoke_session(&session_id).await?;
    append_session_audit(
        &state,
        &session,
        "access.sessions.revoke",
        &audit_resource_label("access.session", Some(&session_id)),
        "revoked",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn revoke_access_user_sessions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session =
        ensure_authorized_session(&state, &headers, "access.sessions.manage", None).await?;
    if session.user_id == user_id {
        return Err(ApiError::from(AppError::invalid_input(
            "current user cannot revoke all active sessions through this route",
        )));
    }
    state.services.auth.revoke_user_sessions(&user_id).await?;
    append_session_audit(
        &state,
        &session,
        "access.sessions.revoke-user",
        &audit_resource_label("access.user-sessions", Some(&user_id)),
        "revoked",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_users(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessUserRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.read", None).await?;
    Ok(Json(state.services.access_control.list_users().await?))
}

pub(crate) async fn create_access_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<AccessUserUpsertRequest>,
) -> Result<Json<AccessUserRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    Ok(Json(
        state.services.access_control.create_user(request).await?,
    ))
}

pub(crate) async fn update_access_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<AccessUserUpsertRequest>,
) -> Result<Json<AccessUserRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_user(&user_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_user(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.users.manage", None).await?;
    state.services.access_control.delete_user(&user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_org_units(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<OrgUnitRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(state.services.access_control.list_org_units().await?))
}

pub(crate) async fn create_access_org_unit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<OrgUnitUpsertRequest>,
) -> Result<Json<OrgUnitRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .create_org_unit(request)
            .await?,
    ))
}

pub(crate) async fn update_access_org_unit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(org_unit_id): Path<String>,
    Json(request): Json<OrgUnitUpsertRequest>,
) -> Result<Json<OrgUnitRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_org_unit(&org_unit_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_org_unit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(org_unit_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_org_unit(&org_unit_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_positions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PositionRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(state.services.access_control.list_positions().await?))
}

pub(crate) async fn create_access_position(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<PositionUpsertRequest>,
) -> Result<Json<PositionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .create_position(request)
            .await?,
    ))
}

pub(crate) async fn update_access_position(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(position_id): Path<String>,
    Json(request): Json<PositionUpsertRequest>,
) -> Result<Json<PositionRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_position(&position_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_position(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(position_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_position(&position_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_user_groups(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserGroupRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(
        state.services.access_control.list_user_groups().await?,
    ))
}

pub(crate) async fn create_access_user_group(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UserGroupUpsertRequest>,
) -> Result<Json<UserGroupRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .create_user_group(request)
            .await?,
    ))
}

pub(crate) async fn update_access_user_group(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(request): Json<UserGroupUpsertRequest>,
) -> Result<Json<UserGroupRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_user_group(&group_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_user_group(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_user_group(&group_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_user_org_assignments(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserOrgAssignmentRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.read", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .list_user_org_assignments()
            .await?,
    ))
}

pub(crate) async fn upsert_access_user_org_assignment(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<UserOrgAssignmentUpsertRequest>,
) -> Result<Json<UserOrgAssignmentRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .upsert_user_org_assignment(request)
            .await?,
    ))
}

pub(crate) async fn delete_access_user_org_assignment(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((user_id, org_unit_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.org.manage", None).await?;
    state
        .services
        .access_control
        .delete_user_org_assignment(&user_id, &org_unit_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_permission_definitions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PermissionDefinition>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(default_permission_definitions()))
}

pub(crate) async fn list_access_roles(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AccessRoleRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.read", None).await?;
    Ok(Json(state.services.access_control.list_roles().await?))
}

pub(crate) async fn create_access_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RoleUpsertRequest>,
) -> Result<Json<AccessRoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.manage", None).await?;
    Ok(Json(
        state.services.access_control.create_role(request).await?,
    ))
}

pub(crate) async fn update_access_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
    Json(request): Json<RoleUpsertRequest>,
) -> Result<Json<AccessRoleRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.manage", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .update_role(&role_id, request)
            .await?,
    ))
}

pub(crate) async fn delete_access_role(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(role_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    ensure_authorized_session(&state, &headers, "access.roles.manage", None).await?;
    state.services.access_control.delete_role(&role_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_role_bindings(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RoleBindingRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        state.services.access_control.list_role_bindings().await?,
    ))
}

pub(crate) async fn create_access_role_binding(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RoleBindingUpsertRequest>,
) -> Result<Json<RoleBindingRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .create_role_binding(request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.role-bindings.create",
        &audit_resource_label("access.role-binding", Some(&record.id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_role_binding(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
    Json(request): Json<RoleBindingUpsertRequest>,
) -> Result<Json<RoleBindingRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .update_role_binding(&binding_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.role-bindings.update",
        &audit_resource_label("access.role-binding", Some(&binding_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_role_binding(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    state
        .services
        .access_control
        .delete_role_binding(&binding_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.role-bindings.delete",
        &audit_resource_label("access.role-binding", Some(&binding_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_data_policies(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DataPolicyRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        state.services.access_control.list_data_policies().await?,
    ))
}

pub(crate) async fn create_access_data_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<DataPolicyUpsertRequest>,
) -> Result<Json<DataPolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .create_data_policy(request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.data-policies.create",
        &audit_resource_label("access.data-policy", Some(&record.id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_data_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
    Json(request): Json<DataPolicyUpsertRequest>,
) -> Result<Json<DataPolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .update_data_policy(&policy_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.data-policies.update",
        &audit_resource_label("access.data-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_data_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    state
        .services
        .access_control
        .delete_data_policy(&policy_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.data-policies.delete",
        &audit_resource_label("access.data-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_resource_policies(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ResourcePolicyRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(
        state
            .services
            .access_control
            .list_resource_policies()
            .await?,
    ))
}

pub(crate) async fn create_access_resource_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<ResourcePolicyUpsertRequest>,
) -> Result<Json<ResourcePolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .create_resource_policy(request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.resource-policies.create",
        &audit_resource_label("access.resource-policy", Some(&record.id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_resource_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
    Json(request): Json<ResourcePolicyUpsertRequest>,
) -> Result<Json<ResourcePolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let record = state
        .services
        .access_control
        .update_resource_policy(&policy_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.resource-policies.update",
        &audit_resource_label("access.resource-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_resource_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(policy_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    state
        .services
        .access_control
        .delete_resource_policy(&policy_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.resource-policies.delete",
        &audit_resource_label("access.resource-policy", Some(&policy_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_menu_definitions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuDefinition>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    Ok(Json(build_access_menu_definitions(&state).await?))
}

pub(crate) async fn list_access_feature_definitions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FeatureDefinition>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    let menus = build_access_menu_definitions(&state).await?;
    Ok(Json(
        build_access_feature_definitions(&state, &menus).await?,
    ))
}

pub(crate) async fn list_access_menu_gate_results(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuGateResult>>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    Ok(Json(
        build_current_authorization_snapshot(&state, &session)
            .await?
            .menu_gates,
    ))
}

pub(crate) async fn list_access_menu_policies(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MenuPolicyRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.menus.read", None).await?;
    Ok(Json(
        state.services.access_control.list_menu_policies().await?,
    ))
}

pub(crate) async fn create_access_menu_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<CreateMenuPolicyRequest>,
) -> Result<Json<MenuPolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.manage", None).await?;
    let record = state
        .services
        .access_control
        .upsert_menu_policy(
            &request.menu_id,
            MenuPolicyUpsertRequest {
                enabled: request.enabled,
                order: request.order,
                group: request.group,
                visibility: request.visibility,
            },
        )
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.menu-policies.create",
        &audit_resource_label("access.menu-policy", Some(&request.menu_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn update_access_menu_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(menu_id): Path<String>,
    Json(request): Json<MenuPolicyUpsertRequest>,
) -> Result<Json<MenuPolicyRecord>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.manage", None).await?;
    let record = state
        .services
        .access_control
        .upsert_menu_policy(&menu_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.menu-policies.update",
        &audit_resource_label("access.menu-policy", Some(&menu_id)),
        "success",
        None,
    )
    .await?;
    Ok(Json(record))
}

pub(crate) async fn delete_access_menu_policy(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(menu_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.menus.manage", None).await?;
    state
        .services
        .access_control
        .delete_menu_policy(&menu_id)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.menu-policies.delete",
        &audit_resource_label("access.menu-policy", Some(&menu_id)),
        "success",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_access_protected_resources(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProtectedResourceDescriptor>>, ApiError> {
    ensure_authorized_session(&state, &headers, "access.policies.read", None).await?;
    Ok(Json(build_access_protected_resource_descriptors(&state).await?))
}

pub(crate) async fn upsert_access_protected_resource(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path((resource_type, resource_id)): Path<(String, String)>,
    Json(request): Json<ProtectedResourceMetadataUpsertRequest>,
) -> Result<Json<ProtectedResourceDescriptor>, ApiError> {
    let session = ensure_authorized_session(&state, &headers, "access.policies.manage", None).await?;
    let _ = build_access_protected_resource_descriptors(&state)
        .await?
        .into_iter()
        .find(|record| record.resource_type == resource_type && record.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("protected resource")))?;
    state
        .services
        .access_control
        .upsert_protected_resource(&resource_type, &resource_id, request)
        .await?;
    append_session_audit(
        &state,
        &session,
        "access.protected-resources.update",
        &audit_resource_label(&resource_type, Some(&resource_id)),
        "success",
        None,
    )
    .await?;
    let record = build_access_protected_resource_descriptors(&state)
        .await?
        .into_iter()
        .find(|descriptor| descriptor.resource_type == resource_type && descriptor.id == resource_id)
        .ok_or_else(|| ApiError::from(AppError::not_found("protected resource")))?;
    Ok(Json(record))
}

pub(crate) async fn list_access_audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<AccessAuditQuery>,
) -> Result<Json<AccessAuditListResponse>, ApiError> {
    ensure_authorized_session(&state, &headers, "audit.read", None).await?;
    let mut items = state.services.observation.list_audit_records().await?;
    items.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    if let Some(actor_id) = query.actor_id.as_deref() {
        items.retain(|record| record.actor_id == actor_id);
    }
    if let Some(action) = query.action.as_deref() {
        items.retain(|record| record.action == action);
    }
    if let Some(resource_type) = query.resource_type.as_deref() {
        items.retain(|record| {
            record.resource == resource_type
                || record
                    .resource
                    .strip_prefix(resource_type)
                    .is_some_and(|suffix| suffix.starts_with(':'))
        });
    }
    if let Some(outcome) = query.outcome.as_deref() {
        items.retain(|record| record.outcome == outcome);
    }
    if let Some(from) = query.from {
        items.retain(|record| record.created_at >= from);
    }
    if let Some(to) = query.to {
        items.retain(|record| record.created_at <= to);
    }
    if let Some(cursor) = query.cursor.as_deref() {
        items.retain(|record| record.created_at.to_string().as_str() < cursor);
    }
    const PAGE_SIZE: usize = 50;
    let next_cursor = items.get(PAGE_SIZE).map(|record| record.created_at.to_string());
    items.truncate(PAGE_SIZE);
    Ok(Json(AccessAuditListResponse { items, next_cursor }))
}

pub(crate) async fn list_apps(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ClientAppRecord>>, ApiError> {
    ensure_authorized_session(&state, &headers, "app_registry.read", None).await?;
    Ok(Json(state.services.app_registry.list_apps().await?))
}

pub(crate) async fn register_app(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(app): Json<ClientAppRecord>,
) -> Result<Json<ClientAppRecord>, ApiError> {
    ensure_authorized_session(&state, &headers, "app_registry.write", None).await?;
    Ok(Json(state.services.app_registry.register_app(app).await?))
}
