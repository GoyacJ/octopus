# `octopus-harness-engine` · L3 · Agent Engine SPEC

> 层级：L3 · 状态：Accepted
> 依赖：`harness-contracts` + 全部 L1 + 全部 L2

## 1. 职责

实现 **单 Agent 主循环**：turn 编排、中断、预算、grace call、状态机。对齐 HER-004 / CC-06。

**核心能力**：

- Agent Loop（model infer → tool calls → context merge → loop）
- `max_iterations` + `iteration_budget`
- Grace call（预算接近时最后一次 assistant 响应）
- `_interrupt_requested` 线程安全中断（对齐 HER-013）
- 并行 Tool 分派（对齐 CC-07）
- Subagent 触发点
- Hook 拦截点

## 2. 对外 API

### 2.1 Engine

```rust
pub struct Engine {
    model: Arc<dyn ModelProvider>,
    store: Arc<dyn EventStore>,
    tools: ToolRegistrySnapshot,
    permission_broker: Arc<dyn PermissionBroker>,
    hook_registry: HookRegistrySnapshot,
    context_engine: Arc<ContextEngine>,
    sandbox: Arc<dyn SandboxBackend>,
    mcp_registry: Arc<McpRegistry>,
    memory: Arc<MemoryManager>,
    observability: Arc<Observer>,
    /// Capability 注入点（ADR-011）。Engine 在装配期填充全部内置 capability，
    /// `ToolOrchestrator` 在分派每个 ToolCall 时把 `Arc<CapabilityRegistry>` 注入 `ToolContext`。
    cap_registry: Arc<CapabilityRegistry>,
    blob_store: Arc<dyn BlobStore>,
}

impl Engine {
    pub fn builder() -> EngineBuilder;
    pub async fn run(
        &self,
        spec: RunSpec,
    ) -> Result<EventStream, EngineError>;
}

pub struct RunSpec {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub parent_run_id: Option<RunId>,
    pub input: TurnInput,
    pub max_iterations: u32,
    pub token_budget: TokenBudget,
    pub permission_mode: PermissionMode,
    pub interrupt_token: InterruptToken,
    /// 仅当本 Run 需要**覆写** `SessionOptions::permission_defaults` 时设置；
    /// 例：Subagent / Cron Driver 强制 `NoInteractive + DenyAll`。
    /// 覆写策略 = "整体替换"，**不**做字段级合并；详见 §6.3。
    pub permission_overrides: Option<PermissionDefaults>,
}

impl RunSpec {
    /// `Engine` 在 `dispatch` 前调用：优先用 `permission_overrides`，否则回落 SessionOptions 默认。
    pub fn resolved_permission_defaults(&self) -> PermissionDefaults { /* ... */ }
}
```

### 2.2 EngineRunner Trait（对 subagent/team 暴露）

```rust
#[async_trait]
pub trait EngineRunner: Send + Sync + 'static {
    async fn run(
        &self,
        spec: RunSpec,
    ) -> Result<EventStream, EngineError>;
}
```

`harness-subagent` 与 `harness-team` 通过此 trait 依赖 Engine，不直接 `use harness_engine::Engine`（避免循环依赖）。

### 2.3 主循环状态机

```rust
enum LoopState {
    AwaitingModel,
    ProcessingToolUses { pending: Vec<ToolCall> },
    ApplyingHookResults,
    MergingContext,
    Ended(StopReason),
}
```

## 3. Agent Loop 主流程

