# `octopus-harness-permission` · L1 原语 · Permission Broker + 规则引擎 SPEC

> 层级：L1 · 状态：Accepted
> 依赖：`harness-contracts`

## 1. 职责

实现 **权限审批模型 + 双 Broker 形态 + 规则引擎 + 危险命令库**。对齐 ADR-007（事件化）。

**核心能力**：

- 6 种权限模式（Default / Plan / AcceptEdits / BypassPermissions / DontAsk / Auto）
- 双 Broker：`DirectBroker`（回调）与 `StreamBasedBroker`（事件流）
- 规则引擎：allow / deny / priority / scope
- 危险命令正则库（对齐 HER-039）
- 决策持久化（Event + Rule）

## 2. 对外 API

### 2.1 核心 Trait

```rust
#[async_trait]
pub trait PermissionBroker: Send + Sync + 'static {
    async fn decide(
        &self,
        request: PermissionRequest,
        ctx: PermissionContext,
    ) -> Decision;

    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), PermissionError>;
}
```

### 2.2 核心类型

```rust
pub struct PermissionRequest {
    pub request_id: RequestId,
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub subject: PermissionSubject,        // contracts §3.4.1
    pub severity: Severity,
    pub scope_hint: DecisionScope,
    pub created_at: DateTime<Utc>,
}

pub struct PermissionContext {
    pub permission_mode: PermissionMode,
    pub previous_mode: Option<PermissionMode>,   // 进入 Plan 前的原 mode
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub interactivity: InteractivityLevel,       // contracts §3.4.1
    pub timeout_policy: Option<TimeoutPolicy>,   // contracts §3.4.1
    pub fallback_policy: FallbackPolicy,         // contracts §3.4.1
    pub rule_snapshot: Arc<RuleSnapshot>,
    pub hook_overrides: Vec<OverrideDecision>,
}

pub enum PermissionCheck {
    Allowed,
    Denied { reason: String },
    AskUser {
        subject: PermissionSubject,
        scope: DecisionScope,
    },
    DangerousCommand {
        pattern: String,
        severity: Severity,
    },
}
```

> `Decision` / `DecidedBy` / `DecisionScope` / `Severity` / `PermissionMode` / `PermissionSubject` /
> `InteractivityLevel` / `TimeoutPolicy` / `FallbackPolicy` / `RuleSource` / `ShellKind` 等核心枚举与
> 结构体均定义在 `harness-contracts §3.4 / §3.4.1`（L0 契约层），本 crate 仅 `use`，不重复声明。
> 详见 `crates/harness-contracts.md §3.4 / §3.4.1` 与 `permission-model.md §2 / §3 / §4`。

#### 2.2.1 调用契约（与 `harness-tool` / `harness-engine` 的边界）

`PermissionBroker::decide` **仅由 `harness-engine` 的 Tool Orchestrator 调用一次**；其他层不得直接调用：

| 层 | 责任 | 禁止 |
|---|---|---|
| `Tool::check_permission` | 投影 input → `PermissionCheck`（`Allowed` / `Denied` / `AskUser` / `DangerousCommand`），并把 `ExecFingerprint` / `subject` 准备好 | 不得自行调 `broker.decide`、不得直接访问 `RuleSnapshot` |
| `harness-engine` Orchestrator | 唯一调 broker 的层：装配 `PermissionRequest` + `PermissionContext`，按链顺序：rule_engine 预检 → hooks `PreToolUse` → broker.decide → 落 Event 对 → 通知 Sandbox | 不得跳过任一步骤；不得在 `decide` 之前执行 sandbox |
| `PermissionBroker` 实现方 | 给出 `Decision`；不得在 `decide` 内启动 Tool 调用 / Sandbox 执行 / Memory 写入 | 不得"边决策边执行"——决策是纯函数 |

**为什么这条契约重要**：

- 防止"重复询问"：若 Tool 与 Engine 都调 broker，会两次 `PermissionRequested` 写入 Journal，replay 时无法分辨语义。
- 保 causation 链：所有 `PermissionRequested` 的 `causation_id` 必须指向同一个 `ToolUseRequested`，唯一调用点是这条因果链的前提。
- 解耦升级路径：未来加 `ChainedBroker` / `AuxLlmBroker` / 远端审批桥时，调用方不变，只换链组合即可。

详见 `crates/harness-tool.md §调用契约` 与 `permission-model.md §10.1` 的层间边界。

### 2.3 AllowList

```rust
pub struct AllowList {
    entries: Vec<AllowEntry>,
    fingerprint_index: HashMap<ExecFingerprint, AllowEntryRef>,
    tenant_id: TenantId,
}

pub struct AllowEntry {
    pub scope: DecisionScope,
    pub decision: Decision,
    pub granted_at: DateTime<Utc>,
    pub granted_by: DecidedBy,
    pub expires_at: Option<DateTime<Utc>>,
    /// 仅当 `scope == DecisionScope::ExactCommand { .. }` 时填充；
    /// 由 `ExecSpec::canonical_fingerprint(&base)` 在写入时计算（见 `harness-sandbox.md` §2.2）。
    pub fingerprint: Option<ExecFingerprint>,
}

impl AllowList {
    /// 通用查询：按 scope 形态走对应索引。
    /// - `ExactCommand` → 必须命中 `fingerprint_index`，比对原始 `command` 字符串只为 UI/审计用，不参与匹配
    /// - `PathPrefix` / `GlobPattern` → 走线性扫描（条目通常 < 100）
    /// - 其他 variant → 直接 equal 比较
    pub fn contains(&self, scope: &DecisionScope) -> bool;

    /// `ExactCommand` 命中专用快路径：调用方（Shell 类 Tool）把 `ExecSpec`
    /// 投影成 `ExecFingerprint` 后直接查询，避免反复 canonicalize。
    pub fn lookup_by_fingerprint(&self, fp: &ExecFingerprint) -> Option<&AllowEntry>;

    pub fn apply(&mut self, resolved: &PermissionResolvedEvent);
    pub fn replay(events: impl Iterator<Item = &Event>) -> Self;
}
```

`AllowList` 本身就是一种 Projection：从 Event Journal 重建。

**指纹索引的生命周期**：

- `apply` 在写入 `Decision::AllowSession` / `AllowPermanent` 且 `scope == ExactCommand` 时，要求事件的 `scope_hint` envelope 元数据携带 `ExecFingerprint`（见 `event-schema.md §3.6`），否则视为校验失败。
- `replay` 在重建时按相同规则补齐 `fingerprint_index`；读取旧版未携带指纹的事件按 ADR-007 的迁移策略升版补算（不可静默忽略，否则匹配会发散）。
- 指纹**不进入** `Decision::DenyOnce` / `DenyPermanent` 的索引，这两类决策走危险命令库 / 规则引擎在 `decide()` 时即拒绝，不需要快路径。

## 3. 内置 Broker

### 3.1 `DirectBroker`

```rust
pub struct DirectBroker<F>
where
    F: Fn(PermissionRequest, PermissionContext) -> BoxFuture<'static, Decision>
        + Send + Sync + 'static,
{
    callback: F,
    persistence: Arc<dyn DecisionPersistence>,
}

impl<F> DirectBroker<F> {
    pub fn new(callback: F) -> Self;
    pub fn with_persistence(self, persistence: Arc<dyn DecisionPersistence>) -> Self;
}
```

业务层提供同步回调。典型场景：CLI 直接 `rprompt::prompt_reply`。

### 3.2 `StreamBasedBroker`

