# `octopus-harness-hook` · L2 复合能力 · Hook System SPEC

> 层级：L2 · 状态：Accepted
> 依赖：`harness-contracts`

## 1. 职责

提供 **Hook Dispatcher**：在 Agent 运行关键节点（PreToolUse / PostToolUse / UserPromptSubmit 等）触发业务逻辑，支持改写输入、覆盖权限、注入上下文、阻止执行。对齐 HER-034 / HER-036 / OC-29 / CC-22 / CC-23 / CC-25。

**核心能力**：

- 20 类标准 HookEvent（与 D7 · `extensibility.md §5` 一一对应）
- In-process / Exec / HTTP 三种 transport（`HookTransport` trait 同时是开放扩展点；自实现 transport 视作 in-process handler 的子类，详见 §3.4）
- `HookOutcome` 受限能力矩阵 + `PreToolUse` 复合三件套形态
- 多 handler 串联（按 priority；同 priority 取 `handler_id` 字典序）
- Allow / Deny / Rewrite 三种语义（`emit_collect`）
- 失败语义闭环：`HookOutcomeInconsistent` / `HookReturnedUnsupported` / `HookPanicked` / `HookTimeout` 等都有专用事件落 Journal，并由 `failure_mode` 决定 fail-open 或 fail-closed（详见 §2.6.1）
- Replay 幂等：所有 handler 必须满足 `replay_idempotent` 契约（详见 §11）

## 2. 对外 API

### 2.1 核心 Trait

```rust
#[async_trait]
pub trait HookHandler: Send + Sync + 'static {
    fn handler_id(&self) -> &str;
    fn interested_events(&self) -> &[HookEventKind];
    fn priority(&self) -> i32 { 0 }

    async fn handle(
        &self,
        event: HookEvent,
        ctx: HookContext,
    ) -> Result<HookOutcome, HookError>;
}
```

### 2.2 Events（20 类，分 5 组；与 `extensibility.md §5.1` 一一对应）

```rust
#[non_exhaustive]
pub enum HookEventKind {
    // ── A · 核心生命周期 ─────────────────────────────────
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    PermissionRequest,
    SessionStart,
    Setup,
    SessionEnd,
    SubagentStart,
    SubagentStop,
    Notification,
    // ── B · LLM/API 层 ──────────────────────────────────
    PreLlmCall,
    PostLlmCall,
    PreApiRequest,
    PostApiRequest,
    // ── C · 转换层（独占改写权）─────────────────────────
    TransformToolResult,
    TransformTerminalOutput,
    // ── D · MCP ─────────────────────────────────────────
    Elicitation,
    // ── E · Tool Search（ADR-009） ──────────────────────
    PreToolSearch,
    PostToolSearchMaterialize,
}

#[non_exhaustive]
pub enum HookEvent {
    // ── A · 核心生命周期 ─────────────────────────────────
    UserPromptSubmit { run_id: RunId, input: TurnInput },
    PreToolUse { tool_use_id: ToolUseId, tool_name: String, input: Value },
    PostToolUse { tool_use_id: ToolUseId, result: ToolResult },
    PostToolUseFailure { tool_use_id: ToolUseId, error: ToolErrorView },
    PermissionRequest { request_id: RequestId, subject: String, detail: Option<String> },
    SessionStart { session_id: SessionId },
    Setup { workspace_root: Option<PathBuf> },
    SessionEnd { session_id: SessionId, reason: EndReason },
    SubagentStart { subagent_id: SubagentId, spec: SubagentSpecView },
    SubagentStop { subagent_id: SubagentId, status: SubagentStatus },
    Notification { kind: NotificationKind, body: Value },
    // ── B · LLM/API 层 ──────────────────────────────────
    PreLlmCall { run_id: RunId, request_view: ModelRequestView },
    PostLlmCall { run_id: RunId, usage: UsageSnapshot },
    PreApiRequest { request_id: RequestId, endpoint: String },
    PostApiRequest { request_id: RequestId, status: u16 },
    // ── C · 转换层（独占改写权）─────────────────────────
    TransformToolResult { tool_use_id: ToolUseId, result: ToolResult },
    TransformTerminalOutput { tool_use_id: ToolUseId, raw: Bytes },
    // ── D · MCP ─────────────────────────────────────────
    Elicitation { mcp_server_id: McpServerId, schema: JsonSchema },
    // ── E · Tool Search（ADR-009） ──────────────────────
    /// `tool_search` 调用发起前。可用于审计/拦截特定查询（详见 ADR-009 §2.6）。
    PreToolSearch {
        tool_use_id: ToolUseId,
        query: String,
        query_kind: ToolSearchQueryKind,
    },
    /// backend 完成 materialize 之后、schema 可见之前。可用于追加 hint 或记录；
    /// **不允许**改变 `materialized` 列表（见 §2.4）。
    PostToolSearchMaterialize {
        tool_use_id: ToolUseId,
        materialized: Vec<ToolName>,
        backend: ToolLoadingBackendName,
        cache_impact: CacheImpact,
    },
}
```

> `ToolSearchQueryKind` / `ToolLoadingBackendName` / `CacheImpact` 定义在 `harness-contracts §3.4` 与 `harness-session §3.5`。

### 2.2.1 HookContext

`HookContext` 是 dispatcher 在调用 handler 时构造的**只读**入参，不允许 handler 自行写入或副作用调用 Engine 内部对象。

