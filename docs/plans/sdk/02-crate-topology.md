# 02 · SDK Crate 拓扑与公共面

> 本文档定义 **SDK 15 个 crate + 业务 5 个 crate 的依赖方向、命名、对外 trait/struct 公共面、契约差异清单、UI Intent IR 登记表**。
> 所有新增 `pub` 符号在合入前必须在此文档登记。
> 本文档随 W1–W8 推进持续迭代；每次迭代必须在 `§变更日志` 追加条目。

## 0. 阅读顺序与关系

- 先读 `00-overview.md §2 目标 crate 矩阵` 建立整体印象。
- 再读本文 `§1 依赖图` 锁定方向不变量。
- `§2 SDK Crate 逐个公共面` 是每周落码时的签名契约。
- `§3 业务侧 crate` 是 SDK 重构对业务层产生的对应动作。
- `§4 命名与路径规约` 是 AI 在创建新符号时的硬约束。
- `§5 契约差异清单` / `§6 UI Intent IR 登记表` 为运行时发现问题提供 append 通道。

## 1. 依赖图（方向不变量）

**唯一合法依赖方向**：下层 → 上层；反向依赖视为违规。业务层只依赖 `octopus-sdk` 门面。

```
           ┌──────────────────────────────────────────────┐
Level 5:   │                octopus-sdk                    │  ← 业务唯一入口
           │   (re-export + AgentRuntimeBuilder)           │
           └────────────────────┬─────────────────────────┘
                                │
           ┌────────────────────▼─────────────────────────┐
Level 4:   │              octopus-sdk-core                  │  ← Brain Loop
           │      (AgentRuntime + run_turn orchestrator)    │
           └──────┬──────────┬──────────────┬──────────────┘
                  │          │              │
           ┌──────▼──┐ ┌─────▼──┐ ┌─────────▼──────┐
Level 3:   │subagent │ │context │ │ observability  │   ← 编排 / 工程侧
           └──┬──────┘ └───┬────┘ └────────────────┘
              │            │
 ┌────────────┼────────────┼────────────────────────────────┐
 │  Level 2 (Hands 侧能力，可互相持有 handle 但不成环)         │
 │  ┌──────┐ ┌────┐ ┌──────┐ ┌───────┐ ┌───────────┐ ┌─────┐│
 │  │tools │ │mcp │ │hooks │ │sandbox│ │permissions│ │ui-  ││
 │  └─┬────┘ └─┬──┘ └─┬────┘ └───────┘ └───────────┘ │intent││
 │    │        │      │                              └──┬──┘│
 └────┼────────┼──────┼─────────────────────────────────┼───┘
      │        │      │                                 │
      │        │      │          ┌──────────────────────┘
      │        │      │          │ (events → session)
 ┌────▼────────▼──────▼──────────▼──────┐
Level 1:  │ sdk-model  │    │ sdk-session │
          │(Provider/  │    │(SessionStore│
          │Surface/    │    │ trait +     │
          │Model+Adapt)│    │ SqliteJsonl)│
          └─────────┬──┘    └─┬───────────┘
                    └────┬────┘
                         │
                ┌────────▼────────┐
Level 0:        │octopus-sdk-     │   ← 纯数据/trait，无运行时状态
                │contracts        │
                │(Message/Tool/   │
                │ Event/IR/Usage/ │
                │ SecretVault)    │
                └─────────────────┘
```

### 1.1 层内规则

- **Level 0**：只含 `serde` / `thiserror` / `uuid` / `async-trait` 级别的依赖；禁止任何 I/O、async runtime、HTTP、SQLite。`SecretVault` trait 定义于此层以便 Level 1–2 直接引用。
- **Level 1**：允许 async、HTTP（`reqwest`）、SQLite；但禁止引用 Level 2+ 任意 crate。
- **Level 2**：Hands 侧能力层，允许 OS 原语（bubblewrap/seatbelt）、MCP client；`permissions`、`sandbox`、`ui-intent`、`hooks`、`plugin`、`mcp`、`tools` 7 个 crate 同层。同层 crate **允许**按以下协作点相互持有 handle：
  - `tools::ToolContext` 持有 `permissions::PermissionGateHandle` 与 `sandbox::SandboxHandle`；
  - `ui-intent::RenderEmitter` 写入 `session::SessionStore`（Level 1）；
  - `plugin` 登记并反查 `tools / mcp / hooks` 的 registry handle。
  禁止出现"tools → hooks → tools"之类的传递环；PR review 必须核对。
