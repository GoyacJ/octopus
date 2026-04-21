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
  - `plugin` 持有 `tools / hooks / mcp` 的 runtime handle；其中 W5 的 tools/hooks 插件接线直接落到 `ToolRegistry` / `HookRunner`，不经 declaration-only shim。
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

**本周公共面修订清单（W3 / W4）**

1. 新增 `ToolCallRequest`，作为 tools / permissions 共用的最小调用形状。
2. 新增 `PermissionMode`，收敛 W3 工具执行的权限模式枚举。
3. 新增 `PermissionOutcome`，定义 `Allow / Deny / AskApproval` 三态握手。
4. 新增 `PermissionGate` trait，供 W3 `ToolContext` 持有窄接口。
5. 新增 `AskResolver / AskAnswer / AskError`，承接 `AskPrompt` 的业务侧回答回流。

#### W4 公共面修订清单

| # | 符号 | 变更 |
|---|---|---|
| 1 | `PermissionOutcome` | 追加 `RequireAuth { prompt: AskPrompt }`，权限握手完成四态。 |
| 2 | `ToolCategory` | 从 `§2.4 octopus-sdk-tools` 反向下沉到 `§2.1 octopus-sdk-contracts`；`sdk-tools` 只做 `pub use` 保持源兼容。 |
| 3 | `HookEvent` | 新增 8 类 hook 事件：`PreToolUse / PostToolUse / Stop / SessionStart / SessionEnd / UserPromptSubmit / PreCompact / PostCompact`。 |
| 4 | `EndReason` | 收敛为 `Normal / MaxTurns / UserCancelled / Error(String) / Compaction`，供 `SessionEnded` 与 hook 生命周期共用。 |
| 5 | `HookDecision` | 新增 `Continue / Rewrite / Abort / InjectMessage`，承接 hooks 运行结果。 |
| 6 | `HookToolResult / RewritePayload` | 新增 hooks 跨层薄镜像，避免 `sdk-hooks` 反向依赖 `sdk-tools`。 |
| 7 | `CompactionCtx` | 新增压缩前上下文，供 `PreCompact` rewrite 使用。 |
| 8 | `CompactionResult` | 新增压缩结果契约：`summary / folded_turn_ids / tool_results_cleared / tokens_before / tokens_after / strategy`。 |
| 9 | `CompactionStrategyTag` | 新增 `Summarize / ClearToolResults / Hybrid` 三种策略标签。 |
| 10 | `MemoryItem / MemoryKind / MemoryError` | 新增最小记忆值形状；具体后端 trait 留在 `sdk-context`。 |

```rust
pub struct SessionId(pub String);
pub struct RunId(pub String);
pub struct ToolCallId(pub String);
pub struct EventId(pub String);
impl SessionId { pub fn new_v4() -> Self; }
impl RunId { pub fn new_v4() -> Self; }
impl ToolCallId { pub fn new_v4() -> Self; }
impl EventId { pub fn new_v4() -> Self; }

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
pub struct RenderMeta {
    pub id: EventId,
    pub parent: Option<EventId>,
    pub ts_ms: i64,
}
pub struct RenderBlock {
    pub kind: RenderKind,
    pub payload: serde_json::Value,
    pub meta: RenderMeta,
}
pub enum RenderKind {
    Text, Markdown, Code, Diff, ListSummary, Progress,
    ArtifactRef, Record, Error, Raw,
}
pub struct AskPrompt {
    pub kind: String,
    pub questions: Vec<AskQuestion>,
}
pub struct AskQuestion {
    pub id: String,
    pub question: String,
    pub header: String,
    pub multi_select: bool,
    pub options: Vec<AskOption>,
}
pub struct AskOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub preview: Option<String>,
    pub preview_format: Option<String>,
}
pub struct ArtifactRef {
    pub kind: String,
    pub artifact_id: String,
    pub artifact_kind: ArtifactKind,
    pub title: Option<String>,
    pub preview: Option<String>,
    pub preview_format: Option<String>,
    pub version: Option<u32>,
    pub parent_version: Option<u32>,
    pub status: Option<ArtifactStatus>,
    pub content_type: Option<String>,
    pub superseded_by_version: Option<u32>,
}
pub enum ArtifactKind { Markdown, Code, Html, Svg, Mermaid, React, Json, Binary }
pub enum ArtifactStatus { Draft, Review, Approved, Published }
pub enum RenderLifecycle {
    OnToolUse, OnToolProgress, OnToolResult, OnToolRejected, OnToolError
}

// Session 事件
pub enum SessionEvent {
    SessionStarted {
        config_snapshot_id: String,
        effective_config_hash: String,
        plugins_snapshot: Option<PluginsSnapshot>, // preferred path; if None, next event must be SessionPluginsSnapshot
    },
    SessionPluginsSnapshot { plugins_snapshot: PluginsSnapshot },
    UserMessage(Message),
    AssistantMessage(Message),
    ToolExecuted { call: ToolCallId, name: String, duration_ms: u64, is_error: bool },
    Render { block: RenderBlock, lifecycle: RenderLifecycle },
    Ask { prompt: AskPrompt },
    Checkpoint { id: String, anchor_event_id: EventId },
    SessionEnded { reason: EndReason },
}
pub enum EndReason { Normal, MaxTurns, UserCancelled, Error(String), Compaction }

pub struct PromptCacheEvent {
    pub cache_read_input_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub breakpoint_count: u32,
}

pub struct CacheBreakpoint {
    pub position: usize,
    pub ttl: CacheTtl,
}

pub enum CacheTtl { FiveMinutes, OneHour }

pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub enum ToolCategory { Read, Write, Network, Shell, Subagent, Skill, Meta }

pub struct ToolCallRequest {
    pub id: ToolCallId,
    pub name: String,
    pub input: serde_json::Value,
}

pub enum PermissionMode { Default, AcceptEdits, BypassPermissions, Plan }
pub enum PermissionOutcome {
    Allow,
    Deny { reason: String },
    AskApproval { prompt: AskPrompt },
    RequireAuth { prompt: AskPrompt },
}

#[async_trait]
pub trait PermissionGate: Send + Sync {
    async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome;
}

#[async_trait]
pub trait AskResolver: Send + Sync {
    async fn resolve(&self, prompt_id: &str, prompt: &AskPrompt) -> Result<AskAnswer, AskError>;
}
pub struct AskAnswer {
    pub prompt_id: String,
    pub option_id: String,
    pub text: String,
}
pub enum AskError { NotResolvable, Timeout, Cancelled }

pub trait EventSink: Send + Sync {
    fn emit(&self, event: SessionEvent);
}

pub struct HookToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub render: Option<RenderBlock>,
}
pub struct CompactionCtx {
    pub session: SessionId,
    pub strategy: CompactionStrategyTag,
    pub threshold: f32,
    pub tokens_current: u32,
    pub tokens_budget: u32,
}
pub enum RewritePayload {
    ToolCall { call: ToolCallRequest },
    ToolResult { result: HookToolResult },
    UserPrompt { message: Message },
    Compaction { ctx: CompactionCtx },
}
pub enum HookDecision {
    Continue,
    Rewrite(RewritePayload),
    Abort { reason: String },
    InjectMessage(Message),
}
pub enum HookPoint {
    PreToolUse,
    PostToolUse,
    Stop,
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreCompact,
    PostCompact,
}
pub enum HookEvent {
    PreToolUse { call: ToolCallRequest, category: ToolCategory },
    PostToolUse { call: ToolCallRequest, result: HookToolResult },
    Stop { session: SessionId },
    SessionStart { session: SessionId },
    SessionEnd { session: SessionId, reason: EndReason },
    UserPromptSubmit { message: Message },
    PreCompact { session: SessionId, ctx: CompactionCtx },
    PostCompact { session: SessionId, result: CompactionResult },
}
pub enum CompactionStrategyTag { Summarize, ClearToolResults, Hybrid }
pub struct CompactionResult {
    pub summary: String,
    pub folded_turn_ids: Vec<EventId>,
    pub tool_results_cleared: u32,
    pub tokens_before: u32,
    pub tokens_after: u32,
    pub strategy: CompactionStrategyTag,
}
pub struct MemoryItem {
    pub id: String,
    pub kind: MemoryKind,
    pub payload: serde_json::Value,
    pub created_at_ms: i64,
}
pub enum MemoryKind { Note, Decision, Todo, SkillLog, Custom(String) }
pub enum MemoryError {
    NotFound,
    Backend { reason: String },
    Serialization(serde_json::Error),
}

pub struct PluginsSnapshot {
    pub api_version: String,
    pub plugins: Vec<PluginSummary>,
}
pub struct PluginSummary {
    pub id: String,
    pub version: String,
    pub git_sha: Option<String>,
    pub source: PluginSourceTag,
    pub enabled: bool,
    pub components_count: u16,
}
pub enum PluginSourceTag { Local, Bundled }

pub struct ToolDecl {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
    pub source: DeclSource,
}
pub struct HookDecl {
    pub id: String,
    pub point: HookPoint,
    pub source: DeclSource,
}
pub struct SkillDecl { pub id: String, pub manifest_path: PathBuf }
pub struct ModelProviderDecl { pub id: String, pub provider_ref: String }
pub enum DeclSource { Bundled, Plugin { plugin_id: String } }

// W5: Level 0 declaration 只保留 opaque key；真正解析到 sdk-model 的 ProviderId / ModelRole 留给 Level 1

// 凭据抽象：定义在 contracts，以便 sdk-model / sdk-tools / 业务层共用
#[async_trait]
pub trait SecretVault: Send + Sync {
    async fn get(&self, ref_id: &str) -> Result<SecretValue, VaultError>;
    async fn put(&self, ref_id: &str, value: SecretValue) -> Result<(), VaultError>;
}

pub struct SecretValue(/* 内部零化；Drop 擦除；禁止 Debug 明文 */);
impl SecretValue {
    pub fn new(value: impl AsRef<[u8]>) -> Self;
    pub fn as_bytes(&self) -> &[u8];
}
pub enum VaultError { NotFound, Backend(String), Redacted }
```

