use super::*;

pub(super) fn access_string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(super) fn has_any_permission_codes(
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

pub(super) fn recommended_access_section(section_grants: &[AccessSectionGrant]) -> String {
    section_grants
        .iter()
        .find(|grant| grant.allowed)
        .map(|grant| grant.section.clone())
        .unwrap_or_else(|| "members".into())
}

pub(super) fn has_allowed_access_section(
    section_grants: &[AccessSectionGrant],
    section: &str,
) -> bool {
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