- **Level 3**：编排层（`subagent / context / observability`）；可依赖 Level 0–2 全部。**不允许**在 Level 2 内相互依赖引入 Level 3 符号。
- **Level 4**：只允许一个 crate（`sdk-core`）；聚合 Level 0–3 成 Brain Loop。
- **Level 5**：门面 crate；只做 re-export 与 builder 模式整合，无内部逻辑。

### 1.2 禁止项

- 任何 SDK crate 引用 `octopus-core` / `octopus-platform` / `octopus-infra` / `octopus-server`。
- 任何 SDK crate 直接 `use rusqlite::Connection`（必须经 `octopus-sdk-session::SessionStore` 的默认实现或业务侧 `octopus-persistence` 提供）。
- 任何 SDK crate 引用 `tauri` / axum 路由 / Vue。
- 任何 SDK crate 引用业务域对象（`Project` / `Task` / `Workspace` / `Deliverable` / `User` / `Org` / `Team`）。

---

## 2. SDK Crate 逐个公共面

> 以下签名为 **目标态**，不是现状拷贝。W1–W6 各周实现时，新增 `pub` 必须在此节追加或同步修正。
> 签名采用 Rust 类型示意；`// TODO(Wn)` 标注尚未落地的签名。

### 2.1 `octopus-sdk-contracts`（Level 0）

**职责**：SDK 内部与业务共用的数据 IR。零 I/O。

```rust
pub struct SessionId(pub String);
pub struct RunId(pub String);
pub struct ToolCallId(pub String);
pub struct EventId(pub String);

pub enum Role { System, User, Assistant, Tool }

pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: ToolCallId, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: ToolCallId, content: Vec<ContentBlock>, is_error: bool },
    Thinking { text: String },
}

pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

pub enum AssistantEvent {
    TextDelta(String),
    ToolUse { id: ToolCallId, name: String, input: serde_json::Value },
    Usage(Usage),
    PromptCache(PromptCacheEvent),
    MessageStop { stop_reason: StopReason },
}

pub enum StopReason { EndTurn, ToolUse, MaxTokens, StopSequence }

// UI Intent IR（完整 kind 登记见 §6）
pub struct RenderBlock {
    pub kind: RenderKind,
    pub payload: serde_json::Value,
    pub meta: RenderMeta,
}
pub enum RenderKind {
    Text, Markdown, Code, Diff, ListSummary, Progress,
    ArtifactRef, Record, Error, Raw,
}
pub struct AskPrompt { /* ... */ }
pub struct ArtifactRef { /* ... */ }
pub enum RenderLifecycle { Start, Update, End }

// Session 事件
pub enum SessionEvent {
    SessionStarted { config_snapshot_id: String, effective_config_hash: String },
    UserMessage(Message),
    AssistantMessage(Message),
    ToolExecuted { call: ToolCallId, name: String, duration_ms: u64, is_error: bool },
    Render { block: RenderBlock, lifecycle: RenderLifecycle },
    Ask { prompt: AskPrompt },
    Checkpoint { id: String, anchor_event_id: EventId },
    SessionEnded { reason: EndReason },
}
pub enum EndReason { Completed, Interrupted, Error }

// 凭据抽象：定义在 contracts，以便 sdk-model / sdk-tools / 业务层共用
#[async_trait]
pub trait SecretVault: Send + Sync {
    async fn get(&self, ref_id: &str) -> Result<SecretValue, VaultError>;
    async fn put(&self, ref_id: &str, value: SecretValue) -> Result<(), VaultError>;
}

pub struct SecretValue(/* 内部零化；Drop 擦除；禁止 Debug 明文 */);
pub enum VaultError { NotFound, Backend(String), Redacted }
```

> `SecretVault` 定义于 Level 0 是刻意的：Level 1 `sdk-model`（HTTP 认证）、Level 2 `sdk-tools`（工具内请求）均需使用同一 trait，而 Level 5 门面 crate 不得承担 trait 定义职责（层内规则禁止其引入逻辑）。

### 2.2 `octopus-sdk-session`（Level 1）

