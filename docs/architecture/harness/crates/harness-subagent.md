# `octopus-harness-subagent` · L3 · 父→子 委派 SPEC

> 层级：L3 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-engine`（via trait） + `harness-session` + `harness-tool` + `harness-permission`

## 1. 职责

实现 **Subagent（父→子任务委派）**：父 Agent 临时委派有界子任务给子 Agent，子 Agent 执行后以结构化结果回父。对齐 HER-014 / CC-08 / OC-27。

**核心能力**：

- `SubagentRunner`：创建子 Engine 实例运行委派任务
- 硬约束：blocklist / max_depth / max_concurrent_children
- `SubagentAnnouncement` 结构化结果（不直接对用户说话）
- Prompt Cache 共享（frozen system prompt）
- RAII 资源管理（inline MCP server / sandbox 自动释放）

## 2. 对外 API

### 2.1 核心 Trait

```rust
#[async_trait]
pub trait SubagentRunner: Send + Sync + 'static {
    async fn spawn(
        &self,
        spec: SubagentSpec,
        input: TurnInput,
        parent_ctx: ParentContext,
    ) -> Result<SubagentHandle, SubagentError>;
}
```

### 2.2 核心类型

```rust
pub struct SubagentSpec {
    pub role: String,
    pub prompt_template: PromptTemplate,
    pub toolset: ToolsetSelector,
    pub tool_blocklist: HashSet<String>,
    pub sandbox_policy: SandboxInheritance,
    pub context_mode: SubagentContextMode,
    pub permission_mode: PermissionMode,
    pub max_turns: u32,
    pub max_depth: u8,
    pub announce_mode: AnnounceMode,
    pub mcp_servers: Vec<McpServerRef>,

    /// 子代理装配时**必须满足**的 MCP server 依赖；任何 pattern 不满足
    /// （未注册 / 未就绪 / Inline 来源不被允许）则 `_run` 返回
    /// `SubagentError::McpRequirementUnsatisfied`，不进入工具执行。
    /// 校验逻辑见 `crates/harness-mcp.md §5.3`。
    pub required_mcp_servers: Vec<McpServerPattern>,

    /// 子代理在审批链中的可交互性。默认 `DeferredInteractive`：
    /// 审批请求挂到父 Session 的 EventStream，由父级 UI / Engine 处置；
    /// 子代理本身不会阻塞等待终端用户答复。详见 §6.2。
    ///
    /// 业务层若有"子代理走和父代理同一前台 UI"的需求，可显式声明
    /// `FullyInteractive`，但仅当父 Session 仍处于交互式上下文时合法
    /// （Cron / Gateway 触发的子代理必须 `DeferredInteractive` 或 `NoInteractive`）。
    pub interactivity: InteractivityLevel,

    /// 单次子代理运行的资源配额。`None` 表示沿用父 Session 的 `RunBudget`，
    /// 不施加额外约束；`Some(quota)` 时由子 Engine 在循环边界比对 token /
    /// tool_calls / wall-clock / cost 各维度，命中即以
    /// `SubagentStatus::MaxBudget(BudgetKind)` 收尾。
    ///
    /// `ResourceQuota` 形状定义在 `agents-design.md §7.1`，与
    /// `TeamMemberSpec.quota` 共享同一份类型；契约层 `harness-contracts §3.4`
    /// 仅暴露已被事件层引用的 `BudgetKind`，`ResourceQuota` 留在 SDK 共享
    /// 类型层（避免 contracts 强行吸纳所有运行期配额结构）。
    pub quota: Option<ResourceQuota>,

    /// 父→子 Memdir 装配范围。默认 `Inherit`：与父 `Session.memory_scope`
    /// （`harness-session.md §2.1`）一致；子代理可声明 `Subset` 进一步收缩，
    /// 但 **不得扩展超出父可见集合**（装配期校验，越权直接 fail-closed）。
    /// 由 `harness-context §11.1` 的"memory snapshot"约束反向引用本字段。
    pub memory_scope: SubagentMemoryScope,

    /// 父→子 prompt 输入裁剪策略。`Hook::PreToolUse(tool="agent")::RewriteInput`
    /// 是首要装载点（业务可主动重写）；本字段是其 fall-back —— 业务未挂 hook 时
    /// 仍能给出确定性的默认形态，避免父 transcript 全量传递把子 prompt 起步
    /// 即推至 budget 上限。`harness-context §11.1` 表格中的"父 transcript 传递量"
    /// 以本字段为兜底。
    pub input_strategy: SubagentInputStrategy,

    /// 声明式 system header 附加段，spawn 期由 `ContextEngine` 一次性合成进
    /// `FrozenSystemPrompt::rendered`（§5），对父 cache **完全无影响**（父子
    /// 各自的 frozen face 不共享实例，详见 §5 "实例独立 / 字面共享" 边界与
    /// `harness-context §11.1`）。运行期可变的注入仍走
    /// `Hook::SubagentStart::AddContext`（仅落 user message 前置补丁）。
    /// `harness-context §11.5` 反模式条目"用 hook 注入子 frozen 段"以本字段为正解。
    pub system_header_extra: Option<String>,

    /// Workspace Bootstrap 文件（`AGENTS.md` / `CLAUDE.md` / `IDENTITY.md` ...）
    /// 在子代理装配期的过滤策略。默认 `ExcludeAll`：子代理是有界、聚焦、单任务
    /// 的执行体，不应继承父级项目长说明（对齐 Claude Code 的 `omitClaudeMd`
    /// 默认行为，见 `reference-analysis/claude-code-sourcemap.md`）。
    /// 业务可声明 `Allow(["IDENTITY.md"])` 等子集，把人物身份段一并带入；
    /// `InheritAll` 仅供评测/复现，不建议线上启用。
    pub bootstrap_filter: BootstrapFilter,
}

/// 子代理 Memdir 可见范围；与 `harness-memory.md` 的 `MemoryVisibility` /
/// `MemoryKind` 直接耦合，不引入新 trait。
pub enum SubagentMemoryScope {
    /// 与父 Session.memory_scope 一致
    Inherit,
    /// 不装配任何 Memdir（极小子代理 / 评测路径）
    Empty,
    /// 仅装配命中以下条件交集的条目；`recall` 由 `allow_runtime_recall` 控制
    Subset(MemoryScopeFilter),
}

