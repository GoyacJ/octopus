# `octopus-harness-context` · L2 复合能力 · Context Engine SPEC

> 层级：L2 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-memory` + `harness-journal`（读）

## 1. 职责

实现 **上下文工程**：Prompt 组装、Compact 管线、Prompt Cache 策略、Token Budget、Bootstrap 文件注入。

**核心能力**：

- Prompt 组装（system header / tools / memory / bootstrap / history）
- Compact 管线（五阶段固定序，对齐 CC-06 / HER-026）
- Token Budget 控制
- Prompt Cache 断点插入（HER-027）
- Workspace Bootstrap 文件注入（OC-07）

## 2. 对外 API

### 2.1 ContextEngine

```rust
pub struct ContextEngine {
    providers: Vec<Arc<dyn ContextProvider>>,
    budget: TokenBudget,
    cache_policy: PromptCachePolicy,
    /// 辅助 LLM。类型为 `AuxModelProvider`（不是 `ModelProvider`），以便
    /// 共享 `AuxOptions::{max_concurrency, per_task_timeout, fail_open}`。
    /// 详见 `crates/harness-model.md` §5.1。
    aux_provider: Option<Arc<dyn AuxModelProvider>>,
    sanitizer: Arc<ContextSanitizer>,
    memory_manager: Arc<MemoryManager>,
    bootstrap: Option<WorkspaceBootstrap>,
}

impl ContextEngine {
    pub fn builder() -> ContextEngineBuilder;

    pub async fn assemble(
        &self,
        session: &SessionState,
        turn_input: &TurnInput,
    ) -> Result<AssembledPrompt, ContextError>;

    pub async fn after_turn(
        &self,
        session: &SessionState,
        results: &[ToolResultEnvelope],
    ) -> Result<ContextOutcome, ContextError>;
}
```

### 2.2 ContextProvider

```rust
#[async_trait]
pub trait ContextProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    fn stage(&self) -> ContextStageId;
    async fn apply(
        &self,
        ctx: &mut ContextBuffer,
        hint: &CompactHint,
    ) -> Result<ContextOutcome, ContextError>;
}

/// Compact 五阶段的身份标识。
///
/// 与 `api-contracts.md §11.1` 的 trait `ContextStage` 区分：
/// - **trait `ContextStage`** = 阶段执行接口（带 `apply()`，每阶段一个实现者）
/// - **enum `ContextStageId`** = 阶段身份（用于 provider 声明归属、事件落库、可观测性分桶）
///
/// 历史命名为 enum `ContextStage`，与 trait 同名构成硬冲突，本字段已统一为 `ContextStageId`。
pub enum ContextStageId {
    ToolResultBudget,
    Snip,
    Microcompact,
    Collapse,
    Autocompact,
}

pub enum ContextOutcome {
    NoChange,
    Modified { bytes_saved: u64 },
    Forked { new_session_id: SessionId },
}
```

### 2.3 Prompt Cache Policy

```rust
pub struct PromptCachePolicy {
    pub style: PromptCacheStyle,
    pub max_breakpoints: usize,
    pub breakpoint_strategy: BreakpointStrategy,
}

pub enum BreakpointStrategy {
    SystemAnd3,   // Anthropic 默认
    SystemOnly,
    EveryN(usize),
    Custom(Arc<dyn BreakpointSelector>),
}
```

### 2.4 Token Budget

```rust
pub struct TokenBudget {
    pub max_tokens_per_turn: u64,
    pub max_tokens_per_session: u64,
    pub soft_budget_ratio: f32,
    pub hard_budget_ratio: f32,
    pub per_tool_max_chars: u64,
}
```

### 2.5 Assembled Prompt

```rust
pub struct AssembledPrompt {
    pub messages: Vec<Message>,
    pub system: Option<String>,
    pub tools_snapshot: Vec<ToolDescriptor>,
    pub cache_breakpoints: Vec<CacheBreakpoint>,
    pub tokens_estimate: u64,
    pub budget_utilization: f32,
}
```

### 2.6 Memory 生命周期桥接视图（v1.4）

`harness-memory` 在 v1.4 引入 `MemoryLifecycle` 后，`ContextEngine` 负责把当前 turn 的只读上下文借用给 memory provider。为避免拷贝整段 transcript，这里定义借用视图类型：

```rust
pub struct UserMessageView<'a> {
    pub text: &'a str,
    pub turn: u32,
    pub at: DateTime<Utc>,
}

