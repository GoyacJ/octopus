use super::*;

pub(super) fn system_menu_definitions() -> Vec<MenuDefinition> {
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

pub(super) fn org_unit_ancestor_closure(org_units: &[OrgUnitRecord], org_unit_id: &str) -> BTreeSet<String> {
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

pub(super) async fn build_current_authorization_snapshot(
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

