use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    PermissionGate, PermissionOutcome, PluginsSnapshot, SessionId, SprintContract, SubagentError,
    SubagentOutput, SubagentSummary, ToolCallRequest, Verdict,
};
use octopus_sdk_model::{
    ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor,
    ProviderId,
};
use octopus_sdk_observability::{session_span_id, session_trace_id, NoopTracer};
use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};
use octopus_sdk_subagent::{
    Draft, Evaluator, Generator, GeneratorEvaluator, ParentTraceContext, Planner,
};
use octopus_sdk_tools::ToolRegistry;

#[cfg(feature = "test-utils")]
use octopus_sdk_subagent::MockEvaluator;

struct StaticPlanner;

#[async_trait]
impl Planner for StaticPlanner {
    async fn expand(&self, prompt: &str) -> Result<SprintContract, SubagentError> {
        Ok(SprintContract {
            scope: prompt.into(),
            done_definition: "ship a passing draft".into(),
            out_of_scope: vec!["generator thinking".into()],
            invariants: vec!["return summary".into()],
        })
    }
}

struct VersionedGenerator {
    rounds: AtomicUsize,
    seen_feedback: Mutex<Vec<Verdict>>,
}

impl VersionedGenerator {
    fn new() -> Self {
        Self {
            rounds: AtomicUsize::new(0),
            seen_feedback: Mutex::new(Vec::new()),
        }
    }

