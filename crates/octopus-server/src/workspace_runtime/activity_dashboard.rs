use super::*;

pub(super) fn project_member_ids(project: &ProjectRecord) -> Vec<String> {
    let mut members = BTreeSet::new();
    members.insert(project.owner_user_id.clone());
    for user_id in &project.member_user_ids {
        if !user_id.trim().is_empty() {
            members.insert(user_id.clone());
        }
    }
    members.into_iter().collect()
}

pub(super) fn workspace_activity_from_audit(record: &AuditRecord) -> WorkspaceActivityRecord {
    WorkspaceActivityRecord {
        id: record.id.clone(),
        title: record.action.clone(),
        description: format!(
            "{} {} {}",
            record.actor_type, record.actor_id, record.outcome
        ),
        timestamp: record.created_at,
        actor_id: Some(record.actor_id.clone()),
        actor_type: Some(record.actor_type.clone()),
        resource: Some(record.resource.clone()),
        outcome: Some(record.outcome.clone()),
    }
}

pub(super) async fn load_project_session_details(
    state: &ServerState,
    sessions: &[octopus_core::RuntimeSessionSummary],
) -> Result<HashMap<String, octopus_core::RuntimeSessionDetail>, ApiError> {
    let mut details = HashMap::new();
    for session in sessions {
        if let Ok(detail) = state
            .services
            .runtime_session
            .get_session(&session.id)
            .await
        {
            details.insert(session.id.clone(), detail);
        }
    }
    Ok(details)
}