```text
run(spec)
    │
    ▼
[记 Event::RunStarted]
    │
    ▼
[Hook::UserPromptSubmit] (可 RewriteInput / Block)
    │
    ▼
[context_engine.assemble] → AssembledPrompt
    │
    ▼
─── Iteration Loop ──────────────────────────────
    │
    ▼
[检查 iteration_budget / interrupt_token]
    │
    ▼
[steering.drain_and_merge()] (ADR-0017 §2.5)
    │  ├─ visible_at <= now 且未过 ttl 的消息进 drain 集合
    │  ├─ 合并语义：
    │  │    Append      → 顺序拼接到下一轮 user 消息尾部
    │  │    Replace     → 仅保留最新一条 Replace（同时清掉之前未 drain 的 Append）
    │  │    NudgeOnly   → 不进 prompt；仅 emit `Event::SteeringMessageApplied`
    │  ├─ Event::SteeringMessageApplied { ids, merged_into_message_id? }
    │  ├─ ttl 已过 / dedup 命中 → Event::SteeringMessageDropped { id, reason }
    │  └─ 队列为空 → 0 操作 0 事件
    │
    ▼
[Hook::UserPromptSubmit on synthesized user_message]
    │  仅当本轮 drain_and_merge 产生了新的 user 消息时触发；
    │  Hook 链尾仍按 ADR-0003 守住 prompt cache 锁定字段。
    │
    ▼
[model.infer(prompt)] → stream events
    │    │
    │    ├─ ContentBlockDelta(Text) → Event::AssistantDeltaProduced
    │    ├─ ContentBlockDelta(ToolUse) → 累积到本轮 tool_calls
    │    └─ MessageStop → 退出 stream
    │
    ▼
[Event::AssistantMessageCompleted]
    │
    ▼
if 本轮无 tool_calls → [StopReason::AssistantFinished] 退出
    │
    ▼
[orchestrator.dispatch(tool_calls, ctx)]
    │    │
    │    ├─ concurrency_safe → 并行（max 10）
    │    └─ 其他 → 串行
    │
    ▼
[针对每个 tool_call]:
    [Hook::PreToolUse] (可 Continue / Block / PreToolUse(PreToolUseOutcome { rewrite_input?, override_permission?, additional_context? }))
        │
        ▼
    [tool.check_permission] → PermissionCheck
        │
        ▼
    [permission_broker.decide] (如需) → Decision
        │
        ▼
    [Event::ToolUseApproved/Denied]
        │
        ▼
    [tool.execute] → ToolStream                                        ← 流式（harness-tool §2.1）
        │     │  Shell 类工具内部展开为：
        │     │  ├─ sandbox.before_execute → execute → wait → after_execute
        │     │  └─ 详见 harness-tool §2.7.1（含失败原子性 / CWD marker 通道 / Hook 边界）
        │     ├─ Progress             → ToolUseHeartbeat（按 stall_threshold 自动注入）
        │     ├─ Partial(bytes)       → [Hook::TransformTerminalOutput]（仅 Bash/Subagent）
        │     └─ Final(result) | Error
        │
        ▼
    [Orchestrator::collect_with_budget] → ToolResultEnvelope          ← ADR-010
        │     └─ 命中预算 → BlobStore::put_streaming + Event::ToolResultOffloaded
        │
        ▼
    [Hook::TransformToolResult] → 可改写 envelope.result
        │
        ▼
    [Hook::PostToolUse] / [Hook::PostToolUseFailure]
        │
        ▼
    [Event::ToolUseCompleted/Failed]
    │
    ▼
[context_engine.after_turn(results)] → Context 注入 + budget 检查
    │
    ▼
循环 → AwaitingModel（带新 context）
─── End Loop ─────────────────────────────────────
    │
    ▼
[Event::RunEnded(reason, usage)]
    │
    ▼
EventStream 关闭
```

## 4. 预算控制

### 4.1 Iteration Budget

```rust
pub struct IterationBudget {
    pub max_iterations: u32,
    pub grace_enabled: bool,
}
```

- 每轮递增计数
- 达到 `max_iterations - 1` 触发 Grace Call（最后一次让 Assistant 说完而不发新 tool）
- 达到 `max_iterations` 强制结束，`Event::RunEnded { reason: MaxIterationsReached }`

### 4.2 Token Budget（对齐 D8 §6）

由 `ContextEngine` 驱动：

- 超 `soft_budget_ratio` → 启动 compact pipeline
- 超 `hard_budget_ratio` → autocompact + fork
- 超 `max_tokens_per_turn` → `StopReason::TokenBudgetExceeded`