**职责**：Append-only 事件日志抽象 + `SqliteJsonlSessionStore` 默认实现。

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn append(&self, id: &SessionId, event: SessionEvent) -> Result<EventId, SessionError>;
    async fn stream(&self, id: &SessionId, range: EventRange) -> Result<EventStream, SessionError>;
    async fn snapshot(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError>;
    async fn fork(&self, id: &SessionId, from: EventId) -> Result<SessionId, SessionError>;
    async fn wake(&self, id: &SessionId) -> Result<SessionSnapshot, SessionError>;
}

pub struct EventRange { pub after: Option<EventId>, pub limit: Option<usize> }
pub type EventStream = Pin<Box<dyn Stream<Item = Result<SessionEvent, SessionError>> + Send>>;

pub struct SessionSnapshot {
    pub id: SessionId,
    pub config_snapshot_id: String,
    pub effective_config_hash: String,
    pub head_event_id: EventId,
    pub usage: Usage,
}

pub enum SessionError { /* ... */ }

pub struct SqliteJsonlSessionStore { /* impl SessionStore */ }
impl SqliteJsonlSessionStore {
    pub fn open(db: &Path, jsonl_root: &Path) -> Result<Self, SessionError>;
}
```

### 2.3 `octopus-sdk-model`（Level 1）

**职责**：Provider / Surface / Model 三层 + 5 种 `ProtocolAdapter`。

```rust
pub struct ProviderId(pub String);
pub struct SurfaceId(pub String);
pub struct ModelId(pub String);

pub struct Provider { pub id: ProviderId, pub auth: AuthKind, pub surfaces: Vec<SurfaceId> }
pub struct Surface { pub id: SurfaceId, pub protocol: ProtocolFamily, pub base_url: String }
pub struct Model { pub id: ModelId, pub surface: SurfaceId, pub family: String, pub track: ModelTrack }

pub enum ProtocolFamily {
    AnthropicMessages, OpenAiChat, OpenAiResponses, GeminiNative, VendorNative,
}
pub enum ModelTrack { Preview, Stable, LatestAlias, Deprecated, Sunset }
pub enum AuthKind { ApiKey, XApiKey, OAuth, AwsSigV4, GcpAdc, AzureAd, None }

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError>;
    fn describe(&self) -> ProviderDescriptor;
}

pub struct ModelRequest {
    pub model: ModelId,
    pub system_prompt: Vec<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub role: ModelRole,
    pub cache_breakpoints: Vec<CacheBreakpoint>,
}
pub type ModelStream = Pin<Box<dyn Stream<Item = Result<AssistantEvent, ModelError>> + Send>>;

pub enum ModelRole { Main, Fast, Best, Plan, Compact, Vision, WebExtract, Embedding, Eval, SubagentDefault }

pub struct RoleRouter { /* 静态映射 role → ModelId */ }
pub struct FallbackPolicy { /* overloaded / 5xx / prompt_too_long */ }

pub trait ProtocolAdapter: Send + Sync {
    fn family(&self) -> ProtocolFamily;
    fn to_request(&self, req: &ModelRequest) -> Result<serde_json::Value, ModelError>;
    fn parse_stream(&self, raw: StreamBytes) -> Result<ModelStream, ModelError>;
}
```

### 2.4 `octopus-sdk-tools`（Level 2）

**职责**：`Tool` trait + 15 个内置工具 + 并发分区。

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn is_concurrency_safe(&self, input: &serde_json::Value) -> bool;
    async fn execute(&self, ctx: ToolContext, input: serde_json::Value) -> ToolResult;
}

pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,  // JSON Schema
    pub category: ToolCategory,
}
pub enum ToolCategory { Read, Write, Network, Shell, Subagent, Skill, Meta }

pub struct ToolRegistry { /* deterministic order guaranteed */ }
impl ToolRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, tool: Arc<dyn Tool>);
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn schemas_sorted(&self) -> Vec<&ToolSpec>;  // deterministic for prompt cache
}

pub struct ToolContext {
    pub session: SessionId,
    pub permissions: PermissionGateHandle,
    pub sandbox: SandboxHandle,
    pub session_store: Arc<dyn SessionStore>,
    pub secret_vault: Arc<dyn SecretVault>,
}

pub struct ToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub render: Option<RenderBlock>,  // UI Intent IR 由工具发出
}

pub fn partition_tool_calls<'a>(
    calls: &'a [ToolCallRequest],
    registry: &ToolRegistry,
) -> Vec<ExecBatch<'a>>;
// 只读工具并发（默认 max = 10），写工具串行

// 15 内置工具（全部走 MCP in-process shim，取舍 #4）
pub mod builtin {
    pub struct FileReadTool; pub struct FileWriteTool; pub struct FileEditTool;
    pub struct GlobTool; pub struct GrepTool; pub struct BashTool;
    pub struct WebSearchTool; pub struct WebFetchTool;
    pub struct AskUserQuestionTool; pub struct TodoWriteTool;
    pub struct AgentTool; pub struct SkillTool; pub struct SleepTool;
    pub struct TaskListTool; pub struct TaskGetTool;
}

pub const BASH_MAX_OUTPUT_DEFAULT: usize = 30_000;
pub const DEFAULT_TOOL_MAX_CONCURRENCY: usize = 10;
```

