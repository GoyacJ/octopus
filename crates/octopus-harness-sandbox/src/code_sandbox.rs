//! Code runtime sandbox contracts.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use harness_contracts::{RunId, SandboxError, SessionId, ToolName, ToolUseId};

use crate::EventSink;

#[async_trait]
pub trait CodeSandbox: Send + Sync + 'static {
    fn capabilities(&self) -> CodeSandboxCapabilities;

    async fn run(
        &self,
        script: &CompiledScript,
        ctx: CodeSandboxRunContext,
    ) -> Result<CodeSandboxResult, SandboxError>;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CodeSandboxCapabilities {
    pub language: ScriptLanguage,
    pub max_instructions: u64,
    pub max_call_depth: u32,
    pub max_string_bytes: u64,
    pub max_table_entries: u64,
    pub wall_clock_budget: Duration,
    pub deterministic: bool,
}

impl Default for CodeSandboxCapabilities {
    fn default() -> Self {
        Self {
            language: ScriptLanguage::MiniLua,
            max_instructions: 1_000_000,
            max_call_depth: 32,
            max_string_bytes: 4 * 1024 * 1024,
            max_table_entries: 65_536,
            wall_clock_budget: Duration::from_secs(30),
            deterministic: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ScriptLanguage {
    MiniLua,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CompiledScript {
    pub language: ScriptLanguage,
    pub source_hash: [u8; 32],
    pub bytecode: Vec<u8>,
}

#[derive(Clone)]
pub struct CodeSandboxRunContext {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub parent_tool_use_id: ToolUseId,
    pub embedded_dispatcher: Arc<dyn EmbeddedToolDispatcherCap>,
    pub usage_meter: Arc<dyn UsageMeter>,
    pub event_sink: Arc<dyn EventSink>,
}

#[async_trait]
pub trait EmbeddedToolDispatcherCap: Send + Sync + 'static {
    async fn dispatch_embedded_tool(
        &self,
        request: EmbeddedToolRequest,
    ) -> Result<EmbeddedToolResponse, SandboxError>;
}

pub trait UsageMeter: Send + Sync + 'static {
    fn record_instructions(&self, count: u64);

    fn record_wall_clock(&self, elapsed: Duration);
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EmbeddedToolRequest {
    pub name: ToolName,
    pub args_json: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EmbeddedToolResponse {
    pub result_json: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeSandboxResult {
    pub stdout: String,
    pub return_value: Option<LuaValue>,
    pub steps_summary: Vec<EmbeddedStepSummary>,
    pub stats: SandboxRunStats,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LuaValue {
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
    Json(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EmbeddedStepSummary {
    pub name: ToolName,
    pub accepted: bool,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct SandboxRunStats {
    pub instructions: u64,
    pub wall_clock: Duration,
    pub peak_memory_bytes: u64,
}