## 5. 中断

```rust
pub struct InterruptToken {
    flag: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}

impl InterruptToken {
    pub fn trigger(&self);
    pub async fn wait(&self);
    pub fn is_triggered(&self) -> bool;
}
```

Engine 在**每个 safe point** 检查：

- 每次 model stream 的 delta 后
- 每次 tool dispatch 前
- 每次 hook dispatch 前

触发时：

- 正在执行的 tool 收到 `ToolError::Interrupted`
- 正在请求的 model 请求被 abort（`reqwest::Request::abort`）
- 写 `Event::RunEnded { reason: Interrupted }`

## 6. Orchestrator 分派

使用 `harness-tool::ToolOrchestrator`：

```rust
let results = orchestrator.dispatch(
    tool_calls,
    OrchestratorContext {
        tenant_id,
        session_id,
        run_id,
        sandbox: sandbox.clone(),
        permission_broker: permission_broker.clone(),
        hook_registry: hook_registry.clone(),
        cap_registry: cap_registry.clone(),         // ADR-011
        blob_store: blob_store.clone(),             // ADR-010
        observer: observer.clone(),
        permission_defaults: spec.resolved_permission_defaults(), // §6.3
    },
).await;
```

对齐 CC-07：`is_concurrency_safe` Tool 并行（max 10），其他串行。Orchestrator 负责调用 `tool.execute` 拿到 `ToolStream`、按 `ResultBudget` 流式收集、串接 5 个 Hook 介入点（详见 `harness-hook.md §2.7`）。

### 6.2 CapabilityRegistry 装配（ADR-011 §2.6）

Engine 在 `EngineBuilder::build` 阶段构造 `CapabilityRegistry`，把内置高权限子系统按 capability 命名空间挂入：

```rust
fn build_cap_registry(
    subagent_runner: Arc<dyn SubagentRunner>,
    todo_store: Arc<TodoStore>,
    run_canceller: Arc<RunCanceller>,
    clarify_channel: Arc<dyn ClarifyChannelImpl>,
    user_messenger: Arc<dyn UserMessengerImpl>,
    blob_store: Arc<dyn BlobStore>,
    skill_registry: Arc<SkillRegistry>,
) -> Arc<CapabilityRegistry> {
    let mut caps = CapabilityRegistry::default();

    caps.install::<dyn SubagentRunnerCap>(
        ToolCapability::SubagentRunner,
        SubagentRunnerCapAdapter::from_runner(subagent_runner),
    );
    caps.install::<dyn TodoStoreCap>(
        ToolCapability::TodoStore,
        TodoStoreCapAdapter::from_store(todo_store),
    );
    caps.install::<dyn RunCancellerCap>(
        ToolCapability::RunCanceller,
        RunCancellerCapAdapter::from_canceller(run_canceller),
    );
    caps.install::<dyn ClarifyChannelCap>(
        ToolCapability::ClarifyChannel,
        clarify_channel,
    );
    caps.install::<dyn UserMessengerCap>(
        ToolCapability::UserMessenger,
        user_messenger,
    );
    caps.install::<dyn BlobReaderCap>(
        ToolCapability::BlobReader,
        BlobReaderCapAdapter::from_store(blob_store),
    );
    caps.install::<dyn SkillRegistryCap>(
        ToolCapability::SkillRegistry,
        skill_registry,
    );

    Arc::new(caps)
}
```

**装配顺序约束**：

1. `CapabilityRegistry` 必须在 `ToolRegistry::register` 之前完成，否则注册阶段的 `CapabilityPolicy::check` 找不到对应实现 → `RegistrationError::CapabilityMissing`；
2. 装配后 `cap_registry` 视为不可变（与 `ToolRegistrySnapshot` 一样，参见 ADR-003 prompt cache 锁定）；
3. 任何运行期注入新 capability 的尝试被拒绝（避免破坏 prompt cache）；
4. Plugin / MCP / Skill 来源的工具**不能注入新 capability**——它们只能从已有 capability 集合中按 trust 矩阵借用。

