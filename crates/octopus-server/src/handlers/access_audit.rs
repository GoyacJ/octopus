use super::*;

pub(crate) async fn list_access_audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<AccessAuditQuery>,
) -> Result<Json<AccessAuditListResponse>, ApiError> {
    const PAGE_SIZE: usize = 50;
    ensure_authorized_session(&state, &headers, "audit.read", None).await?;
    let mut items = state.services.observation.list_audit_records().await?;
    items.sort_by_key(|item| std::cmp::Reverse(item.created_at));
    if let Some(actor_id) = query.actor_id.as_deref() {
        items.retain(|record| record.actor_id == actor_id);
    }
    if let Some(action) = query.action.as_deref() {
        items.retain(|record| record.action == action);
    }
    if let Some(resource_type) = query.resource_type.as_deref() {
        items.retain(|record| {
            record.resource == resource_type
                || record
                    .resource
                    .strip_prefix(resource_type)
                    .is_some_and(|suffix| suffix.starts_with(':'))
        });
    }
    if let Some(outcome) = query.outcome.as_deref() {
        items.retain(|record| record.outcome == outcome);
    }
    if let Some(from) = query.from {
        items.retain(|record| record.created_at >= from);
    }
    if let Some(to) = query.to {
        items.retain(|record| record.created_at <= to);
    }
    if let Some(cursor) = query.cursor.as_deref() {
        items.retain(|record| record.created_at.to_string().as_str() < cursor);
    }
    let next_cursor = items
        .get(PAGE_SIZE)
        .map(|record| record.created_at.to_string());
    items.truncate(PAGE_SIZE);
    Ok(Json(AccessAuditListResponse { items, next_cursor }))
}
