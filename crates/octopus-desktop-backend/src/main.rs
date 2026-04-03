use std::{
  collections::HashMap,
  env,
  net::SocketAddr,
  sync::{Arc, Mutex},
  time::{SystemTime, UNIX_EPOCH},
};

use axum::{
  extract::{Path, State},
  http::{header, HeaderMap, StatusCode},
  response::{IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use octopus_core::{
  HealthcheckBackendPayload, HealthcheckStatus, ProviderConfig, RuntimeApprovalRequest, RuntimeBootstrap,
  RuntimeDecisionAction, RuntimeEventEnvelope, RuntimeMessage, RuntimeRunSnapshot, RuntimeSessionDetail,
  RuntimeSessionSummary, RuntimeTraceItem,
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState {
  auth_token: String,
  app_version: String,
  cargo_workspace: bool,
  runtime: Arc<Mutex<RuntimeStore>>,
}

#[derive(Debug, Default)]
struct RuntimeStore {
  sessions: HashMap<String, RuntimeSessionDetail>,
  events: HashMap<String, Vec<RuntimeEventEnvelope>>,
  next_sequence: u64,
}

impl RuntimeStore {
  fn next_id(&mut self, prefix: &str) -> String {
    self.next_sequence += 1;
    format!("{prefix}-{}", self.next_sequence)
  }

  fn enqueue_event(&mut self, event: RuntimeEventEnvelope) {
    self
      .events
      .entry(event.session_id.clone())
      .or_default()
      .push(event);
  }

  fn list_sessions(&self) -> Vec<RuntimeSessionSummary> {
    let mut sessions = self
      .sessions
      .values()
      .map(|detail| detail.summary.clone())
      .collect::<Vec<_>>();
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    sessions
  }
}

#[derive(Debug, Deserialize)]
struct CliArgs {
  port: u16,
  auth_token: String,
  app_version: String,
  cargo_workspace: bool,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
  error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest {
  conversation_id: String,
  project_id: String,
  title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitTurnRequest {
  content: String,
  model_id: String,
  permission_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveApprovalRequest {
  decision: RuntimeDecisionAction,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = parse_args()?;
  let state = AppState {
    auth_token: args.auth_token,
    app_version: args.app_version,
    cargo_workspace: args.cargo_workspace,
    runtime: Arc::new(Mutex::new(RuntimeStore::default())),
  };

  let address = SocketAddr::from(([127, 0, 0, 1], args.port));
  let listener = tokio::net::TcpListener::bind(address).await?;
  axum::serve(listener, app(state)).await?;
  Ok(())
}

fn app(state: AppState) -> Router {
  Router::new()
    .route("/health", get(health))
    .route("/runtime/bootstrap", get(runtime_bootstrap))
    .route("/runtime/sessions", get(list_runtime_sessions).post(create_runtime_session))
    .route("/runtime/sessions/{session_id}", get(load_runtime_session))
    .route("/runtime/sessions/{session_id}/events", get(poll_runtime_events))
    .route("/runtime/sessions/{session_id}/turns", post(submit_runtime_turn))
    .route("/runtime/sessions/{session_id}/approvals/{approval_id}", post(resolve_runtime_approval))
    .with_state(state)
}

fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
  let mut port = None;
  let mut auth_token = None;
  let mut app_version = None;
  let mut cargo_workspace = None;

  let mut args = env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--port" => port = args.next().and_then(|value| value.parse().ok()),
      "--auth-token" => auth_token = args.next(),
      "--app-version" => app_version = args.next(),
      "--cargo-workspace" => cargo_workspace = args.next().and_then(|value| value.parse().ok()),
      "--preferences-path" | "--runtime-root" => {
        let _ = args.next();
      }
      _ => {}
    }
  }

  Ok(CliArgs {
    port: port.ok_or("missing --port")?,
    auth_token: auth_token.ok_or("missing --auth-token")?,
    app_version: app_version.ok_or("missing --app-version")?,
    cargo_workspace: cargo_workspace.ok_or("missing --cargo-workspace")?,
  })
}

async fn health(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  Json(HealthcheckStatus {
    status: "ok".into(),
    host: "tauri".into(),
    mode: "local".into(),
    cargo_workspace: state.cargo_workspace,
    backend: HealthcheckBackendPayload {
      state: "ready".into(),
      transport: "http".into(),
    },
  })
  .into_response()
}

async fn runtime_bootstrap(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let sessions = state
    .runtime
    .lock()
    .expect("runtime store should lock")
    .list_sessions();

  Json(RuntimeBootstrap {
    provider: ProviderConfig {
      provider: "anthropic".into(),
      api_key: None,
      base_url: None,
      default_model: Some("claude-sonnet-4-5".into()),
    },
    sessions,
  })
  .into_response()
}

async fn list_runtime_sessions(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let sessions = state
    .runtime
    .lock()
    .expect("runtime store should lock")
    .list_sessions();
  Json(sessions).into_response()
}

async fn create_runtime_session(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(payload): Json<CreateSessionRequest>,
) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let session_id = format!("runtime-session-{}", payload.conversation_id);
  let mut runtime = state.runtime.lock().expect("runtime store should lock");
  if let Some(existing) = runtime.sessions.get(&session_id) {
    return Json(existing.clone()).into_response();
  }

  let timestamp = now_millis();
  let detail = session_detail(
    &session_id,
    &payload.conversation_id,
    &payload.project_id,
    &payload.title,
    &state.app_version,
    timestamp,
  );
  runtime.sessions.insert(session_id, detail.clone());

  Json(detail).into_response()
}

async fn load_runtime_session(
  State(state): State<AppState>,
  headers: HeaderMap,
  Path(session_id): Path<String>,
) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let runtime = state.runtime.lock().expect("runtime store should lock");
  match runtime.sessions.get(&session_id) {
    Some(detail) => Json(detail.clone()).into_response(),
    None => error_response(StatusCode::NOT_FOUND, "runtime session not found"),
  }
}

async fn poll_runtime_events(
  State(state): State<AppState>,
  headers: HeaderMap,
  Path(session_id): Path<String>,
) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let mut runtime = state.runtime.lock().expect("runtime store should lock");
  if !runtime.sessions.contains_key(&session_id) {
    return error_response(StatusCode::NOT_FOUND, "runtime session not found");
  }

  let events = runtime.events.remove(&session_id).unwrap_or_default();
  Json(events).into_response()
}

