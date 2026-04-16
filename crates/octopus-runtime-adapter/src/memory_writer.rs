use super::*;

const MIN_MEMORY_CONTENT_CHARS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryProposalSourceKind {
    ExplicitInput,
    RuntimeOutcome,
    ValidatedWorkflowOutput,
}

#[derive(Debug, Clone)]
struct MemoryProposalCandidate {
    kind: &'static str,
    content: String,
    source: MemoryProposalSourceKind,
}

fn normalize_memory_kind(value: &str) -> &'static str {
    match value.trim() {
        "user" => "user",
        "feedback" => "feedback",
        "project" => "project",
        _ => "reference",
    }
}

fn policy_contains_scope(policy: &octopus_core::MemoryPolicy, candidates: &[&str]) -> bool {
    policy
        .durable_scopes
        .iter()
        .any(|scope| candidates.iter().any(|candidate| scope == candidate))
}

fn normalize_memory_scope(
    project_id: &str,
    kind: &str,
    policy: &octopus_core::MemoryPolicy,
    actor_manifest: &actor_manifest::CompiledActorManifest,
) -> String {
    if kind == "user" && policy_contains_scope(policy, &["user-private", "user"]) {
        return "user-private".into();
    }
    match actor_manifest {
        actor_manifest::CompiledActorManifest::Agent(_) => {
            if policy_contains_scope(policy, &["agent-private"]) {
                return "agent-private".into();
            }
        }
        actor_manifest::CompiledActorManifest::Team(_) => {
            if policy_contains_scope(policy, &["team-shared", "team"]) {
                return "team-shared".into();
            }
        }
    }
    if !project_id.trim().is_empty()
        && policy_contains_scope(policy, &["project-shared", "project"])
    {
        return "project-shared".into();
    }
    if policy.allow_workspace_shared_write
        && policy_contains_scope(policy, &["workspace-shared", "workspace"])
    {
        return "workspace-shared".into();
    }
    policy
        .durable_scopes
        .first()
        .cloned()
        .unwrap_or_else(|| "user-private".into())
}

fn proposal_reason(kind: &str) -> &'static str {
    match kind {
        "feedback" => "user-feedback",
        "project" => "project-context",
        "user" => "user-preference",
        _ => "runtime-memory",
    }
}

fn proposal_title(kind: &str) -> &'static str {
    match kind {
        "feedback" => "Feedback memory proposal",
        "project" => "Project memory proposal",
        "user" => "User memory proposal",
        _ => "Reference memory proposal",
    }
}

fn proposal_body(content: &str, kind: &str) -> serde_json::Value {
    json!({
        "kind": kind,
        "content": content.trim(),
    })
}

fn normalize_memory_content(content: &str) -> String {
    content.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalized_text_eq(left: &str, right: &str) -> bool {
    normalize_memory_content(left).eq_ignore_ascii_case(&normalize_memory_content(right))
}

fn filler_memory_content(content: &str) -> bool {
    let normalized = normalize_memory_content(content).to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "ok" | "okay"
            | "thanks"
            | "thank you"
            | "got it"
            | "sounds good"
            | "noted"
            | "remember this"
            | "please remember this"
    )
}