> `RenderBlock` 在 W1 Rust contracts 中保持 `kind + payload + meta` 的事件载体形式，以适配 `SessionEvent::Render` 与后续 `sdk-ui-intent::RenderEmitter`。`docs/sdk/14-ui-intent-ir.md` 的 TypeScript 伪代码描述的是工具侧逻辑 IR；两者通过 `kind` 与 payload schema 对齐，而不是逐字段 1:1 同构。
>
> W5 硬门禁：`HookPoint` 是 manifest/declaration 层的静态点位枚举；带 payload 的 `HookEvent` 仍只属于运行时执行面。`plugins_snapshot` 不是 session 尾部补丁，必须和 `SessionStarted` / `SessionPluginsSnapshot` / `SessionSnapshot` / store fixture 同批次扩面；优先内嵌在首事件，若首事件无法扩面则退回紧随其后的 `SessionPluginsSnapshot`。

> `SecretVault` 定义于 Level 0 是刻意的：Level 1 `sdk-model`（HTTP 认证）、Level 2 `sdk-tools`（工具内请求）均需使用同一 trait，而 Level 5 门面 crate 不得承担 trait 定义职责（层内规则禁止其引入逻辑）。

### 2.2 `octopus-sdk-session`（Level 1）

**职责**：Append-only 事件日志抽象 + `SqliteJsonlSessionStore` 默认实现。

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn append(&self, id: &SessionId, event: SessionEvent) -> Result<EventId, SessionError>;
    async fn append_session_started(
        &self,
        id: &SessionId,
        config_snapshot_id: String,
        effective_config_hash: String,
        plugins_snapshot: Option<PluginsSnapshot>,
    ) -> Result<EventId, SessionError>;
    async fn new_child_session(&self, parent_id: &SessionId, spec: &SubagentSpec) -> Result<SessionId, SessionError>;
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
    pub plugins_snapshot: PluginsSnapshot,
    pub head_event_id: EventId,
    pub usage: Usage,
}

pub enum SessionError {
    NotFound,
    Corrupted { reason: String },
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Serde(serde_json::Error),
}