pub struct MessageView<'a> {
    pub role: MessageRole,
    pub text_snippet: &'a str,
    pub tool_use_id: Option<ToolUseId>,
}

pub struct SessionSummaryView<'a> {
    pub end_reason: EndReason,
    pub turn_count: u32,
    pub tool_use_count: u32,
    pub usage: UsageSnapshot,
    pub final_assistant_text: Option<&'a str>,
}

pub struct MemorySessionCtx<'a> {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub workspace_id: Option<WorkspaceId>,
    pub user_id: Option<&'a str>,
    pub team_id: Option<TeamId>,
}
```

ContextEngine 与 `MemoryManager` 的调用顺序约束：

1. `assemble` 起始阶段：`memory.on_turn_start(turn, &UserMessageView)`；
2. recall 决策为 yes：调用 `memory.recall(query)`，并把结果包装为 `<memory-context>`；
3. compact 前：调用 `memory.on_pre_compress(&[MessageView])`，若返回 `Some(extra)` 则拼入摘要 prompt；
4. Session 结束：调用 `memory.on_session_end(&MemorySessionCtx, &SessionSummaryView)`。

这些调用都必须是 **best-effort**：默认 fail-open（provider 失败不阻塞主对话），并落 `MemoryRecallDegraded` 或 `MemoryThreatDetected` 事件供观测。

### 2.7 ContextBuffer（Compact 管线的核心借用对象）

> `ContextProvider::apply` 与 `ContextStage::apply`（`api-contracts.md §11.1` 的 trait）都以 `&mut ContextBuffer` 为唯一可变借用对象。本节给出其字段定义与可变性边界，作为五阶段 Compact 的契约基线。

#### 2.7.1 顶层结构（冻结面 / 活跃面 / 补丁 / 记账）

```rust
pub struct ContextBuffer {
    /// 冻结面：Session 创建期锁定（ADR-003 §2.1）。
    /// Compact 管线的任何阶段 **不得** 修改本字段；试图修改者必须返回
    /// `ContextError::Provider("frozen face violation")` 并中止该阶段。
    pub frozen: FrozenContext,

    /// 活跃面：Compact 管线可修改；删除消息时必须以 ToolUsePair 为最小边界（§2.7.3）。
    pub active: ActiveContext,

    /// 本轮活跃补丁：仅注入到 user message 头部，不进 system 段（context-engineering.md §11）。
    /// `Transient` patch 在下一轮 `assemble` 起始的 sanitize 阶段被剥离；
    /// `Persistent` patch 在 compact 时优先级低于历史消息（context-engineering.md §11.4）。
    pub patches: Vec<ContextPatch>,

    /// MCP `tools/list_changed` 攒批后的增量摘要；以 `<system-reminder>` 拼到消息尾部。
    /// 不破坏 cache（context-engineering.md §9.1 / ADR-009）。
    pub deferred_tools_delta: Option<DeferredToolsDelta>,

    /// 记账域：不参与 prompt 体本身，仅供 budget / metrics / replay 使用。
    pub bookkeeping: ContextBookkeeping,
}
```

#### 2.7.2 冻结面：`FrozenContext`

```rust
pub struct FrozenContext {
    /// 已合成的 system header 字面值（产品身份 / 政策 / skills 入口名单）。
    pub system_header: Arc<str>,
    /// 工具池快照（ADR-003 §2.4）；分区 1/2/3 详见 ADR-009。
    pub tools_snapshot: Arc<ToolRegistrySnapshot>,
    /// Memdir 装配快照；运行期不重读（harness-memory.md §3.5）。
    pub memory_snapshot: Arc<MemdirSnapshot>,
    /// 已加载的 Bootstrap 文件展开内容（含截断标记）。
    pub bootstrap_snapshot: Arc<BootstrapSegment>,
}

pub struct BootstrapSegment {
    pub files: Arc<[BootstrapFileLoaded]>,
}

