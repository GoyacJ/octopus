use super::*;

const MIN_MEMORY_CONTENT_CHARS: usize = 12;

fn normalize_memory_kind(value: &str) -> &'static str {
    match value.trim() {
        "user" => "user",
        "feedback" => "feedback",
        "project" => "project",
        "reference" => "reference",
        _ => "reference",
    }
}

fn normalize_memory_scope(
    project_id: &str,
    kind: &str,
    policy: &octopus_core::MemoryPolicy,
) -> String {
    if kind == "user" && policy.durable_scopes.iter().any(|scope| scope == "user") {
        return "user".into();
    }
    if !project_id.is_empty() && policy.durable_scopes.iter().any(|scope| scope == "project") {
        return "project".into();
    }
    policy
        .durable_scopes
        .first()
        .cloned()
        .unwrap_or_else(|| "user".into())
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

pub(crate) fn build_memory_proposal(
    session_id: &str,
    run_id: &str,
    project_id: &str,
    policy: &octopus_core::MemoryPolicy,
    input: &SubmitRuntimeTurnInput,
    candidate_memory: &[RuntimeSelectedMemoryItem],
) -> Option<RuntimeMemoryProposal> {
    let intent = input.memory_intent.as_deref()?.trim();
    let content = input.content.trim();
    if intent.is_empty() || content.is_empty() {
        return None;
    }
    let normalized_content = normalize_memory_content(content);
    if reject_memory_content(&normalized_content) {
        return None;
    }

    let kind = normalize_memory_kind(intent);
    let summary = memory_runtime::truncate_memory_summary(&normalized_content);
    let matching_memory = candidate_memory
        .iter()
        .find(|item| item.kind == kind && normalized_text_eq(&item.summary, &summary));
    if matching_memory.is_some_and(|item| item.freshness_state == "fresh") {
        return None;
    }

    Some(RuntimeMemoryProposal {
        proposal_id: format!("memory-proposal-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        source_run_id: run_id.to_string(),
        memory_id: matching_memory
            .map(|item| item.memory_id.clone())
            .unwrap_or_else(|| format!("mem-{}", Uuid::new_v4())),
        title: proposal_title(kind).into(),
        summary,
        kind: kind.into(),
        scope: normalize_memory_scope(project_id, kind, policy),
        proposal_state: "pending".into(),
        proposal_reason: proposal_reason(kind).into(),
        review: None,
        normalized_content: Some(normalized_content),
    })
}

pub(crate) fn build_persisted_memory_record(
    proposal: &RuntimeMemoryProposal,
    project_id: &str,
    reviewed_at: u64,
) -> memory_runtime::PersistedRuntimeMemoryRecord {
    memory_runtime::PersistedRuntimeMemoryRecord {
        memory_id: proposal.memory_id.clone(),
        project_id: if project_id.trim().is_empty() {
            None
        } else {
            Some(project_id.to_string())
        },
        owner_ref: Some(
            if proposal.scope == "project" && !project_id.trim().is_empty() {
                format!("project:{project_id}")
            } else {
                "user:runtime".into()
            },
        ),
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

    fn memory_input(content: &str, intent: &str) -> SubmitRuntimeTurnInput {
        SubmitRuntimeTurnInput {
            content: content.into(),
            permission_mode: None,
            recall_mode: None,
            ignored_memory_ids: Vec::new(),
            memory_intent: Some(intent.into()),
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

    #[test]
    fn build_memory_proposal_rejects_filler_temp_and_derivable_content() {
        let policy = memory_policy();
        assert!(build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &memory_input("thanks", "feedback"),
            &[],
        )
        .is_none());
        assert!(build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &memory_input("For this task, keep the TODO list open.", "project"),
            &[],
        )
        .is_none());
        assert!(build_memory_proposal(
            "session-1",
            "run-1",
            "proj-runtime",
            &policy,
            &memory_input(
                "Update `config/runtime/workspace.json` to set quota-model as the default.",
                "project",
            ),
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
            &memory_input(
                "Approval reviews need the finance tag on every request.",
                "feedback",
            ),
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
}