```rust
pub struct HookContext {
    /// 路由轴：tenant/session/run/turn/correlation/causation 五元组。
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub turn_index: Option<u32>,
    pub correlation_id: CorrelationId,
    pub causation_id: CausationId,

    /// 信任与权限上下文（只读快照）。
    pub trust_level: TrustLevel,
    pub permission_mode: PermissionMode,
    pub interactivity: InteractivityLevel,

    /// 触发时刻（dispatcher 测得的 UTC，handler 内部不应再次取系统时钟）。
    pub at: DateTime<Utc>,

    /// 已脱敏的 session 视图——handler 需要查询历史消息/工具结果时
    /// **必须**通过这个 view，而不是直接读 Journal/Session。
    /// 该 view 已应用 `Redactor` 与 `BlobRef.preview_only` 规则。
    pub view: Arc<dyn HookSessionView>,

    /// Hook 链内可见的"上一 handler 输出"——仅在 PreToolUse 链式 rewrite 中非空，
    /// 用于让链中段 handler 识别本次 input 是否已被前序改写过（避免重复改写）。
    pub upstream_outcome: Option<UpstreamOutcomeView>,

    /// Replay 上下文：当 dispatcher 处于 `ReplayMode::Audit` 时，
    /// handler 的副作用通道（`HookEmitter` 等 capability）会被替换成 NoOp，
    /// 详见 §11 Replay 语义。
    pub replay_mode: ReplayMode,
}

pub trait HookSessionView: Send + Sync {
    fn workspace_root(&self) -> Option<&Path>;
    fn recent_messages(&self, n: usize) -> Vec<MessageView>;
    fn permission_mode(&self) -> PermissionMode;
    fn redacted(&self) -> &dyn Redactor;
    fn current_tool_descriptor(&self) -> Option<&ToolDescriptorView>;
}

pub struct UpstreamOutcomeView {
    pub last_handler_id: HandlerId,
    pub rewrote_input: bool,
    pub override_permission_present: bool,
    pub additional_context_bytes: Option<u64>,
}

#[non_exhaustive]
pub enum ReplayMode {
    /// 在线运行：handler 可使用全部 capability。
    Live,
    /// Replay 重建：handler 只读，`HookEmitter` 等副作用 capability 被替换为 NoOp，
    /// dispatcher 用 Journal 中已有的 `HookTriggered` / `HookFailed` 等事件还原结果，
    /// **不会再次调用** in-process handler 的 `handle()`；该字段只用于
    /// 业务方在嵌入式 replay 工具中区分场景（如审计扫描器禁止外发请求）。
    Audit,
}
```

**约束**：

- `HookContext` 字段全部 `Clone + Send`，跨 Exec/HTTP transport 时由 dispatcher 序列化为 JSON `context` 段（见 §3.2/§3.3）。
- `HookSessionView` 是 trait object——Exec/HTTP transport 透传给 handler 的是其 JSON 投影 `HookSessionViewJson { workspace_root, recent_messages_summary, permission_mode }`，**不**承诺携带完整消息历史。
- handler **不得**对 `HookContext` 做内部缓存（每次调用语义相同的字段在 in-process 路径上是 `Arc` 共享，跨进程路径是新副本）；否则违反 §11 replay 幂等契约。

### 2.3 Outcome 能力矩阵

```rust
/// 分发型 outcome：每个 variant 对应一个或一组事件能用的"形状"。
/// `PreToolUse(PreToolUseOutcome)` 是**唯一**支持"复合三件套"的形态——
/// 同一个 handler 可以在一次返回里同时改写输入、覆盖审批决策、追加上下文。
/// 其他事件保留单一形态以避免歧义（详见 §2.4）。
#[non_exhaustive]
pub enum HookOutcome {
    /// 默认 / 透传：不做修改。所有事件均可用。
    Continue,

    /// 通用阻断：handler 拒绝继续。所有事件均可用（除 `Notification`）。
    Block { reason: String },

    /// PreToolUse 专属：复合三件套（任意子集）。
    PreToolUse(PreToolUseOutcome),

    /// `UserPromptSubmit` 专属的输入改写。
    RewriteInput(Value),

    /// `PermissionRequest` 单独覆盖决策的形态——与 PreToolUse 区分，
    /// 因为 PermissionRequest 上下文里没有 `tool_input` 可一同改写。
    OverridePermission(Decision),

    /// `PostToolUse` / `PostToolUseFailure` / `SessionStart` / `SubagentStart` /
    /// `SubagentStop` 的单一上下文注入形态。
    AddContext(ContextPatch),

    /// `TransformToolResult` / `TransformTerminalOutput` 专属。
    Transform(Value),
}

/// `PreToolUse` 的"三件套 + 可选阻断"复合 outcome（对齐 CC-23）。
///
/// 设计要点：**字段同时生效**——并不是互斥关系——
/// 这是 `HookOutcome` 在 PreToolUse 之外的事件不能用单一 `Block { reason }`
/// 表达的语义：业务可以"先 rewrite_input 把命令改安全，再 override_permission
/// 强制 AllowOnce，同时 additional_context 给模型解释为什么这样改"。
///
/// 任一字段为 `None` 表示"该子能力不参与本次 hook 输出"；全字段为 None
/// 等价于 `HookOutcome::Continue`，dispatcher 视为透传。
pub struct PreToolUseOutcome {
    /// 改写工具输入（与 ADR-010 §2.7 流水线的 `tool.validate` 之前生效）。
    /// 与 `block` 互斥：dispatcher 校验时 `block.is_some() && rewrite_input.is_some()`
    /// 为 `Outcome::Inconsistent`，按 `Continue` 处理并记 Event::HookOutcomeInconsistent。
    pub rewrite_input: Option<Value>,

    /// 覆盖权限决策（最终走 `DecidedBy::Hook { handler_id }`）；
    /// 多 handler 同时覆盖时取 `priority` 最高者，平手按 `handler_id` 字典序。
    pub override_permission: Option<Decision>,

    /// 注入到下一轮 prompt 的附加上下文。
    pub additional_context: Option<ContextPatch>,

    /// 阻断当前工具调用——置 Some 时其他三个字段必须 None。
    pub block: Option<String>,
}

pub struct ContextPatch {
    pub role: ContextPatchRole,
    pub content: String,
    pub apply_to_next_turn_only: bool,
}

pub enum ContextPatchRole {
    SystemAppend,     // 注入 system message 尾部（仅 SessionStart）
    UserPrefix,       // 注入当前 user message 前
    UserSuffix,
    AssistantHint,
}
```

### 2.4 能力许可（对齐 D7 §5.3）