pub struct BootstrapFileLoaded {
    pub spec: BootstrapFileSpec,
    pub content: Arc<str>,
    /// 命中 `BootstrapFileSpec::max_chars` 时为 true；尾部已附 `[truncated]`。
    pub truncated: bool,
}
```

冻结面对应 §2 全景图步骤 1–4（System Header / Tools / Memory / Bootstrap），其字节序列在整个 Session 内**逐字节稳定**，是 Anthropic `system_and_3` 第一个 cache breakpoint 的物理基础（ADR-003 §6.1）。

#### 2.7.3 活跃面：`ActiveContext` 与 `ToolUsePair`

```rust
pub struct ActiveContext {
    /// User / Assistant / Tool 三类消息的 append-only 序列；
    /// Compact 管线可删除（snip）/ 摘要替换（microcompact, autocompact）/ 合并（collapse）。
    pub history: Vec<Message>,

    /// tool_use ↔ tool_result 配对索引。维护此索引是为了让 snip / microcompact
    /// 阶段在丢弃消息时能以 **pair 为最小边界**：
    /// - 若 `tool_result_message_id == None`（即 tool 仍在执行 / 失败未上报），
    ///   则对应的 `tool_use_message_id` 也不得被删；
    /// - 若两端齐全，必须**同进同出**（要么都保留，要么都删；要么都摘要）。
    /// 违反此约束会导致 Anthropic API `tool_use_id` 引用悬空 → 400 BadRequest。
    pub tool_use_pairs: Vec<ToolUsePair>,
}

pub struct ToolUsePair {
    pub tool_use_id: ToolUseId,
    pub tool_use_message_id: MessageId,
    pub tool_result_message_id: Option<MessageId>,
}
```

#### 2.7.4 活跃补丁：`ContextPatch`

补丁是 §2 全景图步骤 6 的统一抽象，覆盖 `<memory-context>` / `<skill-injection>` / `<hook-add-context>` 三类内容（context-engineering.md §11.2 内部次序固定）：

```rust
pub enum ContextPatch {
    MemoryRecall {
        fence: String,                // 已经过 escape_for_fence + wrap_memory_context
        lifecycle: ContentLifecycle,  // 默认 Transient（每轮 sanitize 重做）
    },
    SkillInjection {
        skill_id: SkillId,
        body: String,                 // SkillRenderer::render 产物
        lifecycle: ContentLifecycle,
    },
    HookAddContext {
        handler_id: HandlerId,
        body: String,
        lifecycle: ContentLifecycle,
    },
}
```

`ContentLifecycle` 的定义在 `context-engineering.md §11.3`（`Transient` / `Persistent { ttl_turns }`）；本节只引用，不重复定义。

#### 2.7.5 记账域：`ContextBookkeeping`

```rust
pub struct ContextBookkeeping {
    /// `ContentPresence::Offloaded` 引用：消息体里只剩占位字符串，
    /// 真实内容指向 `BlobStore`（ADR-010）。
    pub offloads: HashMap<MessageId, BlobRef>,
    /// 当前剩余预算快照；每个 stage `apply` 完成后由 ContextEngine 重算。
    pub budget_snapshot: TokenBudget,
    /// 由 tokenizer / 启发式估算得到的当前 prompt 体 token 数；
    /// 与 `budget_snapshot` 共同作为 stage 触发判据。
    pub estimated_tokens: u64,
}
```

#### 2.7.6 可变性矩阵

| 字段 | `Snip` | `Microcompact` | `Collapse` | `Autocompact` | `ToolResultBudget` |
|---|---|---|---|---|---|
| `frozen.*` | ❌ | ❌ | ❌ | ❌ | ❌ |
| `active.history` | 删除（保 pair）| 摘要替换（保 pair）| 同类合并 | 整段摘要后 fork | 不动结构（仅 offload 指向变更）|
| `active.tool_use_pairs` | 同步收缩 | 同步收缩 | 不变 | fork 后由子 session 重建 | 不变 |
| `patches` | Transient 优先剔除 | 进入摘要 | 不参与（每条独立段）| 进入摘要 | 不参与 |
| `deferred_tools_delta` | 不变 | 不变 | 不变 | 不变 | 不变 |
| `bookkeeping.offloads` | 同步剔除被删消息条目 | 同步剔除 | 同步剔除 | fork 后清零 | **新增 / 替换** |
| `bookkeeping.estimated_tokens` | 重算 | 重算 | 重算 | 重算 | 重算 |

> **跨 session 不可共享**：`ContextBuffer` 实例严格属于单个 Run；任何 `Arc<ContextBuffer>` 跨 Session 的尝试都视为反模式（详见 §12）。

### 2.8 Assemble 内部子阶段（Ingest / Recall / Activate / Patch）

本节说明 `ContextEngine::assemble` 的内部执行序，等价于业内常说的「ingest 阶段」。**不引入新的 trait 方法**，仅把"看不见的内部决策"对齐到现有扩展点，避免后续被误读为缺失。

#### 2.8.1 子阶段时序

```text
ContextEngine::assemble(session, turn_input)
  │
  ├── 子阶段 1：Ingest 决策
  │     · MemoryLifecycle::on_turn_start(turn, &UserMessageView)   // best-effort
  │     · Hook::UserPromptSubmit(run_id, input)                    // 业务侧 hook
  │     · RecallTriggerStrategy::should_recall(turn, message)      // 启发式判定
  │
  ├── 子阶段 2：Memory Recall
  │     · 仅当子阶段 1 决策为 yes 时执行
  │     · MemoryProvider::recall(query) → Vec<MemoryRecord>
  │     · 经 escape_for_fence + wrap_memory_context 包成 ContextPatch::MemoryRecall
  │
  ├── 子阶段 3：Skill Activation
  │     · SkillRegistry 依 user message + skill metadata 决定激活集合
  │     · SkillRenderer::render(skill) → ContextPatch::SkillInjection
  │
  ├── 子阶段 4：Hook AddContext 收集
  │     · 子阶段 1 的 UserPromptSubmit hook 可返回 HookOutput::AddContext { body }
  │     · ContextEngine 收集为 ContextPatch::HookAddContext
  │
  └── 子阶段 5：Patch 装配
        · 子阶段 2/3/4 产生的 patches 按 `context-engineering.md §11.2` 固定顺序拼接到 user message 头部
        · FrozenContext / BootstrapSegment 不变（保 cache 稳定）
        · 输出 AssembledPrompt（含 cache breakpoints）
