use super::*;

pub(crate) fn system_menu_definitions() -> Vec<MenuDefinition> {
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
            id: "menu-project-tasks".into(),
            parent_id: None,
            label: "任务".into(),
            route_name: Some("project-tasks".into()),
            source: "main-sidebar".into(),
            status: "active".into(),
            order: 35,
            feature_code: "feature:project-tasks".into(),
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

pub(super) async fn build_access_menu_definitions(
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

pub(super) async fn build_access_feature_definitions(
    _state: &ServerState,
    menus: &[MenuDefinition],
) -> Result<Vec<FeatureDefinition>, ApiError> {
    let policy_by_feature = HashMap::from([
        ("menu-project-tasks", vec!["task.view".into()]),
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
        .iter()
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

fn access_string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn has_any_permission_codes(
    effective_permission_codes: &BTreeSet<String>,
    required_codes: &[&str],
) -> bool {
    required_codes
        .iter()
        .any(|code| effective_permission_codes.contains(*code))
}

pub(super) fn build_access_section_grants(
    effective_permission_codes: &BTreeSet<String>,
) -> Vec<AccessSectionGrant> {
    let members_allowed = has_any_permission_codes(
        effective_permission_codes,
        &["access.users.read", "access.users.manage"],
    );
    let access_allowed = members_allowed;
    let governance_allowed = has_any_permission_codes(
        effective_permission_codes,
        &[
            "access.org.read",
            "access.org.manage",
            "access.policies.read",
            "access.policies.manage",
            "access.menus.read",
            "access.menus.manage",
            "access.sessions.read",
            "access.sessions.manage",
            "audit.read",
        ],
    );

    vec![
        AccessSectionGrant {
            section: "members".into(),
            allowed: members_allowed,
        },
        AccessSectionGrant {
            section: "access".into(),
            allowed: access_allowed,
        },
        AccessSectionGrant {
            section: "governance".into(),
            allowed: governance_allowed,
        },
    ]
}

fn recommended_access_section(section_grants: &[AccessSectionGrant]) -> String {
    section_grants
        .iter()
        .find(|grant| grant.allowed)
        .map(|grant| grant.section.clone())
        .unwrap_or_else(|| "members".into())
}

fn has_allowed_access_section(section_grants: &[AccessSectionGrant], section: &str) -> bool {
    section_grants
        .iter()
        .any(|grant| grant.section == section && grant.allowed)
}

pub(super) fn recommended_access_section_for_snapshot(
    snapshot: &AccessExperienceSnapshot,
    section_grants: &[AccessSectionGrant],
) -> String {
    if has_allowed_access_section(section_grants, "members") && snapshot.member_count > 1 {
        return "members".into();
    }
    if has_allowed_access_section(section_grants, "access") {
        return "access".into();
    }
    if snapshot.experience_level == "enterprise"
        && has_allowed_access_section(section_grants, "governance")
    {
        return "governance".into();
    }
    recommended_access_section(section_grants)
}

pub(super) fn build_access_role_templates() -> Vec<AccessRoleTemplate> {
    vec![
        AccessRoleTemplate {
            code: "owner".into(),
            name: "Owner".into(),
            description: "Full workspace ownership across members, presets, and governance.".into(),
            managed_role_codes: vec!["system.owner".into()],
            editable: false,
        },
        AccessRoleTemplate {
            code: "admin".into(),
            name: "Admin".into(),
            description: "Manage members, presets, and governance workflows for the workspace."
                .into(),
            managed_role_codes: vec!["system.admin".into()],
            editable: false,
        },
        AccessRoleTemplate {
            code: "member".into(),
            name: "Member".into(),
            description: "Collaborate in projects, resources, and day-to-day workspace work."
                .into(),
            managed_role_codes: vec!["system.member".into()],
            editable: false,
        },
        AccessRoleTemplate {
            code: "viewer".into(),
            name: "Viewer".into(),
            description: "Read workspace context and published work without making changes.".into(),
            managed_role_codes: vec!["system.viewer".into()],
            editable: false,
        },
        AccessRoleTemplate {
            code: "auditor".into(),
            name: "Auditor".into(),
            description: "Review members, policy state, sessions, and audit activity.".into(),
            managed_role_codes: vec!["system.auditor".into()],
            editable: false,
        },
    ]
}

pub(super) fn build_access_capability_bundles() -> Vec<AccessCapabilityBundle> {
    vec![
        AccessCapabilityBundle {
            code: "workspace_governance".into(),
            name: "Workspace Governance".into(),
            description: "Organization structure, custom roles, and policy management.".into(),
            permission_codes: access_string_vec(&[
                "access.org.read",
                "access.org.manage",
                "access.roles.read",
                "access.roles.manage",
                "access.policies.read",
                "access.policies.manage",
                "access.menus.read",
                "access.menus.manage",
            ]),
        },
        AccessCapabilityBundle {
            code: "member_management".into(),
            name: "Member Management".into(),
            description: "Invite people, review membership, and adjust practical workspace access."
                .into(),
            permission_codes: access_string_vec(&["access.users.read", "access.users.manage"]),
        },
        AccessCapabilityBundle {
            code: "project_and_resource_access".into(),
            name: "Project And Resource Access".into(),
            description: "Project delivery, resource operations, and knowledge access.".into(),
            permission_codes: access_string_vec(&[
                "project.view",
                "project.manage",
                "team.view",
                "team.manage",
                "resource.view",
                "resource.upload",
                "resource.update",
                "resource.publish",
                "knowledge.view",
                "knowledge.create",
                "knowledge.edit",
                "knowledge.publish",
                "knowledge.retrieve",
            ]),
        },
        AccessCapabilityBundle {
            code: "automation_and_tools".into(),
            name: "Automation And Tools".into(),
            description: "Runtime work, agents, and tool enablement for everyday execution.".into(),
            permission_codes: access_string_vec(&[
                "agent.view",
                "agent.run",
                "agent.edit",
                "tool.builtin.enable",
                "tool.skill.enable",
                "tool.mcp.enable",
                "runtime.session.read",
                "runtime.submit_turn",
            ]),
        },
        AccessCapabilityBundle {
            code: "security_and_audit".into(),
            name: "Security And Audit".into(),
            description: "Session review, revocation, and audit visibility.".into(),
            permission_codes: access_string_vec(&[
                "access.sessions.read",
                "access.sessions.manage",
                "audit.read",
            ]),
        },
    ]
}

pub(super) fn build_access_role_presets() -> Vec<AccessRolePreset> {
    vec![
        AccessRolePreset {
            code: "owner".into(),
            name: "Owner".into(),
            description: "Run the workspace, manage members, and control governance.".into(),
            recommended_for: "Workspace founders and final decision makers.".into(),
            template_codes: vec!["owner".into()],
            capability_bundle_codes: access_string_vec(&[
                "workspace_governance",
                "member_management",
                "project_and_resource_access",
                "automation_and_tools",
                "security_and_audit",
            ]),
        },
        AccessRolePreset {
            code: "admin".into(),
            name: "Admin".into(),
            description: "Operate the workspace, members, and governance day to day.".into(),
            recommended_for: "People who run collaboration and workspace operations.".into(),
            template_codes: vec!["admin".into()],
            capability_bundle_codes: access_string_vec(&[
                "workspace_governance",
                "member_management",
                "project_and_resource_access",
                "automation_and_tools",
                "security_and_audit",
            ]),
        },
        AccessRolePreset {
            code: "member".into(),
            name: "Member".into(),
            description: "Contribute to active work without full governance control.".into(),
            recommended_for: "Core contributors working on projects and resources.".into(),
            template_codes: vec!["member".into()],
            capability_bundle_codes: access_string_vec(&[
                "project_and_resource_access",
                "automation_and_tools",
            ]),
        },
        AccessRolePreset {
            code: "viewer".into(),
            name: "Viewer".into(),
            description: "Read project context and published results without editing.".into(),
            recommended_for: "Stakeholders who need visibility but not day-to-day control.".into(),
            template_codes: vec!["viewer".into()],
            capability_bundle_codes: vec!["project_and_resource_access".into()],
        },
        AccessRolePreset {
            code: "auditor".into(),
            name: "Auditor".into(),
            description: "Review governance, sessions, and audit activity without editing.".into(),
            recommended_for: "Risk, compliance, and review-oriented collaborators.".into(),
            template_codes: vec!["auditor".into()],
            capability_bundle_codes: access_string_vec(&[
                "workspace_governance",
                "security_and_audit",
            ]),
        },
    ]
}