fn temporary_task_state(content: &str) -> bool {
    let normalized = normalize_memory_content(content).to_ascii_lowercase();
    [
        "for this task",
        "for this run",
        "for this session",
        "for now",
        "temporarily",
        "temporary",
        "currently",
        "next step",
        "todo",
        "to do",
        "work in progress",
        "wip",
        "in progress",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn derivable_repo_or_config_content(content: &str) -> bool {
    let normalized = normalize_memory_content(content);
    let lower = normalized.to_ascii_lowercase();
    let references_repo_shape = [
        "cargo.toml",
        "package.json",
        "pnpm-lock",
        "config/",
        "src/",
        ".json",
        ".yaml",
        ".yml",
        ".toml",
        ".rs",
        ".ts",
        ".tsx",
        ".js",
        ".vue",
        "`",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    let sounds_derivable = [
        "set ",
        "update ",
        "change ",
        "create ",
        "write ",
        "run ",
        "install ",
        "configure ",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    references_repo_shape && sounds_derivable
}

fn reject_memory_content(content: &str) -> bool {
    let normalized = normalize_memory_content(content);
    normalized.chars().count() < MIN_MEMORY_CONTENT_CHARS
        || filler_memory_content(&normalized)
        || temporary_task_state(&normalized)
        || derivable_repo_or_config_content(&normalized)
}

fn durable_outcome_signal(content: &str) -> bool {
    let normalized = normalize_memory_content(content).to_ascii_lowercase();
    [
        "user prefers",
        "preference",
        "prefer ",
        "must ",
        "must be",
        "should ",
        "always ",
        "required",
        "needs ",
        "need ",
        "remember that",
        "constraint",
        "checklist",
        "policy",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn infer_outcome_memory_kind(
    content: &str,
    workflow_detail: Option<&RuntimeWorkflowRunDetail>,
) -> Option<(&'static str, MemoryProposalSourceKind)> {
    let normalized = normalize_memory_content(content).to_ascii_lowercase();
    if normalized.contains("user prefers")
        || normalized.contains("user preference")
        || normalized.starts_with("prefer ")
    {
        return Some(("user", MemoryProposalSourceKind::RuntimeOutcome));
    }
    if workflow_detail.is_some_and(|detail| detail.status == "completed")
        && durable_outcome_signal(&normalized)
    {
        return Some(("project", MemoryProposalSourceKind::ValidatedWorkflowOutput));
    }
    if normalized.contains("required")
        || normalized.contains("needs ")
        || normalized.contains("need ")
        || normalized.contains("must ")
        || normalized.contains("should ")
        || normalized.contains("always ")
        || normalized.contains("remember that")
    {
        return Some(("feedback", MemoryProposalSourceKind::RuntimeOutcome));
    }
    durable_outcome_signal(&normalized)
        .then_some(("reference", MemoryProposalSourceKind::RuntimeOutcome))
}

fn proposal_candidates(
    input: &SubmitRuntimeTurnInput,
    execution: Option<&ModelExecutionResult>,
    workflow_detail: Option<&RuntimeWorkflowRunDetail>,
) -> Vec<MemoryProposalCandidate> {
    let mut candidates = Vec::new();
    if let Some(intent) = input
        .memory_intent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let content = input.content.trim();
        if !content.is_empty() {
            candidates.push(MemoryProposalCandidate {
                kind: normalize_memory_kind(intent),
                content: content.to_string(),
                source: MemoryProposalSourceKind::ExplicitInput,
            });
        }
    }
    if let Some(response) = execution {
        let content = response.content.trim();
        if !content.is_empty() {
            if let Some((kind, source)) = infer_outcome_memory_kind(content, workflow_detail) {
                candidates.push(MemoryProposalCandidate {
                    kind,
                    content: content.to_string(),
                    source,
                });
            }
        }
    }
    candidates
}

fn candidate_proposal_reason(candidate: &MemoryProposalCandidate) -> String {
    match candidate.source {
        MemoryProposalSourceKind::ExplicitInput => proposal_reason(candidate.kind).into(),
        MemoryProposalSourceKind::RuntimeOutcome => "runtime-outcome".into(),
        MemoryProposalSourceKind::ValidatedWorkflowOutput => "validated-workflow-output".into(),
    }
}

pub(crate) fn build_memory_proposal(
    session_id: &str,
    run_id: &str,
    project_id: &str,
    policy: &octopus_core::MemoryPolicy,
    actor_manifest: &actor_manifest::CompiledActorManifest,
    input: &SubmitRuntimeTurnInput,
    execution: Option<&ModelExecutionResult>,
    workflow_detail: Option<&RuntimeWorkflowRunDetail>,
    candidate_memory: &[RuntimeSelectedMemoryItem],
) -> Option<RuntimeMemoryProposal> {
    for candidate in proposal_candidates(input, execution, workflow_detail) {
        let normalized_content = normalize_memory_content(&candidate.content);
        if reject_memory_content(&normalized_content) {
            continue;
        }

        let summary = memory_runtime::truncate_memory_summary(&normalized_content);
        let matching_memory = candidate_memory.iter().find(|item| {
            item.kind == candidate.kind && normalized_text_eq(&item.summary, &summary)
        });
        if matching_memory.is_some_and(|item| item.freshness_state == "fresh") {
            continue;
        }

        return Some(RuntimeMemoryProposal {
            proposal_id: format!("memory-proposal-{}", Uuid::new_v4()),
            session_id: session_id.to_string(),
            source_run_id: run_id.to_string(),
            memory_id: matching_memory
                .map(|item| item.memory_id.clone())
                .unwrap_or_else(|| format!("mem-{}", Uuid::new_v4())),
            title: proposal_title(candidate.kind).into(),
            summary,
            kind: candidate.kind.into(),
            scope: normalize_memory_scope(project_id, candidate.kind, policy, actor_manifest),
            proposal_state: "pending".into(),
            proposal_reason: candidate_proposal_reason(&candidate),
            review: None,
            normalized_content: Some(normalized_content),
        });
    }

    None
}

pub(crate) fn build_persisted_memory_record(
    proposal: &RuntimeMemoryProposal,
    project_id: &str,
    workspace_id: &str,
    selected_actor_ref: &str,
    user_id: Option<&str>,
    reviewed_at: u64,
) -> memory_runtime::PersistedRuntimeMemoryRecord {
    let owner_ref = match proposal.scope.as_str() {
        "project" | "project-shared" if !project_id.trim().is_empty() => {
            Some(format!("project:{project_id}"))
        }
        "workspace" | "workspace-shared" if !workspace_id.trim().is_empty() => {
            Some(format!("workspace:{workspace_id}"))
        }
        "team" | "team-shared" | "agent-private" if !selected_actor_ref.trim().is_empty() => {
            Some(selected_actor_ref.to_string())
        }
        "user" | "user-private" => Some(
            user_id
                .filter(|value| !value.trim().is_empty())
                .map(|value| format!("user:{value}"))
                .unwrap_or_else(|| "user:runtime".into()),
        ),
        _ => Some("user:runtime".into()),
    };
    memory_runtime::PersistedRuntimeMemoryRecord {
        memory_id: proposal.memory_id.clone(),
        project_id: if project_id.trim().is_empty() {
            None
        } else {
            Some(project_id.to_string())
        },
        owner_ref,
        source_run_id: Some(proposal.source_run_id.clone()),
        kind: proposal.kind.clone(),
        scope: proposal.scope.clone(),
        title: proposal.title.clone(),
        summary: proposal.summary.clone(),
        freshness_state: if proposal.proposal_state == "revalidated" {
            "revalidated".into()
        } else {
            "fresh".into()
        },
        last_validated_at: Some(reviewed_at),
        proposal_state: proposal.proposal_state.clone(),
        storage_path: None,
        content_hash: None,
        updated_at: reviewed_at,
    }
}

pub(crate) fn build_persisted_memory_body(
    proposal: &RuntimeMemoryProposal,
    note: Option<&str>,
    reviewed_at: u64,
) -> serde_json::Value {
    let kind = proposal.kind.as_str();
    let normalized_content = proposal
        .normalized_content
        .as_deref()
        .map(normalize_memory_content)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| proposal.summary.clone());
    let mut body = proposal_body(&normalized_content, kind);
    body["summary"] = json!(proposal.summary);
    body["scope"] = json!(proposal.scope);
    body["sessionId"] = json!(proposal.session_id);
    body["sourceRunId"] = json!(proposal.source_run_id);
    body["proposalReason"] = json!(proposal.proposal_reason);
    body["normalizedContent"] = json!(normalized_content);
    if let Some(review) = proposal.review.as_ref() {
        body["review"] = json!({
            "decision": review.decision,
            "reviewerRef": review.reviewer_ref,
            "reviewedAt": review.reviewed_at,
            "note": review.note,
        });
    } else if let Some(note) = note.filter(|value| !value.trim().is_empty()) {
        body["review"] = json!({
            "reviewedAt": reviewed_at,
            "note": note.trim(),
        });
    }
    body["proposalId"] = json!(proposal.proposal_id);
    body["reviewedAt"] = json!(reviewed_at);
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_manifest(actor_ref: &str) -> actor_manifest::CompiledActorManifest {
        actor_manifest::CompiledActorManifest::Agent(actor_manifest::CompiledAgentManifest {
            actor_ref: actor_ref.into(),
            record: octopus_core::AgentRecord {
                id: actor_ref.trim_start_matches("agent:").into(),
                workspace_id: octopus_core::DEFAULT_WORKSPACE_ID.into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                scope: "project".into(),
                name: "Agent".into(),
                avatar_path: None,
                avatar: None,
                personality: "Focused".into(),
                tags: Vec::new(),
                prompt: "Help.".into(),
                builtin_tool_keys: Vec::new(),
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: Vec::new(),
                manifest_revision: "test".into(),
                default_model_strategy: octopus_core::default_model_strategy(),
                capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
                permission_envelope: octopus_core::default_permission_envelope(),
                memory_policy: octopus_core::default_agent_memory_policy(),
                delegation_policy: octopus_core::default_agent_delegation_policy(),
                approval_preference: octopus_core::default_approval_preference(),
                output_contract: octopus_core::default_output_contract(),
                shared_capability_policy: octopus_core::default_agent_shared_capability_policy(),
                integration_source: None,
                trust_metadata: octopus_core::default_asset_trust_metadata(),
                dependency_resolution: Vec::new(),
                import_metadata: octopus_core::default_asset_import_metadata(),
                description: "Test agent".into(),
                status: "active".into(),
                updated_at: 1,
            },
        })
    }

    fn team_manifest(actor_ref: &str) -> actor_manifest::CompiledActorManifest {
        actor_manifest::CompiledActorManifest::Team(actor_manifest::CompiledTeamManifest {
            actor_ref: actor_ref.into(),
            record: octopus_core::TeamRecord {
                id: actor_ref.trim_start_matches("team:").into(),
                workspace_id: octopus_core::DEFAULT_WORKSPACE_ID.into(),
                project_id: Some(octopus_core::DEFAULT_PROJECT_ID.into()),
                scope: "project".into(),
                name: "Team".into(),
                avatar_path: None,
                avatar: None,
                personality: "Collaborative".into(),
                tags: Vec::new(),
                prompt: "Coordinate.".into(),
                builtin_tool_keys: Vec::new(),
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: Vec::new(),
                manifest_revision: "test".into(),
                default_model_strategy: octopus_core::default_model_strategy(),
                capability_policy: octopus_core::capability_policy_from_sources(&[], &[], &[]),
                permission_envelope: octopus_core::default_permission_envelope(),
                memory_policy: octopus_core::default_team_memory_policy(),
                delegation_policy: octopus_core::default_team_delegation_policy(),
                approval_preference: octopus_core::default_approval_preference(),
                output_contract: octopus_core::default_output_contract(),
                shared_capability_policy: octopus_core::default_team_shared_capability_policy(),
                leader_agent_id: None,
                member_agent_ids: Vec::new(),
                leader_ref: "agent:leader".into(),
                member_refs: Vec::new(),
                team_topology: octopus_core::team_topology_from_refs(
                    Some("agent:leader".into()),
                    Vec::new(),
                ),
                shared_memory_policy: octopus_core::default_shared_memory_policy(),
                mailbox_policy: octopus_core::default_mailbox_policy(),
                artifact_handoff_policy: octopus_core::default_artifact_handoff_policy(),
                workflow_affordance: octopus_core::workflow_affordance_from_task_domains(
                    &Vec::new(),
                    true,
                    false,
                ),
                worker_concurrency_limit: 1,
                integration_source: None,
                trust_metadata: octopus_core::default_asset_trust_metadata(),
                dependency_resolution: Vec::new(),
                import_metadata: octopus_core::default_asset_import_metadata(),
                description: "Test team".into(),
                status: "active".into(),
                updated_at: 1,
            },
        })
    }

    fn memory_input(content: &str, intent: &str) -> SubmitRuntimeTurnInput {
        SubmitRuntimeTurnInput {
            content: content.into(),
            permission_mode: None,
            recall_mode: None,
            ignored_memory_ids: Vec::new(),
            memory_intent: Some(intent.into()),
        }
    }

    fn memory_input_without_intent(content: &str) -> SubmitRuntimeTurnInput {
        SubmitRuntimeTurnInput {
            content: content.into(),
            permission_mode: None,
            recall_mode: None,
            ignored_memory_ids: Vec::new(),
            memory_intent: None,
        }
    }

    fn selected_memory_item(
        memory_id: &str,
        kind: &str,
        summary: &str,
        freshness_state: &str,
    ) -> RuntimeSelectedMemoryItem {
        RuntimeSelectedMemoryItem {
            memory_id: memory_id.into(),
            title: format!("{kind} memory"),
            summary: summary.into(),
            kind: kind.into(),
            scope: "project".into(),
            owner_ref: Some("project:proj-runtime".into()),
            source_run_id: Some("run-seed".into()),
            freshness_state: freshness_state.into(),
            last_validated_at: Some(1),
        }
    }

    fn memory_policy() -> octopus_core::MemoryPolicy {
        octopus_core::MemoryPolicy {
            durable_scopes: vec!["project".into(), "user".into()],
            write_requires_approval: true,
            allow_workspace_shared_write: false,
            max_selections: 3,
            freshness_required: false,
        }
    }

    fn execution_response(content: &str) -> ModelExecutionResult {
        ModelExecutionResult {
            content: content.into(),
            request_id: None,
            total_tokens: None,
        }
    }

    fn completed_workflow_detail() -> RuntimeWorkflowRunDetail {
        RuntimeWorkflowRunDetail {
            workflow_run_id: "workflow-1".into(),
            status: "completed".into(),
            current_step_id: Some("step-2".into()),
            current_step_label: Some("Worker".into()),
            total_steps: 2,
            completed_steps: 2,
            background_capable: false,
            steps: Vec::new(),
            blocking: None,
        }
    }

    #[test]
    fn build_memory_proposal_rejects_filler_temp_and_derivable_content() {
        let policy = memory_policy();
        let manifest = agent_manifest("agent:agent-project-delivery");
        assert!(build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &manifest,
            &memory_input("thanks", "feedback"),
            None,
            None,
            &[],
        )
        .is_none());
        assert!(build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &manifest,
            &memory_input("For this task, keep the TODO list open.", "project"),
            None,
            None,
            &[],
        )
        .is_none());
        assert!(build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &manifest,
            &memory_input(
                "Update `config/runtime/workspace.json` to set quota-model as the default.",
                "project",
            ),
            None,
            None,
            &[],
        )
        .is_none());
    }

    #[test]
    fn build_memory_proposal_reuses_existing_stale_memory_for_revalidation() {
        let policy = memory_policy();
        let proposal = build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &agent_manifest("agent:agent-project-delivery"),
            &memory_input(
                "Approval reviews need the finance tag on every request.",
                "feedback",
            ),
            None,
            None,
            &[selected_memory_item(
                "mem-existing",
                "feedback",
                "Approval reviews need the finance tag on every request.",
                "stale",
            )],
        )
        .expect("proposal");

        assert_eq!(proposal.memory_id, "mem-existing");
        assert_eq!(
            proposal.normalized_content.as_deref(),
            Some("Approval reviews need the finance tag on every request.")
        );
    }

    #[test]
    fn build_memory_proposal_uses_runtime_outcome_when_explicit_intent_is_absent() {
        let proposal = build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &memory_policy(),
            &agent_manifest("agent:agent-project-delivery"),
            &memory_input_without_intent("What should I remember from this run?"),
            Some(&execution_response(
                "The user prefers concise implementation summaries.",
            )),
            None,
            &[],
        )
        .expect("proposal from runtime outcome");

        assert_eq!(proposal.kind, "user");
        assert_eq!(proposal.proposal_reason, "runtime-outcome");
        assert_eq!(
            proposal.normalized_content.as_deref(),
            Some("The user prefers concise implementation summaries.")
        );
    }

    #[test]
    fn build_memory_proposal_marks_completed_workflow_output_as_validated_source() {
        let proposal = build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &memory_policy(),
            &agent_manifest("agent:agent-project-delivery"),
            &memory_input_without_intent("Summarize the completed workflow."),
            Some(&execution_response(
                "Approval reviews need the finance tag on every request.",
            )),
            Some(&completed_workflow_detail()),
            &[],
        )
        .expect("proposal from workflow output");

        assert_eq!(proposal.kind, "project");
        assert_eq!(proposal.proposal_reason, "validated-workflow-output");
    }

    #[test]
    fn build_persisted_memory_body_includes_normalized_content_and_review_metadata() {
        let body = build_persisted_memory_body(
            &RuntimeMemoryProposal {
                proposal_id: "proposal-1".into(),
                session_id: "session-1".into(),
                source_run_id: "run-1".into(),
                memory_id: "mem-1".into(),
                title: "Feedback memory proposal".into(),
                summary: "Approval reviews need the finance tag.".into(),
                kind: "feedback".into(),
                scope: "project".into(),
                proposal_state: "revalidated".into(),
                proposal_reason: "user-feedback".into(),
                review: Some(RuntimeMemoryProposalReview {
                    decision: "revalidate".into(),
                    reviewed_at: 42,
                    reviewer_ref: Some("session:session-1".into()),
                    note: Some("freshened".into()),
                }),
                normalized_content: Some(
                    "Approval reviews need the finance tag on every request.".into(),
                ),
            },
            Some("freshened"),
            42,
        );

        assert_eq!(
            body.get("normalizedContent").and_then(Value::as_str),
            Some("Approval reviews need the finance tag on every request.")
        );
        assert_eq!(body.get("scope").and_then(Value::as_str), Some("project"));
        assert_eq!(
            body.get("sourceRunId").and_then(Value::as_str),
            Some("run-1")
        );
        assert_eq!(
            body.get("review")
                .and_then(|value| value.get("decision"))
                .and_then(Value::as_str),
            Some("revalidate")
        );
    }

    #[test]
    fn build_memory_proposal_respects_shared_write_gating_and_actor_scope() {
        let agent_manifest = agent_manifest("agent:agent-project-delivery");
        let team_manifest = team_manifest("team:team-runtime");

        let agent_policy = octopus_core::MemoryPolicy {
            durable_scopes: vec!["user-private".into(), "agent-private".into()],
            write_requires_approval: true,
            allow_workspace_shared_write: false,
            max_selections: 3,
            freshness_required: false,
        };
        let team_policy = octopus_core::MemoryPolicy {
            durable_scopes: vec!["team-shared".into(), "workspace-shared".into()],
            write_requires_approval: true,
            allow_workspace_shared_write: false,
            max_selections: 3,
            freshness_required: false,
        };

        let agent_proposal = build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &agent_policy,
            &agent_manifest,
            &memory_input("Remember my implementation style preference.", "user"),
            None,
            None,
            &[],
        )
        .expect("agent proposal");
        assert_eq!(agent_proposal.scope, "user-private");

        let team_proposal = build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &team_policy,
            &team_manifest,
            &memory_input("Remember the team review checklist.", "project"),
            None,
            None,
            &[],
        )
        .expect("team proposal");
        assert_eq!(team_proposal.scope, "team-shared");
    }
}