```

#### 2.8.2 扩展点矩阵

| 子阶段 | 默认实现 | 业务扩展接口 | 失败语义 |
|---|---|---|---|
| 1. Ingest 决策 | 启发式 + Memory + Hook | `Hook::UserPromptSubmit` / `MemoryLifecycle::on_turn_start` / `RecallTriggerStrategy` | hook 失败按 `harness-hook.md §6` 走配置策略；memory 失败 fail-open |
| 2. Memory Recall | `MemoryProvider::recall` | 替换 `MemoryProvider` 实现 | fail-open：失败仅写 `MemoryRecallDegraded`，不阻塞 |
| 3. Skill Activation | `SkillRegistry` 内置 metadata 路径 | `SkillRegistryBuilder::with_eager(...)` / `with_metadata_resolver(...)` | 失败仅跳过激活，不阻塞 |
| 4. Hook AddContext | `Hook::UserPromptSubmit` 返回 `AddContext` | 业务自定义 hook handler | 同子阶段 1 |
| 5. Patch 装配 | `ContextEngine` 内部固定逻辑 | **不可扩展**（顺序由 ADR-003 锁定） | 装配失败 → `ContextError::Provider("patch assembly")`，turn 终止 |

> **设计取舍**：上述 4 个扩展点（`Hook::UserPromptSubmit` / `MemoryLifecycle::on_turn_start` / `RecallTriggerStrategy` / `SkillRegistry`）已经覆盖业务在 ingest 阶段的全部插桩需求，无须再在 `ContextEngine` 上暴露独立的 `ingest()` 方法。维持 `assemble()` 单一入口可减少调用方心智负担与 replay 复杂度。

#### 2.8.3 与 Compact 管线的边界

- `assemble` 是 **构造性** 步骤（产出新 prompt）；Compact 管线是 **缩减性** 步骤（基于已构造 buffer 做 token 收敛）
- 子阶段 5 装配完成 → `bookkeeping.estimated_tokens` 计算 → **若超 `soft_budget`，立即触发 §3 Compact 管线**（同一个 turn 内联走完）
- Compact 管线只读 `frozen`、可写 `active` / `patches` / `bookkeeping`（详见 §2.7.6）；不会回炉到子阶段 1–4

#### 2.8.4 反模式

- 在子阶段 1–4 中直接修改 `frozen.*`（违反 ADR-003 cache 锁）
- 在子阶段 4 通过 hook 注入到 system 段（必须落 `ContextPatch::HookAddContext` 进 user 段，详见 `context-engineering.md §11`）
- 在子阶段 2 之后旁路 `escape_for_fence`（绕开 `<memory-context>` 注入防护，参考 `harness-memory.md §5`）
- 用 `Hook::UserPromptSubmit` 触发任何阻塞性同步等待（应走 `MemoryLifecycle` 异步路径）

## 3. Compact 管线

### 3.1 固定顺序（对齐 CC-06）

```text
┌────────────────────────────────────────┐
│ 1. tool-result-budget (每工具结果上限) │
│ 2. snip             (裁剪最旧单条)      │
│ 3. microcompact     (小规模摘要 N 条)   │
│ 4. collapse         (合并同类)          │
│ 5. autocompact      (整段摘要 + Fork)   │
└────────────────────────────────────────┘
```

### 3.2 默认实现

```rust
pub mod providers {
    pub struct ToolResultBudgetProvider {
        pub per_tool_max_chars: u64,
        pub blob_offload: Arc<dyn BlobStore>,
    }