```rust
pub struct StreamBasedBroker {
    requests: mpsc::Sender<PermissionRequest>,
    resolutions: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Decision>>>>,
    persistence: Arc<dyn DecisionPersistence>,
    config: StreamBrokerConfig,
    pending: Arc<DashMap<RequestId, PendingResolution>>,
    sweeper: Option<JoinHandle<()>>,
}

pub struct StreamBrokerConfig {
    /// 等待 UI/外部决策的超时时间。`None` 时由 `PermissionContext::timeout_policy` 决定；
    /// 两者皆缺省时按 `Duration::from_secs(300)`（5 分钟）。
    pub default_timeout: Option<Duration>,
    /// 心跳间隔。Sweeper 每隔此周期对 `pending` 中超过 `heartbeat_interval`
    /// 没有动静的请求广播 `PermissionAwaitingHeartbeat`（详见 `event-schema.md`）；
    /// `None` 时不发送心跳。
    pub heartbeat_interval: Option<Duration>,
    /// `pending` 表的最大容量；超出后 `decide` 立即返回 `PermissionError::QueueOverflow`，
    /// 由 Engine 按 `FallbackPolicy` 兜底。防止 UI 永不消费导致内存膨胀。
    pub max_pending: usize,
}

struct PendingResolution {
    sender: oneshot::Sender<Decision>,
    request: PermissionRequest,
    enqueued_at: Instant,
    last_heartbeat_at: Instant,
}

impl StreamBasedBroker {
    pub fn new(config: StreamBrokerConfig)
        -> (Self, mpsc::Receiver<PermissionRequest>, ResolverHandle);
}

pub struct ResolverHandle {
    pending: Arc<DashMap<RequestId, PendingResolution>>,
}

impl ResolverHandle {
    pub async fn resolve(&self, request_id: RequestId, decision: Decision) -> Result<()>;
    /// 主动取消（如 UI 关闭 / Session 结束），不写 `PermissionResolved`，
    /// 由 Broker 写一条 `PermissionRequestCancelled` 事件并按 `FallbackPolicy` 兜底
    pub async fn cancel(&self, request_id: RequestId, reason: CancelReason) -> Result<()>;
}
```

典型场景：Desktop UI / Web UI 异步审批。Harness 把 `PermissionRequested` 事件推到 EventStream；业务层 UI 接收后 `resolver.resolve(id, decision)`。

**Sweeper 与超时**：

- 启动时由 `StreamBasedBroker::new` 拉起一个后台任务，按 `heartbeat_interval` 周期扫描 `pending`：
  - 超过 `default_timeout`（或 `PermissionContext::timeout_policy.deadline`）→ 用 `TimeoutPolicy::default_on_timeout`（缺省 `Decision::DenyOnce`）resolve，并写 `PermissionResolved { decided_by: Timeout { default } }`。
  - 距上次心跳超过 `heartbeat_interval` → 写 `PermissionAwaitingHeartbeat`（让 UI / 外部桥接器知道请求还活着）。
- `resolve` / `cancel` 命中后立刻把条目从 `pending` 移除；不需要等 sweeper 走过。
- Session 结束时 Engine 调 `ResolverHandle::cancel(...)` 清空 `pending`，避免 `oneshot::Sender` 永久滞留。

**禁止裸 `oneshot::Sender` 设计**：旧版"`HashMap<RequestId, oneshot::Sender>`"在 UI 永不答复时会内存泄漏；新设计强制带 sweeper 与 `max_pending` 上限。

### 3.3 `AllowAllBroker` / `DenyAllBroker`（testing）

```rust
pub struct AllowAllBroker;
pub struct DenyAllBroker;
```

用于测试。生产**禁用**。

### 3.4 `RuleEngineBroker`

```rust
pub struct RuleEngineBroker {
    snapshot: Arc<ArcSwap<RuleSnapshot>>,   // 支持原子替换
    rule_providers: Vec<Arc<dyn RuleProvider>>,
    fallback: FallbackPolicy,                // contracts §3.4.1
    dangerous_patterns: DangerousPatternLibrary,
    watch_task: Option<JoinHandle<()>>,
}

pub struct RuleSnapshot {
    pub rules: Vec<PermissionRule>,
    pub generation: u64,
    pub built_at: DateTime<Utc>,
}

pub struct PermissionRule {
    pub id: String,
    pub priority: i32,
    pub scope: DecisionScope,
    pub action: RuleAction,
    pub source: RuleSource,                  // contracts §3.4.1
}

pub enum RuleAction {
    Allow,
    Deny,
    AskWithDefault(Decision),
}
```

> `RuleSource` 是 9 元枚举，定义在 `harness-contracts §3.4.1`：
> `User < Workspace < Project < Local < Flag < Policy < CliArg < Command < Session`。
> 对齐 CC-14 的分层 + Octopus 多租户 `Policy` 源；具体合并语义见 `permission-model.md §4.1`。

#### 3.4.1 `RuleProvider` trait

```rust
#[async_trait]
pub trait RuleProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    fn source(&self) -> RuleSource;
    async fn resolve_rules(&self, tenant: TenantId)
        -> Result<Vec<PermissionRule>, PermissionError>;
    fn watch(&self) -> Option<BoxStream<RulesUpdated>>;
}

pub struct RulesUpdated {
    pub provider_id: String,
    pub tenant_id: TenantId,
    pub new_rules: Vec<PermissionRule>,
    pub at: DateTime<Utc>,
}
```

**内置实现**（覆盖契约层 9 元 RuleSource）：

| 实现 | RuleSource | 来源 |
|---|---|---|
| `SettingsRuleProvider` | `User` / `Workspace` / `Project` | 分层 settings 文件（按 `harness-context` settings 源） |
| `LocalRuleProvider` | `Local` | `~/.octopus/local-rules.json` 等本机覆写（不入版本管理） |
| `FlagRuleProvider` | `Flag` | feature flag / experiment 平台下发的临时规则 |
| `PolicyRuleProvider` | `Policy` | Admin-Trusted 通道下发（如 MDM、组织策略服务）；**只读**，不可被运行时持久化覆盖 |
| `CliArgRuleProvider` | `CliArg` | 解析 `--allow-tool xxx` / `--deny-tool yyy` 等 CLI 标志（仅 session 生命周期） |
| `SlashCommandRuleProvider` | `Command` | 用户在对话内的 `/permission allow ...` 命令 |
| `SessionRuleProvider` | `Session` | `Session::reload_with(ConfigDelta { permission_rule_patch })` 注入 |
| `FileRuleProvider`（通用底座） | 由实例化时指定 | 读任意 JSON / TOML，配合 `notify` watch 文件变更 |

**`PolicyRuleProvider` 的特殊约束**：

- `resolve_rules` 返回的规则 `source` 字段必须为 `RuleSource::Policy`，否则 `RuleEngineBroker::bootstrap` 拒绝加载并写 `Event::PluginRejected`（即把误配视同插件违规）。
- 所有 `Decision::DenyOnce` / `DenyPermanent` 视为硬闸门：合并阶段如果同 scope 出现低优先级源的 `Allow*`，直接丢弃后者；高优先级源（CliArg/Command/Session）出现的 `Allow*` 也无效（即 Policy Deny > 任何 Allow）。
- `DecisionPersistence::save` 必须拒绝 `Source = Policy` 的写入（即任何"运行时学到的"决策都不能写回 Policy 域）。

#### 3.4.2 Broker 与 RuleProvider 联动

