use super::*;

fn org_unit_ancestor_closure(org_units: &[OrgUnitRecord], org_unit_id: &str) -> BTreeSet<String> {
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

pub(super) async fn build_access_experience_response(
    state: &ServerState,
    session: &SessionRecord,
) -> Result<AccessExperienceResponse, ApiError> {
    let authorization = build_current_authorization_snapshot(state, session).await?;
    let effective_permission_codes = authorization
        .effective_permission_codes
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let section_grants = build_access_section_grants(&effective_permission_codes);
    let snapshot = state
        .services
        .access_control
        .get_experience_snapshot()
        .await?;
    let summary = AccessExperienceSummary {
        experience_level: snapshot.experience_level.clone(),
        member_count: snapshot.member_count,
        has_org_structure: snapshot.has_org_structure,
        has_custom_roles: snapshot.has_custom_roles,
        has_advanced_policies: snapshot.has_advanced_policies,
        has_menu_governance: snapshot.has_menu_governance,
        has_resource_governance: snapshot.has_resource_governance,
        recommended_landing_section: recommended_access_section_for_snapshot(
            &snapshot,
            &section_grants,
        ),
    };

    Ok(AccessExperienceResponse {
        summary,
        section_grants,
        role_templates: build_access_role_templates(),
        role_presets: build_access_role_presets(),
        capability_bundles: build_access_capability_bundles(),
        counts: snapshot.counts,
    })
}

pub(super) async fn build_access_session_payloads(
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
