use super::*;

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

pub(crate) fn build_memory_proposal(
    session_id: &str,
    run_id: &str,
    project_id: &str,
    policy: &octopus_core::MemoryPolicy,
    input: &SubmitRuntimeTurnInput,
    selected_memory: &[RuntimeSelectedMemoryItem],
) -> Option<RuntimeMemoryProposal> {
    let intent = input.memory_intent.as_deref()?.trim();
    let content = input.content.trim();
    if intent.is_empty() || content.is_empty() {
        return None;
    }

    let kind = normalize_memory_kind(intent);
    let summary = memory_runtime::truncate_memory_summary(content);
    if selected_memory.iter().any(|item| {
        item.kind == kind && item.summary.eq_ignore_ascii_case(&summary) && item.freshness_state == "fresh"
    }) {
        return None;
    }

    Some(RuntimeMemoryProposal {
        proposal_id: format!("memory-proposal-{}", Uuid::new_v4()),
        session_id: session_id.to_string(),
        source_run_id: run_id.to_string(),
        memory_id: format!("mem-{}", Uuid::new_v4()),
        title: proposal_title(kind).into(),
        summary,
        kind: kind.into(),
        scope: normalize_memory_scope(project_id, kind, policy),
        proposal_state: "pending".into(),
        proposal_reason: proposal_reason(kind).into(),
        review: None,
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
        owner_ref: Some(if proposal.scope == "project" && !project_id.trim().is_empty() {
            format!("project:{project_id}")
        } else {
            "user:runtime".into()
        }),
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
    let mut body = proposal_body(&proposal.summary, kind);
    if let Some(note) = note.filter(|value| !value.trim().is_empty()) {
        body["reviewNote"] = json!(note.trim());
    }
    body["proposalId"] = json!(proposal.proposal_id);
    body["reviewedAt"] = json!(reviewed_at);
    body
}