pub struct MemoryScopeFilter {
    /// 仅装配命中此 visibility 集合的条目；空集表示不限制
    pub visibilities: HashSet<MemoryVisibility>,
    /// 仅装配此 kind 子集；空集表示不限制
    pub kinds: HashSet<MemoryKind>,
    /// 是否允许子代理在线触发 `MemoryProvider::recall`；`false` 时只能用
    /// 装配期的快照（适合受限沙箱 / 凭证最小化场景）
    pub allow_runtime_recall: bool,
}

/// 父→子 prompt 输入裁剪策略；扩展点是 `Hook::PreToolUse::RewriteInput`，
/// 本枚举仅作为 hook 缺位时的确定性兜底。
pub enum SubagentInputStrategy {
    /// 默认：仅传父最近一次 UserMessage（对齐 CC-08 安全裁剪）
    LatestUserOnly,
    /// 父 transcript 整段透传（`AnnounceMode::FullTranscript` debug / 评测用）
    InheritAll,
    /// 业务自定义：装配期由 `ContextEngine` 通过 `selector_id` 反查注册表，
    /// selector 实现住在业务层；selector 必须是纯函数（不可有 IO 副作用），
    /// 否则 replay 不确定。
    Custom { selector_id: String },
}

/// Workspace Bootstrap 文件继承策略；命中文件按 `BootstrapFileSpec.filename`
/// 精确匹配，匹配后由 `ContextEngine` 的 `BootstrapFileLoaded` 流程处理截断。
pub enum BootstrapFilter {
    /// 默认：完全不继承父 Workspace Bootstrap（对齐 Claude Code `omitClaudeMd`）
    ExcludeAll,
    /// 仅继承指定文件名集合
    Allow(Vec<String>),
    /// 与父保持一致（仅用于评测 / 复现，正常配置应避免）
    InheritAll,
}

pub enum SubagentContextMode {
    Isolated,                              // 空 context
    ForkFromParent { include_tool_results: bool },
}

pub enum SandboxInheritance {
    /// 完全承袭父 `SandboxPolicy`（含 mode / scope / network / resource_limits）
    Inherit,
    /// 承袭父策略，但额外要求当前 backend 满足一组 capabilities；
    /// runner 在选择 backend 前调用 `SandboxBackend::capabilities()` 校验，
    /// 不匹配返回 `SandboxError::CapabilityMismatch`，绝不静默降级。
    Require(RequiredSandboxCapabilities),
    /// 覆盖父策略；`SandboxPolicy` 的形状定义在 `harness-contracts.md` §3.4，
    /// 详细语义在 `crates/harness-sandbox.md` §2.3。
    Override(SandboxPolicy),
}

/// 子任务对 backend 能力的最小要求；与 `SandboxCapabilities` 对偶。
/// 任一字段为 `Some(true)` 表示**必须支持**；`None` 表示不关心。
pub struct RequiredSandboxCapabilities {
    pub network: Option<bool>,
    pub filesystem_write: Option<bool>,
    pub gpu: Option<bool>,
    pub interactive_shell: Option<bool>,
    pub session_snapshot: Option<bool>,
    pub min_concurrent_execs: Option<u32>,
}

pub enum AnnounceMode {
    StructuredOnly,   // 仅结构化 summary
    SummaryText,      // 摘要文本
    FullTranscript,   // 完整转写（调试）
}

/// 子代理工具集选择器。
///
/// **二次过滤语义**：无论 selector 选哪条分支，最终对子 Engine 暴露的工具集都满足
/// `final = selector(parent_pool) - SubagentSpec.tool_blocklist - SubagentPolicy.blocklist`。
/// 也就是说 `tool_blocklist` 与 policy 默认 blocklist 是 selector 之后的硬剥离闸，
/// 不存在 "selector 选了某工具但 blocklist 没生效" 的路径——`InheritAll` 也不例外。
/// 这与 ToolRegistry 同名裁决（`harness-tool §2.5.1`）相互独立：blocklist 是按
/// **canonical name** 匹配，包括 `mcp__<server>__<tool>` 这类 MCP 衍生工具。
pub enum ToolsetSelector {
    InheritAll,
    InheritWithBlocklist(HashSet<String>),
    Preset(String),
    Custom(Vec<String>),
}

pub struct ParentContext {
    pub tenant_id: TenantId,
    pub parent_session_id: SessionId,
    pub parent_run_id: RunId,
    pub depth: u8,
    pub sibling_count: u32,
}
```

### 2.3 SubagentHandle

```rust
pub struct SubagentHandle {
    pub subagent_id: SubagentId,
    pub events: EventStream,
    cancel: oneshot::Sender<()>,
    wait_channel: oneshot::Receiver<SubagentAnnouncement>,
}

impl SubagentHandle {
    pub async fn wait(self) -> Result<SubagentAnnouncement, SubagentError>;
    pub async fn cancel(self);
}
```

### 2.4 Announcement

```rust
pub struct SubagentAnnouncement {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    pub status: SubagentStatus,
    pub summary: String,
    pub result: Option<Value>,
    pub usage: UsageSnapshot,
    pub transcript_ref: Option<TranscriptRef>,
    pub ended_at: DateTime<Utc>,
}

pub enum SubagentStatus {
    Completed,
    InterruptedByParent,
    MaxIterationsReached,
    /// `SubagentSpec.quota` 命中；`BudgetKind` 形状见 `harness-contracts §3.8.1`。
    /// 与 `MaxIterationsReached` 不同：后者是 `max_turns` 触顶（轮次驱动），
    /// `MaxBudget` 强调资源维度耗尽（token / cost / wall-clock / tool_calls）。
    MaxBudget(BudgetKind),
    Failed(String),
}
```

### 2.5 Default Policy（对齐 HER-014）

```rust
pub struct SubagentPolicy {
    pub blocklist: HashSet<String>,