async fn submit_runtime_turn(
  State(state): State<AppState>,
  headers: HeaderMap,
  Path(session_id): Path<String>,
  Json(payload): Json<SubmitTurnRequest>,
) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let mut runtime = state.runtime.lock().expect("runtime store should lock");
  let Some(existing) = runtime.sessions.get(&session_id).cloned() else {
    return error_response(StatusCode::NOT_FOUND, "runtime session not found");
  };

  let timestamp = now_millis();
  let needs_approval = requires_approval(&payload.content);
  let run_id = format!("runtime-run-{session_id}-{}", runtime.next_id("seq"));

  let user_message = RuntimeMessage {
    id: runtime.next_id("runtime-message-user"),
    session_id: session_id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    sender_type: "user".into(),
    sender_label: "You".into(),
    content: payload.content.clone(),
    timestamp,
    model_id: Some(payload.model_id.clone()),
    status: "completed".into(),
  };
  let assistant_message = RuntimeMessage {
    id: runtime.next_id("runtime-message-assistant"),
    session_id: session_id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    sender_type: "assistant".into(),
    sender_label: "Octopus Runtime".into(),
    content: if needs_approval {
      "运行前需要审批。".into()
    } else {
      "已记录你的运行请求，并生成了运行摘要。".into()
    },
    timestamp: timestamp + 1,
    model_id: Some(payload.model_id.clone()),
    status: if needs_approval {
      "waiting_approval".into()
    } else {
      "completed".into()
    },
  };
  let trace = RuntimeTraceItem {
    id: runtime.next_id("runtime-trace"),
    session_id: session_id.clone(),
    run_id: run_id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    kind: if needs_approval { "approval".into() } else { "step".into() },
    title: if needs_approval {
      "Requested approval for workspace terminal access".into()
    } else {
      "Captured runtime execution step".into()
    },
    detail: if needs_approval {
      "The runtime requested approval before executing a terminal command.".into()
    } else {
      format!("Processed a runtime turn with permission mode {}.", payload.permission_mode)
    },
    tone: if needs_approval { "warning".into() } else { "success".into() },
    timestamp: timestamp + 2,
    actor: "Octopus Runtime".into(),
    related_message_id: Some(assistant_message.id.clone()),
    related_tool_name: if needs_approval {
      Some("terminal".into())
    } else {
      None
    },
  };
  let approval = needs_approval.then(|| RuntimeApprovalRequest {
    id: runtime.next_id("runtime-approval"),
    session_id: session_id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    run_id: run_id.clone(),
    tool_name: "terminal".into(),
    summary: "Workspace terminal access requested".into(),
    detail: payload.content.clone(),
    risk_level: "medium".into(),
    created_at: timestamp + 1,
  });
  let run = RuntimeRunSnapshot {
    id: run_id.clone(),
    session_id: session_id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    status: if needs_approval {
      "waiting_approval".into()
    } else {
      "completed".into()
    },
    current_step: if needs_approval {
      format!("runtime.run.waitingApproval:{}", payload.permission_mode)
    } else {
      format!("runtime.run.completed:{}:{}", payload.model_id, payload.permission_mode)
    },
    started_at: timestamp,
    updated_at: timestamp + 2,
    model_id: Some(payload.model_id),
    next_action: Some(if needs_approval {
      "runtime.run.awaitingApproval".into()
    } else {
      format!("runtime.run.idle:{}", payload.content.len())
    }),
  };

  let next_detail = RuntimeSessionDetail {
    summary: RuntimeSessionSummary {
      id: existing.summary.id.clone(),
      conversation_id: existing.summary.conversation_id.clone(),
      project_id: existing.summary.project_id.clone(),
      title: existing.summary.title.clone(),
      status: run.status.clone(),
      updated_at: run.updated_at,
      last_message_preview: Some(payload.content),
    },
    run: run.clone(),
    messages: vec![existing.messages, vec![user_message.clone(), assistant_message.clone()]].concat(),
    trace: vec![existing.trace, vec![trace.clone()]].concat(),
    pending_approval: approval.clone(),
  };

  runtime.sessions.insert(session_id.clone(), next_detail.clone());
  let run_updated_event_id = runtime.next_id("runtime-event");
  let user_message_event_id = runtime.next_id("runtime-event");
  let assistant_message_event_id = runtime.next_id("runtime-event");
  let trace_event_id = runtime.next_id("runtime-event");
  let approval_event_id = approval.as_ref().map(|_| runtime.next_id("runtime-event"));
  let session_updated_event_id = runtime.next_id("runtime-event");

  runtime.enqueue_event(RuntimeEventEnvelope {
    id: run_updated_event_id,
    kind: "run_updated".into(),
    session_id: session_id.clone(),
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(run_id.clone()),
    emitted_at: timestamp + 2,
    run: Some(run.clone()),
    message: None,
    trace: None,
    approval: None,
    decision: None,
    summary: None,
    error: None,
  });
  runtime.enqueue_event(RuntimeEventEnvelope {
    id: user_message_event_id,
    kind: "message_created".into(),
    session_id: session_id.clone(),
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(run_id.clone()),
    emitted_at: user_message.timestamp,
    run: None,
    message: Some(user_message),
    trace: None,
    approval: None,
    decision: None,
    summary: None,
    error: None,
  });
  runtime.enqueue_event(RuntimeEventEnvelope {
    id: assistant_message_event_id,
    kind: "message_created".into(),
    session_id: session_id.clone(),
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(run_id.clone()),
    emitted_at: assistant_message.timestamp,
    run: None,
    message: Some(assistant_message),
    trace: None,
    approval: None,
    decision: None,
    summary: None,
    error: None,
  });
  runtime.enqueue_event(RuntimeEventEnvelope {
    id: trace_event_id,
    kind: "trace_emitted".into(),
    session_id: session_id.clone(),
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(run_id.clone()),
    emitted_at: trace.timestamp,
    run: None,
    message: None,
    trace: Some(trace),
    approval: None,
    decision: None,
    summary: None,
    error: None,
  });
  if let Some(approval) = approval {
    runtime.enqueue_event(RuntimeEventEnvelope {
      id: approval_event_id.expect("approval event id"),
      kind: "approval_requested".into(),
      session_id: session_id.clone(),
      conversation_id: next_detail.summary.conversation_id.clone(),
      run_id: Some(run_id.clone()),
      emitted_at: approval.created_at,
      run: Some(run.clone()),
      message: None,
      trace: None,
      approval: Some(approval),
      decision: None,
      summary: None,
      error: None,
    });
  }
  runtime.enqueue_event(RuntimeEventEnvelope {
    id: session_updated_event_id,
    kind: "session_updated".into(),
    session_id,
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(run_id),
    emitted_at: next_detail.summary.updated_at,
    run: None,
    message: None,
    trace: None,
    approval: None,
    decision: None,
    summary: Some(next_detail.summary.clone()),
    error: None,
  });

  Json(run).into_response()
}