pub struct SqliteJsonlSessionStore { /* impl SessionStore */ }
impl SqliteJsonlSessionStore {
    pub fn open(db: &Path, jsonl_root: &Path) -> Result<Self, SessionError>;
}
```

> W5 前置合同：`plugins_snapshot` 优先扩进 `SessionEvent::SessionStarted` 与 `SessionSnapshot`；若 W1 首事件不能扩面，则退回紧随其后的 `SessionEvent::SessionPluginsSnapshot`，但 `SessionSnapshot` 仍必须持有快照。store 持久化、golden fixture、OpenAPI/schema 对齐在同一批次继续跟进，不能留到 session 尾部小补丁。

#### 契约不变量

- 首事件必须为 `SessionStarted`；若对一个全新 `SessionId` 首次 append 非 `SessionStarted` 事件，`SessionStore::append` 必须返回 `SessionError::Corrupted { reason: "first_event_must_be_session_started" }`。
- W5 plugin session 快照必须满足二选一：要么首事件 `SessionStarted.plugins_snapshot.is_some()`，要么首事件 `plugins_snapshot == None` 且第二事件必须是 `SessionPluginsSnapshot`；两条分支最终都必须投影到 `SessionSnapshot.plugins_snapshot`。
- `SqliteJsonlSessionStore::open` 必须把 `runtime/events/*.jsonl` 视为 append-only 真相源；若启动时发现 `JSONL` 尾部领先于 SQLite `events/sessions` 投影，必须在 `open()` 内完成最小重建，使 `snapshot()/stream()` 重新看见 JSONL 已落盘的事件。
- `SessionSnapshot.usage` 不是占位字段；默认实现必须从会话事件流投影出累计 `Usage`，至少覆盖 assistant usage 事件的累加。
- `wake(id)` 虽然仍返回 `SessionSnapshot`，但若存在最新 `Checkpoint`，必须至少验证其 `anchor_event_id` 可解析到更早事件，并能为后续 replay 准备 `anchor_event_id` 之后的尾部事件；锚点缺失时返回 `SessionError::Corrupted`。

### 2.3 `octopus-sdk-model`（Level 1）

**职责**：Provider / Surface / Model 三层 + 5 种 `ProtocolAdapter`。

```rust
pub struct ProviderId(pub String);
pub struct SurfaceId(pub String);
pub struct ModelId(pub String);

pub struct Provider {
    pub id: ProviderId,
    pub display_name: String,
    pub status: ProviderStatus,
    pub auth: AuthKind,
    pub surfaces: Vec<SurfaceId>,
}
pub enum ProviderStatus { Active, Deprecated, Experimental }
pub struct Surface {
    pub id: SurfaceId,
    pub provider_id: ProviderId,
    pub protocol: ProtocolFamily,
    pub base_url: String,
    pub auth: AuthKind,
}
pub struct Model {
    pub id: ModelId,
    pub surface: SurfaceId,
    pub family: String,
    pub track: ModelTrack,
    pub context_window: ContextWindow,
    pub aliases: Vec<String>,
}
pub struct ContextWindow {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub supports_1m: bool,
}

pub enum ProtocolFamily {
    AnthropicMessages, OpenAiChat, OpenAiResponses, GeminiNative, VendorNative,
}
pub enum ModelTrack { Preview, Stable, LatestAlias, Deprecated, Sunset }
pub enum AuthKind { ApiKey, XApiKey, OAuth, AwsSigV4, GcpAdc, AzureAd, None }
// W2 执行层公共面暂不含 `Rerank`；见 `docs/sdk/README.md` 的 Fact-Fix 勘误。
pub enum ModelRole { Main, Fast, Best, Plan, Compact, Vision, WebExtract, Embedding, Eval, SubagentDefault }
pub struct ModelCatalog { /* built-in providers/surfaces/models + alias index */ }
pub struct ResolvedModel {
    pub provider: Provider,
    pub surface: Surface,
    pub model: Model,
}
impl ModelCatalog {
    pub fn new_builtin() -> Self;
    pub fn list_providers(&self) -> &[Provider];
    pub fn list_models(&self) -> &[Model];
    pub fn resolve(&self, reference: &str) -> Option<ResolvedModel>;
    pub fn canonicalize(&self, id: &str) -> Option<ModelId>;
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError>;
    fn describe(&self) -> ProviderDescriptor;
}

pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub supported_families: Vec<ProtocolFamily>,
    pub catalog_version: String,
}
pub struct ModelRequest {
    pub model: ModelId,
    pub system_prompt: Vec<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub role: ModelRole,
    pub cache_breakpoints: Vec<CacheBreakpoint>,
    pub response_format: Option<ResponseFormat>,
    pub thinking: Option<ThinkingConfig>,
    pub cache_control: CacheControlStrategy,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}