    /// 业务可调的"软上限"——`HarnessBuilder` / 业务层可以按场景调高，
    /// 但仍受 `depth_cap` 强制裁剪。
    pub max_depth: u8,

    /// 系统硬上限（"硬闸"）。任何业务层覆盖、Agent profile 注入或运行期
    /// 提升都不能突破它；最终 effective 上限恒为
    /// `min(spec.max_depth, policy.depth_cap)`。
    /// 与 Hermes `_MAX_SPAWN_DEPTH_CAP=3` 同构（见 reference-analysis/hermes-agent.md）。
    pub depth_cap: u8,

    pub max_concurrent_children: u32,
    pub default_announce_mode: AnnounceMode,
    pub shared_prompt_cache: bool,
}

impl Default for SubagentPolicy {
    fn default() -> Self {
        Self {
            blocklist: ["delegate", "clarify", "memory_write",
                        "send_user_message", "execute_code"]
                .iter().map(|s| (*s).to_string()).collect(),
            max_depth: 1,
            depth_cap: 3,
            max_concurrent_children: 3,
            default_announce_mode: AnnounceMode::StructuredOnly,
            shared_prompt_cache: true,
        }
    }
}
```

**软/硬双闸的语义**：

- `max_depth`：业务可见、可调；用来表达"这一类 agent 我希望最多嵌套几层"。
- `depth_cap`：架构红线、不可被业务覆盖；保护 SDK 自身不会因为某个错配的
  AgentProfile 触发指数级递归 / token 失控。
- `DefaultSubagentRunner::spawn` 计算
  `effective = min(spec.max_depth.unwrap_or(policy.max_depth), policy.depth_cap)`，
  `ParentContext.depth >= effective` → `SubagentError::DepthExceeded { current, max: effective }`。
- `depth_cap` 可在 `HarnessBuilder` 装配期一次性提升（如离线评测场景设到 5），
  但**不允许**在运行期由 Tool / Agent 篡改；`SubagentRunnerCap` 投影也只暴露
  `spawn`，没有任何 setter。

**`execute_code` 的延续闸门**（ADR-0016 §2.8）：

- 默认 blocklist 中的 `execute_code` 是**装配期闸门**：`SubagentRunner::filter_for(spec)`
  在计算"子代理可见工具集"时直接剔除该名字，子代理永远看不到该工具的 schema
- 即使业务通过 `team_config.toml` 显式 allow 给某 Subagent，**运行期闸门**仍会
  fail-closed：`ExecuteCodeTool::check_permission` 检测 `ctx.caller_chain` 含
  Subagent 标记 → 返回 `ToolError::DeniedToSubagent`，写
  `Event::ToolUseDenied { reason: SubagentBlocked }`
- 这一双层闸门与上文 `max_depth / depth_cap` 同模式（声明 + 执行），
  两条独立路径都失效才会让 PTC 漏出到子代理；**不**因 ADR-0016 引入而放宽

> 本节默认值在 ADR-0016 引入后**不**修改；`execute_code` 名字早已在 `blocklist`
> 默认集合中，本节仅显式确认其语义延续与双层闸门约束。

## 3. SubagentRunner 默认实现

```rust
pub struct DefaultSubagentRunner {
    engine_runner: Arc<dyn EngineRunner>,
    policy: SubagentPolicy,
    concurrency_pool: Arc<ConcurrentSubagentPool>,
    memory_mgr: Arc<MemoryManager>,
    mcp_registry: Arc<McpRegistry>,
}

impl DefaultSubagentRunner {
    pub fn new(
        engine_runner: Arc<dyn EngineRunner>,
        policy: SubagentPolicy,
    ) -> Self;
}

#[async_trait]
impl SubagentRunner for DefaultSubagentRunner {
    async fn spawn(/* ... */) -> Result<SubagentHandle, SubagentError> {
        // 0. 若 SubagentAdmin::is_spawning_paused() == true → SpawningPaused
        // 1. 计算 effective_max_depth = min(spec.max_depth, policy.depth_cap)
        //    若 ParentContext.depth >= effective_max_depth → DepthExceeded { max: effective }
        //    再走 ConcurrentSubagentPool::acquire(parent_session, depth)
        // 2. 创建子 session（Isolated 或 ForkFromParent）
        // 3. 计算子 toolset
        // 4. 继承或创建 sandbox
        // 5. 处理 mcp_servers
        //    5.1 Shared / Required：从父 McpRegistry 获取连接，Required 必须 Ready
        //    5.2 Inline：先按 `harness-mcp.md §5.2` 校验 trust：
        //        - spec.source ∈ {Workspace, Policy, Plugin{Admin}, Managed} → 通过
        //        - 父 Subagent 自身为 AdminTrusted → 通过
        //        - PermissionMode = BypassPermissions ∧ HarnessBuilder.with_inline_user_mcp(true) → 通过 + 必须落审计
        //        - 其余 → fail-closed `SubagentError::InlineMcpTrustViolation`
        //    5.3 evaluate_required(SubagentSpec.required_mcp_servers)
        //        任一 Missing/NotReady/InlineDisallowed → SubagentError::McpRequirementUnsatisfied
        // 6. 冻结 renderedSystemPrompt（对齐 CC-08）
        // 7. 创建子 Engine
        // 8. 启动 run + 监听结果
        // 9. 返回 Handle
    }
}
```

`SubagentError`（§9）相应增加 `McpRequirementUnsatisfied { evaluations }` 与 `InlineMcpTrustViolation { server_id, source }` 两个变体。

### 3.1 `SubagentRunnerCap` 投影（ADR-011）

`harness-tool` 不直接依赖本 crate；`AgentTool` 等内置工具通过 `ToolCapability::SubagentRunner` 借用 `Arc<dyn SubagentRunnerCap>` 实现解耦。本 crate 提供把 `SubagentRunner` trait wrap 成 contracts 投影的薄适配器：

```rust
use harness_contracts::capability::SubagentRunnerCap;

pub struct SubagentRunnerCapAdapter {
    inner: Arc<dyn SubagentRunner>,
}

