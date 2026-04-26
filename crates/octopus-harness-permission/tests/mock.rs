#![cfg(feature = "mock")]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use harness_contracts::{
    Decision, DecisionId, DecisionScope, FallbackPolicy, InteractivityLevel, PermissionError,
    PermissionMode, PermissionSubject, RequestId, SessionId, Severity, TenantId, ToolUseId,
};
use harness_permission::{
    DecisionPersistence, MockBroker, PermissionBroker, PermissionContext, PermissionRequest,
    RuleSnapshot,
};

#[derive(Default)]
struct RecordingPersistence {
    calls: Mutex<Vec<(DecisionId, DecisionScope)>>,
}

#[async_trait]
impl DecisionPersistence for RecordingPersistence {
    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        self.calls.lock().unwrap().push((decision_id, scope));
        Ok(())
    }
}

#[tokio::test]
async fn mock_broker_replays_scripted_decisions_in_order() {
    let broker = MockBroker::new(vec![Decision::AllowOnce, Decision::DenyPermanent]);

    assert_eq!(
        broker
            .decide(permission_request("first"), permission_context())
            .await,
        Decision::AllowOnce
    );
    assert_eq!(
        broker
            .decide(permission_request("second"), permission_context())
            .await,
        Decision::DenyPermanent
    );
}

#[tokio::test]
async fn mock_broker_fails_closed_when_script_is_exhausted() {
    let broker = MockBroker::default();

    assert_eq!(
        broker
            .decide(permission_request("exhausted"), permission_context())
            .await,
        Decision::DenyOnce
    );
}

#[tokio::test]
async fn mock_broker_records_request_and_context() {
    let broker = MockBroker::new(vec![Decision::AllowSession]);
    let request = permission_request("recorded");
    let ctx = permission_context();
    let expected_request_id = request.request_id;
    let expected_session_id = ctx.session_id;

    broker.decide(request, ctx).await;

    let calls = broker.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].request.request_id, expected_request_id);
    assert_eq!(calls[0].ctx.session_id, expected_session_id);
}

#[tokio::test]
async fn mock_broker_persist_delegates_to_persistence() {
    let persistence = Arc::new(RecordingPersistence::default());
    let broker = MockBroker::default().with_persistence(persistence.clone());
    let decision_id = DecisionId::new();
    let scope = DecisionScope::ToolName("shell".to_owned());

    broker.persist(decision_id, scope.clone()).await.unwrap();

    assert_eq!(
        persistence.calls.lock().unwrap().as_slice(),
        &[(decision_id, scope)]
    );
}

#[tokio::test]
async fn mock_broker_can_be_used_as_dyn_permission_broker() {
    let broker: Box<dyn PermissionBroker> = Box::new(MockBroker::new(vec![Decision::AllowOnce]));

    assert_eq!(
        broker
            .decide(permission_request("dyn"), permission_context())
            .await,
        Decision::AllowOnce
    );
}

fn permission_request(command: &str) -> PermissionRequest {
    let tenant_id = TenantId::SHARED;
    let session_id = SessionId::new();
    PermissionRequest {
        request_id: RequestId::new(),
        tenant_id,
        session_id,
        tool_use_id: ToolUseId::new(),
        tool_name: "shell".to_owned(),
        subject: PermissionSubject::CommandExec {
            command: command.to_owned(),
            argv: vec![command.to_owned()],
            cwd: None,
            fingerprint: None,
        },
        severity: Severity::Low,
        scope_hint: DecisionScope::ToolName("shell".to_owned()),
        created_at: Utc::now(),
    }
}

fn permission_context() -> PermissionContext {
    PermissionContext {
        permission_mode: PermissionMode::Default,
        previous_mode: None,
        session_id: SessionId::new(),
        tenant_id: TenantId::SHARED,
        interactivity: InteractivityLevel::FullyInteractive,
        timeout_policy: None,
        fallback_policy: FallbackPolicy::AskUser,
        rule_snapshot: Arc::new(RuleSnapshot {
            rules: Vec::new(),
            generation: 0,
            built_at: Utc::now(),
        }),
        hook_overrides: Vec::new(),
    }
}
