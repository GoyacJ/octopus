use super::*;

#[async_trait]
impl AuthorizationService for InfraAuthorizationService {
    async fn authorize_request(
        &self,
        session: &SessionRecord,
        request: &AuthorizationRequest,
    ) -> Result<AuthorizationDecision, AppError> {
        fn requested_action(capability: &str) -> &str {
            capability.rsplit('.').next().unwrap_or(capability)
        }

        fn resource_type_matches(policy_type: &str, request_type: Option<&str>) -> bool {
            let Some(request_type) = request_type else {
                return policy_type == "project";
            };
            policy_type == request_type
                || (policy_type == "tool" && request_type.starts_with("tool."))
        }

        fn resource_policy_action_matches(
            policy_action: &str,
            requested_action: &str,
            capability: &str,
        ) -> bool {
            policy_action == "*" || policy_action == requested_action || policy_action == capability
        }

        let connection = self._state.open_db()?;
        let org_units = load_org_units(&connection)?;
        let _assignments =
            assignments_for_user(&load_user_org_assignments(&connection)?, &session.user_id);
        let (permission_codes, bindings) =
            resolve_effective_permission_codes(&connection, &session.user_id)?;
        let data_policies = resolve_subject_data_policies(&connection, &session.user_id)?;
        let all_resource_policies = load_resource_policies(&connection)?;
        let resource_policies = resolve_subject_resource_policies(&connection, &session.user_id)?;

        if !permission_codes
            .iter()
            .any(|code| code == &request.capability)
        {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: Some("no matching role permission".into()),
                matched_role_binding_ids: bindings.into_iter().map(|binding| binding.id).collect(),
                matched_policy_ids: data_policies
                    .into_iter()
                    .map(|policy| policy.id)
                    .chain(resource_policies.into_iter().map(|policy| policy.id))
                    .collect(),
            });
        }

        let matched_role_binding_ids = bindings
            .iter()
            .map(|binding| binding.id.clone())
            .collect::<Vec<_>>();
        let requested_action = requested_action(&request.capability);
        let request_resource_type = request.resource_type.as_deref();
        let request_project_id = request.project_id.as_deref();
        let request_classification = request.classification.as_deref();
        let request_owner_type = request.owner_subject_type.as_deref();
        let request_owner_id = request.owner_subject_id.as_deref();

        let owner_org_ancestor_ids = match (request_owner_type, request_owner_id) {
            (Some("org-unit"), Some(owner_org_unit_id))
            | (Some("org_unit"), Some(owner_org_unit_id)) => {
                org_unit_ancestor_ids(&org_units, owner_org_unit_id)
            }
            _ => BTreeSet::new(),
        };

        let data_policy_matches_scope = |policy: &DataPolicyRecord| match policy.scope_type.as_str()
        {
            "all" | "all-projects" => true,
            "selected-projects" => request_project_id
                .map(|project_id| {
                    policy
                        .project_ids
                        .iter()
                        .any(|candidate| candidate == project_id)
                })
                .unwrap_or(false),
            "org-unit-self" => matches!(
                (request_owner_type, request_owner_id),
                (Some("org-unit"), Some(owner_id)) | (Some("org_unit"), Some(owner_id))
                    if owner_id == policy.subject_id
            ),
            "org-unit-subtree" => {
                matches!(request_owner_type, Some("org-unit") | Some("org_unit"))
                    && owner_org_ancestor_ids.contains(&policy.subject_id)
            }
            "tag-match" => {
                !policy.tags.is_empty()
                    && policy
                        .tags
                        .iter()
                        .all(|tag| request.tags.iter().any(|candidate| candidate == tag))
            }
            _ => false,
        };

        let data_policy_matches = |policy: &DataPolicyRecord| {
            resource_type_matches(&policy.resource_type, request_resource_type)
                && (policy.classifications.is_empty()
                    || request_classification
                        .map(|classification| {
                            policy
                                .classifications
                                .iter()
                                .any(|candidate| candidate == classification)
                        })
                        .unwrap_or(false))
                && data_policy_matches_scope(policy)
        };

        let relevant_data_policies = data_policies
            .iter()
            .filter(|policy| resource_type_matches(&policy.resource_type, request_resource_type))
            .collect::<Vec<_>>();
        let matched_data_policies = relevant_data_policies
            .iter()
            .filter(|policy| data_policy_matches(policy))
            .collect::<Vec<_>>();
        let mut matched_policy_ids = matched_data_policies
            .iter()
            .map(|policy| policy.id.clone())
            .collect::<Vec<_>>();

        if matched_data_policies
            .iter()
            .any(|policy| policy.effect == "deny")
        {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: Some("data policy denied".into()),
                matched_role_binding_ids,
                matched_policy_ids,
            });
        }

        let has_domain_constraints = !relevant_data_policies.is_empty();
        let has_data_allow = matched_data_policies
            .iter()
            .any(|policy| policy.effect == "allow");
        if has_domain_constraints && !has_data_allow {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: Some("data policy allow missing".into()),
                matched_role_binding_ids,
                matched_policy_ids,
            });
        }

        if let (Some(resource_type), Some(resource_id)) = (
            request.resource_type.as_deref(),
            request.resource_id.as_deref(),
        ) {
            let relevant_resource_policies = all_resource_policies
                .iter()
                .filter(|policy| {
                    policy.resource_type == resource_type
                        && policy.resource_id == resource_id
                        && resource_policy_action_matches(
                            &policy.action,
                            requested_action,
                            &request.capability,
                        )
                })
                .collect::<Vec<_>>();
            let matching_resource_policies = resource_policies
                .iter()
                .filter(|policy| {
                    policy.resource_type == resource_type
                        && policy.resource_id == resource_id
                        && resource_policy_action_matches(
                            &policy.action,
                            requested_action,
                            &request.capability,
                        )
                })
                .collect::<Vec<_>>();

            matched_policy_ids.extend(
                relevant_resource_policies
                    .iter()
                    .map(|policy| policy.id.clone()),
            );

            if matching_resource_policies
                .iter()
                .any(|policy| policy.effect == "deny")
            {
                return Ok(AuthorizationDecision {
                    allowed: false,
                    reason: Some("resource access denied".into()),
                    matched_role_binding_ids,
                    matched_policy_ids,
                });
            }

            if !relevant_resource_policies.is_empty()
                && !matching_resource_policies
                    .iter()
                    .any(|policy| policy.effect == "allow")
            {
                return Ok(AuthorizationDecision {
                    allowed: false,
                    reason: Some("resource allow missing".into()),
                    matched_role_binding_ids,
                    matched_policy_ids,
                });
            }
        }

        Ok(AuthorizationDecision {
            allowed: true,
            reason: None,
            matched_role_binding_ids,
            matched_policy_ids,
        })
    }
}