impl SubagentRunnerCapAdapter {
    pub fn from_runner(runner: Arc<dyn SubagentRunner>) -> Arc<dyn SubagentRunnerCap> {
        Arc::new(Self { inner: runner })
    }
}

impl SubagentRunnerCap for SubagentRunnerCapAdapter {
    fn spawn(
        &self,
        spec: SubagentSpec,
        parent: ParentContext,
    ) -> BoxFuture<'static, Result<SubagentHandle, SubagentError>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.spawn(spec, /* input */ Value::Null, parent).await })
    }
}
```

> Engine 装配期使用：
>
> ```rust
> let runner = Arc::new(DefaultSubagentRunner::new(engine_runner, policy));
> cap_registry.install::<dyn SubagentRunnerCap>(
>     ToolCapability::SubagentRunner,
>     SubagentRunnerCapAdapter::from_runner(runner),
> );
> ```
>
> 通过这个薄包装，`harness-tool` 始终只依赖 `harness-contracts`；`harness-subagent` 既保留它原本的 `SubagentRunner` 强类型 API（供 Engine 内部直接调用），又对工具层暴露 capability 投影（详见 `harness-engine.md §6.x · Capability 装配`）。

**Trust 校验**：`CapabilityPolicy::default_locked()` 仅允许 `origin = ToolOrigin::Builtin` 或 `ToolOrigin::Plugin { trust: AdminTrusted }` 的 Tool 申请 `ToolCapability::SubagentRunner`；UserControlled Plugin 工具试图申请会在 `ToolRegistry::register` 阶段失败为 `RegistrationError::CapabilityNotPermitted`。

### 3.2 管理面：`SubagentAdmin`

`SubagentRunner` 是"装配期面向 Engine"的 trait；`SubagentAdmin` 是"运行期面向运维 / Bugbot / 灰度回退"的旁路 trait，与 spawn 解耦：

```rust
#[async_trait]
pub trait SubagentAdmin: Send + Sync + 'static {
    /// 列出当前所有活跃子代理快照（不含已结束的句柄）。
    fn list_active(&self) -> Vec<SubagentSnapshot>;

    /// 定向中断；幂等：已结束的 id 直接返回 Ok(())。
    async fn interrupt(&self, id: &SubagentId) -> Result<(), SubagentError>;

    /// 暂停**新的** spawn；已在跑的子代理不受影响。
    /// 暂停期间 `SubagentRunner::spawn` 立即返回 `SubagentError::SpawningPaused`。
    fn pause_spawning(&self, paused: bool);

    fn is_spawning_paused(&self) -> bool;
}

/// `SubagentAdmin::list_active` 的只读快照；不持有句柄、不阻断生命周期。
pub struct SubagentSnapshot {
    pub subagent_id: SubagentId,
    pub parent_session_id: SessionId,
    pub agent_ref: AgentRef,
    pub depth: u8,
    pub spawned_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub running_tool: Option<ToolUseId>,
}
```

**与 `SubagentRunner` 的关系**：

- `DefaultSubagentRunner` 同时实现 `SubagentRunner + SubagentAdmin`；前者面向 `EngineRunner` 装配，后者由 `Harness::admin_facets()` 暴露给运维链路（CLI / Server health 端点 / Bugbot 钩子）。
- `pause_spawning(true)` 与 `interrupt` 都通过 §6.1 的事件主链落 Journal：
  - `Event::SubagentSpawnPaused { paused, by, at }`（新增；事件 schema 由 `event-schema.md §3` 后续吸纳）；
  - `interrupt` 触发常规 `SubagentTerminated { reason: Reason::AdminInterrupted, .. }`，与父 cancel 走同一路径，复用现有 watchdog。
- 该 trait **不**通过 `ToolCapability::SubagentRunner` 暴露给业务工具——`AgentTool` 只看到 `SubagentRunnerCap.spawn`，无法 pause / interrupt 其他子代理（避免同租户内的恶意工具互相干扰）。

**典型场景**：

| 场景 | 调用 |
|---|---|
| 灰度回退："全局停掉 subagent 派生 30 分钟，等热修复" | `admin.pause_spawning(true)`；observability 在 `subagent_admin_paused` 上看到 1 |
| Bugbot 检测到某 SubagentId 进入死循环 | `admin.interrupt(id).await?` |
| Server `/healthz` 渲染当前活跃子代理 | `admin.list_active()` |
| 集成测试断言 "spawn 在 paused 期间一定 fail-closed" | `pause_spawning(true)` + `runner.spawn(..)` 必返 `SpawningPaused` |

## 4. 并发控制

### 4.1 `ConcurrentSubagentPool` 与死锁防御

```rust
pub struct ConcurrentSubagentPool {
    per_parent_semaphores: DashMap<SessionId, Arc<Semaphore>>,
    per_parent_limit: u32,             // 默认 3 = max_concurrent_children
    per_depth_limit: u32,               // 默认 6 = 全局总活跃上限 per 深度层
    global_semaphore: Arc<Semaphore>,   // 默认 32：整机全局上限
    running: DashMap<SubagentId, SubagentHandle>,
}

impl ConcurrentSubagentPool {
    pub fn new(policy: ConcurrencyPolicy) -> Self;

    /// 获取 slot：按 (parent_session, depth) 分桶，避免递归 subagent 死锁
    pub async fn acquire(
        &self,
        parent_session: SessionId,
        depth: u8,
    ) -> Result<SubagentSlot, SubagentError>;

    pub fn running_count(&self) -> usize;
    pub async fn cancel_all(&self);
}

