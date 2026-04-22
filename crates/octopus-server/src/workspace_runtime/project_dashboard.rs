use super::task_helpers::task_summary_from_record;
use super::*;

pub(crate) async fn project_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectDashboardSnapshot>, ApiError> {
    ensure_capability_session(
        &state,
        &headers,
        "project.view",
        Some(&project_id),
        Some("project"),
        Some(&project_id),
    )
    .await?;

    let project = lookup_project(&state, &project_id).await?;
    let runtime_document = load_project_runtime_document(&state, &project, None).await?;
    let project_scope = resolve_project_granted_scope(&state, &project, &runtime_document).await?;
    let mut sessions = state.services.runtime_session.list_sessions().await?;
    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated_at));
    sessions.retain(|record| record.project_id == project_id);
    let conversations = sessions
        .iter()
        .map(|record| ConversationRecord {
            id: record.conversation_id.clone(),
            workspace_id: project.workspace_id.clone(),
            project_id: record.project_id.clone(),
            session_id: record.id.clone(),
            title: record.title.clone(),
            status: record.status.clone(),
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview.clone(),
        })
        .collect::<Vec<_>>();

    let mut audit_records = state.services.observation.list_audit_records().await?;
    audit_records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
    audit_records.retain(|record| record.project_id.as_deref() == Some(project_id.as_str()));
    let recent_activity = audit_records
        .iter()
        .take(8)
        .map(workspace_activity_from_audit)
        .collect::<Vec<_>>();

    let resources = state
        .services
        .workspace
        .list_project_resources(&project_id)
        .await?;
    let knowledge = state
        .services
        .workspace
        .list_project_knowledge(&project_id)
        .await?;
    let agents = project_scope.agents.clone();
    let teams = project_scope.teams.clone();
    let cost_entries = state
        .services
        .observation
        .list_cost_entries()
        .await?
        .into_iter()
        .filter(|record| {
            record.project_id.as_deref() == Some(project_id.as_str())
                && record.metric == "tokens"
                && record.amount > 0
        })
        .collect::<Vec<_>>();
    let session_details = load_project_session_details(&state, &sessions).await?;
    let tool_source_keys = project_scope.tool_source_keys.clone();
    let tool_ranking = build_tool_ranking(&session_details, &audit_records);
    let model_breakdown = build_model_breakdown(&cost_entries);
    let trend = build_dashboard_trend(&sessions, &session_details, &cost_entries, &audit_records);
    let users = state.services.access_control.list_users().await?;
    let user_stats = build_user_stats(&project, &users, &audit_records, &trend);
    let conversation_insights =
        build_conversation_insights(&sessions, &session_details, &audit_records);
    let used_tokens = state
        .services
        .observation
        .project_used_tokens(&project_id)
        .await?;
    let task_records = state.services.project_tasks.list_tasks(&project_id).await?;
    let recent_tasks = task_records
        .iter()
        .take(8)
        .map(task_summary_from_record)
        .collect::<Vec<_>>();
    let total_tokens =
        used_tokens.max(cost_entries.iter().map(|record| record.amount as u64).sum());
    let approval_count = session_details
        .values()
        .filter(|detail| detail.pending_mediation.is_some())
        .count() as u64
        + audit_records
            .iter()
            .filter(|record| is_mediation_activity(record))
            .count() as u64;
    let overview = ProjectDashboardSummary {
        member_count: project_member_ids(&project).len() as u64,
        active_user_count: user_stats
            .iter()
            .filter(|item| item.activity_count > 0)
            .count() as u64,
        agent_count: agents.len() as u64,
        team_count: teams.len() as u64,
        conversation_count: conversations.len() as u64,
        message_count: session_details
            .values()
            .map(|detail| detail.messages.len() as u64)
            .sum(),
        tool_call_count: tool_ranking.iter().map(|item| item.value).sum(),
        approval_count,
        resource_count: resources.len() as u64,
        knowledge_count: knowledge.len() as u64,
        tool_count: tool_source_keys.len() as u64,
        token_record_count: cost_entries.len() as u64,
        total_tokens,
        activity_count: audit_records.len() as u64,
        task_count: task_records.len() as u64,
        active_task_count: task_records
            .iter()
            .filter(|record| record.status == "running")
            .count() as u64,
        attention_task_count: task_records
            .iter()
            .filter(|record| record.view_status == "attention")
            .count() as u64,
        scheduled_task_count: task_records
            .iter()
            .filter(|record| record.schedule_spec.is_some())
            .count() as u64,
    };
    let resource_breakdown = vec![
        dashboard_breakdown_item("resources", "resources", resources.len() as u64, None),
        dashboard_breakdown_item("knowledge", "knowledge", knowledge.len() as u64, None),
        dashboard_breakdown_item("agents", "agents", agents.len() as u64, None),
        dashboard_breakdown_item("teams", "teams", teams.len() as u64, None),
        dashboard_breakdown_item(
            "tools",
            "tools",
            tool_source_keys.len() as u64,
            Some(tool_source_keys.join(", ")),
        ),
        dashboard_breakdown_item("sessions", "sessions", conversations.len() as u64, None),
    ];

    Ok(Json(ProjectDashboardSnapshot {
        project,
        metrics: vec![
            metric_record("conversations", "Conversations", conversations.len()),
            metric_record("resources", "Resources", resources.len()),
            metric_record("knowledge", "Knowledge", knowledge.len()),
            metric_record("agents", "Agents", agents.len()),
        ],
        overview,
        trend,
        user_stats,
        conversation_insights,
        tool_ranking,
        resource_breakdown,
        model_breakdown,
        recent_conversations: conversations.into_iter().take(8).collect(),
        recent_activity,
        recent_tasks,
        used_tokens,
    }))
}

fn project_member_ids(project: &ProjectRecord) -> Vec<String> {
    let mut members = BTreeSet::new();
    members.insert(project.owner_user_id.clone());
    for user_id in &project.member_user_ids {
        if !user_id.trim().is_empty() {
            members.insert(user_id.clone());
        }
    }
    members.into_iter().collect()
}

async fn load_project_session_details(
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

fn usage_total_tokens(value: &serde_json::Value) -> Option<u64> {
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

fn message_token_count(message: &RuntimeMessage) -> u64 {
    message
        .usage
        .as_ref()
        .and_then(usage_total_tokens)
        .unwrap_or(0)
}

fn tool_call_label(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(raw) => Some(raw.clone()),
        serde_json::Value::Object(_) => ["toolName", "tool_name", "name", "id"]
            .iter()
            .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
            .map(str::to_string),
        _ => None,
    }
}

fn is_mediation_activity(record: &AuditRecord) -> bool {
    let action = record.action.to_ascii_lowercase();
    let resource = record.resource.to_ascii_lowercase();
    action.contains("approval")
        || action.contains("auth")
        || resource.contains("approval")
        || resource.contains("auth")
}

fn build_bucket_timestamps(
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

fn bucket_index(timestamp: u64, start: u64, step: u64, bucket_count: usize) -> usize {
    if bucket_count <= 1 {
        return 0;
    }
    let raw = timestamp.saturating_sub(start) / step.max(1);
    raw.min(bucket_count.saturating_sub(1) as u64) as usize
}

fn build_dashboard_trend(
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

fn build_conversation_insights(
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

fn build_tool_ranking(
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

fn build_model_breakdown(cost_entries: &[CostLedgerEntry]) -> Vec<ProjectDashboardBreakdownItem> {
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

fn build_user_stats(
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

fn dashboard_breakdown_item(
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