### 2.5 `octopus-sdk-mcp`（Level 2）

**职责**：MCP 协议客户端 + 三种传输。

```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;
}

pub struct StdioTransport { /* ... */ }
pub struct HttpTransport { /* ... */ }
pub struct SdkTransport { /* in-process */ }

pub struct McpClient {
    transport: Arc<dyn McpTransport>,
    server_id: String,
}
impl McpClient {
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError>;
    pub async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>;
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError>;
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError>;
}

pub struct McpServerManager { /* lifecycle, crash-as-tool-error */ }
```

### 2.6 `octopus-sdk-context`（Level 3）

**职责**：Prompt builder + compaction + tool-result clearing + memory backend trait。

```rust
pub struct SystemPromptBuilder { /* identity + tool guidance + output format + examples */ }
impl SystemPromptBuilder {
    pub fn build(&self, ctx: &PromptCtx) -> Vec<String>;  // stable sections; order locked
}

pub struct Compactor { /* compaction + tool-result clearing */ }
impl Compactor {
    pub async fn maybe_compact(&self, session: &mut SessionView) -> Option<CompactionResult>;
}

pub enum CompactionStrategy { Summarize, ClearToolResults, Hybrid }

#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn recall(&self, session: &SessionId, query: &str) -> Result<Vec<MemoryItem>, MemoryError>;
    async fn commit(&self, session: &SessionId, item: MemoryItem) -> Result<(), MemoryError>;
}

pub struct DurableScratchpad { /* runtime/notes/<session>.md */ }
```

### 2.7 `octopus-sdk-permissions`（Level 2，Hands 侧 gate）

**职责**：权限模式、审批闸门、提示生成（对接 `AskPrompt`）。

```rust
pub enum PermissionMode { Default, AcceptEdits, BypassPermissions, Plan }

pub struct PermissionPolicy { /* allowlist / denylist / by-argument rules */ }

pub struct PermissionGate { /* 持有 policy + mode */ }
impl PermissionGate {
    pub async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome;
}

pub enum PermissionOutcome {
    Allow,
    Deny { reason: String },
    AskApproval { prompt: AskPrompt },
    RequireAuth { prompt: AskPrompt },
}

pub struct ApprovalBroker { /* 把 AskPrompt 推向业务层；等待业务回调 */ }
```

### 2.8 `octopus-sdk-sandbox`（Level 2）

**职责**：OS 级沙箱抽象 + 默认两种后端。

```rust
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    async fn provision(&self, spec: SandboxSpec) -> Result<SandboxHandle, SandboxError>;
    async fn execute(&self, handle: &SandboxHandle, cmd: SandboxCommand) -> Result<SandboxOutput, SandboxError>;
    async fn terminate(&self, handle: SandboxHandle) -> Result<(), SandboxError>;
}

pub struct BubblewrapBackend; // Linux
pub struct SeatbeltBackend;   // macOS
pub struct NoopBackend;       // test-only

pub struct SandboxSpec {
    pub fs_whitelist: Vec<PathBuf>,
    pub network_proxy: Option<NetworkProxy>,
    pub env_allowlist: Vec<String>,
}
```

### 2.9 `octopus-sdk-hooks`（Level 2）

**职责**：生命周期钩子。