**测试支持**：`harness-contracts/testing` feature gate 暴露 `MockCapabilityRegistry`，业务/集成测试只需注入需要用到的 capability，不必拼整套依赖（参见 ADR-011 §2.7）。

### 6.3 `PermissionContext` 装配责任（对齐 ADR-007 / `permission-model.md §3.2`）

`PermissionContext` 的三个交互/容错字段——`interactivity` / `fallback_policy` / `timeout_policy`——是 Broker 决策的输入而非默认值；**Broker 不得猜测**，必须由调用方按上下文显式设置。Engine 是唯一调 `broker.decide()` 的层（`harness-permission §2.2.1`），因此本节集中描述各 caller 在装配阶段如何向 Engine 注入这三项。

| 调用方 | `interactivity` | `fallback_policy` | `timeout_policy` | 注入路径 |
|---|---|---|---|---|
| **CLI / Desktop / Web 前台** | `FullyInteractive` | `AskUser` | `None`（沿用 Broker 默认 5min） | `SessionOptions::permission_defaults`（由 SDK 装配 Session 时填） |
| **Subagent**（默认） | `DeferredInteractive` | `AskUser` | `Some(deadline = 30s, default_on_timeout = DenyOnce)` | `SubagentSpec::interactivity`（`harness-subagent §6.2`），由 `SubagentRunner` 在 `EngineRunner::run` 调用前注入到 `RunSpec.permission_overrides` |
| **Subagent（受信任脚本式）** | `NoInteractive` | `AllowReadOnly` 或 `DenyAll` | `Some(deadline = 5s, default_on_timeout = DenyOnce)` | 同上；通过 `SubagentSpec::interactivity = NoInteractive` 显式声明 |
| **Cron / Background Worker** | `NoInteractive` | `DenyAll` | `Some(deadline = 1s, default_on_timeout = DenyOnce)` | `RunSpec::permission_overrides` 由 Cron Driver 装配；如无 override 则从 `SessionOptions::permission_defaults` 取，缺省时 fail-closed 拒绝启动 |
| **Gateway 多平台 / IM Bridge** | `DeferredInteractive` | `AskUser` | `Some(deadline = 5min, heartbeat = 30s)` | Gateway 在 `Session::run_turn` 之前 set `SessionOptions::permission_defaults`，确保 `StreamBasedBroker` 的请求挂入 Bridge 队列 |
| **Replay / Audit Tool**（只读重放） | `NoInteractive` | `DenyAll` | 不适用（不会写新决策） | Replay 不写新事件，但仍需注入避免 `Decision::Escalate` 链尾活锁 |

**装配链路**：

```text
[业务装配]
    │  CLI / Desktop / Cron / Gateway 各自决定
    ▼
SessionOptions { permission_defaults }
    │
    ▼
Session::run_turn(input)
    │  Engine 取 session.options.permission_defaults
    ▼
RunSpec { permission_mode, permission_overrides? }
    │  permission_overrides 优先于 SessionOptions.permission_defaults
    ▼
ToolOrchestrator::dispatch(...)
    │  装配 PermissionContext { interactivity, fallback_policy, timeout_policy }
    ▼
permission_broker.decide(req, ctx)
```

**`OrchestratorContext` 的最终形态**（在 §6 基础上扩展）：

```rust
pub struct OrchestratorContext {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub run_id: RunId,
    pub sandbox: Arc<dyn SandboxBackend>,
    pub permission_broker: Arc<dyn PermissionBroker>,
    pub hook_registry: HookRegistrySnapshot,
    pub cap_registry: Arc<CapabilityRegistry>,         // ADR-011
    pub blob_store: Arc<dyn BlobStore>,                // ADR-010
    pub observer: Arc<Observer>,
    /// 决定 `decide()` 时 `PermissionContext` 的三件套；
    /// **不可省略**——空值会触发 fail-closed 报错（详见下文装配检查）。
    pub permission_defaults: PermissionDefaults,
}

pub struct PermissionDefaults {
    pub interactivity: InteractivityLevel,
    pub fallback_policy: FallbackPolicy,
    pub timeout_policy: Option<TimeoutPolicy>,
}
```