| Event | 允许的 `HookOutcome` variant | 备注 |
|---|---|---|
| `UserPromptSubmit` | `Continue` / `RewriteInput` / `Block` | 单一形态，仅改 user 输入 |
| `PreToolUse` | `Continue` / `Block` / `PreToolUse(PreToolUseOutcome)` | **三件套唯一入口**；普通形态不再单独使用 |
| `PostToolUse` | `Continue` / `AddContext` | 只读 + 上下文注入 |
| `PostToolUseFailure` | `Continue` / `AddContext` | 只读 + 上下文注入 |
| `TransformToolResult` | `Continue` / `Transform` | budget 后改写信封 |
| `TransformTerminalOutput` | `Continue` / `Transform` | 流式片段改写 |
| `PermissionRequest` | `Continue` / `OverridePermission` | 单独覆盖（无 tool_input 可改） |
| `SessionStart` | `Continue` / `AddContext`（SystemAppend only） | preamble 拼接 |
| `SubagentStart` / `SubagentStop` | `Continue` / `AddContext` | announce 增强 |
| `Notification` | `Continue` | 不允许 Block / Rewrite |
| `Elicitation` | `Continue` / `Block` | 只能透传或阻断 MCP 弹窗 |
| `PreToolSearch` | `Continue` / `Block` | Tool Search 调用前审计/拦截（ADR-009） |
| `PostToolSearchMaterialize` | `Continue` / `AddContext` | materialize 之后追加提示；**不能改变 materialized 列表** |
| `PreLlmCall` | `Continue` / `RewriteInput`（`ModelRequest` patch） | 改写模型请求（如注入 system suffix） |
| `PostLlmCall` | `Continue` | 仅观察，用于 usage 审计 |
| `PreApiRequest` | `Continue` / `Block` | 可拦截原始 HTTP 请求（合规场景） |
| `PostApiRequest` | `Continue` | 仅观察 |

Dispatcher 在收到不在允许列表的 `HookOutcome` variant 时视为 `Continue` 并记 `Event::HookReturnedUnsupported { handler_id, kind }`（结构定义见 `event-schema.md §3.7`）。`PreToolUseOutcome` 内部的字段一致性（`block` 与其他字段互斥）由 dispatcher 在调用前 `validate()` 校验：违例同样降级为 `Continue` 并记 `Event::HookOutcomeInconsistent { handler_id, reason }`。

**`PreLlmCall::RewriteInput` 的额外校验**：rewrite 后的 `ModelRequest` 必须**保持 system prompt 与 tool 定义稳定**（ADR-003 Prompt Cache 锁定），仅允许在 user messages 尾部追加；否则 dispatcher 判 `HookOutcomeInconsistent { reason: PromptCacheViolation }` 并降级 `Continue`。详细字段白名单见 `harness-model.md §3`。

### 2.4.1 多 handler 串联的合并语义（PreToolUse 专属）

多 PreToolUse handler 按 priority 降序逐一执行，输入流为：

```text
原始 ToolCall.input
    │
    ▼  handler A
PreToolUseOutcome { rewrite_input: Some(input_a), additional_context: Some(ctx_a), .. }
    │
    │ rewrite_input → 作为 handler B 的输入
    │ override_permission → 缓存到 PendingOverride 列表
    │ additional_context → 累加到 ContextPatchSet
    │ block → 立即终止链，返回 Block
    ▼  handler B
PreToolUseOutcome { override_permission: Some(AllowOnce), .. }
    │
    │ override_permission → 加入 PendingOverride
    ▼  ...
```

**最终合并规则**：

1. **`rewrite_input`**：链尾的最后一份非 None 值生效（即"流水线最后一次改写"）。
2. **`override_permission`**：取最高 priority handler 的非 None 值；同 priority 取 `handler_id` 字典序最小者；产生 `DecidedBy::Hook { handler_id }`。
3. **`additional_context`**：所有非 None 值按 handler 执行顺序累加（`ContextPatchSet`）；同 `ContextPatchRole` 不去重，由 context engineering 阶段决定是否折叠。
4. **`block`**：任一 handler 置 Some 立即短路；`reason` 取该 handler 的 reason，前序累积的 outcome **全部丢弃**（已落 Journal 的 Hook 事件保留作审计）。
5. 所有 handler 都返回 `Continue` 或 `PreToolUseOutcome::default()` 时，dispatcher 视为"无 hook 影响"，工具流水线按未注入路径继续（详见 §2.7）。

### 2.5 Registry

```rust
pub struct HookRegistry {
    inner: Arc<RwLock<HookRegistryInner>>,
}

struct HookRegistryInner {
    by_event: HashMap<HookEventKind, Vec<Arc<dyn HookHandler>>>,
}

impl HookRegistry {
    pub fn builder() -> HookRegistryBuilder;
    pub fn register(&self, handler: Box<dyn HookHandler>) -> Result<(), RegistrationError>;
    pub fn snapshot(&self) -> HookRegistrySnapshot;
}

pub struct HookRegistrySnapshot {
    handlers: Arc<HashMap<HookEventKind, Vec<Arc<dyn HookHandler>>>>,
    generation: u64,
}
```

### 2.6 Dispatcher

```rust
pub struct HookDispatcher {
    snapshot: HookRegistrySnapshot,
    observer: Arc<Observer>,
}

impl HookDispatcher {
    pub async fn dispatch(
        &self,
        event: HookEvent,
        ctx: HookContext,
    ) -> Result<DispatchResult, HookError>;
}

pub struct DispatchResult {
    pub final_outcome: HookOutcome,
    pub trail: Vec<HookInvocationRecord>,
    /// 链路中是否出现过失败（含 timeout / panic / unsupported / inconsistent / schema 校验不过）；
    /// dispatcher 不会因为单个 handler 失败而失败 dispatch：失败语义由 §2.6.1 的 `failure_mode` 决定。
    pub failures: Vec<HookFailureRecord>,
}

pub struct HookInvocationRecord {
    pub handler_id: HandlerId,
    pub outcome: HookOutcome,
    pub duration: Duration,
}

pub struct HookFailureRecord {
    pub handler_id: HandlerId,
    pub mode: HookFailureMode,
    pub cause: HookFailureCause,
    pub duration: Duration,
}

#[non_exhaustive]
pub enum HookFailureCause {
    /// handler 返回了不在 §2.4 允许列表内的 `HookOutcome` variant。
    Unsupported { kind: HookOutcomeDiscriminant },
    /// `PreToolUseOutcome` 内部字段冲突（如 `block` 与其他字段共存）；或 `RewriteInput` 违反 schema。
    Inconsistent { reason: InconsistentReason },
    /// handler 抛 panic（仅 in-process 路径可能；Exec/HTTP 路径表现为非零退出 / 5xx）。
    Panicked { snippet: String },
    /// 超出 handler 自身的 `timeout` 设置。
    Timeout,
    /// Exec/HTTP 协议解析失败、协议版本不兼容、SSRF 触发等 transport 层错误。
    Transport { kind: TransportFailureKind },
    /// 未授权 capability（如 UserControlled plugin 试图申请 `HookEmitter`）。
    Unauthorized { capability: CapabilityId },
}

#[non_exhaustive]
pub enum InconsistentReason {
    /// PreToolUseOutcome 同时设置 block 与其他字段。
    PreToolUseBlockExclusive,
    /// `PreLlmCall::RewriteInput` 改动了被 Prompt Cache 锁定的字段。
    PromptCacheViolation,
    /// `RewriteInput` / `Transform` 输出不通过 schema 校验。
    SchemaInvalid { schema: SchemaId, error: String },
    /// `AddContext` 输出体积超出单次 hook 的硬上限（默认 16 KiB，详见 §2.4.2）。
    ContextPatchTooLarge { limit_bytes: u64, actual_bytes: u64 },
}
```