```rust
impl RuleEngineBroker {
    pub fn builder() -> RuleEngineBrokerBuilder;
}

impl RuleEngineBrokerBuilder {
    pub fn with_rule_provider(mut self, p: Arc<dyn RuleProvider>) -> Self;
    pub fn with_dangerous_library(mut self, lib: DangerousPatternLibrary) -> Self;
    pub fn with_fallback(mut self, policy: FallbackPolicy) -> Self;
    pub async fn build(self) -> Result<RuleEngineBroker, PermissionError>;
}

impl RuleEngineBroker {
    /// 初始化：拉所有 provider 的规则，按 RuleSource 优先级合并成 RuleSnapshot
    async fn bootstrap(&self, tenant: TenantId) -> Result<RuleSnapshot, PermissionError>;

    /// 启动 watch 任务：订阅所有 provider 的 RulesUpdated 流，
    /// 用 debounce（默认 200ms）合并同 tick 内的多次更新，
    /// 原子替换 snapshot（ArcSwap）
    fn spawn_watch_task(&mut self);
}
```

**合并规则**：

1. 每个 Provider 产出自己 Source 的规则
2. 按 `RuleSource` 优先级（`User < Workspace < Project < Local < Flag < Policy < CliArg < Command < Session`）叠加
3. 同优先级内按 `PermissionRule.priority` 降序
4. **Policy Deny 二次扫描**：合并完成后再扫描一次 `RuleSource::Policy` 的 `Decision::Deny*`，把所有同 scope 的高优先级 `Allow*` 标记为不可达（不参与匹配）
5. 热更新时**只替换 snapshot 指针**，正在进行的 `decide()` 使用旧 snapshot（避免死锁）；下一 `decide()` 起用新 snapshot

### 3.5 `AuxLlmBroker`（feature `auto-mode`）

对齐 HER-040 的 smart mode。用辅助 LLM 判定 `APPROVE / DENY / ESCALATE`。仅用于 Claude Code `PermissionMode::Auto`（需 feature `auto-mode` 启用）。

> **依赖**：调用 `AuxModelProvider::call_aux(AuxTask::PermissionAdvisory, ...)`，绑定 `AuxOptions::fail_open = false`（安全相关任务不允许静默回退；详见 `crates/harness-model.md` §5.1）。Aux 缺失时直接抛 `ModelError::AuxModelNotConfigured`，绝不 fallback 到 primary 模型。

### 3.6 `ChainedBroker`（解决 `Decision::Escalate` 闭环）

`Decision::Escalate` 在契约层的语义是"我无法决定，下一个 Broker 来"——单一 Broker 无法消费这一返回值，必须由 `ChainedBroker` 串联多级。

```rust
pub struct ChainedBroker {
    chain: Vec<Arc<dyn PermissionBroker>>,
    terminator: Box<dyn PermissionTerminator>,
    persistence: Arc<dyn DecisionPersistence>,
}

#[async_trait]
pub trait PermissionTerminator: Send + Sync + 'static {
    /// 链尾兜底：拿到 `FallbackPolicy` 与上下文，返回**确定性** Decision（不能再返回 Escalate）
    async fn terminate(
        &self,
        request: &PermissionRequest,
        ctx: &PermissionContext,
    ) -> Decision;
}

impl ChainedBroker {
    pub fn builder() -> ChainedBrokerBuilder;
}

impl ChainedBrokerBuilder {
    /// 链上的 broker 按调用顺序加入；每个 broker 返回 Escalate 则进入下一个
    pub fn push(mut self, broker: Arc<dyn PermissionBroker>) -> Self;
    /// 终结者；不设置时默认 `FallbackTerminator(FallbackPolicy::AskUser)`
    pub fn terminator(mut self, t: Box<dyn PermissionTerminator>) -> Self;
    pub fn with_persistence(mut self, p: Arc<dyn DecisionPersistence>) -> Self;
    pub fn build(self) -> Result<ChainedBroker, PermissionError>;
}

#[async_trait]
impl PermissionBroker for ChainedBroker {
    async fn decide(&self, req: &PermissionRequest, ctx: &PermissionContext)
        -> Result<Decision, PermissionError>
    {
        for broker in &self.chain {
            match broker.decide(req, ctx).await? {
                Decision::Escalate => continue,
                final_decision   => return Ok(final_decision),
            }
        }
        Ok(self.terminator.terminate(req, ctx).await)
    }
}
```

**典型链组合**：

```text
ChainedBroker
  ├── RuleEngineBroker         ← 命中规则即终结
  ├── AuxLlmBroker (可选)      ← Auto 模式下尝试 LLM 决策
  ├── StreamBasedBroker        ← UI 询问（FullyInteractive 时）
  └── FallbackTerminator       ← 链尾兜底（NoInteractive / 超时一并由此处理）
```

**与 `InteractivityLevel` 的联动**：

- `NoInteractive` 上下文：跳过 `StreamBasedBroker`（直接 Escalate 给下一个），由 `FallbackTerminator` 按 `FallbackPolicy` 兜底。
- `DeferredInteractive` 上下文：`StreamBasedBroker` 仍写 `PermissionRequested`，但走父 Session 的 EventStream；超时由 `TimeoutPolicy` 决定。
- `FullyInteractive` 上下文：标准链路。

**约束**：`ChainedBroker` **必须**有终结者；构造时若链尾为 `AuxLlmBroker` / `StreamBasedBroker` 这类可能返回 Escalate 的 broker，`build()` 返回 `PermissionError::ChainNotTerminated`（防活锁）。

### 3.7 `FallbackPolicy` 与终结者

`FallbackPolicy` 定义在 `harness-contracts §3.4.1`，本节给出参考实现：

```rust
pub struct FallbackTerminator {
    policy: FallbackPolicy,
    history: Arc<dyn DecisionHistoryQuery>,   // 用于 ClosestMatchingRule
}

#[async_trait]
impl PermissionTerminator for FallbackTerminator {
    async fn terminate(
        &self,
        req: &PermissionRequest,
        ctx: &PermissionContext,
    ) -> Decision {
        match self.policy {
            FallbackPolicy::AskUser => match ctx.interactivity {
                InteractivityLevel::FullyInteractive => Decision::Escalate, // 不应发生（应已被链中 StreamBasedBroker 消费）
                _                                    => Decision::DenyOnce, // NoInteractive / DeferredInteractive 降级为 fail-closed
            },
            FallbackPolicy::DenyAll        => Decision::DenyOnce,
            FallbackPolicy::AllowReadOnly  => {
                if req.subject.is_read_only() { Decision::AllowOnce } else { Decision::DenyOnce }
            }
            FallbackPolicy::ClosestMatchingRule => {
                self.history.find_closest(&req.scope_hint)
                    .await
                    .map(|prior| prior.decision)
                    .unwrap_or(Decision::DenyOnce)
            }
        }
    }
}

#[async_trait]
pub trait DecisionHistoryQuery: Send + Sync + 'static {
    async fn find_closest(&self, scope: &DecisionScope) -> Option<PriorDecision>;
}
```

**`PriorDecision`**（projection 自 Event Journal）：

```rust
pub struct PriorDecision {
    pub scope: DecisionScope,
    pub decision: Decision,
    pub decided_at: DateTime<Utc>,
    pub decided_by: DecidedBy,
}
```

`DecisionHistoryQuery` 由 `harness-journal` projection 提供（与 `AllowList` 同源），不是新数据源——避免事实源漂移。

### 3.8 `DedupGate`（合流 + 短窗口复用）