**装配检查（fail-closed）**：

1. `EngineBuilder::build` 会校验 `PermissionDefaults` 与 `permission_broker` 是否兼容：
   - `interactivity == FullyInteractive` 且链中没有 `StreamBasedBroker` / `DirectBroker` → `EngineError::InteractivityNotServiceable`；
   - `interactivity == NoInteractive` 且 `fallback_policy == AskUser` → 自动降级为 `DenyAll`，并打 `tracing::warn`；不阻塞启动。
2. Subagent 的 `RunSpec.permission_overrides` 必须被 Engine 当成"覆写"而不是"合并"——避免父 Session 的 `FullyInteractive` 漏到子代理触发"无人响应的弹窗"（详见 `permission-model.md §3.2` 反模式）。
3. Cron / 后台 worker 不允许 `interactivity == FullyInteractive`；`EngineBuilder::build` 接收到该组合时立即返回 `EngineError::BackgroundContextNotInteractive`。

> 只读重放（`harness-journal::ReplayContext`）执行 `Engine::replay` 时不会触达 broker，但仍按 `NoInteractive + DenyAll` 注入 `PermissionDefaults`，保证旁路装配代码与正常路径一致——避免"replay 路径走另一套兜底"导致的语义漂移。

## 6.1 Skill 预取（可选优化，对齐 CC-26）

Agent Loop 在以下 **idle 窗口** 可以并行做 Skill discovery / 预取，隐藏加载延迟：

- **Model 流式推理期间**（等 delta 返回的空闲）
- **Tool 执行等待期间**

```rust
pub struct SkillPrefetcher {
    loader: Arc<SkillLoader>,
    registry: Arc<SkillRegistry>,
    strategy: SkillPrefetchStrategy,
}

pub enum SkillPrefetchStrategy {
    Disabled,
    Eager,                                  // Session 创建期全加载（default）
    LazyPerTurn { concurrency: usize },     // 每轮 idle 时预取可能用到的 skill
    HintDriven,                             // 由业务 Hook 提供 prefetch 列表
}
```

**默认行为**：`Eager`（与 ADR-003 Prompt Cache 硬约束一致；Session 创建期就把 skill registry snapshot 锁定，避免中途变化）。

**LazyPerTurn / HintDriven** 仅在特殊场景（Skill 数量 > 100、每次只用到少数几个）启用：
- 命中 Skill 不会立即注入 Context（以保 cache）
- Skill 内容 **只加载进 registry**，需要业务通过 `reload_with` 触发才进入 Session
- 因此该策略主要优化"加载延迟"而非"注入延迟"

**与 Prompt Cache 硬约束一致性**：
- 任何 Skill discovery 结果变动**不**自动改 `ContextEngine` 的 `AssembledPrompt`
- 需要业务层显式 `session.reload_with(ConfigDelta { add_skills: vec![...] })` → 按 ADR-003 §2.3 分类处理

### 6.1.1 与 `SkillsInvokeTool` 三件套的关系

`SkillPrefetcher` 解决的是**加载侧**问题（"何时把 skill 文件读进 Registry"），而 `SkillsInvokeTool` 三件套（详见 `harness-skill.md §6`）解决的是**消费侧**问题（"何时把 skill 内容注入 user message"）。两者在 idle 窗口协同：

| 维度 | `SkillPrefetcher` | `SkillsInvokeTool` 三件套 |
|---|---|---|
| 操作对象 | `SkillRegistry`（加载状态） | `ContextBuffer`（消费状态） |
| 触发方 | Engine idle scheduler | LLM 主动调工具 |
| 默认策略 | `Eager`（创建期全加载） | `AlwaysLoad`（`skills_list`）+ `AutoDefer`（`skills_view` / `skills_invoke`） |
| 影响 Cache | **不影响**（只动 Registry，不动 Prompt） | **不影响**（注入到 user message 活跃面，不碰 system） |