**串联规则**（对齐 HER-036 `emit_collect`）：

- 按 priority **降序**执行；同 priority 取 `handler_id` 字典序升序（确定性 tiebreaker，replay 必须复现）。
- 事件特定的复合合并见 §2.4.1（PreToolUse 三件套）。
- 任一 handler 返回 `HookOutcome::Block` 或 `PreToolUseOutcome { block: Some(_), .. }` → 立即停止、返回 Block。
- 其他事件单一形态：
  - `RewriteInput` / `Transform`：链式生效，每个 handler 看到的都是上一个的输出。链尾输出在交付下游之前由 dispatcher 跑一次 schema 校验（输入端用 `tool.descriptor.input_schema`，结果端用 `ToolResultEnvelope` schema），失败按 `Inconsistent::SchemaInvalid` 计入 `HookFailureRecord` 并降级为"该 handler 的输出忽略"，链路按上一份合法输出继续。
  - `AddContext`：跨 handler 累积，order 与 handler 执行顺序一致；累积总字节超过 `HookContextBudget`（默认 64 KiB / 单事件）时，**末尾溢出条目**被丢弃并记一条 `Inconsistent::ContextPatchTooLarge` 失败记录。
  - `OverridePermission`（PermissionRequest / PreToolUse 事件下）：取 priority 最高者；同 priority 同时给出冲突决策（如 A 给 `AllowOnce`、B 给 `DenyOnce`）时，**Deny 压过 Allow**——产生 `DecidedBy::Hook { handler_id }` 取 Deny 一方的 handler_id；写一条 `HookPermissionConflict` 失败记录（结构定义见 `event-schema.md §3.7`）。

### 2.6.1 Failure Mode

每条 `HookManifestEntry`（plugin manifest）/ `HookExecSpec` / `HookHttpSpec` 都需要声明 `failure_mode`，控制单个 handler 失败时 dispatcher 的兜底行为。

```rust
#[non_exhaustive]
pub enum HookFailureMode {
    /// 失败时按 `Continue` 继续后续 handler；写 `HookFailed` 事件但不阻塞业务。
    /// 默认值；适用于 audit / observability 类 hook。
    FailOpen,

    /// 失败时立即终止本次 dispatch，把失败原因转为 `HookOutcome::Block { reason }`
    /// 返回；适用于安全/合规类 hook（DLP、审计强制）。
    /// 仅 `TrustLevel::AdminTrusted` 的 hook 可声明 `FailClosed`——UserControlled 强行声明
    /// 时由 PluginRegistry 在 `validate()` 阶段拒绝（详见 ADR-006）。
    FailClosed,
}
```

**默认值**：

| Hook 来源 | 默认 `failure_mode` |
|---|---|
| `Bundled` / `Plugin{AdminTrusted}` 显式声明的 hook | 取 manifest 中的声明值；未声明默认 `FailOpen` |
| `Plugin{UserControlled}` / `User` / `Workspace` 配置安装的 hook | 强制 `FailOpen`（不允许 `FailClosed`） |
| 内置审计/合规 hook（如 PermissionAudit）| `FailClosed`（必须 fail-closed，否则审计漏写） |

无论 mode 如何，**`HookFailedEvent` 都必记**，确保审计可见。Dispatcher 在 mode = `FailClosed` 路径下还会把失败 reason 串到 `ToolUseDenied { reason: HookFailClosed, handler_id }` 上。

### 2.7 与 Tool 流水线的协同（ADR-010 / harness-tool §2.7 / §2.8）

Hook 与 Tool 的协同点固化在 `ToolOrchestrator` 的单次调用流水线里（**权威定义**位于 `harness-tool.md §2.7` 流程图，本节给出 Hook 视角的等价说明）：

```text
ToolCall arrived
  │
  ├─ [Hook::PreToolUse]                  ← Continue / Block / PreToolUse(PreToolUseOutcome { rewrite_input?, override_permission?, additional_context?, block? })
  │
  ├─ tool.validate / tool.check_permission / Broker
  │
  ├─ tool.execute → ToolStream
  │     ├─ ToolEvent::Progress           ← 仅作可观测，不进 Hook 链
  │     ├─ ToolEvent::Partial(bytes)
  │     │     │
  │     │     └─ [Hook::TransformTerminalOutput]   ← 流式拦截（仅 Bash / Subagent stream）
  │     │
  │     └─ ToolEvent::Final(result)
  │           │
  │           └─ Orchestrator 完成 ResultBudget 流式收集 → ToolResultEnvelope
  │
  ├─ [Hook::TransformToolResult]         ← 看到的是 budget 处理后的 envelope.result
  │
  ├─ [Hook::PostToolUse]                 ← 看到 envelope.result（含 head/tail 预览）
  │     └─ failure 路径 → [Hook::PostToolUseFailure]
  │
  └─ Event::ToolUseCompleted | ToolUseFailed | ToolResultOffloaded（如有）
```

**关键约束**：