pub struct ConcurrencyPolicy {
    pub per_parent_limit: u32,
    pub per_depth_limit: u32,
    pub global_limit: u32,
    pub acquire_timeout: Duration,      // 默认 30s；超时返回 ConcurrentLimitExceeded
}
```

### 4.2 死锁防御

**场景**：若用单个全局 Semaphore 且父 subagent 等待子 subagent 结果时持有 slot，子 subagent 也要 acquire slot，递归链 + 高并发 → 死锁。

**防御设计**：

1. **按 parent_session + depth 分桶**：子 subagent 从**自己 depth 对应的 semaphore** 申请 slot，不与父竞争
2. **父释放后子才可 acquire 同一 parent_session 的新 slot**：保证 `per_parent_limit` 是真的"同一父下并发"上限
3. **`acquire_timeout` 硬超时**：超时返回 `ConcurrentLimitExceeded`，不允许无限等待
4. **`max_depth` + `per_depth_limit` 联合限流**：即使 per_parent 放宽，总活跃 subagent ≤ `max_depth × per_depth_limit`

### 4.3 Watchdog

```rust
impl ConcurrentSubagentPool {
    fn spawn_watchdog(&self) {
        // 周期扫描 running：
        // - 若 handle 长时间未心跳（> activity_timeout）→ 判定卡死 → cancel + 释放 slot + 发 Event::SubagentStalled
        // - 若 running_count 持续达 global_limit 超 5 min → 发 Warning 到 observability
    }
}
```

## 5. Prompt Cache 共享

```rust
pub struct FrozenSystemPrompt {
    pub rendered: String,
    pub tools_snapshot_id: SnapshotId,
    pub memory_snapshot_id: SnapshotId,
    pub breakpoint_hash: [u8; 32],
}

impl DefaultSubagentRunner {
    fn build_frozen_system_prompt(
        &self,
        parent: &SessionState,
        child_toolset: &ToolRegistrySnapshot,
    ) -> FrozenSystemPrompt {
        // 从父 session 复用 tools + memory snapshot
        // 只加 role-specific 的 preamble
    }
}
```

对齐 CC-08：子 agent 与父 agent 共享 tools + memory snapshot，**仅系统提示 preamble 部分不同**，保 cache 高命中。

### 5.1 边界：实例独立 / 字面共享

`harness-context.md §11.1` 的硬约束是"父子 `ContextBuffer` 实例严格独立"——任何 `Arc<ContextBuffer>` 跨 Session 都视为反模式。本节"共享"指的是 **frozen face 的字节序列复用**，不是数据结构实例复用：

| 维度 | 实例 | 字面内容 | 后果 |
|---|---|---|---|
| `ContextEngine` | 父子各一个 | — | 子代理拥有独立 Compact 管线、独立 budget |
| `ContextBuffer` | 父子各一个 | — | 父子的 `active.history` 互不可见 |
| `FrozenContext.system_header` | 父子各一份 `Arc<str>` | **逐字节相等**（`memory_snapshot_id` / `tools_snapshot_id` 也相等） | Anthropic / OpenAI cache 命中 `system_and_3` 的物理基础 |
| `FrozenContext.tools_snapshot` | 同上 | 同上 | 子的工具 schema 直接复用父 cache 段 |
| `FrozenContext.memory_snapshot` | 同上 | **按 `SubagentSpec.memory_scope` 收缩**：默认 `Inherit` 时字节相等；`Subset/Empty` 时字节不同 → 子在该断点 cache miss，仍命中前段断点 |
| `system_header_extra` 段 | 子专属 | 父无此段 | role-specific preamble；放在 `system_and_3` 的最后断点之后，**不破坏前序 cache** |
| `bootstrap_filter` 命中文件 | 子按 filter 装配 | 与父字节相等的子集 | `ExcludeAll` 时子的 BootstrapSegment 为空，断点位移；`Allow` 时仅命中文件参与字节对账 |

**实现规约**：

1. `build_frozen_system_prompt` 不直接 `Arc::clone(parent.frozen.tools_snapshot)`——父子各自持有独立 `Arc`，字节由同一个 `ToolRegistrySnapshot::canonicalize` 路径派生，确保字面相等且无跨 Session 引用泄漏（与 ADR-003 §6.1 一致）。
2. `system_header_extra` 必须**追加**到既有 system header 末端，不得插入到中段——否则破坏 `BreakpointStrategy::SystemAnd3` 的第一个断点字节序列，导致父 cache 失效。
3. `breakpoint_hash` 由 `(rendered, tools_snapshot_id, memory_snapshot_id)` 三元组派生；任一变化都视为新 cache 段，不与父复用。

**反模式（详见 §13）**：
- 把父 `ContextBuffer` 的 `Arc` 直接传给子 Engine（违反 `harness-context §11` 反模式）
- 在 `system_header_extra` 中嵌入会随时间变化的字段（如时间戳），导致每次 spawn cache miss
- 通过 `bootstrap_filter::InheritAll` 把父项目级 `CLAUDE.md` 全量带入子代理（默认 `ExcludeAll` 的设计动机就是规避此路径）

## 6. Event 轨迹

### 6.1 生命周期主链

```text
SubagentSpawned {
    parent_session_id,
    parent_run_id,                          // 与父 RunStartedEvent.run_id 字面相等
    subagent_id,
    agent_ref,
    spec_snapshot_id,                       // BlobStore 引用
    spec_hash,                              // BLAKE3([u8; 32])
    depth,
    trigger_tool_use_id: Option<ToolUseId>, // 触发本次 spawn 的工具调用（建议 9 因果链锚点）
    trigger_tool_name: Option<String>,
    at,
}
    │
    ▼
[子 Session 的完整 Event 流：RunStarted / AssistantDelta / ToolUse / ...]
    │
    ▼
SubagentAnnounced {
    subagent_id,
    parent_session_id,
    status,                                 // SubagentStatus（自我陈述）
    summary,
    result: Option<Value>,
    usage,
    transcript_ref: Option<TranscriptRef>,  // 仅 AnnounceMode::FullTranscript 填充
    renderer_id,                            // AnnouncementRenderer 标识（§7.1）
    at,
}
    │
    ▼