```rust
pub enum HookEvent {
    PreToolUse { call: ToolCallRequest },
    PostToolUse { call: ToolCallRequest, result: ToolResult },
    Stop { session: SessionId },
    SessionStart { session: SessionId },
    SessionEnd { session: SessionId, reason: EndReason },
    UserPromptSubmit { message: Message },
    PreCompact { session: SessionId },
    PostCompact { session: SessionId, result: CompactionResult },
}

#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    async fn on_event(&self, event: &HookEvent) -> HookDecision;
}

pub enum HookDecision { Continue, Abort { reason: String }, InjectMessage(Message) }

pub struct HookRunner { /* 并发安全；按 name deterministic order */ }
```

### 2.10 `octopus-sdk-subagent`（Level 3）

**职责**：子代理编排模式。

```rust
pub struct SubagentSpec {
    pub id: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub model_role: ModelRole,
    pub permission_mode: PermissionMode,
}

pub struct OrchestratorWorkers { /* fan-out → fan-in with condensed summaries */ }
impl OrchestratorWorkers {
    pub async fn run(&self, spec: SubagentSpec, input: String) -> Result<SubagentOutput, SubagentError>;
}

pub struct GeneratorEvaluator { /* sprint-contract + loop until pass */ }
```

### 2.11 `octopus-sdk-plugin`（Level 2）

**职责**：Plugin Manifest / Registry / Lifecycle 三层。

```rust
pub struct PluginManifest {
    pub id: String,
    pub version: String,
    pub git_sha: Option<String>,
    pub compat: PluginCompat,
    pub components: Vec<PluginComponent>,
}

pub enum PluginComponent {
    Tool(ToolDecl),
    Skill(SkillDecl),
    Command(CommandDecl),
    Agent(AgentDecl),
    OutputStyle(OutputStyleDecl),
    Hook(HookDecl),
    McpServer(McpServerDecl),
    LspServer(LspServerDecl),
    ModelProvider(ModelProviderDecl),
    Channel(ChannelDecl),
    ContextEngine(ContextEngineDecl),
    MemoryBackend(MemoryBackendDecl),
}

pub struct PluginRegistry { /* 单向：plugin → registry ← core */ }
pub struct PluginLifecycle { /* discover → enable → load → register */ }

pub enum PluginError { /* 22 型分类错误 */ }
```

### 2.12 `octopus-sdk-ui-intent`（Level 2）

**职责**：UI 意图 IR 发射器 + schema validator；**不含渲染逻辑**。
**依赖声明**：依赖 `octopus-sdk-session::SessionStore`（事件回写）与 `octopus-sdk-contracts`（IR 数据类型）。不得依赖 Level 2 内其他 crate。

```rust
pub struct RenderEmitter { /* 依赖 session::SessionStore，写入 Session 事件流 */ }
impl RenderEmitter {
    pub async fn emit(&self, block: RenderBlock, lifecycle: RenderLifecycle) -> Result<(), RenderError>;
}

pub fn validate_render_block(block: &RenderBlock) -> Result<(), RenderError>;
// 10 种 kind 的 payload schema（见 §6）

pub fn new_text(text: impl Into<String>) -> RenderBlock;
pub fn new_markdown(md: impl Into<String>) -> RenderBlock;
pub fn new_code(lang: impl Into<String>, src: impl Into<String>) -> RenderBlock;
pub fn new_diff(/* ... */) -> RenderBlock;
// 其余 6 种同理
```

### 2.13 `octopus-sdk-observability`（Level 3）

**职责**：Tracing / usage ledger / replay。

```rust
pub struct TraceSpan { /* id / parent_id / name / start_ns / end_ns / attrs */ }

pub trait Tracer: Send + Sync {
    fn start(&self, name: &str) -> TraceSpan;
    fn record(&self, span: &TraceSpan, attr: (&str, TraceValue));
    fn end(&self, span: TraceSpan);
}

pub struct UsageLedger { /* token / cost 累计 */ }
pub struct ReplayTracer { /* 从 SessionStore 回放事件到 tracer */ }
```

### 2.14 `octopus-sdk-core`（Level 4）

**职责**：Brain Loop；整合 Level 0–3 全部 crate。