pub(super) fn usage_total_tokens(value: &serde_json::Value) -> Option<u64> {
    let direct = ["total_tokens", "totalTokens", "tokens"]
        .iter()
        .find_map(|key| value.get(key).and_then(serde_json::Value::as_u64));
    if direct.is_some() {
        return direct;
    }

    let input = value
        .get("input_tokens")
        .or_else(|| value.get("inputTokens"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let output = value
        .get("output_tokens")
        .or_else(|| value.get("outputTokens"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    (input > 0 || output > 0).then_some(input + output)
}

pub(super) fn message_token_count(message: &RuntimeMessage) -> u64 {
    message
        .usage
        .as_ref()
        .and_then(usage_total_tokens)
        .unwrap_or(0)
}

pub(super) fn tool_call_label(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(raw) => Some(raw.clone()),
        serde_json::Value::Object(_) => ["toolName", "tool_name", "name", "id"]
            .iter()
            .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
            .map(str::to_string),
        _ => None,
    }
}

pub(super) fn is_mediation_activity(record: &AuditRecord) -> bool {
    let action = record.action.to_ascii_lowercase();
    let resource = record.resource.to_ascii_lowercase();
    action.contains("approval")
        || action.contains("auth")
        || resource.contains("approval")
        || resource.contains("auth")
}

pub(super) fn build_bucket_timestamps(
    sessions: &[octopus_core::RuntimeSessionSummary],
    cost_entries: &[CostLedgerEntry],
    audit_records: &[AuditRecord],
    bucket_count: usize,
) -> (Vec<ProjectDashboardTrendPoint>, u64, u64) {
    let mut timestamps = sessions
        .iter()
        .map(|record| record.updated_at)
        .collect::<Vec<_>>();
    timestamps.extend(cost_entries.iter().map(|record| record.created_at));
    timestamps.extend(audit_records.iter().map(|record| record.created_at));

    let max_timestamp = timestamps.iter().copied().max().unwrap_or(0);
    let min_timestamp = timestamps.iter().copied().min().unwrap_or(max_timestamp);
    let span = max_timestamp.saturating_sub(min_timestamp);
    let step =
        ((span.max(bucket_count.saturating_sub(1) as u64)) / bucket_count.max(1) as u64).max(1);
    let start = max_timestamp.saturating_sub(step * bucket_count.saturating_sub(1) as u64);

    let buckets = (0..bucket_count)
        .map(|index| {
            let timestamp = start + step * index as u64;
            ProjectDashboardTrendPoint {
                id: format!("bucket-{index}"),
                label: timestamp.to_string(),
                timestamp,
                conversation_count: 0,
                message_count: 0,
                tool_call_count: 0,
                approval_count: 0,
                token_count: 0,
            }
        })
        .collect::<Vec<_>>();

    (buckets, start, step)
}

pub(super) fn bucket_index(timestamp: u64, start: u64, step: u64, bucket_count: usize) -> usize {
    if bucket_count <= 1 {
        return 0;
    }
    let raw = timestamp.saturating_sub(start) / step.max(1);
    raw.min(bucket_count.saturating_sub(1) as u64) as usize
}

pub(super) fn build_dashboard_trend(
    sessions: &[octopus_core::RuntimeSessionSummary],
    session_details: &HashMap<String, octopus_core::RuntimeSessionDetail>,
    cost_entries: &[CostLedgerEntry],
    audit_records: &[AuditRecord],
) -> Vec<ProjectDashboardTrendPoint> {
    let bucket_count = 7;
    let (mut buckets, start, step) =
        build_bucket_timestamps(sessions, cost_entries, audit_records, bucket_count);

    for session in sessions {
        let index = bucket_index(session.updated_at, start, step, bucket_count);
        buckets[index].conversation_count += 1;
        if let Some(detail) = session_details.get(&session.id) {
            let mut session_tokens = 0_u64;
            for message in &detail.messages {
                let message_index = bucket_index(message.timestamp, start, step, bucket_count);
                let token_count = message_token_count(message);
                let tool_calls = message.tool_calls.as_ref().map_or(0, Vec::len) as u64;
                buckets[message_index].message_count += 1;
                buckets[message_index].tool_call_count += tool_calls;
                buckets[message_index].token_count += token_count;
                session_tokens += token_count;
            }
            if session_tokens == 0 {
                buckets[index].token_count += u64::from(detail.run.consumed_tokens.unwrap_or(0));
            }
            if detail.pending_mediation.is_some() {
                buckets[index].approval_count += 1;
            }
        }
    }

    for record in cost_entries {
        let index = bucket_index(record.created_at, start, step, bucket_count);
        buckets[index].token_count += record.amount.max(0) as u64;
    }

    for record in audit_records {
        if is_mediation_activity(record) {
            let index = bucket_index(record.created_at, start, step, bucket_count);
            buckets[index].approval_count += 1;
        }
    }

    buckets
}

pub(super) fn build_conversation_insights(
    sessions: &[octopus_core::RuntimeSessionSummary],
    session_details: &HashMap<String, octopus_core::RuntimeSessionDetail>,
    audit_records: &[AuditRecord],
) -> Vec<ProjectDashboardConversationInsight> {
    let mut items = sessions
        .iter()
        .map(|session| {
            let detail = session_details.get(&session.id);
            let message_count = detail.map_or(0, |value| value.messages.len() as u64);
            let tool_call_count = detail.map_or(0, |value| {
                value
                    .messages
                    .iter()
                    .map(|message| message.tool_calls.as_ref().map_or(0, Vec::len) as u64)
                    .sum()
            });
            let token_count = detail.map_or(0, |value| {
                let total = value.messages.iter().map(message_token_count).sum::<u64>();
                if total > 0 {
                    total
                } else {
                    u64::from(value.run.consumed_tokens.unwrap_or(0))
                }
            });
            let approval_count = detail
                .and_then(|value| value.pending_mediation.as_ref())
                .map(|_| 1_u64)
                .unwrap_or(0)
                + audit_records
                    .iter()
                    .filter(|record| {
                        is_mediation_activity(record)
                            && (record.resource.contains(&session.id)
                                || record.resource.contains(&session.conversation_id))
                    })
                    .count() as u64;
            ProjectDashboardConversationInsight {
                id: session.id.clone(),
                conversation_id: session.conversation_id.clone(),
                title: session.title.clone(),
                status: session.status.clone(),
                updated_at: session.updated_at,
                last_message_preview: session.last_message_preview.clone(),
                message_count,
                tool_call_count,
                approval_count,
                token_count,
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .token_count
            .cmp(&left.token_count)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
    items
}

pub(super) fn build_tool_ranking(
    session_details: &HashMap<String, octopus_core::RuntimeSessionDetail>,
    audit_records: &[AuditRecord],
) -> Vec<ProjectDashboardRankingItem> {
    let mut counts = BTreeMap::<String, u64>::new();
    for detail in session_details.values() {
        for message in &detail.messages {
            for tool_call in message.tool_calls.clone().unwrap_or_default() {
                if let Some(label) = tool_call_label(&tool_call) {
                    *counts.entry(label).or_default() += 1;
                }
            }
        }
    }

    if counts.is_empty() {
        for record in audit_records {
            if record.resource.trim().is_empty() {
                continue;
            }
            *counts.entry(record.resource.clone()).or_default() += 1;
        }
    }

    let mut rows = counts
        .into_iter()
        .map(|(label, value)| ProjectDashboardRankingItem {
            id: label.to_ascii_lowercase().replace(' ', "-"),
            label,
            value,
            helper: None,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .value
            .cmp(&left.value)
            .then_with(|| left.label.cmp(&right.label))
    });
    rows.into_iter().take(8).collect()
}

pub(super) fn build_model_breakdown(
    cost_entries: &[CostLedgerEntry],
) -> Vec<ProjectDashboardBreakdownItem> {
    let mut grouped = BTreeMap::<String, u64>::new();
    for record in cost_entries {
        let key = record
            .configured_model_id
            .clone()
            .unwrap_or_else(|| "unassigned".into());
        *grouped.entry(key).or_default() += record.amount.max(0) as u64;
    }

    grouped
        .into_iter()
        .map(|(label, value)| dashboard_breakdown_item(&label, &label, value, None))
        .collect()
}

pub(super) fn build_user_stats(
    project: &ProjectRecord,
    users: &[AccessUserRecord],
    audit_records: &[AuditRecord],
    trend: &[ProjectDashboardTrendPoint],
) -> Vec<ProjectDashboardUserStat> {
    let member_ids = project_member_ids(project);
    let mut display_names = users
        .iter()
        .map(|record| (record.id.clone(), record.display_name.clone()))
        .collect::<HashMap<_, _>>();
    for user_id in &member_ids {
        display_names
            .entry(user_id.clone())
            .or_insert_with(|| user_id.clone());
    }

    let mut stats = member_ids
        .iter()
        .map(|user_id| {
            (
                user_id.clone(),
                ProjectDashboardUserStat {
                    user_id: user_id.clone(),
                    display_name: display_names
                        .get(user_id)
                        .cloned()
                        .unwrap_or_else(|| user_id.clone()),
                    activity_count: 0,
                    conversation_count: 0,
                    message_count: 0,
                    tool_call_count: 0,
                    approval_count: 0,
                    token_count: 0,
                    activity_trend: vec![0; trend.len()],
                    token_trend: vec![0; trend.len()],
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let start = trend.first().map(|item| item.timestamp).unwrap_or(0);
    let step = if trend.len() > 1 {
        trend[1].timestamp.saturating_sub(trend[0].timestamp).max(1)
    } else {
        1
    };

    for record in audit_records {
        let Some(user_id) = Some(&record.actor_id) else {
            continue;
        };
        let Some(item) = stats.get_mut(user_id) else {
            continue;
        };
        let index = bucket_index(record.created_at, start, step, trend.len().max(1));
        item.activity_count += 1;
        item.activity_trend[index] += 1;
        if is_mediation_activity(record) {
            item.approval_count += 1;
        }
    }

    for (index, bucket) in trend.iter().enumerate() {
        let active_ids = stats
            .iter()
            .filter_map(|(user_id, item)| {
                (item.activity_trend[index] > 0).then_some(user_id.clone())
            })
            .collect::<Vec<_>>();
        let total_activity = active_ids
            .iter()
            .map(|user_id| {
                stats
                    .get(user_id)
                    .map_or(0, |item| item.activity_trend[index])
            })
            .sum::<u64>();

        if active_ids.is_empty() {
            if let Some(owner) = stats.get_mut(&project.owner_user_id) {
                owner.token_count += bucket.token_count;
                owner.token_trend[index] += bucket.token_count;
                owner.message_count += bucket.message_count;
                owner.tool_call_count += bucket.tool_call_count;
            }
            continue;
        }

        let fallback_user_id = active_ids.first().cloned();
        let mut remaining_tokens = bucket.token_count;
        let mut remaining_messages = bucket.message_count;
        let mut remaining_tools = bucket.tool_call_count;
        for user_id in &active_ids {
            let share = stats
                .get(user_id)
                .map_or(0, |item| item.activity_trend[index]);
            let denominator = total_activity.max(1);
            let token_share = bucket.token_count * share / denominator;
            let message_share = bucket.message_count * share / denominator;
            let tool_share = bucket.tool_call_count * share / denominator;
            if let Some(item) = stats.get_mut(user_id) {
                item.token_count += token_share;
                item.token_trend[index] += token_share;
                item.message_count += message_share;
                item.tool_call_count += tool_share;
            }
            remaining_tokens = remaining_tokens.saturating_sub(token_share);
            remaining_messages = remaining_messages.saturating_sub(message_share);
            remaining_tools = remaining_tools.saturating_sub(tool_share);
        }

        if let Some(user_id) = fallback_user_id {
            if let Some(item) = stats.get_mut(&user_id) {
                item.token_count += remaining_tokens;
                item.token_trend[index] += remaining_tokens;
                item.message_count += remaining_messages;
                item.tool_call_count += remaining_tools;
            }
        }
    }

    for item in stats.values_mut() {
        item.conversation_count = u64::from(item.activity_count > 0);
    }

    let mut rows = stats.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .token_count
            .cmp(&left.token_count)
            .then_with(|| right.activity_count.cmp(&left.activity_count))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    rows
}

pub(super) fn dashboard_breakdown_item(
    id: &str,
    label: &str,
    value: u64,
    helper: Option<String>,
) -> ProjectDashboardBreakdownItem {
    ProjectDashboardBreakdownItem {
        id: id.into(),
        label: label.into(),
        value,
        helper,
    }
}

pub(crate) async fn list_conversation_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<ConversationRecord>, ApiError> {
    let workspace_id = state.services.workspace.workspace_summary().await?.id;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated_at));
    Ok(sessions
        .into_iter()
        .filter(|record| project_id.map(|id| record.project_id == id).unwrap_or(true))
        .map(|record| ConversationRecord {
            id: record.conversation_id.clone(),
            workspace_id: workspace_id.clone(),
            project_id: record.project_id.clone(),
            session_id: record.id,
            title: record.title,
            status: record.status,
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview,
        })
        .collect())
}

pub(crate) async fn list_activity_records(
    state: &ServerState,
    project_id: Option<&str>,
) -> Result<Vec<WorkspaceActivityRecord>, ApiError> {
    let mut records = state.services.observation.list_audit_records().await?;
    records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
    Ok(records
        .into_iter()
        .filter(|record| {
            project_id
                .map(|id| record.project_id.as_deref() == Some(id))
                .unwrap_or(true)
        })
        .map(|record| workspace_activity_from_audit(&record))
        .collect())
}
