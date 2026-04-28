use std::sync::Arc;

use async_trait::async_trait;
use futures::stream;
use harness_context::ContextEngine;
use harness_contracts::{
    CapabilityRegistry, Decision, DecisionId, DecisionScope, ModelError, NoopRedactor,
    PermissionError,
};
use harness_engine::{Engine, EngineBuilder, EngineId, EngineRunner, LoopState};
use harness_hook::{HookDispatcher, HookRegistry};
use harness_journal::InMemoryEventStore;
use harness_model::{
    HealthStatus, InferContext, ModelCapabilities, ModelDescriptor, ModelProvider, ModelRequest,
    ModelStream,
};
use harness_permission::{PermissionBroker, PermissionContext, PermissionRequest};
use harness_tool::ToolPool;
use serde_json::json;

#[test]
fn engine_builder_exposes_stable_engine_id() {
    let engine = Engine::builder()
        .with_engine_id(EngineId::new("contract-engine"))
        .with_required_test_dependencies()
        .build()
        .unwrap();

    assert_eq!(engine.engine_id(), EngineId::new("contract-engine"));
}

#[tokio::test]
async fn engine_runner_is_object_safe_and_uses_engine_id() {
    let runner: Arc<dyn EngineRunner> = Arc::new(
        EngineBuilder::default()
            .with_engine_id(EngineId::new("runner-engine"))
            .with_required_test_dependencies()
            .build()
            .unwrap(),
    );

    assert_eq!(runner.engine_id(), EngineId::new("runner-engine"));
}

#[test]
fn loop_state_exposes_m5_five_state_contract() {
    let tool_call = harness_tool::ToolCall {
        tool_use_id: harness_contracts::ToolUseId::new(),
        tool_name: "contract-tool".to_owned(),
        input: json!({}),
    };

    let states = [
        LoopState::AwaitingModel,
        LoopState::ProcessingToolUses {
            pending: vec![tool_call],
        },
        LoopState::ApplyingHookResults,
        LoopState::MergingContext,
        LoopState::Ended(harness_contracts::StopReason::EndTurn),
    ];

    assert_eq!(states.len(), 5);
}

trait EngineBuilderTestExt {
    fn with_required_test_dependencies(self) -> Self;
}

impl EngineBuilderTestExt for EngineBuilder {
    fn with_required_test_dependencies(self) -> Self {
        let root = tempfile::tempdir().unwrap();
        self.with_event_store(Arc::new(InMemoryEventStore::new(Arc::new(NoopRedactor))))
            .with_context(ContextEngine::builder().build().unwrap())
            .with_hooks(HookDispatcher::new(
                HookRegistry::builder().build().unwrap().snapshot(),
            ))
            .with_model(Arc::new(DummyModel))
            .with_tools(ToolPool::default())
            .with_permission_broker(Arc::new(DummyBroker))
            .with_workspace_root(root.path())
            .with_model_id("dummy-model")
            .with_cap_registry(Arc::new(CapabilityRegistry::default()))
    }
}

struct DummyModel;

#[async_trait]
impl ModelProvider for DummyModel {
    fn provider_id(&self) -> &'static str {
        "dummy"
    }

    fn supported_models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor {
            provider_id: "dummy".to_owned(),
            model_id: "dummy-model".to_owned(),
            display_name: "Dummy model".to_owned(),
            context_window: 1_000,
            max_output_tokens: 100,
            capabilities: ModelCapabilities::default(),
            pricing: None,
        }]
    }

    async fn infer(
        &self,
        _req: ModelRequest,
        _ctx: InferContext,
    ) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(stream::empty()))
    }

    async fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

struct DummyBroker;

#[async_trait]
impl PermissionBroker for DummyBroker {
    async fn decide(&self, _request: PermissionRequest, _ctx: PermissionContext) -> Decision {
        Decision::DenyOnce
    }

    async fn persist(
        &self,
        _decision_id: DecisionId,
        _scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        Ok(())
    }
}