SubagentTerminated {
    subagent_id,
    parent_session_id,
    reason: SubagentTerminationReason,      // 客观终结路径（§event-schema.md §3.9.3）
    final_usage,
    at,
}
```

`SubagentSpawnPaused`（来自 `SubagentAdmin::pause_spawning(true|false)`）独立于本主链，不参与 spawn / announce / terminate 三步态机；详见 `event-schema.md §3.9.4`。

### 6.2 审批转发到父 Session（`DeferredInteractive` 默认路径）

子代理在 `interactivity = DeferredInteractive` 下命中需要审批的 Tool 时，`harness-permission` 不会在子 Session 上发"前台询问"，而是把决策请求**镜像**到父 Session 的 EventStream，由父 Engine / UI 决策、再回传到子 Session 解锁继续执行：

```text
[Subagent Session A · DeferredInteractive]
    Tool::execute → PermissionBroker::decide
        │
        ▼
    Event::PermissionRequested {                      <- 写到子 Session Journal
        request_id,
        session_id = subagent_session,
        tenant_id,
        subject,
        interactivity = DeferredInteractive,
        presented_options,
        causation_id,
        at,
    }
        │
        ▼
    SubagentRunner.subagent_bridge.forward_request(
        SubagentBridgeMessage::PermissionRequested { .. }
    )
        │
        ▼
[Parent Session · FullyInteractive]
    Event::SubagentPermissionForwarded {              <- 父 Session 的镜像事件
        parent_session_id,
        subagent_id,
        original_request_id,                          <- 与子 Session 的 request_id 同值
        subject,
        presented_options,
        forwarded_at,
    }
        │
        ▼  (父 UI 渲染 + 用户/Broker 决策)
    Harness::resolve_subagent_permission(
        original_request_id,
        Decision,
    )
        │
        ▼
    Event::SubagentPermissionResolved {               <- 父 Session 的镜像事件
        parent_session_id,
        original_request_id,
        decision,
        decided_by,
        at,
    }
        │
        ▼  (Bridge 回写到子 Session)
[Subagent Session A]
    Event::PermissionResolved {                       <- 子 Session 的最终事件
        request_id,
        decision,
        decided_by = ParentForwarded { parent_session_id },
        at,
    }
        │
        ▼
    Tool::execute resumes
```

**关键约束**：

1. **同 request_id 贯穿父子**：`PermissionRequestedEvent.request_id`、`SubagentPermissionForwardedEvent.original_request_id`、`SubagentPermissionResolvedEvent.original_request_id`、`PermissionResolvedEvent.request_id` 必须**字面相等**；replay / 审计才能跨 Session 交叉对账。
2. **decided_by 双重记录**：父 Session 的 `SubagentPermissionResolved.decided_by` 是真实决策者（`User` / `Broker { broker_id }` / `Hook { handler_id }`）；子 Session 收回的 `PermissionResolved.decided_by` 必须是 `DecidedBy::ParentForwarded { parent_session_id, original_decided_by }`，便于子 Session 的 replay 知道决策来自父级。`DecidedBy::ParentForwarded` 在 `harness-contracts §3.4` 与 `event-schema.md §3.6` 一并扩展。
3. **Bridge 不持有用户身份**：`SubagentBridge` 仅做事件路由，不做鉴权——身份/租户由父 Session 上下文承担（`security-trust.md §7.2.1`）。
4. **TimeoutPolicy 落到父侧**：父 Session 决定超时窗口；子 Session 等待父级回写期间不消耗自己的 `TimeoutPolicy.deadline`，仅在 Bridge 心跳缺失时启动本地 watchdog（`acquire_timeout` 默认 30s 沿用 §4.1）。
5. **审计完备**：父子两侧 Journal 都必须有完整事件对（子：`Requested + Resolved`；父：`Forwarded + Resolved`），任一侧缺失视为 Journal 损坏并触发 `Event::EngineFailed`。
6. **`FullyInteractive` 子代理的退化**：若 `SubagentSpec.interactivity = FullyInteractive` 且父 Session 也是 `FullyInteractive`，Bridge 退化为透传（依然写父 Session 的 `Forwarded/Resolved` 镜像事件以保审计；UI 由实现方决定是否折叠展示）。
7. **`NoInteractive` 子代理的硬约束**：若 `SubagentSpec.interactivity = NoInteractive`，Broker **不发** `PermissionRequested`、**不**调 Bridge；走 `FallbackPolicy` 直接定决策；适用于 Cron / 后台批处理。

### 6.3 SubagentBridge 契约

```rust
#[async_trait]
pub trait SubagentBridge: Send + Sync + 'static {
    /// 子 Session 把决策请求转发给父 Session；
    /// 调用本方法后子 Session 阻塞直到 `resolve_subagent_permission` 回写或超时。
    async fn forward_request(
        &self,
        msg: SubagentBridgeMessage,
    ) -> Result<Decision, SubagentError>;
}

#[non_exhaustive]
pub enum SubagentBridgeMessage {
    PermissionRequested {
        parent_session_id: SessionId,
        subagent_id: SubagentId,
        original_request_id: PermissionRequestId,
        subject: PermissionSubject,
        presented_options: Vec<Decision>,
        timeout_policy: Option<TimeoutPolicy>,
    },
    PermissionResolved {
        parent_session_id: SessionId,
        original_request_id: PermissionRequestId,
        decision: Decision,
        decided_by: DecidedBy,
    },
}
```

`DefaultSubagentRunner` 在 `spawn` 期注入 `Arc<dyn SubagentBridge>`，默认实现走父 Session 的 `EventStream` + `Harness::resolve_subagent_permission` 回环；业务层（如 Gateway 多租户路由）可替换为自定义实现，但必须满足上面 7 条约束。

## 7. 结果注入父 Agent

### 7.1 `AnnouncementRenderer` trait

把"announcement → user-role 文本"抽成 trait，便于：

- 不同 provider 用不同标签集（XML for Anthropic 系，JSON code-block for OpenAI 兼容）；
- 离线评测 / Replay 时切到 `PlainTextRenderer` 抑制噪声；
- 业务侧自定义 i18n 包装。

```rust
pub trait AnnouncementRenderer: Send + Sync + 'static {
    fn render(&self, a: &SubagentAnnouncement) -> RenderedAnnouncement;
}

