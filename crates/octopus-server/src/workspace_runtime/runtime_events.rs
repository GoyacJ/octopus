use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct EventsQuery {
    after: Option<String>,
}

pub(crate) async fn runtime_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Response, ApiError> {
    let request_id = request_id(&headers);
    let project_id = runtime_project_scope(&state, &session_id).await?;
    ensure_authorized_session_with_request_id(
        &state,
        &headers,
        "runtime.session.read",
        project_id.as_deref(),
        &request_id,
    )
    .await?;

    let replay_after = query.after.or_else(|| last_event_id(&headers));

    if !accepts_sse(&headers) {
        let events = state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?;
        let payload = runtime_transport_payload(&events, &request_id)?;
        let mut response = Json(payload).into_response();
        insert_request_id(&mut response, &request_id);
        return Ok(response);
    }

    let replay_events = if replay_after.is_some() {
        state
            .services
            .runtime_session
            .list_events(&session_id, replay_after.as_deref())
            .await?
    } else {
        Vec::new()
    };
    let receiver = state
        .services
        .runtime_execution
        .subscribe_events(&session_id)
        .await?;
    let stream_request_id = request_id.clone();
    let stream = stream! {
        for event in replay_events {
            if let Ok(payload) = runtime_transport_payload(&event, &stream_request_id) {
                if let Ok(data) = serde_json::to_string(&payload) {
                    yield Ok::<Event, std::convert::Infallible>(
                        Event::default()
                            .event(event.event_type.clone())
                            .id(event.id.clone())
                            .data(data)
                    );
                }
            }
        }

        let mut receiver = receiver;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(payload) = runtime_transport_payload(&event, &stream_request_id) {
                        if let Ok(data) = serde_json::to_string(&payload) {
                            yield Ok::<Event, std::convert::Infallible>(
                                Event::default()
                                    .event(event.event_type.clone())
                                    .id(event.id.clone())
                                    .data(data)
                            );
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
        .into_response();
    insert_request_id(&mut response, &request_id);
    Ok(response)
}