**典型组合**：

- 小规模（< 20 skill）：`SkillPrefetcher::Eager` + 不启用 `SkillsInvokeTool`，业务直接用 `Skill::render` 注入；
- 中等规模（20–100）：`SkillPrefetcher::Eager` + 启用 `SkillsInvokeTool`，模型按需调用，多余 skill 不进系统面；
- 大规模（> 100）：`SkillPrefetcher::LazyPerTurn` + `SkillsInvokeTool` + `tool_search`（ADR-009），把"加载"与"消费"两端都按需化。

## 7. Subagent 触发

Engine 遇到 `AgentTool::execute` 时，调用 `SubagentRunner`（由 `harness-subagent` 提供）：

```rust
impl Engine {
    async fn handle_agent_tool(&self, call: ToolCall, ctx: EngineContext) -> ToolResult {
        let runner = self.subagent_runner.as_ref()
            .ok_or(EngineError::SubagentDisabled)?;
        let handle = runner.spawn(/* ... */).await?;
        // 等待子 agent 结束，拿到 SubagentAnnouncement
        let announcement = handle.wait().await?;
        ToolResult::from(announcement)
    }
}
```

## 8. Feature Flags

```toml
[features]
default = ["parallel-tools"]
parallel-tools = []
subagent-tool = ["dep:octopus-harness-subagent"]
```

## 9. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("model: {0}")]
    Model(#[from] ModelError),

    #[error("tool dispatch: {0}")]
    ToolDispatch(String),

    #[error("max iterations reached: {0}")]
    MaxIterations(u32),

    #[error("token budget exceeded")]
    TokenBudgetExceeded,

    #[error("interrupted")]
    Interrupted,

    #[error("context: {0}")]
    Context(#[from] ContextError),

    #[error("subagent disabled")]
    SubagentDisabled,

    #[error("internal: {0}")]
    Internal(String),
}
```

## 10. 使用示例

```rust
let engine = Engine::builder()
    .with_model(model_provider.clone())
    .with_store(event_store.clone())
    .with_tools(tool_snapshot)
    .with_permission_broker(broker)
    .with_hooks(hook_snapshot)
    .with_context_engine(context_engine)
    .with_sandbox(sandbox)
    .with_mcp_registry(mcp_registry)
    .with_memory(memory_mgr)
    .with_observability(observer)
    .build()?;

let events = engine.run(RunSpec {
    tenant_id: TenantId::SINGLE,
    session_id,
    parent_run_id: None,
    input: TurnInput::user("review auth.rs"),
    max_iterations: 25,
    token_budget: TokenBudget::default(),
    permission_mode: PermissionMode::Default,
    interrupt_token: InterruptToken::new(),
}).await?;
```

## 11. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | LoopState 转移、grace call 触发 |
| Mock | MockModelProvider 驱动多轮 tool 调用 |
| 中断 | 每个 safe point 触发中断 |
| Budget | iteration / token 边界触发 |
| Subagent | AgentTool 路径 |
| Golden | 给定 Input + MockProvider 响应序列，Event 序列 deterministic |

## 12. 可观测性

| 指标 | 说明 |
|---|---|
| `engine_turn_iterations` | 每 turn 迭代数分布 |
| `engine_turn_duration_ms` | 端到端耗时 |
| `engine_grace_calls_total` | Grace call 触发次数 |
| `engine_tool_dispatch_parallelism` | 平均并行度 |
| `engine_interrupts_handled_total` | |

## 13. 反模式

- Engine 自己处理 UI（违反 P1 内核纯净）
- 不检查 interrupt_token（长任务无法中断）
- 同步 await 阻塞 event stream 发送（应用 `tokio::spawn`）
- Tool dispatch 前不调 `check_permission`（绕过审批）

## 14. 相关

- D1 · `overview.md` §7 核心流程
- D6 · `agents-design.md`
- ADR-003 Prompt Cache
- ADR-007 权限事件化
- Evidence: HER-004, HER-013, CC-06, CC-07