`DedupGate` 是 `PermissionBroker` 的装饰器，位于 `ChainedBroker`（或单一 Broker）之外侧，专治"审批疲劳"——LLM 在流式输出 / 重试期间对同一语义的工具调用反复触发审批。语义模型见 `permission-model.md §6.3`。

```rust
pub struct DedupGate {
    inner: Arc<dyn PermissionBroker>,
    in_flight: Arc<DashMap<DedupKey, broadcast::Sender<Decision>>>,
    recent: Arc<RecentDecisionCache>,
    config: DedupConfig,
    suppression_counter: Arc<SuppressionCounter>,
}

#[derive(Clone)]
pub struct DedupConfig {
    /// 短窗口复用 TTL；`Duration::ZERO` 关闭窗口复用。
    pub recent_window: Duration,
    /// LRU 容量上限（按 dedup_key）。
    pub recent_cache_capacity: usize,
    /// 是否允许 in-flight 合流；测试场景可关。
    pub allow_inflight_join: bool,
    /// 单 `recent_window` 内最多写多少条 `PermissionRequestSuppressed`；
    /// 超阈值后只递增 `SuppressionCounter`，不再写新事件，防止刷屏。
    pub suppression_max_events_per_window: u32,
}

#[derive(Hash, Eq, PartialEq)]
pub struct DedupKey([u8; 32]);

impl DedupKey {
    /// `dedup_key = blake3(tenant_id || session_id ||
    ///                     subject.dedup_signature() || canonical(scope_hint))`
    /// `subject.dedup_signature()` 的取值表见 `permission-model.md §6.3.2`。
    pub fn derive(req: &PermissionRequest) -> Self;
}

struct RecentDecisionCache {
    /// 按 `SessionId` 分桶；每个 Session 一份独立 LRU，避免跨 Session 的
    /// `dedup_key` 冲突（同一 fingerprint 在两个 session 应被视为不同请求）。
    /// 桶内 `parking_lot::Mutex<LruCache<..>>`：临界区只做内存读写，
    /// 持锁时间 < 5µs，**不**用 `tokio::Mutex`（避免 `decide()` 阻塞 reactor）。
    buckets: dashmap::DashMap<SessionId, SessionBucket>,
    window: Duration,
    capacity_per_session: usize,
}

struct SessionBucket {
    entries: parking_lot::Mutex<lru::LruCache<DedupKey, RecentEntry>>,
    /// 桶级单调时钟，仅供测试 mock；生产用 `Instant::now()`。
    now_fn: Arc<dyn Fn() -> Instant + Send + Sync>,
}