    fn feedback(&self) -> Vec<Verdict> {
        self.seen_feedback
            .lock()
            .expect("feedback lock should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl Generator for VersionedGenerator {
    async fn run(
        &self,
        contract: &SprintContract,
        feedback: Option<&Verdict>,
    ) -> Result<Draft, SubagentError> {
        if let Some(feedback) = feedback.cloned() {
            self.seen_feedback
                .lock()
                .expect("feedback lock should not be poisoned")
                .push(feedback);
        }

        let round = self.rounds.fetch_add(1, Ordering::Relaxed) + 1;
        Ok(Draft {
            content: SubagentOutput::Summary {
                text: format!("{} v{}", contract.scope, round),
                meta: summary_meta("draft-session"),
            },
            metadata: serde_json::json!({
                "round": round,
                "generator_thinking": format!("internal-{round}")
            }),
        })
    }
}

struct InspectingEvaluator {
    seen_metadata: Mutex<Vec<serde_json::Value>>,
}

impl InspectingEvaluator {
    fn new() -> Self {
        Self {
            seen_metadata: Mutex::new(Vec::new()),
        }
    }

    fn seen_metadata(&self) -> Vec<serde_json::Value> {
        self.seen_metadata
            .lock()
            .expect("metadata lock should not be poisoned")
            .clone()
    }
}

#[async_trait]
impl Evaluator for InspectingEvaluator {
    async fn judge(&self, draft: &Draft) -> Result<Verdict, SubagentError> {
        self.seen_metadata
            .lock()
            .expect("metadata lock should not be poisoned")
            .push(draft.metadata.clone());

        Ok(Verdict::Pass {
            notes: vec!["draft is isolated".into()],
        })
    }
}

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct SilentModelProvider;

#[async_trait]
impl ModelProvider for SilentModelProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(futures::stream::empty()))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

#[cfg(not(feature = "test-utils"))]
struct MockEvaluator {
    rubric: Arc<dyn Fn(&Draft) -> Verdict + Send + Sync>,
}

#[cfg(not(feature = "test-utils"))]
impl MockEvaluator {
    fn new<F>(rubric: F) -> Self
    where
        F: Fn(&Draft) -> Verdict + Send + Sync + 'static,
    {
        Self {
            rubric: Arc::new(rubric),
        }
    }
}

#[cfg(not(feature = "test-utils"))]
#[async_trait]
impl Evaluator for MockEvaluator {
    async fn judge(&self, draft: &Draft) -> Result<Verdict, SubagentError> {
        Ok((self.rubric)(draft))
    }
}

#[tokio::test]
async fn test_gen_eval_pass_on_round_2() {
    let generator = Arc::new(VersionedGenerator::new());
    let runtime = GeneratorEvaluator::new(
        Arc::new(StaticPlanner),
        generator.clone(),
        Arc::new(MockEvaluator::new(|draft| {
            if draft_text(draft).contains("v2") {
                Verdict::Pass {
                    notes: vec!["looks good".into()],
                }
            } else {
                Verdict::Fail {
                    reasons: vec!["need another pass".into()],
                    next_actions: vec!["revise".into()],
                }
            }
        })),
        3,
    );

    let draft = runtime
        .run("landing page")
        .await
        .expect("second round should pass");

    assert_eq!(draft_text(&draft), "landing page v2");
    assert_eq!(
        generator.feedback(),
        vec![Verdict::Fail {
            reasons: vec!["need another pass".into()],
            next_actions: vec!["revise".into()],
        }]
    );
}

#[tokio::test]
async fn test_evaluator_sees_only_draft() {
    let evaluator = Arc::new(InspectingEvaluator::new());
    let runtime = GeneratorEvaluator::new(
        Arc::new(StaticPlanner),
        Arc::new(VersionedGenerator::new()),
        evaluator.clone(),
        1,
    );

    let draft = runtime
        .run("review draft")
        .await
        .expect("first round should pass");
    let seen = evaluator.seen_metadata();

    assert_eq!(seen.len(), 1);
    assert!(seen[0].get("generator_thinking").is_none());
    assert!(draft.metadata.get("generator_thinking").is_none());
}

#[tokio::test]
async fn test_evaluator_runs_in_independent_child_session() {
    let root = tempfile::tempdir().expect("tempdir should create");
    let db_path = root.path().join("data").join("main.db");
    let jsonl_root = root.path().join("runtime").join("events");
    let store =
        Arc::new(SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open"));
    let parent_session = SessionId("gen-eval-parent".into());

    store
        .append_session_started(
            &parent_session,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-parent".into(),
            "hash-parent".into(),
            8_192,
            Some(PluginsSnapshot::default()),
        )
        .await
        .expect("parent session should start");

    let runtime = GeneratorEvaluator::new(
        Arc::new(StaticPlanner),
        Arc::new(VersionedGenerator::new()),
        Arc::new(MockEvaluator::new(|_| Verdict::Pass {
            notes: vec!["ok".into()],
        })),
        1,
    )
    .with_evaluator_parent(octopus_sdk_subagent::ParentSessionContext {
        session_id: parent_session.clone(),
        session_store: store,
        model: Arc::new(SilentModelProvider),
        tools: Arc::new(ToolRegistry::new()),
        permissions: Arc::new(AllowAllGate),
        scratchpad: DurableScratchpad::new(root.path().to_path_buf()),
        trace: ParentTraceContext {
            trace_id: session_trace_id(&parent_session.0),
            span_id: session_span_id(&parent_session.0),
            agent_role: "main".into(),
            model_id: "main".into(),
            model_version: "test".into(),
            config_snapshot_id: "cfg-parent".into(),
            tracer: Arc::new(NoopTracer),
        },
    });

    runtime
        .run("review draft")
        .await
        .expect("evaluator session should succeed");

    let mut files = std::fs::read_dir(&jsonl_root)
        .expect("jsonl dir should exist")
        .map(|entry| entry.expect("entry should read").file_name())
        .collect::<Vec<_>>();
    files.sort();

    assert_eq!(files.len(), 2);
    let child = files
        .into_iter()
        .find(|name| name != &std::ffi::OsString::from("gen-eval-parent.jsonl"))
        .expect("child session jsonl should exist");
    let child_json =
        std::fs::read_to_string(jsonl_root.join(child)).expect("child session jsonl should read");

    assert!(child_json.contains("\"kind\":\"session_started\""));
    assert!(child_json.contains("review draft v1"));
    assert!(child_json.contains("pass: ok"));
}

fn draft_text(draft: &Draft) -> &str {
    match &draft.content {
        SubagentOutput::Summary { text, .. } => text,
        other => panic!("expected summary draft, got {other:?}"),
    }
}

fn summary_meta(session_id: &str) -> SubagentSummary {
    SubagentSummary {
        session_id: SessionId(session_id.into()),
        parent_session_id: SessionId("gen-eval-parent".into()),
        resume_session_id: Some(SessionId(session_id.into())),
        spec_id: "generator".into(),
        agent_role: "worker".into(),
        parent_agent_role: "main".into(),
        turns: 1,
        tokens_used: 10,
        duration_ms: 5,
        trace_id: session_trace_id("gen-eval-parent"),
        span_id: format!("subagent:{session_id}"),
        parent_span_id: session_span_id("gen-eval-parent"),
        model_id: "main".into(),
        model_version: "test".into(),
        config_snapshot_id: "cfg-parent".into(),
        permission_mode: octopus_sdk_contracts::PermissionMode::Default,
        allowed_tools: Vec::new(),
    }
}