```rust
pub struct AgentRuntime { /* private fields */ }
impl AgentRuntime {
    pub async fn start_session(&self, input: StartSessionInput) -> Result<SessionHandle, RuntimeError>;
    pub async fn submit_turn(&self, session: &SessionId, msg: Message) -> Result<RunHandle, RuntimeError>;
    pub async fn resume(&self, session: &SessionId) -> Result<SessionHandle, RuntimeError>;
    pub async fn cancel(&self, session: &SessionId) -> Result<(), RuntimeError>;
    pub fn events(&self, session: &SessionId) -> EventStream;
}

pub struct AgentRuntimeBuilder { /* ... */ }
impl AgentRuntimeBuilder {
    pub fn new() -> Self;
    pub fn with_session_store(self, store: Arc<dyn SessionStore>) -> Self;
    pub fn with_model_provider(self, provider: Arc<dyn ModelProvider>) -> Self;
    pub fn with_secret_vault(self, vault: Arc<dyn SecretVault>) -> Self;
    pub fn with_tool_registry(self, registry: ToolRegistry) -> Self;
    pub fn with_permission_policy(self, policy: PermissionPolicy) -> Self;
    pub fn with_sandbox_backend(self, backend: Arc<dyn SandboxBackend>) -> Self;
    pub fn with_plugin_registry(self, registry: PluginRegistry) -> Self;
    pub fn build(self) -> Result<AgentRuntime, RuntimeError>;
}
```

### 2.15 `octopus-sdk`（Level 5，门面）

**职责**：业务唯一入口；受控 re-export。**禁止**在本 crate 内定义新 trait / struct / fn；仅允许 `pub use` 与 `//!` 文档。

```rust
pub use octopus_sdk_contracts::*;    // 含 SecretVault / SecretValue / VaultError
pub use octopus_sdk_core::{
    AgentRuntime, AgentRuntimeBuilder,
    StartSessionInput, SessionHandle, RunHandle, RuntimeError,
};
pub use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};
pub use octopus_sdk_model::{ModelProvider, ProviderId, ModelId, ModelRole, AuthKind};
```

**不允许 re-export**：`Tool` trait、`McpClient`、`HookRunner`、`Compactor`、`PermissionGate`、`SandboxBackend`、`PluginRegistry`。这些是 SDK 内部构造；业务层通过 Builder 注入而非直接持有。

---

## 3. 业务侧 crate

### 3.1 `octopus-platform`

- 保留 `AccessControlService / AuthService / AuthorizationService / AppRegistryService / ArtifactService / InboxService / KnowledgeService / ObservationService / ProjectTaskService / WorkspaceService`。
- **删除**：`runtime.rs`（783 行）——`RuntimeSessionService / RuntimeExecutionService / RuntimeConfigService / ModelRegistryService / RuntimeProjectionService / AutomationService / ToolExecutionService` 全部由 `octopus-sdk` 的 `AgentRuntime` 替代。业务只保留"把业务域对象映射到 `StartSessionInput`"的薄壳，放 `session_bridge.rs`（≤ 300 行）。

### 3.2 `octopus-persistence`（新）

- 职责：SQLite schema 定义 + migration + repository trait；所有 `rusqlite::Connection` 生命周期集中管理。
- 公共面：
  ```rust
  pub struct Database { /* ... */ }
  impl Database {
      pub fn open(path: &Path) -> Result<Self, DbError>;
      pub fn acquire(&self) -> Result<Connection, DbError>;
      pub fn run_migrations(&self) -> Result<(), DbError>;
  }
  pub mod repositories {
      pub struct ProjectRepository;
      pub struct WorkspaceRepository;
      pub struct TaskRepository;
      pub struct AccessControlRepository;
      // ...
  }
  ```
- SDK 的 `SqliteJsonlSessionStore` **不**走本 crate（它在 SDK 侧自持一个独立 Connection pool），避免业务 schema 与 SDK 事件 schema 相互污染。

### 3.3 `octopus-server`

- 只依赖 `octopus-sdk` + `octopus-platform` + `octopus-persistence` + `octopus-core`（领域类型）。
- `handlers.rs`（4300 行）拆：`handlers/access.rs` / `workspace.rs` / `project.rs` / `task.rs` / `runtime.rs` / `catalog.rs` / `host.rs` / `knowledge.rs` / `inbox.rs` / `misc.rs`。
- `workspace_runtime.rs`（9890 行）拆：按 `/api/v1/runtime/*` 的 8 类资源切 8 文件，每个 ≤ 800 行。

### 3.4 `octopus-desktop`（替换 `octopus-desktop-backend`）