struct RecentEntry {
    request_id: RequestId,
    decision_id: DecisionId,
    decision: Decision,
    decided_by: DecidedBy,
    decided_at: Instant,
}
```

#### 3.8.1 `RecentDecisionCache` 的 LRU 策略

| 维度 | 策略 | 备注 |
|---|---|---|
| **桶分区** | 按 `SessionId` 分独立桶 | 跨 Session 的 dedup 通过 `DedupKey` 已经 blake3 入哈希，但桶分区把"清理 / 容量 / 锁竞争"也做隔离 |
| **桶内逐出（容量）** | LRU；满桶（达 `capacity_per_session`，默认 256）时弹出**最久未访问**条目 | `lru::LruCache::put` 自带；命中时按"访问时间"刷头部 |
| **条目失效（时间）** | **Lazy expire** —— `get` 时若 `now - decided_at >= window` 视作 miss 并立即 remove | 不起后台线程做主动扫描；窗口默认 5s 时窗口过短不值 |
| **桶级清理** | `Session::drop` / `Engine::shutdown` 时调 `RecentDecisionCache::evict_session(session_id)` 整桶清空 | 防长 session 泄漏；详见 §3.8.3 |
| **持锁时间** | 临界区只做哈希查 / 时间比较 / `Decision` 克隆 | `Decision` 是 `Copy`-like 小枚举，clone 成本忽略 |
| **并发读** | 同 session 内通过 `Mutex` 串行；不同 session 走 `DashMap` 分片，读不互斥 | 标准 `dashmap` 分片粒度由 ncpu 决定 |

**复杂度**：

- `get` / `put` 期望 O(1)：`DashMap::get` O(1) → `Mutex::lock` 微秒级 → `LruCache::{get, put}` O(1)。
- `evict_session` O(N)：N 为该 session 的条目数（≤ `capacity_per_session`）。

**逐出统计**（落入 §12 指标）：

```rust
pub struct EvictionTelemetry {
    pub by_capacity: AtomicU64,    // LRU 满桶逐出
    pub by_window:   AtomicU64,    // lazy expire 命中
    pub by_session:  AtomicU64,    // Session::drop 触发
}
```

#### 3.8.2 `recent` 命中判定

```rust
impl RecentDecisionCache {
    fn lookup(&self, session_id: SessionId, key: &DedupKey) -> Option<RecentEntry> {
        let bucket = self.buckets.get(&session_id)?;
        let mut g = bucket.entries.lock();
        let entry = g.get(key)?;            // O(1) + bumps LRU head
        let now = (bucket.now_fn)();
        if now.saturating_duration_since(entry.decided_at) >= self.window {
            g.pop(key);                     // lazy expire
            self.telemetry.by_window.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        Some(entry.clone())
    }

    fn record(&self, session_id: SessionId, key: DedupKey, entry: RecentEntry) {
        let bucket = self.buckets
            .entry(session_id)
            .or_insert_with(|| SessionBucket::new(self.capacity_per_session));
        let mut g = bucket.entries.lock();
        if let Some(_evicted) = g.put(key, entry) {
            self.telemetry.by_capacity.fetch_add(1, Ordering::Relaxed);
        }
    }
}
```

**关键不变量**：

1. `lookup` 命中只在"未过 `window` 且键存在"时返回，**不**做"软续期"——dedup 是疲劳治理，不是 cache 续命。
2. `record` 在**链路返回真实决策后**调用；`AllowOnce` / `DenyOnce` 都进缓存（与 §6.3.5 默认一致）；仅"被合流方"不再 record（因为它本就没走完整链路）。
3. `Decision::Escalate` **永不** record——它不是终态，进了缓存会污染下一次复用。

#### 3.8.3 与 Session / Run 生命周期的耦合

| 时机 | 行为 | 责任方 |
|---|---|---|
| `Session::create` | 桶按需在 `record` 时延迟创建（不预分配） | `DedupGate` 内部 |
| `Run::end` | **不**清桶（同一 Session 跨 Run 仍可复用最近决策） | — |
| `Session::drop` / `Engine::shutdown` | 调 `RecentDecisionCache::evict_session(session_id)` 整桶清空 | `harness-session` 在 `Drop` 实现里调；`harness-engine` `shutdown` 兜底调一次 |
| `PermissionMode` 切换 | **不**清桶（`PermissionMode` 是上下文，不是缓存键的一部分；切到 `Plan` 后由 §3.8 流程的"强制独立"分支跳过 dedup 即可） | `DedupGate.decide` |
| `RuleSnapshot` 热更新 | **不**清桶（dedup 缓存只复用最近 `inner.decide` 的结果；规则变化后旧缓存条目在 `recent_window` 过期前最多再被复用一次，属可接受窗口） | — |

**长 session 漂移防护**：若 `recent_cache_capacity` 长期接近上限（指标 `permission_dedup_recent_cache_size` 见 §12），说明业务对同一 session 的命令多样性极高，应：

- 缩小 `recent_window`（比如 5s → 2s），让 lazy expire 更激进；
- 或拉大 `capacity_per_session` 并接受额外内存（每条目 ~ 200B，256 → 1024 仅 0.2MB / session）；
- 不建议关闭 dedup —— LLM 突发并发是真实场景，关闭会立即把审批 UI 打爆。

#### 3.8.4 `in_flight` 表的并发与广播

```rust
struct InFlightEntry {
    sender: tokio::sync::broadcast::Sender<Decision>,
    /// 用于审计：被合流方写 `PermissionRequestSuppressed.original_request_id` 时引用。
    leader_request_id: RequestId,
    leader_started_at: Instant,
}

type InFlightTable = dashmap::DashMap<DedupKey, InFlightEntry>;
```

**约束**：

- `broadcast::channel(capacity = 64)`：单条 `decide` 决策最多让 64 个并发请求合流；超出 64 的并发请求按"领导者已写决策、本次按缓存命中处理"降级，不报错。
- 第一个写入 `in_flight[key]` 的请求是 **leader**；后续合流方调 `sender.subscribe()` 拿 `Receiver` 等结果，**不**进入 `inner.decide()`。
- leader 的 `inner.decide()` 完成后：
  1. `sender.send(decision)` 通知所有 follower（broadcast 容量满了就丢，但 follower 既然已订阅就一定收到了 send 之前的消息——`broadcast` 的语义）；
  2. `record(...)` 写入 `recent`；
  3. `in_flight.remove(key)` 让下一波请求看 `recent` 命中。
- leader 异常 panic / 取消时由 `tokio::select!` 兜底：`InFlightEntry::sender` 被 drop 时所有 `Receiver` 收到 `RecvError::Closed`，follower 把这视作"领导者放弃"，回 `inner.decide()` 自己重试一次（**不**复用未到达的决策）。


**装配位置**（推荐）：

```text
DedupGate
  └── ChainedBroker
        ├── RuleEngineBroker
        ├── AuxLlmBroker (可选)
        ├── StreamBasedBroker
        └── FallbackTerminator
```

**`decide()` 流程**：

1. 依据 `req` + `ctx.permission_mode` + `req.severity` 判断是否**强制独立**：
   - `req.severity == Severity::Critical` 且 `subject` 是 `DangerousCommand` → **跳过** dedup，直接走 `inner.decide(...)`。
   - `ctx.permission_mode == PermissionMode::Plan` → 同上跳过（Plan 模式逐次评估）。
2. 计算 `key = DedupKey::derive(&req)`。
3. 查 `recent` 缓存：
   - 命中 `Allow*` → 直接返回缓存的 `Decision`，写 `Event::PermissionRequestSuppressed { reason: RecentlyAllowed, .. }`。
   - 命中 `Deny*` → 同样直接返回，写 `Event::PermissionRequestSuppressed { reason: RecentlyDenied, .. }`。
   - 命中 `Timeout { default }` → 同上，`reason: RecentlyTimedOut`。
   - 命中已过 `recent_window` → 移除缓存项，转步骤 4。
4. 查 `in_flight`：若存在同 key 的 `broadcast::Sender` 且 `allow_inflight_join == true`：
   - 订阅其 `broadcast::Receiver` 等待结果，写 `Event::PermissionRequestSuppressed { reason: JoinedInFlight, .. }`。
   - 拿到决策后**不**单独写 `PermissionResolved`（原请求那条已经写过）。
5. 否则注册 `in_flight[key] = sender`，调 `inner.decide(req, ctx).await`：
   - 决策返回后 `sender.send(decision)` 通知所有等待者；
   - 写入 `recent`（即使是 `AllowOnce` 也写入——本窗口内的下一次相同请求继续走 §6.3 复用，避免 LLM 突发重试拉爆 UI）；
   - 从 `in_flight` 移除 key。

**与 `Severity` 的耦合**：

- `Severity::High` 的 Deny 仍参与窗口复用（典型例：用户首次拒绝 `git push --force` 后，5s 内 LLM 重试同命令应直接 Deny，不再骚扰）；
- `Severity::Critical` 强制旁路（典型例：危险命令库命中），保证危险操作每次都进入完整链路。

**事件守恒**：被去重命中的请求**不**发 `PermissionRequested` / `PermissionResolved`，仅发 `PermissionRequestSuppressed`，并通过 `original_request_id` / `original_decision_id` 反查原决策；详见 `event-schema.md §3.6.3` 与 `permission-model.md §6.3.3`。

**与 `InteractivityLevel` 的联动**：

- `NoInteractive`：dedup 在 `inner.decide` 之前就拦下重复请求，可显著降低 `FallbackTerminator` 的重复执行成本（如 `ClosestMatchingRule` 走 `DecisionHistoryQuery` 的相同查询）。
- `DeferredInteractive`：合流逻辑与 `SubagentBridge`（`crates/harness-subagent.md §6.2`）兼容——子代理对同 fingerprint 的两次请求，父 Session 只看到一次 `PermissionRequested`。

**装配检查**：`HarnessBuilder::with_permission_broker` 接到 `Arc<dyn PermissionBroker>` 时，若链头不是 `DedupGate` 也允许装配（业务可显式关闭），但 `with_dedup_default()` 是推荐姿势：

```rust
let broker = HarnessBuilder::new()
    .with_permission_broker(
        DedupGate::wrap(
            ChainedBroker::builder()
                .push(rule_broker)
                .push(stream_broker)
                .terminator(Box::new(FallbackTerminator { policy, history }))
                .build()?,
            DedupConfig::default(),
        ),
    )
    .build()
    .await?;
```

## 4. 危险命令库（对齐 HER-039）

```rust
pub struct DangerousPatternLibrary {
    patterns: Vec<DangerousPatternRule>,
}

pub struct DangerousPatternRule {
    pub id: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub description: String,
}

impl DangerousPatternLibrary {
    /// 仅 Unix 模式（默认单机 Linux/macOS 使用）
    pub fn default_unix() -> Self {
        Self {
            patterns: vec![
                Self::rm_rf_root(),
                Self::chmod_777_root(),
                Self::curl_pipe_sh(),
                Self::git_force_push_main(),
                Self::fork_bomb(),
                Self::shutdown_unix(),
                // ... ~30 条默认模式
            ],
        }
    }

    /// 仅 Windows/PowerShell 模式
    pub fn default_windows() -> Self {
        Self {
            patterns: vec![
                Self::pwsh_remove_item_recurse_root(),   // Remove-Item -Recurse -Force C:\
                Self::pwsh_format_volume(),              // Format-Volume
                Self::pwsh_stop_computer(),              // Stop-Computer / Restart-Computer
                Self::pwsh_diskpart_clean(),             // diskpart with "clean"
                Self::pwsh_iwr_invoke_expression(),      // IWR ... | IEX
                Self::pwsh_download_invoke(),            // (New-Object Net.WebClient).DownloadString | IEX
                Self::cmd_del_rq_root(),                 // del /s /q C:\
                Self::cmd_rmdir_rq_root(),               // rmdir /s /q C:\
                Self::pwsh_set_executionpolicy_unrestricted(),
                Self::pwsh_disable_defender(),           // Set-MpPreference -DisableRealtimeMonitoring
                Self::pwsh_add_trusted_publisher(),
                Self::pwsh_registry_run_key(),           // 写 HKCU:\Software\...\Run
                // ... 补齐常见 Windows 破坏性模式
            ],
        }
    }