impl ModelRequest {
    pub fn tools_fingerprint(&self) -> String;
}
pub enum ResponseFormat { Json { schema: serde_json::Value }, Text }
pub struct ThinkingConfig { pub enabled: bool, pub budget_tokens: Option<u32> }
pub enum CacheControlStrategy {
    None,
    PromptCaching { breakpoints: Vec<&'static str> },
    ContextCacheObject { cache_id: String },
}
pub type ModelStream = Pin<Box<dyn Stream<Item = Result<AssistantEvent, ModelError>> + Send>>;

pub enum ModelError {
    AuthUnsupported { kind: AuthKind },
    AuthMissing { provider: ProviderId },
    UpstreamStatus { status: u16, body_preview: String },
    UpstreamTimeout,
    Overloaded { retry_after_ms: Option<u64> },
    PromptTooLong { estimated_tokens: u32, max: u32 },
    AdapterNotImplemented { family: ProtocolFamily },
    CapabilityUnsupported { capability: String, model: ModelId },
    Serialization(serde_json::Error),
    Transport(reqwest::Error),
    ModelNotFound { id: ModelId },
}

pub struct RoleRouter { /* 静态映射 role → ModelId */ }
impl RoleRouter {
    pub fn new_builtin(catalog: &ModelCatalog) -> Self;
    pub fn with_override(self, role: ModelRole, model: ModelId) -> Self;
    pub fn resolve(&self, role: ModelRole) -> Option<ModelId>;
}
pub enum FallbackTrigger { Overloaded, Upstream5xx, PromptTooLong, ModelDeprecated }
pub struct FallbackPolicy { /* chain + triggers */ }
impl FallbackPolicy {
    pub fn with_route(self, current: ModelId, next: ModelId) -> Self;
    pub fn should_fallback(&self, err: &ModelError) -> Option<FallbackTrigger>;
    pub fn next_model(&self, current: &ModelId) -> Option<&ModelId>;
}

pub type StreamBytes = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, ModelError>> + Send>>;
pub struct AnthropicMessagesAdapter;
pub struct OpenAiChatAdapter;
pub struct OpenAiResponsesAdapter;
pub struct GeminiNativeAdapter;
pub struct VendorNativeAdapter;
pub trait ProtocolAdapter: Send + Sync {
    fn family(&self) -> ProtocolFamily;
    fn to_request(&self, req: &ModelRequest) -> Result<serde_json::Value, ModelError>;
    fn parse_stream(&self, raw: StreamBytes) -> Result<ModelStream, ModelError>;
    async fn auth_headers(
        &self,
        vault: &dyn SecretVault,
        provider: &Provider,
    ) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError>;
}

pub struct DefaultModelProvider { /* catalog + adapters + http_client + secret_vault */ }
impl DefaultModelProvider {
    pub fn new(
        catalog: Arc<ModelCatalog>,
        adapters: HashMap<ProtocolFamily, Arc<dyn ProtocolAdapter>>,
        http_client: reqwest::Client,
        secret_vault: Arc<dyn SecretVault>,
    ) -> Self;
    pub async fn complete_with_fallback(
        &self,
        req: ModelRequest,
        policy: &FallbackPolicy,
    ) -> Result<ModelStream, ModelError>;
}
```

### 2.4 `octopus-sdk-tools`（Level 2）

**职责**：`Tool` trait + 15 个内置工具 + 并发分区。

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn is_concurrency_safe(&self, input: &serde_json::Value) -> bool;
    async fn execute(&self, ctx: ToolContext, input: serde_json::Value) -> Result<ToolResult, ToolError>;
}

pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,  // JSON Schema
    pub category: ToolCategory,
}
impl ToolSpec {
    pub fn to_mcp(&self) -> ToolSchema;
}

pub struct ToolRegistry { /* deterministic order guaranteed */ }
impl ToolRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<(), RegistryError>;
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn schemas_sorted(&self) -> Vec<&ToolSpec>;  // deterministic for prompt cache
    pub fn tools_fingerprint(&self) -> String;
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Arc<dyn Tool>)>;
    pub fn as_directory(&self) -> Arc<dyn ToolDirectory>;
}

pub struct ToolContext {
    pub session_id: SessionId,
    pub permissions: Arc<dyn PermissionGate>,
    pub sandbox: SandboxHandle,
    pub session_store: Arc<dyn SessionStore>,
    pub secret_vault: Arc<dyn SecretVault>,
    pub ask_resolver: Arc<dyn AskResolver>,
    pub event_sink: Arc<dyn EventSink>,
    pub working_dir: PathBuf,
    pub cancellation: CancellationToken,
}

pub struct SandboxHandle {
    pub cwd: PathBuf,
    pub env_allowlist: Vec<String>,
}

pub struct ToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub render: Option<RenderBlock>,  // UI Intent IR 由工具发出
}

pub enum ToolError {
    Validation { message: String },
    Permission { message: String },
    Execution { message: String },
    Timeout { message: String },
    Cancelled { message: String },
    NotYetImplemented { crate_name: &'static str, week: &'static str },
    Transport(reqwest::Error),
    Serialization(serde_json::Error),
    Sandbox { reason: String },
}
impl ToolError {
    pub fn as_tool_result(&self) -> ToolResult;
}

pub enum RegistryError {
    DuplicateName(String),
    InvalidSpec(String),
}

pub enum ExecBatch<'a> {
    Concurrent(Vec<&'a ToolCallRequest>),
    Serial(Vec<&'a ToolCallRequest>),
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
    impl FileReadTool { pub fn new() -> Self; }
    impl FileWriteTool { pub fn new() -> Self; }
    impl FileEditTool { pub fn new() -> Self; }
    impl GlobTool { pub fn new() -> Self; }
    impl GrepTool { pub fn new() -> Self; }
    impl BashTool { pub fn new() -> Self; }
    impl WebSearchTool { pub fn new() -> Self; }
    impl WebFetchTool { pub fn new() -> Self; }
    impl AskUserQuestionTool { pub fn new() -> Self; }
    impl TodoWriteTool { pub fn new() -> Self; }
    impl SleepTool { pub fn new() -> Self; }
    impl AgentTool { pub fn new() -> Self; }
    impl SkillTool { pub fn new() -> Self; }
    impl TaskListTool { pub fn new() -> Self; }
    impl TaskGetTool { pub fn new() -> Self; }
    pub fn register_builtins(registry: &mut ToolRegistry) -> Result<(), RegistryError>;
}

pub const BASH_MAX_OUTPUT_DEFAULT: usize = 30_000;
pub const BASH_MAX_OUTPUT_UPPER_LIMIT: usize = 150_000;
pub const DEFAULT_TOOL_MAX_CONCURRENCY: usize = 10;
```

#### W4 反向修订

- `ToolCategory` 已在 W4 下沉到 `§2.1 octopus-sdk-contracts`。
- `octopus-sdk-tools` 保持 `pub use octopus_sdk_contracts::ToolCategory`，现有 call-site 无需改名。
- 排序稳定性契约不变：`ToolRegistry::schemas_sorted()` 继续按 `category_priority() + name` 排序。

### 2.5 `octopus-sdk-mcp`（Level 2）

**职责**：MCP 协议客户端 + 三种传输。

```rust
pub enum TransportKind { Stdio, Http, Sdk }

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;
    async fn notify(&self, msg: JsonRpcNotification) -> Result<(), McpError>;
}

pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}
impl JsonRpcRequest {
    pub fn new(id: serde_json::Value, method: impl Into<String>, params: Option<serde_json::Value>) -> Self;
    pub fn method(&self) -> &str;
    pub fn params(&self) -> Option<&serde_json::Value>;
}
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}
impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self;
    pub fn failure(id: serde_json::Value, error: JsonRpcError) -> Self;
}
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}
impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self;
}
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
}
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
pub struct McpToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
}

pub enum McpError {
    Transport { message: String },
    Protocol { message: String },
    Timeout { message: String },
    Handshake { message: String },
    ServerCrashed { server_id: String, exit_code: Option<i32> },
    ToolNotFound { name: String },
    InvalidResponse { body_preview: String },
}

pub struct InitializeResult {
    pub protocol_version: String,
}

pub struct McpClient {
    transport: Arc<dyn McpTransport>,
    server_id: String,
    initialized: AtomicBool,
}
impl McpClient {
    pub fn new(server_id: impl Into<String>, transport: Arc<dyn McpTransport>) -> Self;
    pub fn server_id(&self) -> &str;
    pub fn is_initialized(&self) -> bool;
    pub async fn initialize(&self) -> Result<InitializeResult, McpError>;
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError>;
    pub async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>;
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError>;
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError>;
}

pub enum McpLifecyclePhase { Starting, Ready, Degraded, Stopped }

pub struct ServerHandle {
    pub client: Arc<McpClient>,
    pub kind: TransportKind,
    pub phase: McpLifecyclePhase,
}
pub struct McpServerSpec {
    pub server_id: String,
    pub transport: McpServerTransport,
}
pub enum McpServerTransport {
    Stdio { command: String, args: Vec<String>, env: HashMap<String, String>, transport: Arc<dyn McpTransport> },
    Http { transport: Arc<dyn McpTransport> },
    Sdk { transport: Arc<dyn McpTransport> },
}

pub struct McpServerManager { /* lifecycle, crash-as-tool-error */ }
impl McpServerManager {
    pub fn new() -> Self;
    pub async fn spawn(&self, spec: McpServerSpec) -> Result<String, McpError>;
    pub async fn shutdown(&self, server_id: &str) -> Result<(), McpError>;
    pub fn get_client(&self, server_id: &str) -> Option<Arc<McpClient>>;
    pub fn list_servers(&self) -> Vec<String>;
}

#[async_trait]
pub trait ToolDirectory: Send + Sync {
    fn list_tools(&self) -> Vec<McpTool>;
    async fn call_tool(&self, name: &str, input: serde_json::Value) -> Result<McpToolResult, McpError>;
}

pub struct StdioTransport { /* ... */ }
impl StdioTransport {
    pub fn spawn(
        command: impl AsRef<str>,
        args: impl IntoIterator<Item = impl Into<String>>,
        env: HashMap<String, String>,
    ) -> Result<Self, McpError>;
}
pub struct HttpTransport { /* ... */ }
impl HttpTransport {
    pub fn new(base_url: impl Into<String>, headers: HashMap<String, String>) -> Result<Self, McpError>;
}
pub struct SdkTransport { /* in-process */ }
impl SdkTransport {
    pub fn from_directory(directory: Arc<dyn ToolDirectory>) -> Self;
}
```

### 2.6 `octopus-sdk-context`（Level 3）

**职责**：Prompt builder + compaction + tool-result clearing + memory backend trait。

```rust
pub struct SystemPromptBuilder { /* identity + tool guidance + output format + examples */ }
impl SystemPromptBuilder {
    pub fn new() -> Self;
    pub fn with_section(self, section: SystemPromptSection) -> Self;
    pub fn build(&self, ctx: &PromptCtx) -> Vec<String>;   // stable sections; order locked
    pub fn fingerprint(&self, ctx: &PromptCtx) -> [u8; 32];
}

pub struct PromptCtx<'a> {
    pub session: SessionId,
    pub mode: PermissionMode,
    pub project_root: PathBuf,
    pub tools: &'a ToolRegistry,
}

pub struct SystemPromptSection {
    pub id: &'static str,
    pub order: u32,
    pub body: String,
}

pub struct Compactor { /* compaction + tool-result clearing */ }
impl Compactor {
    pub fn new(
        threshold: f32,
        strategy: CompactionStrategyTag,
        provider: Arc<dyn ModelProvider>,
    ) -> Self;
    pub async fn maybe_compact(&self, session: &mut SessionView)
        -> Result<Option<CompactionResult>, CompactionError>;
    pub async fn clear_tool_results(&self, session: &mut SessionView) -> u32;
    pub async fn summarize(&self, session: &mut SessionView)
        -> Result<CompactionResult, CompactionError>;
}

pub struct SessionView<'a> {
    pub messages: &'a mut Vec<Message>,
    pub tokens: u32,
    pub tokens_budget: u32,
    pub event_ids: Vec<EventId>,
}

pub enum CompactionError {
    ModelUnavailable,
    SummaryTooLarge,
    Aborted { reason: String },
    Provider(ModelError),
}

#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn recall(&self, query: &str) -> Result<Vec<MemoryItem>, MemoryError>;
    async fn commit(&self, item: MemoryItem) -> Result<(), MemoryError>;
}

pub struct DurableScratchpad { /* runtime/notes/<session>.md */ }
impl DurableScratchpad {
    pub fn new(base: PathBuf) -> Self;
    pub async fn read(&self, session: &SessionId) -> Result<Option<String>, MemoryError>;
    pub async fn write(&self, session: &SessionId, content: &str) -> Result<(), MemoryError>;
}
```

> W4 Task 9 回填：`SystemPromptBuilder` 已落 `role / tools_guidance / process / safety / output` 五段内置段生成器；`tools_guidance` 只消费 `ToolRegistry::schemas_sorted()` 的 `name / description`，不把 `input_schema` 写进 system prompt。
>
> W4 Task 10 回填：`Compactor` 已落 `ClearToolResults / Summarize / Hybrid(abort)` 三分支；`SessionView` 薄壳在 W4 额外带 `tokens_budget + event_ids`，只为阈值判断与 `folded_turn_ids` 审计；`DurableScratchpad::write()` 已改为 `NamedTempFile + persist` 的原子写。

### 2.7 `octopus-sdk-permissions`（Level 2，Hands 侧 gate）

**职责**：权限模式、审批闸门、提示生成（对接 `AskPrompt`）。

```rust
pub use octopus_sdk_contracts::PermissionMode;

pub enum PermissionBehavior { Allow, Deny, Ask }
pub enum PermissionRuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    FlagSettings,
    PolicySettings,
    CliArg,
    Command,
    Session,
}
pub struct PermissionRule {
    pub source: PermissionRuleSource,
    pub behavior: PermissionBehavior,
    pub tool_name: String,
    pub rule_content: Option<String>,
}

pub struct PermissionPolicy { /* source-priority-sorted rules */ }
impl PermissionPolicy {
    pub fn new() -> Self;
    pub fn from_sources(rules: Vec<PermissionRule>) -> Self;
    pub fn match_rules(&self, call: &ToolCallRequest)
        -> (Vec<&PermissionRule>, Vec<&PermissionRule>, Vec<&PermissionRule>);
    pub fn evaluate(&self, ctx: &PermissionContext) -> Option<PermissionOutcome>;
}

pub struct PermissionContext {
    pub call: ToolCallRequest,
    pub mode: PermissionMode,
    pub category: ToolCategory,
}

pub struct DefaultPermissionGate {
    pub policy: PermissionPolicy,
    pub mode: PermissionMode,
    pub broker: Arc<ApprovalBroker>,
    pub category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync>,
}
impl DefaultPermissionGate {
    pub fn new(
        policy: PermissionPolicy,
        mode: PermissionMode,
        broker: Arc<ApprovalBroker>,
        category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync>,
    ) -> Self;
}

pub enum PermissionOutcome {
    Allow,
    Deny { reason: String },
    AskApproval { prompt: AskPrompt },
    RequireAuth { prompt: AskPrompt },
}

pub struct ApprovalBroker { /* emit SessionEvent::Ask; resolve approval:<call_id> */ }
impl ApprovalBroker {
    pub fn new(event_sink: Arc<dyn EventSink>, ask_resolver: Arc<dyn AskResolver>) -> Self;
    pub async fn request_approval(
        &self,
        call: &ToolCallRequest,
        prompt: AskPrompt,
    ) -> PermissionOutcome;
}
```

> W4 Task 4 回填：`DefaultPermissionGate::check()` 按 `deny → allow → bypass → plan → ask → mode fallback` 执行 `canUseTool` 决策链；`ApprovalBroker` 负责 `SessionEvent::Ask` 发射与 `AskAnswer.option_id -> PermissionOutcome` 映射。

### 2.8 `octopus-sdk-sandbox`（Level 2）

**职责**：OS 级沙箱抽象 + 三后端实现。

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
    pub cpu_time_limit_ms: Option<u64>,
    pub wall_time_limit_ms: Option<u64>,
    pub memory_limit_bytes: Option<u64>,
}