async fn resolve_runtime_approval(
  State(state): State<AppState>,
  headers: HeaderMap,
  Path((session_id, approval_id)): Path<(String, String)>,
  Json(payload): Json<ResolveApprovalRequest>,
) -> impl IntoResponse {
  if let Err(response) = authorize(&state, &headers) {
    return response;
  }

  let mut runtime = state.runtime.lock().expect("runtime store should lock");
  let Some(existing) = runtime.sessions.get(&session_id).cloned() else {
    return error_response(StatusCode::NOT_FOUND, "runtime session not found");
  };
  let Some(pending_approval) = existing.pending_approval.clone() else {
    return error_response(StatusCode::NOT_FOUND, "runtime approval not found");
  };
  if pending_approval.id != approval_id {
    return error_response(StatusCode::NOT_FOUND, "runtime approval not found");
  }

  let timestamp = now_millis();
  let run = RuntimeRunSnapshot {
    id: existing.run.id.clone(),
    session_id: session_id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    status: match payload.decision {
      RuntimeDecisionAction::Approve => "completed".into(),
      RuntimeDecisionAction::Reject => "blocked".into(),
    },
    current_step: match payload.decision {
      RuntimeDecisionAction::Approve => "runtime.run.resuming".into(),
      RuntimeDecisionAction::Reject => "runtime.run.blocked".into(),
    },
    started_at: existing.run.started_at,
    updated_at: timestamp,
    model_id: existing.run.model_id.clone(),
    next_action: Some(match payload.decision {
      RuntimeDecisionAction::Approve => "runtime.run.idle".into(),
      RuntimeDecisionAction::Reject => "runtime.run.manualRecovery".into(),
    }),
  };
  let trace = RuntimeTraceItem {
    id: runtime.next_id("runtime-trace-approval"),
    session_id: session_id.clone(),
    run_id: existing.run.id.clone(),
    conversation_id: existing.summary.conversation_id.clone(),
    kind: "approval".into(),
    title: match payload.decision {
      RuntimeDecisionAction::Approve => "Approval resolved and run resumed".into(),
      RuntimeDecisionAction::Reject => "Approval rejected and run blocked".into(),
    },
    detail: format!("Approval {approval_id} was {:?}.", payload.decision).to_lowercase(),
    tone: match payload.decision {
      RuntimeDecisionAction::Approve => "success".into(),
      RuntimeDecisionAction::Reject => "warning".into(),
    },
    timestamp,
    actor: "Octopus Runtime".into(),
    related_message_id: None,
    related_tool_name: Some(pending_approval.tool_name.clone()),
  };

  let next_detail = RuntimeSessionDetail {
    summary: RuntimeSessionSummary {
      id: existing.summary.id.clone(),
      conversation_id: existing.summary.conversation_id.clone(),
      project_id: existing.summary.project_id.clone(),
      title: existing.summary.title.clone(),
      status: run.status.clone(),
      updated_at: timestamp,
      last_message_preview: existing.summary.last_message_preview.clone(),
    },
    run: run.clone(),
    messages: existing.messages,
    trace: vec![existing.trace, vec![trace.clone()]].concat(),
    pending_approval: None,
  };

  runtime.sessions.insert(session_id.clone(), next_detail.clone());
  let approval_resolved_event_id = runtime.next_id("runtime-event");
  let trace_event_id = runtime.next_id("runtime-event");
  let session_updated_event_id = runtime.next_id("runtime-event");

  runtime.enqueue_event(RuntimeEventEnvelope {
    id: approval_resolved_event_id,
    kind: "approval_resolved".into(),
    session_id: session_id.clone(),
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(next_detail.run.id.clone()),
    emitted_at: timestamp,
    run: Some(run.clone()),
    message: None,
    trace: None,
    approval: Some(pending_approval),
    decision: Some(payload.decision),
    summary: None,
    error: None,
  });
  runtime.enqueue_event(RuntimeEventEnvelope {
    id: trace_event_id,
    kind: "trace_emitted".into(),
    session_id: session_id.clone(),
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(next_detail.run.id.clone()),
    emitted_at: trace.timestamp,
    run: None,
    message: None,
    trace: Some(trace),
    approval: None,
    decision: None,
    summary: None,
    error: None,
  });
  runtime.enqueue_event(RuntimeEventEnvelope {
    id: session_updated_event_id,
    kind: "session_updated".into(),
    session_id,
    conversation_id: next_detail.summary.conversation_id.clone(),
    run_id: Some(next_detail.run.id.clone()),
    emitted_at: timestamp,
    run: Some(run),
    message: None,
    trace: None,
    approval: None,
    decision: None,
    summary: Some(next_detail.summary.clone()),
    error: None,
  });

  StatusCode::NO_CONTENT.into_response()
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
  let expected = format!("Bearer {}", state.auth_token);
  let actual = headers
    .get(header::AUTHORIZATION)
    .and_then(|value| value.to_str().ok());

  if actual == Some(expected.as_str()) {
    Ok(())
  } else {
    Err(error_response(StatusCode::UNAUTHORIZED, "unauthorized"))
  }
}