| 介入点 | 看到什么 | 不能做什么 |
|---|---|---|
| `PreToolUse` | 原始 `input` + `descriptor` | 直接调用其它 Tool |
| `TransformTerminalOutput` | **未进入 budget 计量**的原始字节流片段 | 改 `tool_use_id` / 跳出当前 stream |
| `TransformToolResult` | **已 budget 处理后**的 `ToolResultEnvelope.result`（含 head/tail 与 BlobRef 标记） | 改写已落盘的 BlobRef |
| `PostToolUse` | 同 `TransformToolResult`，但只读 | 抛 panic 中断流水线（Dispatcher 会 catch + emit `HookPanicked`） |
| `PostToolUseFailure` | `ToolErrorView` | 把失败"伪造"成成功 |

**Capability 约束（ADR-011）**：

- Hook 实现**不允许**直接持有 `Arc<dyn SubagentRunner>` 等 Engine 内部对象；
- 如需跨工具效应，使用 `ToolCapability::HookEmitter` 通过事件总线发布，而非直接写 Journal；
- Plugin 来源的 Hook 默认 `Trust::UserControlled`，不允许申请 `SubagentRunner / RunCanceller / HookEmitter` 等敏感 capability（参见 `CapabilityPolicy::default_locked`）。

**与 ResultBudget 的边界**：

- `Progress` / `Partial` 事件**不**单独触发 Hook 调用——避免每个流式片段进 Hook 造成额外开销；
- 仅当流尾产生 `Final` 事件时，Orchestrator 才把信封交给 `TransformToolResult` 与 `PostToolUse`；
- 若工具命中 `OverflowAction::Reject`，则跳过 `TransformToolResult` / `PostToolUse`，直接走 `PostToolUseFailure` 路径。

## 3. Transport 形态

### 3.1 In-Process

直接实现 `trait HookHandler`（Rust 代码内嵌）：

```rust
pub struct AuditHook {
    store: Arc<dyn AuditStore>,
}

#[async_trait]
impl HookHandler for AuditHook {
    fn handler_id(&self) -> &str { "audit" }
    fn interested_events(&self) -> &[HookEventKind] {
        &[HookEventKind::PreToolUse, HookEventKind::PostToolUse]
    }
    async fn handle(&self, event: HookEvent, _ctx: HookContext)
        -> Result<HookOutcome, HookError>
    {
        self.store.record(&event).await?;
        Ok(HookOutcome::Continue)
    }
}
```

### 3.2 Exec（对齐 HER-036）

```rust
pub struct HookExecSpec {
    pub handler_id: HandlerId,
    pub interested_events: Vec<HookEventKind>,
    pub failure_mode: HookFailureMode,
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: WorkingDir,
    pub timeout: Duration,
    pub resource_limits: HookExecResourceLimits,
    pub signal_policy: HookExecSignalPolicy,
    pub protocol_version: HookProtocolVersion,
    pub trust: TrustLevel,
}

#[non_exhaustive]
pub enum WorkingDir {
    /// dispatcher 自动选 session 工作区（最常用）
    SessionWorkspace,
    /// 显式 pin 到某路径——必须在 `harness-permission.PermissionRoot` 允许范围内；
    /// 越界由 PluginRegistry 在 `validate()` 阶段 fail-closed
    Pinned(PathBuf),
    /// 在临时目录里跑（dispatcher 会在 hook 结束后清理）；
    /// 只允许 `AdminTrusted` + `failure_mode = FailOpen`
    EphemeralTemp,
}

pub struct HookExecResourceLimits {
    /// CPU 时长（wall clock + cpu time 双限制；任一命中即超时）
    pub cpu_time: Duration,
    /// 子进程及其 forked 子进程的总内存
    pub memory_bytes: u64,
    /// 标准输出 / 标准错误总字节（超出后 dispatcher 关闭管道并按 `Inconsistent` 处理）
    pub max_stdio_bytes: u64,
    /// 文件描述符上限（默认 64）
    pub max_open_files: u32,
    /// 是否允许子进程联网（默认 false；联网需要在 manifest 显式 opt-in）
    pub allow_network: bool,
}

#[non_exhaustive]
pub enum HookExecSignalPolicy {
    /// 超时先 SIGTERM，宽限期后 SIGKILL（默认）
    GracefulThenKill { grace: Duration },
    /// 立即 SIGKILL（适用于不响应信号的二进制）
    ImmediateKill,
}

impl HookExecSpec {
    pub fn into_handler(self) -> Result<impl HookHandler, HookError>;
}
```

**约束**（对齐 ADR-006）：

- Exec Hook 仅允许 `TrustLevel::AdminTrusted` 安装；UserControlled plugin 在 `validate()` 阶段直接被 PluginRegistry 拒绝。
- `command` **不允许** shell metacharacter（`$ ` `;` `|` `&` 等）；shell 调用必须由 admin 在 manifest 里显式声明完整命令行。
- `env` 默认 `inherit_with_deny()`：屏蔽常见凭据类变量（`AWS_*` / `GH_TOKEN` / `OPENAI_*` 等），白名单由 `harness-config` 管理。
- `working_dir = EphemeralTemp` 时，dispatcher 拒绝 hook 通过 `HookEmitter` 发起任何持久化副作用，确保隔离干净。

IO 协议：

```text
stdin:  JSON { "protocol_version": "v1",
               "event":  "pre_tool_use",
               "body":   {...},
               "context": { ...HookContextJson... } }

stdout: JSON { "protocol_version": "v1",
               "outcome": { "continue": null } }
       或      { "protocol_version": "v1",
                 "outcome": { "block": { "reason": "..." } } }
       或      { "protocol_version": "v1",
                 "outcome": { "rewrite_input": { ...new_input... } } }   // UserPromptSubmit
       或      { "protocol_version": "v1",
                 "outcome": { "pre_tool_use": {
                    "rewrite_input": { ...new_input... },
                    "override_permission": { "allow_once": null },
                    "additional_context": {
                        "role": "user_prefix",
                        "content": "...",
                        "apply_to_next_turn_only": true
                    }
                 } } }
```

> Exec / HTTP transport 的 JSON 编码遵循 `serde(rename_all = "snake_case")`；
> Rust 内部 `PreToolUseOutcome` 字段名与上面 JSON key 完全一一对应，避免实现层做映射。
> `protocol_version` 字段语义与版本协商规则见 §3.4。

### 3.3 HTTP