pub struct NetworkProxy {
    pub endpoint: String,
}

pub struct SandboxCommand {
    pub cmd: String,
    pub args: Vec<String>,
    pub stdin: Option<Vec<u8>>,
}

pub struct SandboxOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub truncated: bool,
    pub timed_out: bool,
}

pub enum SandboxError {
    Provision { reason: String },
    Execute { reason: String },
    Terminate { reason: String },
    UnsupportedPlatform,
    ResourceExhausted { kind: String },
    Timeout,
}

pub struct SandboxHandle { inner: Arc<dyn SandboxHandleInner + Send + Sync> }
pub trait SandboxHandleInner {
    fn cwd(&self) -> &Path;
    fn env_allowlist(&self) -> &[String];
    fn backend_name(&self) -> &'static str;
}
impl SandboxHandle {
    pub fn from_inner(inner: Arc<dyn SandboxHandleInner + Send + Sync>) -> Self;
    pub fn new(cwd: PathBuf, env_allowlist: Vec<String>, backend_name: &'static str) -> Self;
    pub fn cwd(&self) -> &Path;
    pub fn env_allowlist(&self) -> &[String];
    pub fn backend_name(&self) -> &'static str;
}

pub fn default_backend_for_host() -> Arc<dyn SandboxBackend>;
```

> W4 Task 5 回填：`sdk-tools::ToolContext.sandbox` 继续叫 `SandboxHandle`，但具体类型改由 `octopus-sdk-sandbox` 提供，现有 call-site 通过 `cwd()` / `env_allowlist()` getter 保持兼容。
>
> W4 Task 6 回填：`NoopBackend` 走 `tokio::process::Command + env_clear + allowlist env`，`SeatbeltBackend` 在 cwd 下生成 `.octopus-seatbelt.sb` 并用 `sandbox-exec -f` 执行，`BubblewrapBackend` 用 `bwrap --die-with-parent --new-session --unshare-all` 封装最小 Linux 沙箱；`default_backend_for_host()` 在 macOS/Linux 返回真实后端，在 Windows 回退 `NoopBackend` 并记录 `TODO(W8)` warn。

### 2.9 `octopus-sdk-hooks`（Level 2）

**职责**：生命周期钩子。

```rust
pub struct HookToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub render: Option<RenderBlock>, // Level 0 镜像；与 sdk-tools::ToolResult 在 hooks/core 边界互转
}