    /// 跨平台（按 Shell 类型动态启用对应模式；服务端/多平台场景推荐）
    pub fn default_all() -> Self;

    pub fn detect(&self, command: &str) -> Option<&DangerousPatternRule>;
}

/// `ShellKind` 已下沉到 `harness-contracts §3.4`（`harness-sandbox` re-export）；
/// 此处保留 `ShellKindAware` 让具体 SandboxBackend 在不暴露完整类型的前提下汇报 shell 形态。
pub trait ShellKindAware {
    fn shell_kind(&self) -> ShellKind;
}

impl RuleEngineBroker {
    /// 根据 Sandbox 的 ShellKind 自动选择默认库
    pub fn with_platform_dangerous_library(
        mut self,
        shell_kind: ShellKind,
    ) -> Self {
        self.dangerous_patterns = match shell_kind {
            ShellKind::Bash(_) | ShellKind::Zsh(_) => DangerousPatternLibrary::default_unix(),
            ShellKind::PowerShell => DangerousPatternLibrary::default_windows(),
            ShellKind::System => DangerousPatternLibrary::default_all(),
        };
        self
    }
}

fn _normalize_command_for_detection(cmd: &str) -> String {
    let stripped = strip_ansi_escapes::strip_str(cmd);
    unicode_normalization::UnicodeNormalization::nfkc(stripped.chars()).collect()
}
```

**行为**：命中危险模式时，即使 `PermissionMode::BypassPermissions` 或用户 `AllowPermanent`，仍强制询问（`DangerousCommand` severity Critical 的动作）。

## 5. Permission Mode 语义

```rust
pub enum PermissionMode {
    Default,
    Plan,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Auto,
}

impl PermissionMode {
    pub fn policy(&self) -> ModePolicy {
        match self {
            Self::Default => ModePolicy {
                ask_if_no_rule: true,
                readonly_only: false,
                auto_approve_edits: false,
            },
            Self::Plan => ModePolicy {
                ask_if_no_rule: false,  // 直接 Deny
                readonly_only: true,
                auto_approve_edits: false,
            },
            Self::AcceptEdits => ModePolicy {
                ask_if_no_rule: true,
                readonly_only: false,
                auto_approve_edits: true,
            },
            Self::BypassPermissions => ModePolicy {
                ask_if_no_rule: false,  // 直接 Allow（除非危险命令）
                readonly_only: false,
                auto_approve_edits: true,
            },
            Self::DontAsk => ModePolicy {
                ask_if_no_rule: false,  // 直接 Deny
                readonly_only: false,
                auto_approve_edits: false,
            },
            Self::Auto => ModePolicy {
                ask_if_no_rule: true,   // 用 Aux LLM
                readonly_only: false,
                auto_approve_edits: false,
            },
        }
    }
}
```

## 6. 决策持久化

```rust
#[async_trait]
pub trait DecisionPersistence: Send + Sync + 'static {
    async fn save(&self, decision: &ResolvedDecision) -> Result<(), PermissionError>;

    /// 通用查找：按 scope 形态分流到对应索引（`ExactCommand` 走 fingerprint）。
    async fn load(&self, tenant: TenantId, scope: &DecisionScope) -> Option<ResolvedDecision>;

    /// `ExactCommand` 命中专用快路径，与 `AllowList::lookup_by_fingerprint` 对偶。
    /// Shell 类 Tool 在 `check_permission` 路径里用本方法做"已授权？"的快速短路。
    async fn load_by_fingerprint(
        &self,
        tenant: TenantId,
        fingerprint: &ExecFingerprint,
    ) -> Option<ResolvedDecision>;

    async fn revoke(&self, decision_id: DecisionId) -> Result<(), PermissionError>;
}

pub struct ResolvedDecision {
    pub id: DecisionId,
    pub tenant_id: TenantId,
    pub scope: DecisionScope,
    /// 当 `scope == DecisionScope::ExactCommand { .. }` 时必填。
    pub fingerprint: Option<ExecFingerprint>,
    pub decision: Decision,
    pub decided_at: DateTime<Utc>,
    pub decided_by: DecidedBy,
}
```

内置：`JournalBasedPersistence`（存 Event）、`FilePersistence`（`~/.octopus/permission-rules.json`）。

**索引一致性**：

- `JournalBasedPersistence`：`load_by_fingerprint` 直接走 `AllowList` 的 `fingerprint_index`，不重新扫描 Journal。
- `FilePersistence`：磁盘文件按 `tenant_id × ExecFingerprint(hex)` 分桶存储，`save` 与 `revoke` 同时维护文件名索引；指纹算法升版时随 ADR-007 的迁移路径整体重写，不做就地兼容。
- 指纹算法变更属于破坏性升级（参见 `permission-model.md` §2.3.1）；`DecisionPersistence` 实现必须把"持久化时的指纹算法版本"写入 `ResolvedDecision` 的元数据（落 `EventEnvelope.schema_version` 即可），否则跨版本读取会静默错误命中。

### 6.1 完整性签名（HMAC）

`FilePersistence` 写到磁盘的 `permission-rules.json` 是**离线可改写**的资产：任何能读 `~/.octopus/` 的进程都能在 SDK 不知情的情况下追加/篡改 `Decision::AllowPermanent` 行，绕过审批闸门。本节定义统一的完整性保护契约；具体的默认实现选型（算法、canonical bytes、密钥治理、轮换窗口、fail-closed 不可关）见 **ADR-0013 · `IntegritySigner` 默认实现与密钥治理**。

```rust
/// 用于 `FilePersistence`、Plugin manifest、长期 cache 等"离线可写"资产的
/// HMAC 完整性保护契约。所有实现要求：
/// - 算法默认 HMAC-SHA256（FIPS 140-2 兼容）；
/// - 密钥来自 `CredentialSource`（见 `security-trust.md §6.1`）；
/// - 验签失败 fail-closed（永不回退到"未签状态"）。
#[async_trait]
pub trait IntegritySigner: Send + Sync + 'static {
    fn algorithm(&self) -> IntegrityAlgorithm;
    fn key_id(&self) -> &str;

    async fn sign(&self, payload: &[u8]) -> Result<IntegritySignature, PermissionError>;
    async fn verify(
        &self,
        payload: &[u8],
        signature: &IntegritySignature,
    ) -> Result<(), IntegrityError>;
}

#[non_exhaustive]
pub enum IntegrityAlgorithm {
    HmacSha256,
    HmacSha512,
}