```rust
pub struct HookHttpSpec {
    pub handler_id: HandlerId,
    pub interested_events: Vec<HookEventKind>,
    pub failure_mode: HookFailureMode,
    pub url: Url,
    pub auth: HookHttpAuth,
    pub timeout: Duration,
    pub retry: RetryPolicy,
    pub security: HookHttpSecurityPolicy,
    pub protocol_version: HookProtocolVersion,
    pub trust: TrustLevel,
}

pub enum HookHttpAuth {
    None,
    Bearer(SecretString),
    Hmac { key: SecretString },
    Custom(Arc<dyn HookHttpAuthProvider>),
}

pub struct HookHttpSecurityPolicy {
    /// 仅允许命中 allowlist 的 host 才发起请求；UserControlled plugin **必填**。
    /// 空 allowlist 视为 fail-closed（任何 URL 都会被拒）。
    pub allowlist: HostAllowlist,
    /// SSRF 兜底：阻止 DNS 解析到链路本地 / 私网 / loopback 地址
    /// （除非 `allow_private` 在 admin manifest 里显式开启）。
    pub ssrf_guard: SsrfGuardPolicy,
    /// HTTP redirect 最多跟随次数；默认 0（禁止 redirect，防止跨域逃逸）。
    pub max_redirects: u8,
    /// 单次响应体最大字节；超出 dispatcher 截断并按 `Inconsistent::SchemaInvalid` 失败。
    pub max_body_bytes: u64,
    /// 是否允许 hook 调用方自带 mTLS client cert（admin only）。
    pub mtls: Option<MtlsConfig>,
}

pub struct SsrfGuardPolicy {
    pub deny_loopback: bool,        // 默认 true
    pub deny_link_local: bool,      // 默认 true
    pub deny_private: bool,         // 默认 true（10.x / 172.16.x / 192.168.x / fc00::/7）
    pub deny_metadata_endpoints: bool, // 默认 true（169.254.169.254 / metadata.google.internal 等）
    pub allow_private: Vec<IpNet>,  // 例外白名单（admin only）
}
```

POST JSON 到 `url`，响应同 Exec 格式。

**约束**：

- `Plugin{UserControlled}` 安装的 HTTP hook 必须提供非空 `allowlist`，且 `ssrf_guard` 字段全部 `true`；admin manifest 可放宽（详见 ADR-006 §3）。
- DNS 解析在每次请求时重新进行（防 DNS rebinding）；解析后立即按 `ssrf_guard` 校验目标 IP。
- `auth = Bearer / Hmac` 的密钥**不**进 Journal，仅 `redacted_view` 中可见 hash。

### 3.4 协议版本化

Exec / HTTP transport 的 stdin/HTTP body 协议归 `HookProtocolVersion` 管理。`harness-hook` 与 handler 之间的契约通过这一字段做向前兼容协商。

```rust
#[non_exhaustive]
pub enum HookProtocolVersion {
    V1,
}
```

**协商规则**：

1. dispatcher 永远写出当前最高支持的版本号（如 `"v1"`）。
2. handler 在响应里**必须回填**收到的 `protocol_version`；缺失或不匹配 → dispatcher 视为 `Inconsistent::SchemaInvalid` 并按 `failure_mode` 处理。
3. 新增字段（向后兼容）不升版本号；新增/删除/语义变更（破坏性）→ 升 `V2`，并在 `harness-hook.md` 同步发布升级矩阵。
4. dispatcher 同时支持多版本时，按 manifest 中声明的 `protocol_version` 选择对应序列化格式；未声明默认 `V1`。

> 协议版本变更必须**与 ADR-001 Event Sourcing 的 schema 迁移流程**对齐：
> Replay 重建时，dispatcher 不会再次调用 hook（见 §11），但 Audit 工具读取
> Journal 中历史 `HookTriggered.outcome_summary` 时，必须能识别老版本协议生成的字段。

## 4. Feature Flags

```toml
[features]
default = ["in-process"]
in-process = []
exec = ["dep:tokio"]
http = ["dep:reqwest"]
```

## 5. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("handler timeout: {handler_id}")]
    Timeout { handler_id: HandlerId },

    #[error("handler error: {handler_id}: {cause}")]
    HandlerError { handler_id: HandlerId, cause: String },

    #[error("handler panicked: {handler_id}")]
    Panicked { handler_id: HandlerId, snippet: String },

    #[error("outcome inconsistent: {handler_id}: {reason:?}")]
    Inconsistent { handler_id: HandlerId, reason: InconsistentReason },

    #[error("outcome unsupported: {handler_id}: {kind:?}")]
    Unsupported { handler_id: HandlerId, kind: HookOutcomeDiscriminant },

    #[error("protocol parse: {0}")]
    ProtocolParse(String),

    #[error("transport: {kind:?}: {detail}")]
    Transport { kind: TransportFailureKind, detail: String },

    #[error("unauthorized: {0}")]
    Unauthorized(String),
}

#[non_exhaustive]
pub enum TransportFailureKind {
    SsrfBlocked,
    AllowlistMiss,
    ProtocolVersionMismatch,
    BodyTooLarge,
    NetworkError,
    NonZeroExit { code: i32 },
}
```

> `HookError` 仅用于 dispatcher 内部表达单个 handler 的失败原因；所有失败都会被 dispatcher 转换成 `HookFailureRecord` + `HookFailedEvent`（见 `event-schema.md §3.7`）后**才**根据 `failure_mode` 决定是否影响最终 `DispatchResult`。
> Plugin 作者实现 `HookHandler::handle` 时，**返回 `Err(HookError::HandlerError { .. })` 与 panic 等价**——dispatcher 都会按 `Panicked` 路径记录。

## 6. 使用示例

### 6.1 In-Process Audit Hook

```rust
struct AuditLogHook;

#[async_trait]
impl HookHandler for AuditLogHook {
    fn handler_id(&self) -> &str { "audit-log" }
    fn interested_events(&self) -> &[HookEventKind] {
        &[HookEventKind::PreToolUse]
    }
    async fn handle(&self, event: HookEvent, _ctx: HookContext)
        -> Result<HookOutcome, HookError>
    {
        if let HookEvent::PreToolUse { tool_name, .. } = &event {
            tracing::info!(%tool_name, "tool invoked");
        }
        Ok(HookOutcome::Continue)
    }
}