fn error_response(status: StatusCode, error: &str) -> Response {
  (status, Json(ErrorBody { error: error.into() })).into_response()
}

fn requires_approval(content: &str) -> bool {
  let lower = content.to_ascii_lowercase();
  ["pwd", "rm", "delete", "terminal", "bash", "shell"]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn now_millis() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("clock should be after unix epoch")
    .as_millis() as u64
}

fn session_detail(
  session_id: &str,
  conversation_id: &str,
  project_id: &str,
  title: &str,
  app_version: &str,
  timestamp: u64,
) -> RuntimeSessionDetail {
  RuntimeSessionDetail {
    summary: RuntimeSessionSummary {
      id: session_id.into(),
      conversation_id: conversation_id.into(),
      project_id: project_id.into(),
      title: title.into(),
      status: "idle".into(),
      updated_at: timestamp,
      last_message_preview: Some(format!("backend {app_version} ready")),
    },
    run: RuntimeRunSnapshot {
      id: format!("runtime-run-{session_id}-idle"),
      session_id: session_id.into(),
      conversation_id: conversation_id.into(),
      status: "idle".into(),
      current_step: "runtime.run.idle".into(),
      started_at: timestamp,
      updated_at: timestamp,
      model_id: None,
      next_action: Some("runtime.run.awaitingInput".into()),
    },
    messages: vec![],
    trace: vec![],
    pending_approval: None,
  }
}

