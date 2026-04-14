use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedRuntimeMemoryRecord {
    pub(super) memory_id: String,
    pub(super) project_id: Option<String>,
    pub(super) owner_ref: Option<String>,
    pub(super) source_run_id: Option<String>,
    pub(super) kind: String,
    pub(super) scope: String,
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) freshness_state: String,
    pub(super) last_validated_at: Option<u64>,
    pub(super) proposal_state: String,
    pub(super) storage_path: Option<String>,
    pub(super) content_hash: Option<String>,
    pub(super) updated_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedRuntimeMemoryProposalArtifact {
    pub(super) proposal: RuntimeMemoryProposal,
    pub(super) updated_at: u64,
}

pub(super) fn default_memory_policy() -> octopus_core::MemoryPolicy {
    octopus_core::MemoryPolicy {
        durable_scopes: vec!["user".into(), "project".into()],
        write_requires_approval: true,
        allow_workspace_shared_write: false,
        max_selections: 3,
        freshness_required: false,
    }
}

pub(super) fn parse_memory_policy(value: &serde_json::Value) -> octopus_core::MemoryPolicy {
    serde_json::from_value(value.clone()).unwrap_or_else(|_| default_memory_policy())
}

pub(super) fn runtime_memory_state_ref(run_id: &str, now: u64) -> String {
    format!("memory-state-{run_id}-{now}")
}

pub(super) fn truncate_memory_summary(content: &str) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut summary = collapsed.chars().take(180).collect::<String>();
    if collapsed.chars().count() > 180 {
        summary.push_str("...");
    }
    summary
}

pub(super) fn build_memory_summary(
    selected_memory: &[RuntimeSelectedMemoryItem],
    total_candidate_count: u64,
    ignored_count: u64,
    recall_mode: &str,
    freshness_required: bool,
) -> (
    RuntimeMemorySummary,
    RuntimeMemorySelectionSummary,
    RuntimeMemoryFreshnessSummary,
) {
    let fresh_count = selected_memory
        .iter()
        .filter(|item| item.freshness_state == "fresh")
        .count() as u64;
    let stale_count = selected_memory.len() as u64 - fresh_count;
    let selected_memory_ids = selected_memory
        .iter()
        .map(|item| item.memory_id.clone())
        .collect::<Vec<_>>();

    (
        RuntimeMemorySummary {
            summary: if recall_mode == "skip" {
                "Runtime memory recall skipped for this run.".into()
            } else if selected_memory.is_empty() {
                "No durable memories selected.".into()
            } else {
                format!(
                    "{} durable memory item(s) selected{}.",
                    selected_memory.len(),
                    if freshness_required {
                        "; freshness required"
                    } else {
                        ""
                    }
                )
            },
            durable_memory_count: total_candidate_count,
            selected_memory_ids: selected_memory_ids.clone(),
        },
        RuntimeMemorySelectionSummary {
            total_candidate_count,
            selected_count: selected_memory.len() as u64,
            ignored_count,
            recall_mode: recall_mode.into(),
            selected_memory_ids,
        },
        RuntimeMemoryFreshnessSummary {
            freshness_required,
            fresh_count,
            stale_count,
        },
    )
}

pub(super) fn memory_proposal_state_from_decision(
    decision: &str,
) -> Result<&'static str, AppError> {
    match decision {
        "approve" => Ok("approved"),
        "reject" => Ok("rejected"),
        "ignore" => Ok("ignored"),
        "revalidate" => Ok("revalidated"),
        _ => Err(AppError::invalid_input(
            "memory proposal decision must be approve, reject, ignore, or revalidate",
        )),
    }
}