let registry = HookRegistry::builder()
    .with_hook(Box::new(AuditLogHook))
    .build();
```

### 6.2 Block 危险操作

```rust
struct BlockProdHook;

#[async_trait]
impl HookHandler for BlockProdHook {
    fn handler_id(&self) -> &str { "block-prod" }
    fn interested_events(&self) -> &[HookEventKind] {
        &[HookEventKind::PreToolUse]
    }
    async fn handle(&self, event: HookEvent, _ctx: HookContext)
        -> Result<HookOutcome, HookError>
    {
        if let HookEvent::PreToolUse { input, tool_name, .. } = &event {
            if tool_name == "bash" {
                let cmd = input["command"].as_str().unwrap_or("");
                if cmd.contains("/prod") {
                    return Ok(HookOutcome::Block {
                        reason: "禁止在 /prod 下执行 bash".into()
                    });
                }
            }
        }
        Ok(HookOutcome::Continue)
    }
}
```

### 6.2.1 PreToolUse 三件套（rewrite + override + context）

```rust
struct SafeRmRewriteHook;

#[async_trait]
impl HookHandler for SafeRmRewriteHook {
    fn handler_id(&self) -> &str { "safe-rm-rewrite" }
    fn interested_events(&self) -> &[HookEventKind] {
        &[HookEventKind::PreToolUse]
    }
    fn priority(&self) -> i32 { 100 }

    async fn handle(&self, event: HookEvent, _ctx: HookContext)
        -> Result<HookOutcome, HookError>
    {
        let HookEvent::PreToolUse { input, tool_name, .. } = &event else {
            return Ok(HookOutcome::Continue);
        };
        if tool_name != "bash" { return Ok(HookOutcome::Continue); }
        let cmd = input["command"].as_str().unwrap_or("");
        if !cmd.starts_with("rm -rf ") { return Ok(HookOutcome::Continue); }

        let safer = cmd.replacen("rm -rf", "rm -rfI --one-file-system", 1);
        let new_input = serde_json::json!({ "command": safer });

        Ok(HookOutcome::PreToolUse(PreToolUseOutcome {
            rewrite_input: Some(new_input),
            override_permission: Some(Decision::AllowOnce),
            additional_context: Some(ContextPatch {
                role: ContextPatchRole::UserSuffix,
                content: "[safe-rm-rewrite] 已自动改用 rm -rfI --one-file-system".into(),
                apply_to_next_turn_only: true,
            }),
            block: None,
        }))
    }
}
```

**说明**：

- 单次 hook 同时完成"改命令 + 显式放行 + 告诉模型为何改"，避免传统互斥 enum 必须分两个 hook 实现。
- `override_permission` 走 `DecidedBy::Hook { handler_id: "safe-rm-rewrite" }`，事件 Journal 可追溯。
- `block: None` 与其他三件共存；若该 hook 同时把 `block` 置 Some，dispatcher 会判 `HookOutcomeInconsistent` 并降级 `Continue`。

### 6.3 Exec Hook（Shell 脚本）

```yaml
# ~/.octopus/hooks/my-hook/HOOK.yaml
handler_id: my-hook
interested_events:
  - pre_tool_use
  - post_tool_use
command: /usr/local/bin/my-hook-script
timeout_secs: 30
trust: admin-trusted
```

```bash
#!/usr/bin/env bash
# /usr/local/bin/my-hook-script
input=$(cat)
event_type=$(echo "$input" | jq -r '.event')
case "$event_type" in
    "pre_tool_use") echo '{"outcome":{"continue":null}}' ;;
    *) echo '{"outcome":{"continue":null}}' ;;