    pub struct SnipProvider {
        pub protected_recent_n: usize,  // 默认 3
    }

    pub struct MicrocompactProvider {
        /// 通过 `AuxTask::Compact` 调用；超时 / 并发由 `AuxOptions` 决定
        pub aux_provider: Arc<dyn AuxModelProvider>,
        pub target_ratio: f32,   // 默认 0.2
        pub batch_size: usize,   // 默认 20
    }

    pub struct CollapseProvider {
        pub merge_threshold_chars: usize,
    }

    pub struct AutocompactProvider {
        /// 同上，使用 `AuxTask::Compact`
        pub aux_provider: Arc<dyn AuxModelProvider>,
        pub hard_budget_ratio: f32,
    }
}
```

## 4. Bootstrap 文件注入

```rust
pub struct WorkspaceBootstrap {
    pub workspace_root: PathBuf,
    pub files: Vec<BootstrapFileSpec>,
}

pub struct BootstrapFileSpec {
    pub filename: String,
    pub priority: u32,
    pub max_chars: u64,
    pub required: bool,
}

impl WorkspaceBootstrap {
    pub fn default_files() -> Vec<BootstrapFileSpec> {
        vec![
            BootstrapFileSpec { filename: "AGENTS.md".into(), priority: 1, max_chars: 8000, required: false },
            BootstrapFileSpec { filename: "CLAUDE.md".into(), priority: 2, max_chars: 8000, required: false },
            BootstrapFileSpec { filename: "SOUL.md".into(), priority: 3, max_chars: 4000, required: false },
            BootstrapFileSpec { filename: "IDENTITY.md".into(), priority: 4, max_chars: 2000, required: false },
            BootstrapFileSpec { filename: "USER.md".into(), priority: 5, max_chars: 4000, required: false },
            BootstrapFileSpec { filename: "BOOTSTRAP.md".into(), priority: 6, max_chars: 2000, required: false },
        ]
    }
}
```

注入位置：**system message 尾部**（不影响 ToolPool / Memory 断点）。

## 5. Sanitizer

```rust
pub struct ContextSanitizer {
    pub strip_old_fences: bool,
    pub strip_unknown_xml: bool,
    pub normalize_newlines: bool,
}

impl ContextSanitizer {
    pub fn sanitize(&self, content: &str) -> String;
}
```

每轮 assembly 前运行，剥离上一轮注入的 `<memory-context>` / `<external-untrusted>` 栅栏，防止指令累积。

补充约束（与 `harness-memory.md §5` 对齐）：

- `ContextSanitizer` 只负责剥离旧 fence，不负责特殊 token 清洗；
- 特殊 token 清洗由 `harness-memory::escape_for_fence` 在 recall 记录入栅栏前执行；
- 两者必须同时启用：`escape_for_fence` 防止越狱，`sanitize` 防止跨轮累积注入。

## 6. Feature Flags

```toml
[features]
default = ["anthropic-cache", "compact-aux-llm"]
anthropic-cache = []
compact-aux-llm = ["dep:octopus-harness-model"]
```

## 7. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("token budget exhausted: requested={requested}, max={max}")]
    BudgetExhausted { requested: u64, max: u64 },

    #[error("provider: {0}")]
    Provider(String),

    #[error("memory: {0}")]
    Memory(#[from] MemoryError),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
```