pub struct RenderedAnnouncement {
    /// 注入到父 Engine 的 user-role 文本。
    pub user_message: String,
    /// 渲染器名（用于 metric label 与审计；如 `xml-task-notification`）。
    pub renderer_id: &'static str,
}
```

`DefaultSubagentRunner` 在装配期持有 `Arc<dyn AnnouncementRenderer>`（默认 `XmlTaskNotificationRenderer`），spawn 完成后在父 Session 写入：

```text
[父 Session]
  Event::SubagentAnnounced { subagent_id, summary, result, usage, .. }
  Event::UserMessageAppended { content: rendered.user_message, source: SubagentAnnouncement, .. }
```

`UserMessageAppendedEvent.source` 标记为 `SubagentAnnouncement`（`event-schema.md §3` 后续吸纳）后，UI 层就能识别这条 user-role 消息**不是**真实终端用户输入，按需折叠或换样式。

### 7.2 默认渲染器：`XmlTaskNotificationRenderer`

```rust
pub struct XmlTaskNotificationRenderer;

impl AnnouncementRenderer for XmlTaskNotificationRenderer {
    fn render(&self, a: &SubagentAnnouncement) -> RenderedAnnouncement {
        let user_message = format!(
            r#"<task-notification subagent-id="{}" status="{:?}">
  <rewrite-hint>This is an internal subagent result, not a direct user message. Rewrite naturally before responding to the end user; do not echo XML, subagent-id, or structured-result verbatim.</rewrite-hint>
  <summary>{}</summary>
  <structured-result>{}</structured-result>
</task-notification>"#,
            a.subagent_id.as_ref(),
            a.status,
            a.summary,
            a.result.as_ref().map(|v| v.to_string()).unwrap_or_default(),
        );
        RenderedAnnouncement {
            user_message,
            renderer_id: "xml-task-notification",
        }
    }
}
```

**`<rewrite-hint>` 的设计动机**（对齐 CC-08 / OC-27）：

1. CC-08 在 system preamble 强调 `<task-notification>` 是父代理"自查的内部信号"；OC-27 也要求父 agent **rephrase** 后再回用户。但实际复现里多次出现父原样吐 XML 给终端用户。
2. 把这条提示直接嵌入消息体（而非仅写在系统提示里）能在 **prompt cache miss** 与 **agent profile 自定义 system 提示**两种场景下都仍生效，避免依赖外部约束。
3. 提示故意写在 `<task-notification>` 顶部、用 `<rewrite-hint>` 而非 `<!-- -->`：注释式标签更不容易被父代理"格式无关"地剥离。

业务层若要替换为 JSON 风格，可实现自己的 renderer 并在 `HarnessBuilder::with_announcement_renderer` 注入；`AnnouncementRenderer` 不参与 prompt cache 计算（只影响 user message 文本，不影响系统提示模板），切换无需 invalidate 父 cache。

## 8. Feature Flags

```toml
[features]
default = []
```

## 9. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum SubagentError {
    #[error("depth exceeded: {current} > {max}")]
    DepthExceeded { current: u8, max: u8 },

    #[error("concurrent limit exceeded")]
    ConcurrentLimitExceeded,

    #[error("engine: {0}")]
    Engine(#[from] EngineError),

    #[error("cancelled by parent")]
    Cancelled,

    #[error("tool blocklist violation: {0}")]
    BlocklistViolation(String),

    #[error("required mcp servers unsatisfied: {evaluations:?}")]
    McpRequirementUnsatisfied { evaluations: Vec<RequiredEvaluation> },

    #[error("inline mcp trust violation: server={server_id} source={source:?}")]
    InlineMcpTrustViolation { server_id: McpServerId, source: McpServerSource },

    #[error("subagent spawning is paused by admin")]
    SpawningPaused,
}
```

`RequiredEvaluation` / `McpServerPattern` 定义见 `crates/harness-mcp.md §5.3`；`McpServerSource` 见 `harness-contracts.md §3.4`。

## 10. 使用示例

### 10.1 业务层

通常业务层**不直接**使用此 crate；Engine 内部通过 `AgentTool::execute` 触发 Subagent。

### 10.2 自定义 Subagent Policy

```rust
let runner = DefaultSubagentRunner::new(
    engine_runner,
    SubagentPolicy {
        max_depth: 3,
        max_concurrent_children: 5,
        ..Default::default()
    },
);

let harness = HarnessBuilder::new()
    .with_subagent_runner(Arc::new(runner))
    .build()
    .await?;
```

### 10.3 自定义 Runner

```rust
struct MyLoggingRunner {
    inner: DefaultSubagentRunner,
    logger: Arc<MyLogger>,
}

#[async_trait]
impl SubagentRunner for MyLoggingRunner {
    async fn spawn(/* ... */) -> Result<SubagentHandle, SubagentError> {
        self.logger.log_spawn(&spec);
        self.inner.spawn(spec, input, parent_ctx).await
    }
}
```

## 11. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | Blocklist 生效；Selector + blocklist 二次过滤；Depth `effective = min(max_depth, depth_cap)`；Announcement 渲染含 `<rewrite-hint>` |
| 软/硬双闸 | `policy.depth_cap = 3` + `spec.max_depth = 5` → effective=3；spawn 在 depth=3 时 `DepthExceeded { max: 3 }` |
| 并发 | 超 `max_concurrent_children` 阻塞等待；`acquire_timeout` 超时返回 `ConcurrentLimitExceeded` |
| Quota | `SubagentSpec.quota` 各维度（token / tool_calls / wall-clock / cost）命中 → `SubagentStatus::MaxBudget(kind)` 而非 `MaxIterationsReached` |
| 取消 | 父取消级联子；`SubagentAdmin::interrupt(id)` 等价于父 cancel 路径 |
| Admin | `pause_spawning(true)` 期间 `spawn` 必返 `SpawningPaused`；已在跑的子代理不受影响；`list_active` 不持有句柄不阻断生命周期 |
| Prompt Cache | 父子 cache breakpoint 一致；切换 `AnnouncementRenderer` **不**触发父 cache 失效（系统提示模板未变） |
| Isolated vs Fork | 两种 context mode 的正确性 |
| Permission Forwarding | 见 §11.1 交叉单测矩阵（与 `harness-permission §11.1` 同集，从子代理视角断言） |