esac
```

## 7. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 每个 Event × 每个 Outcome 路径 |
| 串联 | 多 handler 顺序、RewriteInput 传递 |
| Exec | stdin/stdout 协议 + timeout |
| HTTP | 网络失败重试、auth |
| 安全 | User-controlled Plugin 无法注册 Exec Hook |

## 8. 可观测性

| 指标 | 维度 / 说明 |
|---|---|
| `hook_invocations_total` | `{event, handler_id, outcome}`：按事件×handler×outcome 分桶 |
| `hook_duration_ms` | `{event, handler_id, transport}`：每 handler 耗时直方图 |
| `hook_blocks_total` | `{event, handler_id, scope=hard\|fail_closed}`：阻止次数 |
| `hook_rewrites_total` | `{event, handler_id, kind=input\|result\|terminal\|llm_request}` |
| `hook_failures_total` | `{event, handler_id, mode=fail_open\|fail_closed, cause}`：失败发生次数；`cause` 取自 `HookFailureCause` 判别量（`unsupported / inconsistent / panicked / timeout / transport / unauthorized`） |
| `hook_chain_depth` | `{event}`：单次 dispatch 中实际执行的 handler 数（直方图） |
| `hook_context_bytes_added` | `{event}`：单次事件中累加的 `AddContext` 字节数 |
| `hook_permission_conflict_total` | `{event}`：同 priority 多 handler 给出冲突决策的次数（Allow vs Deny） |

> Replay 重建场景下（`ReplayMode::Audit`），dispatcher 仅按 Journal 派生指标，**不**重新触发 in-process handler，因此上述指标在 replay 中只前进不重演。

## 9. 反模式

- 在 Hook 里做长耗时 IO（阻塞 dispatcher 主循环）
- Hook 改写 `TransformToolResult` 之外的工具结果（职责越界）
- 多个 Hook 对 `OverridePermission` 写冲突（应靠 priority + handler_id 字典序仲裁，详见 §2.4.1）
- User-controlled Plugin 尝试安装 Exec Hook（Registry 会拒绝）
- 在 PreToolUse 里返回单一 `HookOutcome::RewriteInput` / `HookOutcome::OverridePermission` / `HookOutcome::AddContext`（dispatcher 视为不允许的 variant，降级 `Continue` 并记 `HookReturnedUnsupported`；正确做法是返回 `HookOutcome::PreToolUse(PreToolUseOutcome { ... })` 三件套形态）
- 在 `PreToolUseOutcome` 里同时设置 `block` 与其他字段（dispatcher 判 `HookOutcomeInconsistent`，降级 `Continue`）
- 想"阻止该工具 + 注入 context 解释为什么被阻"时，**不要**在 PreToolUse 里把 `block` 与 `additional_context` 同时置 Some（违反 §2.4.1 互斥约束）。正确做法：在 `block.reason` 写明拒绝理由（会落到 `HookTriggered.outcome_summary.blocked_reason` 与 `ToolUseDenied { reason: HookBlocked, handler_id }` 上），由 PostToolUseFailure 的另一个 hook 把上下文注入下一轮 prompt

## 10. 反模式（追加）

- Hook 内部缓存 `HookContext` 字段（破坏 replay 幂等，详见 §11）
- 在 `failure_mode = FailOpen` 的安全/合规 hook 上承载强制策略——失败即漏；正确做法是声明 `FailClosed`（admin-trusted 才允许）。
- HTTP hook 不配置 `HookHttpSecurityPolicy.allowlist`（UserControlled 直接被 PluginRegistry 拒绝；admin 写空 allowlist 等同于禁止访问）。
- 在 `PreLlmCall::RewriteInput` 中改动 system prompt 或 tool 定义（违反 ADR-003 Prompt Cache 锁定，dispatcher 判 `Inconsistent::PromptCacheViolation`）。
- 在 Replay 重建过程中尝试再次执行 in-process handler（dispatcher 在 `ReplayMode::Audit` 下不再调用 handler；详见 §11）。

## 11. Replay 与 Event Sourcing 语义（对齐 ADR-001）

Hook 的副作用必须与 Event Sourcing 兼容：**Journal 永远是单一事实来源**，replay 工具基于 Journal 重建状态时不能依赖 hook 再次产生相同输出。

### 11.1 Replay 模型

| 模式 | dispatcher 行为 | handler 调用 |
|---|---|---|
| `ReplayMode::Live`（默认） | 按 §2.6 / §2.6.1 正常分派；写 `HookTriggered` / `HookFailed` / `HookRewroteInput` / `HookContextPatch` 等 | 全部 in-process / Exec / HTTP handler 都被调用 |
| `ReplayMode::Audit`（Journal 重放） | 按 Journal 中已有的 `HookTriggered.outcome_summary` 与配套事件**直接还原** `DispatchResult` | **不**调用任何 handler；in-process handler 的 `HookEmitter` 等副作用 capability 被替换为 NoOp |

### 11.2 幂等契约

每个 `HookHandler` 实现都必须满足下列契约（plugin manifest 上的 `replay_idempotent: true` 默认值）：

1. **纯函数化**：给定相同 `(event, ctx_view)`，必须产生**等价** `HookOutcome`。"等价"指：
   - `Continue` ↔ `Continue`（视为完全相等）
   - `Block { reason }`：reason 文本可不同，但都应导致下游 `ToolUseDenied`
   - `RewriteInput` / `Transform`：输出 hash 必须一致（如果 hook 行为依赖时间或随机数，必须从 `ctx.at` / `ctx.run_id` 派生）
   - `PreToolUseOutcome { override_permission }`：决策 variant（Allow*/Deny*）一致；`AllowOnce` ↔ `AllowSession` 视为不等价
2. **不持久化外部副作用**：handler 内不允许直接写文件/调外部 API；如需通知外部，**必须**走 `HookEmitter` capability 把意图作为 `Event` 落 Journal，由消费者根据事件流外部下沉。
3. **不读取易变全局态**：handler 必须只读 `HookContext`、`event` payload 与依赖注入的不可变配置；不允许直接读 `Session`/`Journal`/全局缓存。

违反契约的后果：

- `RewriteInput` / `Transform` 的输出在 replay 时与 Journal 中 `HookRewroteInput.before_hash → after_hash` 不一致 → `harness-engine` 写 `Event::EngineFailed { kind: ReplayDivergence { handler_id } }` 并把会话标记 `Status::ReplayCorrupted`。
- 用户态 plugin 把 `replay_idempotent: false` 写进 manifest 时，PluginRegistry 直接拒绝（详见 ADR-006）。

### 11.3 Audit 模式下不可见的 hook 行为

下列行为在 `Live` 模式下产生效果，但在 `Audit` 模式下**完全不出现**——因为它们没有进入 Journal：

- handler 内部对外部系统的请求（无论是直接 reqwest 还是 std::process）。
- handler 内部对 OS 信号 / 环境变量 / 文件系统的副作用。
- handler 内部 `tracing` 日志（除非显式接入 `harness-observability` 的 Journal 桥）。

**业务方的应对**：所有需要在 replay 中可见的副作用，必须经 `HookEmitter` 转换为 `Event` 落 Journal。这一约束与 `harness-tool` 的 `ToolCapability::HookEmitter` 设计一致。

### 11.4 与 ADR-003 Prompt Cache 的协同

`PreLlmCall::RewriteInput` 是唯一允许改写发往 LLM 的请求的钩子；它必须遵守 Prompt Cache 锁定字段白名单。Replay 时 dispatcher 不重新生成 ModelRequest，而是直接读 `ToolUseRequested` / `AssistantMessageCompleted` 的 Journal 视图——因此 `PreLlmCall` 的非幂等改写**会**导致 replay 与原 run 的 LLM tool_use 链路对不上，触发上面提到的 `ReplayDivergence`。

## 12. 相关

- D4 · `event-schema.md §3.7`（HookTriggered / HookFailed / HookRewroteInput / HookContextPatch / HookOutcomeInconsistent / HookReturnedUnsupported / HookPanicked / HookPermissionConflict 结构）
- D7 · `extensibility.md §5`（Hook 扩展面向用户视角；与本文件 §2.4 能力矩阵保持一致）
- D2 · `permission-model.md §6 / §9`（OverridePermission 与审批协同）
- ADR-001 Event Sourcing（Replay 语义来源）
- ADR-003 Prompt Cache 锁定字段（`PreLlmCall::RewriteInput` 白名单）
- ADR-006 Plugin Trust Levels（Exec/HTTP transport 信任约束）
- ADR-009 Deferred Tool Loading（`PreToolSearch` / `PostToolSearchMaterialize` 来源）
- ADR-010 Tool Result Budget（`TransformToolResult` / `PostToolUse` 看到的是 budget 处理后视图）
- Evidence: HER-034, HER-036, CC-22, CC-23, CC-25, OC-29