## 8. 使用示例

```rust
let aux = BasicAuxProvider::wrap(AnthropicProvider::from_env()?, AuxOptions::default());

let engine = ContextEngine::builder()
    .with_budget(TokenBudget::default())
    .with_cache_policy(PromptCachePolicy {
        style: PromptCacheStyle::Anthropic { mode: AnthropicCacheMode::SystemAnd3 },
        max_breakpoints: 4,
        breakpoint_strategy: BreakpointStrategy::SystemAnd3,
    })
    .with_aux_provider(Arc::new(aux))
    .with_bootstrap(WorkspaceBootstrap::at("data/workspace"))
    .with_memory_manager(memory_mgr)
    .build()?;

let prompt = engine.assemble(&session_state, &turn_input).await?;
```

## 9. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 每个 provider 的 apply 正确 |
| 管线 | 五阶段顺序固定；任一阶段失败有 fallback |
| Prompt Cache | 同一 session 连续多轮后 breakpoints 稳定 |
| 预算 | soft/hard 阈值分别触发 snip / autocompact |
| Sanitizer | 旧栅栏被剥离，新栅栏被保留 |

## 10. 可观测性

| 指标 | 说明 |
|---|---|
| `context_bytes_assembled` | 每轮组装字节数 |
| `context_cache_hit_ratio` | Anthropic cache hit 比例 |
| `context_compact_pipeline_trigger` | 按阶段分桶 |
| `context_autocompact_forks_total` | 触发 fork 次数 |
| `context_tool_offload_bytes_total` | 工具结果落盘字节 |

## 11. Subagent 上下文协调

子代理（subagent）启动时会创建一个**独立的 `ContextEngine` 实例与 `ContextBuffer`**（§2.7 反模式条目"跨 session 不可共享"已硬约束）。父→子的输入裁剪、子→父的结果折叠，**不引入新的 ContextEngine 接口**，全部走以下既有扩展点协同完成。本节明文化各节点的语义边界与 fail-open 规则，避免被误读为"缺一组 subagent context hook"。

### 11.1 父→子：输入裁剪与隔离

```text
Parent.run_turn → Tool::Agent.run(input)
  │
  ├── Hook::PreToolUse { tool_name = "agent", input }
  │     · 业务可返回 PreToolUseOutcome::RewriteInput { new_input }
  │       → 在 spawn 前裁剪/重写传给子的 prompt（避免父 transcript 全量传递）
  │     · 或 Block { reason } 阻止 spawn（权限拒绝、预算告罄）
  │
  └── Subagent.spawn(spec, input, parent)
        ├── 创建独立 ContextEngine + ContextBuffer
        │     frozen 不与父共享：system_header / tools_snapshot / memory_snapshot
        │     由 SubagentSpec 决定（详见 crates/harness-subagent.md §2）
        │
        └── Hook::SubagentStart { subagent_id, spec }
              · 业务可返回 AddContext { body } 注入子的首条 user message
                （announcement 增强；走子的 §2.8 Patch 装配子阶段）
```

**关键约束**：

| 维度 | 约束 | 违反后果 |
|---|---|---|
| frozen face | 父子各自独立；父的 `FrozenContext` 字节序列 **不**复制进子的 frozen | 父 cache 失效 + 子 cache 起步重建（成本翻倍）|
| `ContextBuffer` 实例 | 父子绝对独立；任何 `Arc<ContextBuffer>` 跨 Session 视为反模式（§2.7） | 信息越权 + replay 不确定 |
| memory snapshot | 子的 Memdir 装配遵循 `SubagentSpec.memory_scope`（业务自定义子集） | 信息越权 |
| 父 transcript 传递量 | 默认走 `Hook::PreToolUse::RewriteInput` 主动裁剪；不裁剪则按 `SubagentSpec.input_strategy` 兜底 | 子 prompt 起步即超 budget |