pub enum HookEvent {
    PreToolUse { call: ToolCallRequest, category: ToolCategory },
    PostToolUse { call: ToolCallRequest, result: HookToolResult },
    Stop { session: SessionId },
    SessionStart { session: SessionId },
    SessionEnd { session: SessionId, reason: EndReason },
    UserPromptSubmit { message: Message },
    PreCompact { session: SessionId, ctx: CompactionCtx },
    PostCompact { session: SessionId, result: CompactionResult },
}

#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    async fn on_event(&self, event: &HookEvent) -> HookDecision;
}

pub enum RewritePayload {
    ToolCall(ToolCallRequest),
    ToolResult(HookToolResult),
    UserPrompt(Message),
    Compaction(CompactionCtx),
}

pub enum HookDecision {
    Continue,
    Rewrite(RewritePayload),
    Abort { reason: String },
    InjectMessage(Message),
}

pub enum HookError {
    RewriteNotAllowed { event_kind: &'static str },
    InjectNotAllowed { event_kind: &'static str },
    HookPanic { name: String },
    Serialization(serde_json::Error),
}

pub enum HookSource {
    Plugin { plugin_id: String },
    Workspace,
    Defaults,
    Project,
    Session,
}

pub struct HookRegistration {
    pub hook: Arc<dyn Hook>,
    pub source: HookSource,
    pub priority: i32,
    pub name: String,
}

pub struct HookRunOutcome {
    pub decisions: Vec<(String, HookDecision)>,
    pub final_payload: Option<RewritePayload>,
    pub aborted: Option<String>,
}

pub struct HookRunner { /* W4 最小子集：8 events；按 source/priority/name deterministic order */ }
impl HookRunner {
    pub fn new() -> Self;
    pub fn register(&self, name: &str, hook: Arc<dyn Hook>, source: HookSource, priority: i32);
    pub fn unregister_by_source(&self, source: HookSource) -> usize;
    pub async fn run(&self, event: HookEvent) -> Result<HookRunOutcome, HookError>;
}
```

> W4 Task 7 回填：`HookRunner` 已落最小执行器，按 `source -> priority -> name` 排序，并把 `Rewrite` 仅绑定到 `PreToolUse / PostToolUse / UserPromptSubmit / PreCompact`，`InjectMessage` 仅绑定到 `Stop / UserPromptSubmit`。W5 叠加 plugin source 后，来源优先级固定为 `session > project > plugin > workspace > defaults`；同来源内再按 `priority -> name` 排序。

### 2.10 `octopus-sdk-subagent`（Level 3）

**职责**：子代理编排模式。

```rust
pub struct SubagentSpec {
    pub id: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub model_role: String,
    pub permission_mode: PermissionMode,
}

pub struct SubagentContext {
    pub session_store: Arc<dyn SessionStore>,
    pub model: Arc<dyn ModelProvider>,
    pub tools: Arc<ToolRegistry>,
    pub permissions: Arc<dyn PermissionGate>,
    pub hooks: Arc<HookRunner>,
    pub parent_session: Option<SessionId>,
    pub depth: u8,
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

pub struct PluginToolRegistration {
    pub decl: ToolDecl,
    pub tool: Arc<dyn Tool>,
}

pub struct PluginHookRegistration {
    pub decl: HookDecl,
    pub hook: Arc<dyn Hook>,
    pub source: HookSource,
    pub priority: i32,
}

pub struct PluginRegistry { /* 单向：plugin → registry ← core；tools/hooks 同时持有 runtime handle */ }
impl PluginRegistry {
    pub fn tools(&self) -> &ToolRegistry;
    pub fn hooks(&self) -> &HookRunner;
    pub fn get_snapshot(&self) -> PluginsSnapshot;
}

pub struct PluginApi<'a> { /* 持有 ToolRegistry / HookRunner + metadata stores */ }
impl PluginApi<'_> {
    pub fn register_tool(&mut self, reg: PluginToolRegistration) -> Result<(), PluginError>;
    pub fn register_hook(&mut self, reg: PluginHookRegistration) -> Result<(), PluginError>;
    pub fn register_skill_decl(&mut self, decl: SkillDecl) -> Result<(), PluginError>;
    pub fn register_model_provider_decl(&mut self, decl: ModelProviderDecl) -> Result<(), PluginError>;
}

pub struct PluginLifecycle { /* discover → enable → load → register */ }

pub enum PluginError { /* 22 型分类错误 */ }
```

> W5 执行边界：`ToolDecl` / `HookDecl` 只用于 manifest、诊断、snapshot；真正接入 runtime 的是 `PluginToolRegistration` / `PluginHookRegistration`。`skills / model providers` 在 W5 仍先停留在 metadata + builder slot。

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
| 1 | 2026-04-21 | `octopus-sdk-contracts::Usage` | `contracts/openapi/src/components/schemas/runtime.yaml#MessageUsage` | `input_tokens / output_tokens / cache_*_input_tokens` vs `inputTokens / outputTokens / totalTokens` | SDK 侧保留四个细粒度 token 计数且使用 snake_case；OpenAPI 侧只暴露 camelCase 的三字段汇总，不承载 prompt cache 读/写计数。 | `dual-carry` | `open` |
| 2 | 2026-04-21 | `octopus-sdk-contracts::{Message, ContentBlock}` | `contracts/openapi/src/components/schemas/runtime.yaml#RuntimeMessage` / `packages/schema/src/workbench.ts#Message` | block-based IR vs flattened message envelope | SDK 侧 `Message` 是 `role + Vec<ContentBlock>`，支持 `tool_use` / `tool_result` / `thinking` 的递归块结构；OpenAPI/workbench 侧仍是 `content: string` + `toolCalls/processEntries/attachments` 的扁平 envelope。 | `align-openapi` | `open` |
| 3 | 2026-04-21 | `octopus-sdk-contracts::SessionEvent::SessionStarted` | `contracts/openapi/src/components/schemas/runtime.yaml` session/message shapes | `config_snapshot_id` / `effective_config_hash` 首事件缺口 | W1 SDK 会话流把 `SessionStarted { config_snapshot_id, effective_config_hash }` 作为首事件强制不变量，但现有 OpenAPI runtime/session schema 未公开等价的首事件载荷或字段。 | `align-openapi` | `open` |
| 4 | 2026-04-21 | `octopus-sdk-contracts::PromptCacheEvent` | `contracts/openapi/src/components/schemas/runtime.yaml#MessageUsage` | prompt cache 事件缺失 | SDK 侧已有 `PromptCacheEvent` 与 `CacheBreakpoint/CacheTtl` 最小签名；OpenAPI runtime schema 目前没有对应事件或 message usage 扩展位。 | `align-openapi` | `open` |
| 5 | 2026-04-21 | `octopus-sdk-model::Surface` | `docs/sdk/11-model-system.md §11.3.2 SurfaceDefinition` / future runtime surface OpenAPI schema | `provider_id` 反向索引字段 | SDK 侧 `Surface` 新增 `provider_id: ProviderId`，用于 catalog 解析与反向索引；`docs/sdk/11` 的 `SurfaceDefinition` 目前未显式声明该字段，后续若对外公开 runtime surface schema 需同步补齐。 | `align-openapi` | `open` |
| 6 | 2026-04-21 | `octopus-sdk-model::OpenAiChatAdapter` | `contracts/openapi/src/components/schemas/runtime.yaml#MessageUsage` / OpenAI-compatible chat usage payload | `cache_creation_input_tokens / cache_read_input_tokens` 缺失 | OpenAI-compatible chat completion usage 当前只稳定提供 `prompt_tokens / completion_tokens`；SDK 侧在 `Usage` 结构上保留四计数，adapter 统一把两项 cache 计数映射为 `0`。 | `dual-carry` | `open` |
| 7 | 2026-04-21 | `octopus-sdk-model::ModelRole` | `contracts/openapi/src/**` / `packages/schema/src/**` runtime model routing shapes | role enum 缺口 | W2 SDK 公共面公开 `main / fast / best / plan / compact / vision / web_extract / embedding / eval / subagent_default` 共 10 个角色值；现有 OpenAPI/schema 侧没有等价的 runtime model role 枚举或路由配置载体。 | `align-openapi` | `open` |
| 8 | 2026-04-21 | `octopus-sdk-model::{SurfaceId, Surface}` | `packages/schema/src/catalog.ts#ModelSurfaceId` | provider-qualified surface id vs generic surface kind | SDK catalog 用 `anthropic.conversation` / `openai.responses` 这类 provider-qualified `SurfaceId` 保证全局唯一；schema 侧 `ModelSurfaceId` 仍是 `conversation / responses / files ...` 的通用枚举，缺少 provider 维度。 | `dual-carry` | `open` |
| 9 | 2026-04-21 | `octopus-sdk-model::ModelRequest` | `contracts/openapi/src/**` / `packages/schema/src/**` runtime request shapes | `cache_breakpoints` / `cache_control` 缺口 | SDK canonical request 已公开 `cache_breakpoints` 与 `cache_control`，用于 prompt cache / context cache 控制；现有 OpenAPI/schema 请求体没有对应字段或等价结构。 | `align-openapi` | `open` |
| 10 | 2026-04-21 | `octopus-sdk-tools::{ToolSpec, ToolCategory}` | `contracts/openapi/src/**` / `packages/schema/src/**` runtime tool catalog shapes | tool category enum 缺口 | W3 SDK 工具目录把 `read / write / network / shell / subagent / skill / meta` 作为 prompt cache 稳定排序键的一部分；现有 OpenAPI `RuntimeToolDefinition` 没有等价 `category` 枚举，也没有排序稳定性的契约文字。 | `align-openapi` | `open` |
| 11 | 2026-04-21 | `octopus-sdk-contracts::{SessionEvent::SessionStarted, SessionEvent::SessionPluginsSnapshot, PluginsSnapshot}` / `octopus-sdk-session::SessionSnapshot` | `contracts/openapi/src/components/schemas/runtime.yaml` / `packages/schema/src/**` runtime session shapes | `plugins_snapshot` 缺口 | W5 子代理/插件周要求 plugin session 快照走显式双分支：优先由首事件 `SessionStarted` 携带，若首事件无法扩面则退回紧随其后的 `SessionPluginsSnapshot`；两条分支都必须能投影出 `SessionSnapshot.plugins_snapshot`，以保证 replay 可复现插件集合。现有 OpenAPI/schema 侧没有等价字段、次事件或插件快照对象。 | `align-openapi` | `open` |
| 12 | 2026-04-21 | `octopus-sdk-mcp::{McpTool, McpPrompt, McpResource, McpToolResult}` | `contracts/openapi/src/**` / `packages/schema/src/**` runtime transport/tool shapes | MCP-native DTO 缺口 | W3 已冻结 `tools/list` / `prompts/list` / `resources/list` / `tools/call` 的 SDK-native DTO；现有 OpenAPI/schema 仍只有 runtime/session 侧 envelope，没有直接承载 MCP 目录与结果的对外契约。 | `align-openapi` | `open` |
| 13 | 2026-04-21 | `octopus-sdk-contracts::{ToolCallRequest, PermissionMode, PermissionOutcome}` | `contracts/openapi/src/components/schemas/runtime.yaml#RuntimePermissionDecision`（现状） | permission handshake 形状不一致 | W3 SDK 使用 `ToolCallRequest + PermissionMode + PermissionOutcome(Allow/Deny/AskApproval)` 作为 tools/permissions 的最小握手面；现有 runtime schema 仍以 adapter 侧 decision/projection 字段为主，未公开等价调用请求与审批 prompt 契约。 | `align-openapi` | `open` |
| 14 | 2026-04-21 | `octopus-sdk-tools::partition_tool_calls` | `docs/sdk/03-tool-system.md §3.2` / future runtime orchestration contract | `partition_tool_calls.resource_bucket` 延后 | W3 只冻结"工具级"并发分区：读工具按 `is_concurrency_safe` 合批，写工具串行；资源级串行桶 `partition_tool_calls.resource_bucket` 明确延到 W4，由 `HookRunner / PermissionPolicy` 外层兜底，当前无需改 OpenAPI。 | `no-op` | `open` |
| 15 | 2026-04-21 | `octopus-sdk-sandbox::default_backend_for_host` | `docs/sdk/06-permissions-sandbox.md §6.10` / Windows host runtime contract | Windows 沙箱后端延后 | W4 只落 `Noop / Seatbelt / Bubblewrap` 三后端；Windows 真实 AppContainer/Job Object 仍未实现，当前公共面固定为 `NoopBackend` fallback + `TODO(W8)` warning。 | `no-op` | `open` |

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
| 2026-04-21 | W1 执行回填：补齐 `RenderMeta`、ID `new_v4()`、`SecretValue::{new, as_bytes}` 公共面；登记 `SessionStarted` 首事件不变量与 W1↔OpenAPI 差异清单首批条目 | Codex |
| 2026-04-21 | W2 计划审计回填：`ToolSchema` 下沉到 `§2.1`；`§2.3` 补齐 `ProviderStatus / ContextWindow / ProviderDescriptor / ResponseFormat / ThinkingConfig / CacheControlStrategy / ModelError / FallbackTrigger / DefaultModelProvider::complete_with_fallback` 与 `ModelRequest` 新字段；注明 `ModelRole` 暂不含 `rerank` 的 Fact-Fix 回链 | Codex |
| 2026-04-21 | W4 Task 1：Level 0 contracts 补丁完成；`PermissionOutcome` 补 `RequireAuth`，新增 `HookEvent / HookDecision / Compaction* / Memory*`，并把 `ToolCategory` 从 `§2.4` 反向下沉到 `§2.1`，`sdk-tools` 以 re-export 保持源兼容 | Codex |
| 2026-04-21 | W4 Task 2：新增 `octopus-sdk-permissions` crate 骨架；`PermissionMode` 改由 `sdk-permissions::mode` re-export contracts 版本，`§2.7` 标注骨架先行、规则实现顺延 Task 3/4 | Codex |
| 2026-04-21 | W4 Task 3：`§2.7` 回填 `PermissionRule / PermissionRuleSource / PermissionBehavior / PermissionPolicy::{new,from_sources,match_rules,evaluate}`；明确规则先按 `source` 排序，再按 `deny > allow > ask` 输出同步决策 | Codex |
| 2026-04-21 | W4 Task 4：`§2.7` 回填 `DefaultPermissionGate` 与 `ApprovalBroker`；`check()` 固化 `canUseTool` 决策链，`request_approval()` 固化 `approval:<call_id>` + `SessionEvent::Ask` 流程 | Codex |
| 2026-04-21 | W4 Task 5：新增 `octopus-sdk-sandbox` crate 骨架；`SandboxSpec / SandboxCommand / SandboxOutput / SandboxError / SandboxHandle` 迁入 `§2.8`，`sdk-tools` 改为 re-export 新 `SandboxHandle` 并以 getter 保持兼容 | Codex |
| 2026-04-21 | W4 Task 6：`§2.8` 回填 `NoopBackend / SeatbeltBackend / BubblewrapBackend / default_backend_for_host()` 的真实后端语义，并在 `§5` 登记 Windows AppContainer 延后到 W8 的契约差异 | Codex |
| 2026-04-21 | W4 Task 7：新增 `octopus-sdk-hooks` crate；`§2.9` 回填 `HookSource / HookRegistration / HookRunOutcome / HookRunner::{new,register,unregister_by_source,run}`，并固定 `source -> priority -> name` 的确定性顺序与 rewrite/inject 的事件边界 | Codex |
| 2026-04-21 | W4 Task 9：新增 `octopus-sdk-context` crate 骨架；`§2.6` 回填 `PromptCtx / SystemPromptSection / SystemPromptBuilder::{new,with_section,build,fingerprint}`，并固定 `tools_guidance` 只读 `ToolRegistry::schemas_sorted()` 的稳定段生成 | Codex |
| 2026-04-21 | W4 Task 10：`§2.6` 回填 `Compactor::{new,maybe_compact,clear_tool_results,summarize}`、`SessionView` 的阈值/审计字段、`CompactionError` 和 `DurableScratchpad::{new,read,write}`；`scratchpad` 写入改为原子 rename | Codex |
| 2026-04-21 | W4 Task 11：补齐 `prompt_cache_fingerprint` 守护与 permissions/hooks 的 `no_credentials_in_events` 合同测试；确认 `SystemPromptBuilder::fingerprint()` 与 `ToolRegistry::tools_fingerprint()` 组合哈希稳定，且摘要事件不暴露原始凭据输入 | Codex |
| 2026-04-21 | W4 Weekly Gate：workspace `build/clippy/test` 全绿；`§2.6 / §2.7 / §2.8 / §2.9` 的实现与 W4 出口状态对齐，Week 4 状态收口为 `done` | Codex |
| 2026-04-21 | W5 审计追补：`ModelProviderDecl.provider_ref` 与 `SubagentSpec.model_role` 明确为 Level 0 opaque key，避免 contracts 直接引用 `ProviderId / ModelRole`；`§2.9` 补记 W5 hook 来源优先级 `session > project > plugin > workspace > defaults` | Codex |
| 2026-04-21 | W5 三轮审计追补：`§2.1` 把 `plugins_snapshot` 调整为“首事件优先 + `SessionPluginsSnapshot` fallback”的显式双分支；`§2.2` 补登记 `append_session_started(..., Option<PluginsSnapshot>)` 与 `new_child_session(...)`；`§5` 的 session 差异描述同步改成双分支 replay 合同 | Codex |