pub struct IntegritySignature {
    pub algorithm: IntegrityAlgorithm,
    pub key_id: String,
    pub mac: Bytes,
    pub signed_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum IntegrityError {
    #[error("signature mismatch")]
    Mismatch,
    #[error("unknown key id: {0}")]
    UnknownKeyId(String),
    #[error("algorithm downgrade not allowed: stored={stored:?}, expected={expected:?}")]
    AlgorithmDowngrade {
        stored: IntegrityAlgorithm,
        expected: IntegrityAlgorithm,
    },
    #[error("missing signature")]
    Missing,
}
```

`FilePersistence` 落盘格式（每个 `tenant_id × ExecFingerprint(hex)` 文件）：

```json
{
  "schema_version": 3,
  "fingerprint_alg": "blake3-v1",
  "decision": { "...": "..." },
  "signature": {
    "algorithm": "HmacSha256",
    "key_id": "octopus-permission-2026-04",
    "mac": "<base64>",
    "signed_at": "2026-04-25T10:32:11Z"
  }
}
```

**写路径**（`FilePersistence::save`）：

1. 序列化 `decision` 与所有非 signature 字段为 canonical JSON（按字段名排序，禁 BOM，UTF-8）。
2. `IntegritySigner::sign` 对 canonical 字节计算 MAC。
3. 整体写入临时文件 + `rename` 原子替换；任一步失败返回 `PermissionError::Persistence`，**不**留下半写文件。

**读路径**（`FilePersistence::load_by_fingerprint`）：

1. 解析 JSON，提取 `signature` 字段后清空，重新生成 canonical 字节。
2. `IntegritySigner::verify` 对 canonical 字节比对 MAC。
3. 验签失败时：
   - **不**返回该 `ResolvedDecision`（fail-closed，命中走"无规则"路径回 `RuleEngineBroker`）；
   - 写出 `Event::PermissionPersistenceTampered { tenant_id, file_path_hash, fingerprint, reason, at }`；
   - 把"被污染的文件"重命名为 `<file>.tampered.<ts>` 备份保留；
   - 必须不阻塞主流程，但 Audit / 监控 hook 应当订阅本事件触发告警。

**密钥与 key_id 管理**：

- `IntegritySigner` 的密钥从 `CredentialSource::fetch(CredentialKey { tenant_id, provider_id: "octopus-permission", key_label: "<key_id>" })` 注入；SDK 不持久化密钥到磁盘。
- 密钥轮换：业务层调用 `CredentialSource::rotate` 后注入新 `key_id` 给 `FilePersistence`，旧记录走"读到旧 key_id 即重签"的延迟迁移；`IntegrityError::UnknownKeyId` 视同 `Mismatch`，发同样的 `PermissionPersistenceTampered`（避免 key 删除变成静默失效）。
- 算法降级硬封禁：`IntegrityError::AlgorithmDowngrade` 永不允许；若需迁移 SHA-256 → SHA-512，必须整体重签（Schema 版本号 +1）。

**约束**：

- `JournalBasedPersistence` 走 Event Journal，已有 ADR-001 的 append-only 与 schema_version 守护，不**额外**走 HMAC（Journal 的篡改保护属于持久层职责）。
- `DecisionPersistence` 的所有外部实现（业务层自定义）**必须**至少满足"保密 + 完整性"二选一：要么走加密通道（如 Vault HTTP API），要么承袭 `IntegritySigner` 接口；SDK 在 `HarnessBuilder::with_decision_persistence` 装配时校验 `Persistence::supports_integrity()`，未声明的纯本地文件实现 fail-closed 拒绝装载。

## 7. Hook 联动

```rust
pub enum OverrideDecision {
    FromHook {
        handler_id: String,
        decision: Decision,
    },
}
```

Hook 在 `PermissionRequest` 事件下可返回 `HookOutcome::OverridePermission(Decision)`；在 `PreToolUse` 事件下则通过 `HookOutcome::PreToolUse(PreToolUseOutcome { override_permission: Some(Decision), .. })` 复合三件套提供（`PreToolUse` 事件不再支持单一 `HookOutcome::OverridePermission`，详见 `crates/harness-hook.md §2.3 / §2.4`）。

此 override 优先级最高；多 handler 在同 priority 下给出冲突决策（如 A: AllowOnce、B: DenyOnce）时按"Deny 压过 Allow"裁决，并写一条 `Event::HookPermissionConflict`（结构见 `event-schema.md §3.7.2`）。

## 8. Feature Flags

```toml
[features]
default = []
interactive = []
stream = []
rule-engine = []
mock = []
auto-mode = ["dep:octopus-harness-model"]
```

## 9. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("broker timeout")]
    Timeout,
    #[error("broker closed")]
    BrokerClosed,
    #[error("rule parse: {0}")]
    RuleParse(String),
    #[error("persistence: {0}")]
    Persistence(String),
    /// 持久化记录完整性校验失败（HMAC 不匹配 / 算法降级 / key_id 未知）。
    /// 该错误必伴随 `Event::PermissionPersistenceTampered`；
    /// 调用方应当把命中视为"无规则"，回 `RuleEngineBroker` 重新决策。
    #[error("integrity: {0}")]
    Integrity(#[from] IntegrityError),
}
```

## 10. 使用示例

### 10.1 DirectBroker（CLI）

```rust
let broker = DirectBroker::new(|req, _ctx| async move {
    println!("{}", req.subject);
    let answer = rprompt::prompt_reply("[a]llow once / [A]lways / [d]eny: ").unwrap();
    match answer.as_str() {
        "a" => Decision::AllowOnce,
        "A" => Decision::AllowPermanent,
        _ => Decision::DenyOnce,
    }
});

let harness = HarnessBuilder::new()
    .with_permission_broker(broker)
    .build()
    .await?;
```

### 10.2 StreamBasedBroker（UI）

```rust
let (broker, mut rx, resolver) = StreamBasedBroker::new();

tokio::spawn(async move {
    while let Some(req) = rx.recv().await {
        let decision = ui.ask_permission(&req).await;
        resolver.resolve(req.request_id, decision).await.unwrap();
    }
});

let harness = HarnessBuilder::new()
    .with_permission_broker(broker)
    .build()
    .await?;
```

### 10.3 RuleEngineBroker

```rust
let rules = vec![
    PermissionRule {
        id: "allow-read-only-tools".into(),
        priority: 100,
        scope: DecisionScope::Category("readonly".into()),
        action: RuleAction::Allow,
        source: RuleSource::Workspace,
    },
    PermissionRule {
        id: "deny-prod-rm".into(),
        priority: 200,
        scope: DecisionScope::GlobPattern("rm * /prod/**".into()),
        action: RuleAction::Deny,
        source: RuleSource::Project,
    },
];

let broker = RuleEngineBroker::builder()
    .with_rules(rules)
    .with_dangerous_library(DangerousPatternLibrary::default())
    .with_fallback(FallbackPolicy::AskUser)
    .build();
```

## 11. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 6 种 Mode 的 policy、规则优先级、危险模式命中 |
| Mock | AllowAll / DenyAll 在 Tool 调用链测试 |
| Stream | 队列 FIFO、多次 resolve 只认第一次 |
| 持久化 | Event 重建 AllowList 的结果等价于直接应用 Decision |
| 安全 | strip_ansi + NFKC 处理绕过尝试（如 `rm\u200b -rf /`） |
| `DedupGate` | in-flight 合流广播一致；`recent_window` 过期；`Severity::Critical` 强制旁路；`PermissionRequestSuppressed` 与原决策的 `original_request_id` 对齐 |
| `DedupGate × SubagentBridge` | 见 §11.1 交叉单测矩阵（父子审批 × 去重 × `InteractivityLevel` × Forwarded 事件） |
| `IntegritySigner` | `DefaultHmacSigner` canonical bytes 排序稳定性；fail-closed 路径写 `PermissionPersistenceTampered`；30 天 legacy_key_grace；详见 ADR-0013 §4 落地清单 |

### 11.1 `DedupGate × SubagentBridge` 交叉单测矩阵

`DedupGate`（§3.8）与 `SubagentBridge`（`harness-subagent.md §6.2`）是两条相互正交但**频繁共现**的链路——子代理在父 Session 之外发审批请求，父 Session 端 dedup gate 仍要正确合流 / 窗口复用。下表是这两个机制交叉时必须覆盖的最小用例集；每一行都对应一组明确的 `Event` 序列断言。

| # | 场景 | 配置 | 期望事件序列 | 关注点 |
|---|---|---|---|---|
| **D-S1** | 子代理对同 fingerprint 连发 2 次（窗口内） | `interactivity = DeferredInteractive`，`recent_window = 5s` | 父侧：1 × `SubagentPermissionForwarded` → 1 × `PermissionRequested` → 1 × `PermissionResolved` → 1 × `SubagentPermissionResolved`；子侧：1 × `PermissionRequestSuppressed { reason: RecentlyAllowed }` → 1 × `SubagentPermissionResolved`（取自缓存） | 验证父 Session 只看一次，子代理仍能拿到决策 |
| **D-S2** | 子代理并发对同 fingerprint 发 5 次（同一 turn） | 同上，`allow_inflight_join = true` | 父侧只 1 个 `PermissionRequested`；子侧 1 × `Forwarded`（leader）+ 4 × `PermissionRequestSuppressed { reason: JoinedInFlight, original_request_id == leader.request_id }` | 验证 in-flight 合流跨进程边界仍工作（父 Engine 与子 Engine 是同进程不同 task） |
| **D-S3** | 父 Session deny 后 5s 内子代理再请求同 fingerprint | `recent_window = 10s` | 父侧无新事件；子侧 1 × `PermissionRequestSuppressed { reason: RecentlyDenied, reused_decision: DenyOnce }` | 验证"父侧拒绝可静默被子侧复用"的安全特性——LLM 不能通过 spawn 子代理绕开父侧拒绝 |
| **D-S4** | 子代理跑 `Severity::Critical` 危险命令两次 | 同上 | 两次都走完整链路：2 × `Forwarded` + 2 × `Requested` + 2 × `Resolved` + 2 × `SubagentPermissionResolved`；**不应**出现 `PermissionRequestSuppressed` | 验证 `Severity::Critical` 强制旁路在子代理路径同样生效 |
| **D-S5** | 子代理 `interactivity = NoInteractive`，无规则匹配 | `fallback_policy = DenyAll` | 子侧立即 `Resolved { decision: DenyOnce, decided_by: Fallback(DenyAll) }`；**无** `Forwarded`（fallback 在子代理本地终结，不打扰父侧） | 验证 `NoInteractive` 子代理不冒泡；与 §6.3 "Subagent 受信任脚本式" 行对齐 |
| **D-S6** | 子代理跨多 turn 复用决策（父 Session lifetime 内） | `recent_window = 60s` | 第二个 turn 的同 fingerprint 子代理请求直接 hit；父侧无新 `Forwarded` | 验证 `RecentDecisionCache` 桶绑定 SessionId 而非 RunId（§3.8.3） |
| **D-S7** | Session::drop 后再次创建同 SessionId（极少，需重建） | — | 旧桶在 `evict_session` 后清空；新 session 的同 fingerprint 第一次请求**必然**走完整链路 | 防"幽灵决策"——drop 后还能复用旧桶决策属于安全漏洞 |
| **D-S8** | 父 Session `Plan` 模式，子代理也走 `Plan` | — | 父子两端 dedup 都跳过；每次请求都进 `inner.decide`，`Plan` 模式逐次评估 | 验证 §3.8 流程的"Plan 模式跳过 dedup"在 forwarded 链路上同样生效 |
| **D-S9** | leader 子代理的 `inner.decide` panic | `tokio::select!` 兜底 | follower 收 `RecvError::Closed`，回 `inner.decide` 各自重试；不会写出"未到达的决策"到 `recent` | 验证异常下的事件守恒（§3.8.4） |
| **D-S10** | `suppression_max_events_per_window` 命中 | 默认 50 | 第 51 次起的合流命中只递增 `permission_dedup_suppressed_total`，**不**写 `PermissionRequestSuppressed` 事件；父侧 `Forwarded` 计数也对应不增加 | 验证刷屏防护与父侧事件守恒一致 |

**测试装配要求**：

- 矩阵全部跑在**真实 `Engine` + 真实 `SubagentRunner`** 之上，**不**用 `MockSubagentRunner` 做端到端断言；mock 仅用于 `inner` broker 侧（`MockBroker` 控制返回的 `Decision`），保证父子 forwarding 的 event-sourcing 路径被覆盖。
- 测试 fixture 提供 `DedupConfig::tight()`：`recent_window = 100ms`、`recent_cache_capacity = 8`、`suppression_max_events_per_window = 4`，便于在毫秒级断言事件序列。
- 同步对偶测试集见 `crates/harness-subagent.md §11.1`（同矩阵，从子代理视角断言）。


## 12. 可观测性

| 指标 | 说明 |
|---|---|
| `permission_requests_total` | 按 tool × mode 分桶 |
| `permission_decisions_total` | 按 decision × decided_by 分桶 |
| `permission_dangerous_hits_total` | 危险模式命中次数 |
| `permission_rule_evaluations` | 规则评估数 |
| `permission_broker_latency_ms` | Broker 响应时间 |
| `permission_dedup_suppressed_total` | 按 `reason`（`JoinedInFlight` / `RecentlyAllowed` / `RecentlyDenied` / `RecentlyTimedOut`）分桶；超 `suppression_max_events_per_window` 后只更指标不发事件 |
| `permission_dedup_recent_cache_size` | `RecentDecisionCache` 当前条目数；用于检查是否长期接近 `recent_cache_capacity` |

## 13. 反模式

- 在 Broker 里做复杂 IO（应用 AsyncCallback）
- 把危险命令检测放在 Hook 里（应在 Broker 层统一）
- `AllowPermanent` 不写持久化（跨重启丢失）
- `Tool::check_permission` 内自行调用 `PermissionBroker::decide`（破坏因果链与单点询问；详见 §2.2.1 调用契约）
- `StreamBasedBroker` 不带 sweeper / `max_pending` 上限（UI 永不答复时内存泄漏；详见 §3.2）
- `ChainedBroker` 链尾未挂终结者（`Decision::Escalate` 活锁；`build()` 必须返回 `ChainNotTerminated`）
- 把 `RuleSource::Policy` 当作可写源（运行时持久化覆盖企业下发文件；`DecisionPersistence::save` 必须拒绝）
- 后台 / Subagent 上下文复用主 Session 的 `InteractivityLevel`（应由调用方按上下文显式设置 `NoInteractive` / `DeferredInteractive`）
- `FilePersistence` 不挂 `IntegritySigner` 落到磁盘（攻击者可在 SDK 不知情时追加 `Decision::AllowPermanent`，绕过审批；`HarnessBuilder` 装配期 fail-closed 拒绝）
- 验签失败时仍返回 `ResolvedDecision`（必须 fail-closed + 写 `PermissionPersistenceTampered`，详见 §6.1）
- 共用同一份密文（不区分 `tenant_id` / `key_id` 轮换）跨租户复用（违反 §6.1 的 `CredentialKey` 三元组分桶约束）
- `DedupGate` 把 `Severity::Critical` 的危险命令也纳入合流 / 窗口复用（触发"危险操作只问一次"的语义破窗；§3.8 强制旁路）
- 把 `dedup_key` 写入 `PermissionRequested` / `PermissionResolved` 字段（dedup 是 Broker 内部索引，不属于审计字段；审计仍以 `subject` + `scope` 为准）
- 被合流方不写 `PermissionRequestSuppressed` 而静默返回（破坏审计与 replay 可重建性；详见 `permission-model.md §6.3.3`）

## 14. 相关

- D5 · `permission-model.md`
- D9 · `security-trust.md` §4 危险命令
- ADR-007 权限决策事件化
- Evidence: CC-13, CC-14, HER-039, HER-040, OC-24