### 11.1 与 `DedupGate` 的交叉单测矩阵（子代理视角）

`SubagentBridge` 把审批请求 forward 给父 Session 的 broker 链；父链顶部的 `DedupGate` 同样作用于这些 forwarded 请求。下表与 `harness-permission §11.1` 同矩阵，但**断言点在子代理 EventStream**：

| # | 子代理观察到的事件 | 关键断言 |
|---|---|---|
| **D-S1** | `Forwarded → Resolved`（首次） + `PermissionRequestSuppressed { reason: RecentlyAllowed } → Resolved`（次次复用） | 子侧两次 `Resolved` 的 `decision` 相等；第二次的 `decided_by == ParentForwarded` 且 `original_decision_id` 指向第一次决策 |
| **D-S2** | leader: `Forwarded → Resolved`；follower(×4): `PermissionRequestSuppressed { reason: JoinedInFlight } → Resolved` | 5 个子代理 task 拿到同一 `decision`；leader 与 follower 的 `request_id` 各自唯一 |
| **D-S3** | 子代理只看到 `PermissionRequestSuppressed { reason: RecentlyDenied } → Resolved { decision: DenyOnce }` | 子代理**不**应能通过 spawn 二次绕开父侧拒绝 |
| **D-S4** | 每次都是完整 `Forwarded → Resolved`，无 `PermissionRequestSuppressed` | `Severity::Critical` 命令子代理也必须每次都打扰人 |
| **D-S5** | 子代理本地 `Resolved { decision: DenyOnce, decided_by: Fallback(DenyAll) }` | `interactivity == NoInteractive` 时子代理不冒泡，无 `Forwarded` 出现 |
| **D-S6** | 跨 turn 第二次仍命中 `PermissionRequestSuppressed { reason: RecentlyAllowed }` | dedup 桶按 SessionId 持久，跨 RunId 仍有效 |
| **D-S7** | 重建 Session 后的子代理首次请求得到完整 `Forwarded → Resolved` | drop 后旧桶必须清空 |
| **D-S8** | `Plan` 模式下子代理每次都走完整 `Forwarded → Resolved`，无 `PermissionRequestSuppressed` | 子代理 `Plan` 模式不被 dedup 影响 |
| **D-S9** | follower 在 leader panic 后**自己**重新发起 `Forwarded` | `RecvError::Closed` 不能让 follower 拿"幽灵决策" |
| **D-S10** | 第 51 次起子代理只观察到 `Resolved` 但**无** `PermissionRequestSuppressed` 事件，对应指标 `subagent_permission_forwarded_suppressed_total` 增长 | 与父侧 `permission_dedup_suppressed_total` 计数对账 |

> 端到端 fixture 与 `harness-permission §11.1` 共用 `DedupConfig::tight()`，避免两边参数不一致引入真假阳性。

## 12. 可观测性

| 指标 | 说明 |
|---|---|
| `subagent_spawned_total{role}` | 按 role 分桶 |
| `subagent_duration_ms` | 端到端耗时 |
| `subagent_depth_histogram` | 委派深度分布 |
| `subagent_blocklist_hits_total{reason}` | 被 blocklist 阻止次数（reason ∈ `blocklist` / `selector_filtered` / `mcp_canonical`） |
| `subagent_concurrent_running` | 当前并发数（gauge） |
| `subagent_depth_cap_exceeded_total` | 命中 `depth_cap` 硬上限的次数（与软上限 `max_depth` 区分） |
| `subagent_quota_exhausted_total{kind}` | `SubagentSpec.quota` 命中分桶（kind 来自 `BudgetKind::*`） |
| `subagent_admin_paused` | gauge：当前是否被 `SubagentAdmin::pause_spawning(true)` 暂停 |
| `subagent_admin_interrupted_total` | 通过 `SubagentAdmin::interrupt` 主动中断的累计次数 |
| `subagent_announce_rendered_total{renderer_id}` | `AnnouncementRenderer::render` 调用次数；用于审核切换默认渲染器对父 cache 的影响 |

## 13. 反模式

- Subagent 直接调 `send_user_message`（违反 announce-only 硬约束）
- Subagent 修改父 `renderedSystemPrompt`（破坏 cache 共享）
- 无限委派（未设 max_depth）
- 忽略 Inline MCP server 的清理
- Cron / Gateway 触发的子代理用 `InteractivityLevel::FullyInteractive`（无终端 UI 等不到答复，会触发 `TimeoutPolicy` 兜底；应当显式 `NoInteractive` + `FallbackPolicy::DenyAll`）
- 自定义 `SubagentBridge` 在转发时改写 `original_request_id`（破坏父子 Journal 对账）
- 父 Session 直接写子 Session 的 `PermissionResolved` 事件（绕开 Bridge → `decided_by = ParentForwarded` 缺失，replay 时分辨不出决策来源）
- 父代理把 `<task-notification>` XML 原样转述给终端用户，忽略 `<rewrite-hint>`（违反 CC-08 / OC-27 的"父需 rephrase"硬约束）
- 用业务层旋钮把 `policy.depth_cap` 调到无穷或在运行期突破它（破坏软/硬双闸隔离；任何提升必须落在 `HarnessBuilder` 装配期且需 ADR 记录）
- 通过 `SubagentAdmin::pause_spawning(true)` 长时间停滞却不通过 `subagent_admin_paused` gauge 通知运维（暂停应有时长上界、伴随告警，避免静默故障）
- 自定义 `AnnouncementRenderer` 时把 `summary` 与 `<rewrite-hint>` 都拿掉只留结构化 JSON（父代理失去"必须改写"提示，更可能直接外露给用户）
- 把 `SubagentSpec.quota` 等同于 `max_turns` 调小（前者按资源耗尽收敛 `MaxBudget`，后者按轮次收敛 `MaxIterationsReached`，两者语义不可互替）

## 14. 相关

- D6 · `agents-design.md` §3
- ADR-004 Agent Team 拓扑抽象
- `crates/harness-engine.md`
- `crates/harness-team.md`
- Evidence: HER-014, CC-08, OC-27