#[cfg(test)]
mod tests {
  use super::{app, AppState, RuntimeStore};
  use reqwest::Client;
  use std::{
    sync::{Arc, Mutex},
    time::Duration,
  };
  use tokio::{net::TcpListener, task::JoinHandle};

  struct TestServer {
    address: std::net::SocketAddr,
    handle: JoinHandle<()>,
  }

  impl TestServer {
    async fn spawn() -> Self {
      let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener should bind");
      let address = listener
        .local_addr()
        .expect("listener should report local address");
      let state = AppState {
        auth_token: "desktop-test-token".into(),
        app_version: "0.1.0-test".into(),
        cargo_workspace: true,
        runtime: Arc::new(Mutex::new(RuntimeStore::default())),
      };
      let handle = tokio::spawn(async move {
        axum::serve(listener, app(state))
          .await
          .expect("server should run");
      });

      Self { address, handle }
    }

    fn url(&self, path: &str) -> String {
      format!("http://{}{}", self.address, path)
    }
  }

  impl Drop for TestServer {
    fn drop(&mut self) {
      self.handle.abort();
    }
  }

  fn auth(_client: Client) -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
      reqwest::header::AUTHORIZATION,
      reqwest::header::HeaderValue::from_static("Bearer desktop-test-token"),
    );
    Client::builder()
      .default_headers(headers)
      .timeout(Duration::from_secs(5))
      .build()
      .expect("client should build")
  }

  #[tokio::test]
  async fn rejects_unauthorized_healthcheck() {
    let server = TestServer::spawn().await;

    let response = Client::new()
      .get(server.url("/health"))
      .send()
      .await
      .expect("request should succeed");

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    let payload = response
      .json::<serde_json::Value>()
      .await
      .expect("payload should parse");
    assert_eq!(payload["error"], "unauthorized");
  }

  #[tokio::test]
  async fn serves_runtime_session_lifecycle() {
    let server = TestServer::spawn().await;
    let client = auth(Client::new());

    let bootstrap = client
      .get(server.url("/runtime/bootstrap"))
      .send()
      .await
      .expect("bootstrap request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("bootstrap payload should parse");
    let created = client
      .post(server.url("/runtime/sessions"))
      .json(&serde_json::json!({
        "conversationId": "conv-1",
        "projectId": "proj-1",
        "title": "Conversation"
      }))
      .send()
      .await
      .expect("create request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("create payload should parse");
    let listed = client
      .get(server.url("/runtime/sessions"))
      .send()
      .await
      .expect("list request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("list payload should parse");
    let turn = client
      .post(server.url("/runtime/sessions/runtime-session-conv-1/turns"))
      .json(&serde_json::json!({
        "content": "hello runtime",
        "modelId": "claude-sonnet-4-5",
        "permissionMode": "auto"
      }))
      .send()
      .await
      .expect("turn request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("turn payload should parse");
    let events = client
      .get(server.url("/runtime/sessions/runtime-session-conv-1/events"))
      .send()
      .await
      .expect("events request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("events payload should parse");
    let loaded = client
      .get(server.url("/runtime/sessions/runtime-session-conv-1"))
      .send()
      .await
      .expect("load request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("load payload should parse");
    let drained_events = client
      .get(server.url("/runtime/sessions/runtime-session-conv-1/events"))
      .send()
      .await
      .expect("drained events request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("drained events payload should parse");

    assert_eq!(bootstrap["provider"]["provider"], "anthropic");
    assert_eq!(bootstrap["sessions"], serde_json::json!([]));
    assert_eq!(created["summary"]["conversationId"], "conv-1");
    assert_eq!(created["summary"]["projectId"], "proj-1");
    assert_eq!(listed.as_array().expect("listed sessions").len(), 1);
    assert_eq!(turn["status"], "completed");
    assert_eq!(turn["modelId"], "claude-sonnet-4-5");
    assert_eq!(loaded["messages"].as_array().expect("messages").len(), 2);
    assert_eq!(loaded["trace"].as_array().expect("trace").len(), 1);
    assert_eq!(loaded["run"]["status"], "completed");
    assert_eq!(loaded["summary"]["lastMessagePreview"], "hello runtime");
    assert!(events.as_array().expect("events").iter().any(|event| event["kind"] == "message_created"));
    assert!(events.as_array().expect("events").iter().any(|event| event["kind"] == "session_updated"));
    assert_eq!(drained_events, serde_json::json!([]));
  }

  #[tokio::test]
  async fn handles_runtime_approval_flow() {
    let server = TestServer::spawn().await;
    let client = auth(Client::new());

    client
      .post(server.url("/runtime/sessions"))
      .json(&serde_json::json!({
        "conversationId": "conv-2",
        "projectId": "proj-2",
        "title": "Approval Conversation"
      }))
      .send()
      .await
      .expect("create request should succeed");

    let turn = client
      .post(server.url("/runtime/sessions/runtime-session-conv-2/turns"))
      .json(&serde_json::json!({
        "content": "bash pwd",
        "modelId": "claude-sonnet-4-5",
        "permissionMode": "auto"
      }))
      .send()
      .await
      .expect("turn request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("turn payload should parse");
    let waiting = client
      .get(server.url("/runtime/sessions/runtime-session-conv-2"))
      .send()
      .await
      .expect("load request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("load payload should parse");
    let approval_id = waiting["pendingApproval"]["id"]
      .as_str()
      .expect("approval id")
      .to_string();
    let waiting_events = client
      .get(server.url("/runtime/sessions/runtime-session-conv-2/events"))
      .send()
      .await
      .expect("events request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("events payload should parse");
    let approval = client
      .post(server.url(&format!(
        "/runtime/sessions/runtime-session-conv-2/approvals/{approval_id}"
      )))
      .json(&serde_json::json!({
        "decision": "approve"
      }))
      .send()
      .await
      .expect("approval request should succeed");
    let resolved_events = client
      .get(server.url("/runtime/sessions/runtime-session-conv-2/events"))
      .send()
      .await
      .expect("resolved events request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("resolved events payload should parse");
    let resolved = client
      .get(server.url("/runtime/sessions/runtime-session-conv-2"))
      .send()
      .await
      .expect("resolved load request should succeed")
      .json::<serde_json::Value>()
      .await
      .expect("resolved payload should parse");

    assert_eq!(turn["status"], "waiting_approval");
    assert_eq!(waiting["run"]["status"], "waiting_approval");
    assert_eq!(waiting["pendingApproval"]["toolName"], "terminal");
    assert!(waiting_events.as_array().expect("events").iter().any(|event| event["kind"] == "approval_requested"));
    assert_eq!(approval.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(resolved_events.as_array().expect("events").iter().any(|event| event["kind"] == "approval_resolved"));
    assert_eq!(resolved["run"]["status"], "completed");
    assert!(resolved["pendingApproval"].is_null());
  }
}
