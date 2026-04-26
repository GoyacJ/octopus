#![cfg(feature = "code-runtime")]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use harness_contracts::{Event, SandboxError, ToolName, ToolUseId};
use harness_sandbox::{
    CodeSandbox, CodeSandboxCapabilities, CodeSandboxResult, CodeSandboxRunContext, CompiledScript,
    EmbeddedToolDispatcherCap, EmbeddedToolRequest, EmbeddedToolResponse, EventSink,
    SandboxRunStats, ScriptLanguage, UsageMeter,
};

struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: Event) -> Result<(), SandboxError> {
        Ok(())
    }
}

struct NullDispatcher;

#[async_trait]
impl EmbeddedToolDispatcherCap for NullDispatcher {
    async fn dispatch_embedded_tool(
        &self,
        _request: EmbeddedToolRequest,
    ) -> Result<EmbeddedToolResponse, SandboxError> {
        Ok(EmbeddedToolResponse {
            result_json: "{}".to_owned(),
        })
    }
}

struct NullMeter;

impl UsageMeter for NullMeter {
    fn record_instructions(&self, _count: u64) {}

    fn record_wall_clock(&self, _elapsed: Duration) {}
}

struct TestCodeSandbox;

#[async_trait]
impl CodeSandbox for TestCodeSandbox {
    fn capabilities(&self) -> CodeSandboxCapabilities {
        CodeSandboxCapabilities::default()
    }

    async fn run(
        &self,
        _script: &CompiledScript,
        _ctx: CodeSandboxRunContext,
    ) -> Result<CodeSandboxResult, SandboxError> {
        Ok(CodeSandboxResult {
            stdout: "ok".to_owned(),
            return_value: None,
            steps_summary: Vec::new(),
            stats: SandboxRunStats::default(),
        })
    }
}

#[tokio::test]
async fn code_sandbox_is_object_safe() {
    let sandbox: Arc<dyn CodeSandbox> = Arc::new(TestCodeSandbox);
    let script = CompiledScript {
        language: ScriptLanguage::MiniLua,
        source_hash: [0; 32],
        bytecode: Vec::new(),
    };
    let ctx = CodeSandboxRunContext {
        session_id: harness_contracts::SessionId::new(),
        run_id: harness_contracts::RunId::new(),
        parent_tool_use_id: ToolUseId::new(),
        embedded_dispatcher: Arc::new(NullDispatcher),
        usage_meter: Arc::new(NullMeter),
        event_sink: Arc::new(NullSink),
    };

    let result = sandbox.run(&script, ctx).await.unwrap();

    assert_eq!(sandbox.capabilities().language, ScriptLanguage::MiniLua);
    assert_eq!(result.stdout, "ok");

    let _name: ToolName = "example".to_owned();
}