- Tauri host bridge；不持有 `AgentRuntime`，通过 IPC 调用 `octopus-server`（保持现有 host 模型）。
- 仅依赖 `octopus-core`。

### 3.5 `octopus-cli`（合并 `rusty-claude-cli` + `commands`）

- 唯一 CLI 入口；`claw` binary。
- 依赖 `octopus-sdk` + `octopus-core`；**删除** `api / runtime / tools / plugins / commands / compat-harness / octopus-model-policy` 依赖。
- `octopus-model-policy` 的内容迁入 `octopus-sdk-model` 作为内置 role router 策略。

---

## 4. 命名与路径规约

### 4.1 Crate 命名

- SDK 所有 crate 前缀 `octopus-sdk-`；顶层门面名 `octopus-sdk`。
- 业务 crate 保留 `octopus-` 前缀（不含 `sdk`）。
- 禁止使用泛化名 `runtime / tools / api / plugins` 作为 crate 名（W7 全部删除）。

### 4.2 模块与文件

- 单 `.rs` 文件 ≤ 800 行硬约束。
- 禁止 `split_module_tests.rs` 风格的超长测试文件；每个 mod 的测试要么同文件 `#[cfg(test)] mod tests`（≤ 300 行测试），要么放到 `crates/<crate>/tests/<feature>.rs`。
- `lib.rs` 只做 `mod` 声明 + 受控 re-export；单 crate 的 `lib.rs` ≤ 80 行。

### 4.3 符号命名

- 对外 `pub` 符号：
  - trait：`XxxService` / `XxxBackend` / `XxxStore` / `XxxProvider` / `XxxRegistry` 之一后缀。
  - 数据类型：领域名词直译，不加 `Runtime` / `Service` 前缀。
  - 错误：`XxxError` + `thiserror`。
- 内部 `pub(crate)` / `pub(super)` 优先；只有在本文档 §2 登记才允许 `pub`。

### 4.4 依赖声明

- SDK crate 的 `[dependencies]` 禁止出现 `octopus-core / octopus-platform / octopus-infra / octopus-server / octopus-persistence`。
- SDK crate 间依赖遵循 §1 依赖图；PR review 必须核对 `Cargo.toml` 的 `[dependencies]` 节。

---

## 5. 契约差异清单（Contract Discrepancy Registry）

> **用途**：当 SDK 侧 IR 与 `contracts/openapi/src/**` / `packages/schema/src/**` 出现字段/命名/枚举差异时，在此节 append；达到阈值必须触发 OpenAPI 变更。
> **初始状态**：空；W1 启动时首次填充。

| # | 日期 | 来源 | 目标 | 字段/枚举 | 差异描述 | 处理方式 | 状态 |
|---|---|---|---|---|---|---|---|
| — | — | — | — | — | — | — | — |

**处理方式取值**：`align-openapi`（优先调整 OpenAPI）/ `align-sdk`（调整 SDK）/ `dual-carry`（短期双写 + deadline）/ `no-op`（仅命名差异，文档标注即可）。

---

## 6. UI Intent IR 登记表

> **用途**：所有 `RenderBlock.kind` 必须登记；插件不得自行扩 kind（`docs/sdk/14` §14.2）。
> **10 种初始 kind** 见下；新增必须同步修改 `octopus-sdk-contracts::RenderKind` + `octopus-sdk-ui-intent::validate_render_block` + 本表 + `docs/sdk/14`。

| # | kind | 用途 | payload 主字段 | 引入位置 | 引入周 |
|---|---|---|---|---|---|
| 1 | `text` | 纯文本 | `text: String` | `sdk-ui-intent` | W4 |
| 2 | `markdown` | Markdown 文本 | `md: String` | `sdk-ui-intent` | W4 |
| 3 | `code` | 代码块 | `language: String, source: String` | `sdk-ui-intent` | W4 |
| 4 | `diff` | Inline diff | `hunks: [Hunk]` | `sdk-ui-intent` | W4 |
| 5 | `list-summary` | 子代理 / 连续工具聚合 | `items: [Item]` | `sdk-ui-intent` | W5 |
| 6 | `progress` | 进度 | `label: String, percent: Option<u8>` | `sdk-ui-intent` | W4 |
| 7 | `artifact-ref` | 成果物引用 | `id, title, version, kind` | `sdk-ui-intent` | W5 |
| 8 | `record` | 表格 / 档案行 | `fields: Map<String, Value>` | `sdk-ui-intent` | W4 |
| 9 | `error` | 错误 | `title, detail, hint` | `sdk-ui-intent` | W4 |
| 10 | `raw` | 逃生舱 | `value: JsonValue` | `sdk-ui-intent` | W4 |