### 11.2 子→父：结果折叠与持久化

```text
Subagent.run_to_end → SubagentResult { output, status, usage }
  │
  ├── Hook::SubagentStop { subagent_id, status }
  │     · 业务可返回 AddContext { body } 在 announcement 阶段补充
  │       （只读 + 上下文注入，参考 harness-hook.md §2.4 表格）
  │
  ├── Tool::Agent 把 SubagentResult.output 包成 ToolResult 回传父 turn
  │
  ├── Hook::PostToolUse { tool_use_id, result }
  │     · 业务可返回 AddContext { body } 折叠/缩写子的输出
  │       → 进入父的 ContextBuffer.patches（HookAddContext，Transient 默认）
  │       → 由父下一轮 §2.8 子阶段 4 收集
  │
  └── MemoryLifecycle::on_delegation(task, result, child_session)
        · best-effort：把"父委派给子的事实"写入持久 Memdir
        · 失败 fail-open，仅落 MemoryRecallDegraded 事件
```

### 11.3 扩展点矩阵

| 协调阶段 | 扩展点 | 默认实现 | 失败语义 |
|---|---|---|---|
| spawn 前输入裁剪 | `Hook::PreToolUse(tool="agent")` 的 `RewriteInput` | 透传父 input（`SubagentSpec.input_strategy` 兜底） | hook 失败按 `harness-hook.md §6` 配置策略 |
| spawn 时身份/工具分配 | `SubagentSpec`（不是 hook，是声明式配置） | `harness-subagent.md §2.1` 默认模板 | spec 校验失败 → `SubagentError::InvalidSpec` |
| spawn 后欢迎语 | `Hook::SubagentStart` 的 `AddContext` | 无注入 | hook 失败 fail-open |
| 子结果折叠 | `Hook::PostToolUse(agent tool)` 的 `AddContext` | 把 `SubagentResult.output` 原样作为 ToolResult | hook 失败 fail-open |
| 子结束 announce | `Hook::SubagentStop` 的 `AddContext` | 无注入 | 同上 |
| 持久事实落地 | `MemoryLifecycle::on_delegation` | NoopMemoryProvider 直接 Ok(()) | best-effort，落 `MemoryRecallDegraded` |

### 11.4 设计取舍

- **不在 ContextEngine 上新增 `on_subagent_spawn` / `on_subagent_returned`**：上述 6 个扩展点已经覆盖父子上下文协调的全部插桩需求；新增 hook 会与 `Hook::*` 体系产生职责重叠，违反 KISS
- **`SubagentBridge` 与本节解耦**：bridge 处理"子的权限请求转发回父"的 broker 路径（详见 `harness-subagent.md`），不参与上下文协调本身
- **Replay 一致性**：父 session 的 `Hook::PreToolUse::RewriteInput` 输出会写入 `Event::HookOutcomeApplied`（`event-schema.md §3.7`）；子 session 启动时的 input 由 replay 重放父的 hook 事件还原，不直接重读父 transcript

### 11.5 反模式

- 子代理直接从父的 `ContextBuffer` 读取消息（违反 §2.7 反模式）
- 把子的 `SubagentResult.output` 原样塞进父的 system 段（cache 失效；应走 `Hook::PostToolUse::AddContext` 进 user 段）
- 用 `Hook::SubagentStart` 注入子的 frozen 段（应走 `SubagentSpec.system_header_extra` 等声明式字段）
- 在 `MemoryLifecycle::on_delegation` 中阻塞父的下一 turn（必须 best-effort 异步）

## 12. 反模式

- Compact 阶段乱序执行
- 每轮重算 system message（应 cache）
- Bootstrap 文件注入 user message（破坏稳定性）
- Sanitize 规则过于激进（剥离用户的合法 XML 内容）
- 子代理与父 session 共享 `ContextBuffer` 实例（详见 §11）

## 13. 相关

- D8 · `context-engineering.md`
- ADR-003 Prompt Cache
- `crates/harness-memory.md`
- `crates/harness-memory.md` §2.5（借用视图）/ §4.2（Recall 编排）/ §5（三道闸门）
- Evidence: HER-026, HER-027, CC-06, CC-32, CC-33, OC-13