---

## 7. SDK 公共符号注册与修改流程

1. 开子 Plan Task 时若需新增/修改 §2 中任何 `pub` 签名 → Task 必须引用本文档 §2 对应小节。
2. 批次 PR 合入时：
   - 修改本文档对应小节（`new_string` 必须包含 diff 说明）。
   - 同批次在 `00-overview.md §10 变更日志` 不追加（避免 double-log）；仅在本文档 §10 追加。
3. 对外符号**移除**时：
   - 必须在 `03-legacy-retirement.md` 登记"被谁替代"。
   - 本文档对应小节的签名也要更新。

---

## 8. Cargo Workspace 变更要点（供 W7 / W8 使用）

- W7 结束时 `members` 应为：
  ```
  apps/desktop/src-tauri,
  crates/octopus-core,
  crates/octopus-persistence,      # W8 新增
  crates/octopus-platform,
  crates/octopus-infra,             # 保留（W8 按资源拆文件）
  crates/octopus-server,
  crates/octopus-desktop,
  crates/octopus-cli,
  crates/octopus-sdk,               # 门面
  crates/octopus-sdk-contracts,
  crates/octopus-sdk-session,
  crates/octopus-sdk-model,
  crates/octopus-sdk-tools,
  crates/octopus-sdk-mcp,
  crates/octopus-sdk-context,
  crates/octopus-sdk-permissions,
  crates/octopus-sdk-sandbox,
  crates/octopus-sdk-hooks,
  crates/octopus-sdk-subagent,
  crates/octopus-sdk-plugin,
  crates/octopus-sdk-ui-intent,
  crates/octopus-sdk-observability,
  crates/octopus-sdk-core,
  crates/telemetry,                 # 保留；W6 被 observability 引用后评估是否并入
  ```
- W7 同步删除：`crates/runtime`、`crates/tools`、`crates/plugins`、`crates/api`、`crates/octopus-runtime-adapter`、`crates/commands`、`crates/compat-harness`、`crates/mock-anthropic-service`、`crates/rusty-claude-cli`、`crates/octopus-desktop-backend`、`crates/octopus-model-policy`。
- `default-members` 列五个业务 crate + Tauri app：`apps/desktop/src-tauri / octopus-core / octopus-persistence / octopus-platform / octopus-server / octopus-desktop`。保证 `cargo build --default-members` 的可编译闭包完整（`octopus-server` 依赖 `octopus-persistence`，后者必须在 default 列中）。

---

## 9. 对外公共面行数预算

> 用于审 PR 时快速判断是否在"简洁对外面"的承诺范围内。

| Crate | `lib.rs` 行数上限 | 对外 `pub` 符号数上限 |
|---|---|---|
| `octopus-sdk` | 60 | 10（按 §2.15 列出即终态） |
| `octopus-sdk-core` | 80 | 12 |
| `octopus-sdk-contracts` | 100 | 40（IR 数据类型多，例外） |
| `octopus-sdk-session` | 60 | 8 |
| `octopus-sdk-model` | 80 | 15 |
| 其他 SDK crate | 50 | 8 |

超出必须在 PR 描述中说明并在本文档 §10 追加决策记录。

---

## 10. 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-20 | 首稿：依赖图、15 crate 公共面草签、命名规约、差异清单/IR 登记表骨架、workspace 变更要点 | Architect |
| 2026-04-20 | P0 架构修订：`sdk-permissions` 由 Level 3 下调到 Level 2，与 `tools/sandbox/ui-intent/hooks/plugin/mcp` 同层并明文同层协作规则；`SecretVault` trait 下沉到 Level 0 `octopus-sdk-contracts`，门面 crate 改为纯 re-export（去除 trait 定义） | Architect |
| 2026-04-20 | P1 修订：§1 依赖图补充 "ui-intent → session" 事件回写箭头；§2.12 注明依赖；§8 `default-members` 补齐 `octopus-persistence`（共 5 业务 crate + Tauri app） | Architect |
