# `octopus-harness-tool` · L2 复合能力 · Tool System SPEC

> 层级：L2 · 状态：Accepted · 修订：v1.4（2026-04-26）
> 依赖：`harness-contracts` + `harness-permission` + `harness-sandbox`
> 关联 ADR：ADR-002（Tool 无 UI）/ ADR-003（Prompt Cache 锁定）/ ADR-006（插件信任域）/ ADR-009（Deferred Tool Loading）/ ADR-010（结果预算与溢出落盘）/ ADR-011（Tool Capability Handle）

## 1. 职责

提供**工具系统**的核心抽象：`Tool` trait、`ToolRegistry`、`ToolPool`（稳定排序）、`ToolOrchestrator`（并发编排 + 流式执行 + 预算控制）、内置工具集与统一的 Hook / Capability / Permission / Sandbox 集成点。

**核心能力**：

- `Tool` trait：声明式 schema/properties + **流式执行**（`ToolStream`），不含 UI（ADR-002）
- `ToolDescriptor`：注册期单一权威信息体，承载 properties / capability / budget / provider 限制 / 分组
- `ToolRegistry`：`Arc<RwLock<...>>` + `snapshot() → ToolRegistrySnapshot`，注册期裁决矩阵（**built-in wins**）
- `ToolPool`：三分区稳定排序（保 Prompt Cache · ADR-003）
- `ToolOrchestrator`：并发分桶、串行化不安全 Tool、`ResultBudget` 流式预算检查、活动心跳
- `coerce_tool_args`：参数强制转换（对齐 HER-010）
- 内置工具集（M0）：`Bash` / `FileRead` / `FileEdit` / `FileWrite` / `Grep` / `Glob` / `ListDir` / `WebFetch` / `WebSearch` / `Todo` / `Agent` / `TaskStop` / `Clarify` / `SendMessage` / `ReadBlob` / `ToolSearch`

## 2. 对外 API

### 2.1 核心 Trait（流式执行 · P0-1）

```rust
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    /// 唯一权威描述：合并 schema / properties / capability / budget / provider 约束。
    fn descriptor(&self) -> &ToolDescriptor;

    /// 静态 schema；动态可变的 schema 走 `resolve_schema`。
    fn input_schema(&self) -> &JsonSchema {
        &self.descriptor().input_schema
    }

    fn output_schema(&self) -> Option<&JsonSchema> {
        self.descriptor().output_schema.as_ref()
    }

    /// 注册时若 `descriptor().dynamic_schema` 为 true，每个 Run 启动期会调用一次。
    /// 返回的 schema 取代静态 schema 进入 ToolPool。
    /// 默认实现：返回静态 input_schema。
    async fn resolve_schema(
        &self,
        ctx: &SchemaResolverContext,
    ) -> Result<JsonSchema, ToolError> {
        Ok(self.input_schema().clone())
    }

    /// 输入校验。失败终止本次 ToolUse，不进入 permission 检查。
    async fn validate(
        &self,
        input: &Value,
        ctx: &ToolContext,
    ) -> Result<(), ValidationError>;

    /// 权限**预判**（声明式，纯函数）。
    ///
    /// 仅返回 Tool 自身能立即得出的结论：`Allowed`（无副作用 / 已通过 AllowList 命中）/
    /// `Denied`（明确违规）/ `AskUser`（需要 Broker 决策，附带 `subject` 与 `scope`）/
    /// `DangerousCommand`（命中危险模式库，强制询问）。
    ///
    /// **不得**在此方法内调用 `ctx.permission_broker.decide`、写 Event、执行 Sandbox、
    /// 修改持久化状态等任何副作用——这些动作由 `harness-engine` 的 Tool Orchestrator
    /// 在拿到 `PermissionCheck` 后统一执行（详见 §调用契约 与 `crates/harness-permission.md §2.2.1`）。
    async fn check_permission(
        &self,
        input: &Value,
        ctx: &ToolContext,
    ) -> PermissionCheck;

    /// 流式执行。返回的 `ToolStream` 由 Orchestrator 消费。
    /// 工具实现必须假设 stream 可以随时被 ctx.interrupt 中断。
    async fn execute(
        &self,
        input: Value,
        ctx: ToolContext,
    ) -> Result<ToolStream, ToolError>;
}

/// 流式输出协议。
pub type ToolStream =
    Pin<Box<dyn Stream<Item = ToolEvent> + Send + 'static>>;

#[derive(Debug, Clone)]
pub enum ToolEvent {
    /// 进度心跳（用于长任务）；不计入 ResultBudget
    Progress(ToolProgress),
    /// 部分输出片段（计入 ResultBudget）
    Partial(MessagePart),
    /// 终态：成功（含完整 / 已 budget 处理后的内容）
    Final(ToolResult),
    /// 终态：失败
    Error(ToolError),
}

#[derive(Debug, Clone)]
pub struct ToolProgress {
    pub message: String,
    pub fraction: Option<f32>,
    pub at: DateTime<Utc>,
}
```

> **为什么改成流式？**
> 1. `Bash` / `WebFetch` / 大文件读取 / Subagent 委派天然就是渐进出结果；
> 2. ADR-010 的预算检查必须在流上做，避免 buffer 整段 GB 级输出；
> 3. UI 上要"边看边等"，必须有中间事件可消费；
> 4. 对一次性短输出工具，可以用 `tool_event::single(MessagePart::Text(s))` 包装成单元素流，迁移成本极低。

#### 2.1.1 调用契约（Tool 与 Permission/Sandbox 的边界）

`Tool` 的四个方法在 Orchestrator 调用顺序里各司其职，**不得越界**：

| 方法 | 责任 | 可做 | 禁止 |
|---|---|---|---|
| `validate` | 输入校验 | 读 `ctx.config` / 静态 schema 校验 / 计算派生字段 | 写 Event、调 Broker、执行外部命令 |
| `check_permission` | 给出 `PermissionCheck` 预判 | 调 `ctx.dangerous_pattern_lib.detect()`、`AllowList::lookup_by_fingerprint`、根据 input 投影 `subject` / `scope_hint` / `fingerprint` | 调 `ctx.permission_broker.decide`、写 Event、执行 Sandbox、读 Journal |
| `execute` | 真正执行（流式） | 通过 `ctx.sandbox` 执行命令、发出 `ToolEvent::*`、消费 `ctx.interrupt` | 自行调 Broker、绕过 Sandbox、写 PermissionResolved |
| `resolve_schema` | 动态 schema | 读运行时只读上下文 | 任何审批 / 执行副作用 |

**Broker 的唯一调用点是 `harness-engine` 的 Tool Orchestrator**：

```text
Engine: ToolOrchestrator::run(...)
  ┌──────────────────────────────────────────────────────────────┐
  │ 1. tool.validate(input, ctx)        ← 失败终止本 ToolUse      │
  │ 2. check = tool.check_permission(input, ctx)                  │
  │ 3. 装配 PermissionRequest + PermissionContext                  │
  │ 4. broker.decide(req, ctx)          ← 唯一调用点              │
  │    └─ 内部：rule_engine 预检 → hooks PreToolUse → 链式 Broker  │
  │ 5. 写 Event::PermissionRequested + PermissionResolved 对       │
  │ 6. Allow → tool.execute(input, ctx) → 流式收尾                 │
  └──────────────────────────────────────────────────────────────┘
```

**这条契约带来的硬约束**：

1. **`PermissionCheck::Allowed` 不等于"跳过 Broker"**：Tool 返回 `Allowed` 表示"我自己看不到拒绝理由"，Engine 仍会调 Broker 走 hooks / DefaultMode / Policy 检查。Tool 不可基于 `Allowed` 直接进入 `execute`。
2. **`PermissionCheck::AskUser` 不指定 broker 类型**：Engine 决定走 Direct / Stream / Chained 哪条路径；Tool 只声明"需要询问"+ `subject` + `scope`。
3. **指纹由 Tool 计算，决策由 Broker 给出**：`fingerprint` 是 Tool 的算力（`ExecSpec::canonical_fingerprint` 在 sandbox crate），但匹配 / 持久化 / 审计的责任在 Broker / AllowList / Journal。这条切分使三者可独立演进（详见 §2.7.1）。
4. **Hook 改 input → Tool 必须重算指纹**：`PreToolUse` Hook 改写 input 后，Engine 重新调 `check_permission` 以获取新指纹；旧指纹的 Allow 不能套到新命令上（详见 §2.7.1 末段）。

> 这条契约在 `crates/harness-permission.md §2.2.1` 与 `permission-model.md §10.1` 也有镜像描述；三处必须保持一致。

### 2.2 `ToolDescriptor`（注册期单一权威 · P0-3）

`ToolDescriptor` 定义在 `harness-contracts`。本 crate 只消费该类型，并负责实现注册、快照、动态 schema 解析和执行期校验。

```rust
pub struct ToolDescriptor {
    /* —— 身份 —— */
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub group: ToolGroup,                 // P1-8
    pub version: semver::Version,

    /* —— Schema —— */
    pub input_schema: JsonSchema,
    pub output_schema: Option<JsonSchema>,
    pub dynamic_schema: bool,             // true → ToolPool 装配前调用 resolve_schema

    /* —— 行为属性 —— */
    pub properties: ToolProperties,

    /* —— Trust × Capability（ADR-006 + ADR-011）—— */
    /// 与 `origin` 分维度建模：`Builtin` 是 `ToolOrigin`，不是 `TrustLevel`。
    /// 内置工具固定 `origin = ToolOrigin::Builtin` 且 `trust_level = TrustLevel::AdminTrusted`。
    pub trust_level: TrustLevel,
    pub required_capabilities: Vec<ToolCapability>,

    /* —— 预算（ADR-010）—— */
    pub budget: ResultBudget,

    /* —— Provider 限制 —— */
    pub provider_restriction: ProviderRestriction,

    /* —— 来源元数据（参与裁决矩阵） —— */
    pub origin: ToolOrigin,

    /* —— ToolSearch 助益 —— */
    pub search_hint: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ToolGroup {
    FileSystem,         // FileRead/FileEdit/FileWrite/ListDir
    Search,             // Grep/Glob/WebSearch
    Network,            // WebFetch/SendMessage
    Shell,              // Bash
    Agent,              // AgentTool/TaskStopTool
    Coordinator,        // 团队协作工具
    Memory,             // TodoTool
    Clarification,      // ClarifyTool
    Meta,               // ToolSearchTool / ReadBlob
    Custom(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ToolOrigin {
    Builtin,
    Plugin { plugin_id: String, trust: TrustLevel },
    Mcp(McpOrigin),
    Skill(SkillOrigin),
}

pub struct ToolProperties {
    pub is_concurrency_safe: bool,
    pub is_read_only: bool,
    pub is_destructive: bool,

    /// 长任务配置：Orchestrator 何时认为该工具进入 long-running 模式
    pub long_running: Option<LongRunningPolicy>,

    /// ADR-009 加载策略（取代旧的 defer_load: bool）
    pub defer_policy: DeferPolicy,
}

pub struct LongRunningPolicy {
    /// 超过该时间未发出 Progress / Partial → Orchestrator 自动注入心跳事件
    pub stall_threshold: Duration,
    /// 超过该时间整体超时
    pub hard_timeout: Duration,
}

impl Default for ToolProperties {
    fn default() -> Self {
        // Fail-closed 默认
        Self {
            is_concurrency_safe: false,
            is_read_only: false,
            is_destructive: true,
            long_running: None,
            defer_policy: DeferPolicy::AlwaysLoad,
        }
    }
}
```

> `ResultBudget` / `BudgetMetric` / `OverflowAction` 定义见 ADR-010；`ToolCapability` 见 ADR-011。两类皆位于 `harness-contracts` §3.4，本 crate 仅引用。

### 2.3 `ProviderRestriction`（P1-12）

```rust
#[derive(Debug, Clone)]
pub enum ProviderRestriction {
    /// 任何 model provider 都可装配（默认）
    All,
    /// 仅在指定 provider 下装配（例如 anthropic_native_only）
    Allowlist(BTreeSet<ModelProvider>),
    /// 在指定 provider 下被剔除
    Denylist(BTreeSet<ModelProvider>),
}

impl Default for ProviderRestriction {
    fn default() -> Self { Self::All }
}
```

`ToolPool::assemble` 在装配时基于 `ToolPoolModelProfile.provider` 过滤，避免如 "Anthropic 原生 ToolSearch" 在不支持 `tool_reference` 的 provider 下被装入。

`octopus-harness-tool` 不依赖 `octopus-harness-model`。Pool 过滤只消费本 crate 定义的最小模型画像；`harness-session` / `harness-sdk` 负责从真实 `harness-model::ModelCapabilities` 映射到该画像。

### 2.4 `ToolContext`（执行期上下文）

```rust
pub struct ToolContext {
    pub tool_use_id: ToolUseId,
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,

    /// 沙箱 Backend；由 SandboxInheritance 决定
    pub sandbox: Option<Arc<dyn SandboxBackend>>,

    /// 权限决策器
    pub permission_broker: Arc<dyn PermissionBroker>,

    /// Hook 链快照
    pub hook_registry: Arc<HookRegistrySnapshot>,

    /// Capability 注入点（ADR-011）
    pub cap_registry: Arc<CapabilityRegistry>,

    /// 可观测
    pub observability: Arc<Observer>,

    /// 中断 token；Orchestrator 在用户取消 / 超时 / 上游中断时翻转
    pub interrupt: InterruptToken,

    /// 子代理执行场景下的父上下文（仅在 Subagent 内可见，否则 None）
    pub parent_run: Option<ParentRunHandle>,
}

impl ToolContext {
    pub fn capability<T: ?Sized + Send + Sync + 'static>(
        &self,
        cap: ToolCapability,
    ) -> Result<Arc<T>, ToolError> {
        self.cap_registry
            .get::<T>(cap)
            .ok_or(ToolError::CapabilityMissing(cap))
    }
}
```

### 2.5 `ToolRegistry` 与裁决矩阵（P0-6）

```rust
pub struct ToolRegistry {
    inner: Arc<RwLock<ToolRegistryInner>>,
}

struct ToolRegistryInner {
    tools: BTreeMap<String, RegisteredTool>,
    /// 同名遮蔽审计（按注册顺序保留全部尝试）
    shadowed: Vec<ShadowedRegistration>,
    generation: u64,
}

struct RegisteredTool {
    tool: Arc<dyn Tool>,
    origin: ToolOrigin,
    registered_at: DateTime<Utc>,
}

impl ToolRegistry {
    pub fn builder() -> ToolRegistryBuilder;
    pub fn register(&self, tool: Box<dyn Tool>) -> Result<(), RegistrationError>;
    pub fn deregister(&self, name: &str) -> Result<(), RegistrationError>;
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn snapshot(&self) -> ToolRegistrySnapshot;
    /// 返回截至当前的所有遮蔽事件，供管控审计
    pub fn shadowed(&self) -> Vec<ShadowedRegistration>;
}

pub struct ShadowedRegistration {
    pub name: String,
    pub kept: ToolOrigin,
    pub rejected: ToolOrigin,
    pub reason: ShadowReason,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShadowReason {
    BuiltinWins,           // 决策：内置不可被覆盖
    HigherTrust,           // 决策：保留高信任来源
    Duplicate,             // 完全重复定义
}
```

#### 2.5.1 裁决矩阵（**built-in wins**）

| 现有 | 新注册 | 决策 | 事件 |
|---|---|---|---|
| Builtin | Plugin / MCP / Skill 同名 | **保留 Builtin**，丢弃新注册 | `RegistrationShadowed { reason: BuiltinWins }` |
| Plugin(AdminTrusted) | Plugin(UserControlled) 同名 | 保留 AdminTrusted | `HigherTrust` |
| Plugin(UserControlled) | Plugin(AdminTrusted) 同名 | 替换为 AdminTrusted | `HigherTrust`（旧的进入 shadowed） |
| MCP/Skill 同名 | Plugin 同名 | 保留先注册者；后续注册被遮蔽 | `Duplicate` |
| 任意 | Builtin 后注册 | 保留先注册的 Builtin（Builtin 启动期已注入） | — |

**理由**：

- 与 ADR-003 prompt cache 稳定性吻合：内置工具集是被 prompt 化为基础短语的"骨架"，被 plugin 替换会让 cache 大概率失效；
- 与 ADR-006 trust 模型协同：用户态插件不能取代由审计构建的内置工具；
- **可观测**：每次裁决都有 `Event::ToolRegistrationShadowed`，UI/审计可见。

```rust
pub struct ToolRegistrySnapshot {
    tools: Arc<BTreeMap<String, Arc<dyn Tool>>>,
    descriptors: Arc<BTreeMap<String, Arc<ToolDescriptor>>>,
    generation: u64,
}

impl ToolRegistrySnapshot {
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>>;
    pub fn descriptor(&self, name: &str) -> Option<&Arc<ToolDescriptor>>;
    pub fn iter_sorted(&self) -> impl Iterator<Item = (&String, &Arc<dyn Tool>)>;
    pub fn as_descriptors(&self) -> Vec<&ToolDescriptor>;
    pub fn by_group(&self, group: &ToolGroup) -> Vec<&Arc<dyn Tool>>;
}
```

Snapshot 对外不可变，支撑 Prompt Cache 硬约束（ADR-003）。

### 2.6 `ToolPool`（三分区稳定排序，ADR-003 + ADR-009）

```rust
pub struct ToolPool {
    /// 第 1 分区：创建期注入且 schema 立即可见（含 ToolSearchTool 自身）
    always_loaded: Vec<Arc<dyn Tool>>,
    /// 第 2 分区：Deferred 集——仅名字可见，schema 由 ToolSearch 按需材化
    deferred: Vec<Arc<dyn Tool>>,
    /// 第 3 分区：Session 运行期通过 reload_with(add_tools) 追加的工具
    runtime_appended: Vec<Arc<dyn Tool>>,
}

impl ToolPool {
    pub async fn assemble(
        snapshot: &ToolRegistrySnapshot,
        filter: &ToolPoolFilter,
        search_mode: &ToolSearchMode,
        model_profile: &ToolPoolModelProfile,
        schema_resolver_ctx: &SchemaResolverContext,
    ) -> Result<Self, ToolError>;

    pub fn always_loaded(&self) -> &[Arc<dyn Tool>];
    pub fn deferred(&self) -> &[Arc<dyn Tool>];
    pub fn runtime_appended(&self) -> &[Arc<dyn Tool>];

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Tool>>;
}

pub struct ToolPoolModelProfile {
    pub provider: ModelProvider,
    pub supports_tool_reference: bool,
    pub max_context_tokens: Option<u32>,
}

pub struct ToolPoolFilter {
    pub allowlist: Option<HashSet<String>>,
    pub denylist: HashSet<String>,
    pub mcp_included: bool,
    pub plugin_included: bool,
    /// 按 ToolGroup 黑/白名单过滤
    pub group_allowlist: Option<HashSet<ToolGroup>>,
    pub group_denylist: HashSet<ToolGroup>,
}
```

**装配步骤**（按序）：

1. 应用 `ToolPoolFilter`（origin / group / 黑白名单）；
2. 应用 `ProviderRestriction`（按 `model_profile.provider` 过滤）；
3. 对 `dynamic_schema = true` 的工具调用 `resolve_schema`；
4. 按 `DeferPolicy` × `ToolSearchMode` 决定分区（沿用 ADR-009 §2.6 的状态机）；
5. 固定分区按工具名稳定排序；
6. 输出 `ToolPool`。

**分区规则**（保 Prompt Cache 长期稳定性）：

| 分区 | 触发条件 | 排序 | 说明 |
|---|---|---|---|
| **1 · AlwaysLoad** | `DeferPolicy::AlwaysLoad` / `AutoDefer` + `ToolSearchMode::Disabled` / `AutoDefer` 未达阈值 | 工具名字典序 | 进入 system prompt 的 `<functions>` |
| **2 · Deferred** | `DeferPolicy::ForceDefer` / `AutoDefer` 达阈值 | 字母序 | 不进 system prompt；作为 `DeferredToolsDelta` attachment 宣告（只名字） |
| **3 · RuntimeAppended** | `Session::reload_with(add_tools=..)` | 加入顺序 | 不重排；一次 `AppliedInPlace + OneShotInvalidation` 只破坏分区 3 一次 |

**为什么分 3 层**（与 ADR-003 §2.3 和 ADR-009 联动）：

- 分区 1 前缀一旦定下，**整个 Session 都稳定**，prompt cache 可长期命中；
- 分区 2 仅在 attachment 尾部出现，**不破坏前缀 cache**；模型通过 `ToolSearchTool` 按需"解锁"；
- 分区 3 是 `InlineReinjectionBackend` 的落点：当模型走 inline 降级路径时，materialize 出的工具落入此分区，**只产生一次 cache miss**，此后重建的 cache prefix = 分区 1 + 分区 3 到目前为止的累计，依然稳定；
- Anthropic native 路径（`AnthropicToolReferenceBackend`）不改 Pool —— `tool_reference` 由服务端展开，schema 不进客户端前缀，`CacheImpact::NoInvalidation`。

### 2.7 `ToolOrchestrator`（流式 + 预算 + 心跳 · P0-1/P0-2/P1-9）

```rust
pub struct ToolOrchestrator {
    concurrency_limit: usize,
    dispatch_semaphore: Arc<Semaphore>,
}

pub struct OrchestratorContext {
    pool: ToolPool,
    tool_context: ToolContext,
    permission_context: PermissionContext,
    blob_store: Option<Arc<dyn BlobStore>>,
    event_emitter: Arc<dyn ToolEventEmitter>,
}

pub trait ToolEventEmitter: Send + Sync + 'static {
    fn emit(&self, event: Event);
}

impl ToolOrchestrator {
    pub async fn dispatch(
        &self,
        calls: Vec<ToolCall>,
        ctx: OrchestratorContext,
    ) -> Vec<ToolResultEnvelope>;
}

pub struct ToolCall {
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub input: Value,
}

pub struct ToolResultEnvelope {
    pub tool_use_id: ToolUseId,
    pub result: Result<ToolResult, ToolError>,
    pub duration: Duration,
    pub progress_emitted: u32,
}
```

**分桶规则**（对齐 CC-07）：

- 本 SDK 采用 **bool 二档**（不是三桶 `Shared / Exclusive / FreeForm`）：
  - `is_concurrency_safe = true` → 并行（`tokio::join_all`，`max = concurrency_limit`，默认 10）
  - `is_concurrency_safe = false` → 串行
- "分桶"在本文档语境中等价于"按 bool 字段分组"：safe 归 1 个并行组，unsafe 归 1 个串行组，**不存在中间档**。
- 不引入三档枚举的理由：90% 内置工具按 safe / unsafe 即可正确编排；引入 `Shared / Exclusive` 等 RWLock 风格语义会逼业务方按资源粒度声明（文件路径、网络端口…），代价远超收益（KISS）。如未来某个工具需要更细粒度的并发控制，应在 `Tool::execute` 内部用业务逻辑互斥，而非升级本字段。

**单次调用流水线**（核心规则，统一所有 Tool）：

```text
ToolCall
  ├─→ Hook::PreToolUse         (可 Continue / Block / PreToolUse(PreToolUseOutcome))
  ├─→ tool.validate(input)
  ├─→ tool.check_permission()  → Broker 决策
  │     └→ deny → emit ToolUseDenied → end
  ├─→ tool.execute(input, ctx) → ToolStream
  ├─→ Stream collector with ResultBudget:
  │     forEach event:
  │       Progress → emit ToolProgress; reset stall timer
  │       Partial  → buffer head + tail preview; budget meter += size
  │           if meter > budget.limit:
  │             switch on budget.on_overflow {
  │                Truncate → cut and continue;
  │                Offload  → drain remainder to BlobStore::put_streaming();
  │                Reject   → ToolError::ResultTooLarge;
  │             }
  │       Final    → finalize ToolResult (with overflow metadata if any)
  │       Error    → finalize ToolError
  ├─→ Hook::TransformToolResult     (可改写 final result)
  ├─→ Hook::PostToolUse / PostToolUseFailure
  └─→ emit Event::ToolUseCompleted | ToolUseFailed
```

**长任务心跳**（P1-9）：

- 工具声明 `properties.long_running = Some(LongRunningPolicy { stall_threshold, hard_timeout })`
- Orchestrator 在 dispatch 时启动 stall timer：连续 `stall_threshold` 内没有任何 `ToolEvent` → 自动注入一条 `ToolEvent::Progress { message: "still running…", fraction: None }`，并 emit `Event::ToolUseHeartbeat { tool_use_id, at }` 用于 UI 渐进显示与监控
- 超过 `hard_timeout` → `ctx.interrupt.cancel()` + `ToolError::Timeout`

**ResultBudget**（ADR-010）：

- 流上做 head/tail 预览采集，命中阈值后剩余流走 `BlobStore::put_streaming()`；
- 落盘成功后产出 `ToolResult { content: head + tail + offload_marker, overflow: Some(meta) }`；
- 同时 emit `Event::ToolResultOffloaded { blob_ref, original_size, effective_limit }`；
- 落盘失败按 `OverflowAction::Reject` 处理。

### 2.7.1 Shell 类工具的 SandboxBackend 桥接子流

§2.7 主流水线中 `tool.check_permission` 与 `tool.execute` 两步对**所有** Tool 形态一致；当 Tool 内部需要执行外部命令时（Bash / Run / 任意 Shell 类工具），其内部展开必须遵循下面这条**指纹生成 → 权限决策 → SandboxBackend 三段式**桥接。

```text
tool.check_permission(input, ctx)
    │
    ├─► 由 input 投影出 ExecSpec   ◄── command + args + env + cwd + workspace_access + policy
    │
    ├─► fp = ExecSpec::canonical_fingerprint(&base)        ◄── harness-sandbox §2.2
    │
    ├─► allowlist.lookup_by_fingerprint(&fp)               ◄── 短路：命中即 Allowed
    │       └─ miss → 走完整流程
    │
    ├─► dangerous_pattern_lib.detect(&command)             ◄── harness-permission §4
    │       └─ hit → severity = Critical, scope 强制降级 ExactCommand
    │
    └─► PermissionCheck { scope_hint: ExactCommand, fingerprint: Some(fp), .. }
            │
            ▼
       Orchestrator → permission_broker.decide(...)        ◄── 详见主流水线
            │
            ▼
       Decision::Allow* ─► 进入 tool.execute
       Decision::Deny*  ─► ToolError::Denied · 短路

tool.execute(input, ctx)                                   ◄── Shell 类工具的内部展开
    │
    ├─► sandbox = ctx.capability::<SandboxRunnerCap>()?    ◄── ADR-011 capability handle
    │
    ├─► sandbox.before_execute(&spec, &ctx).await?         ◄── 远端 backend 同步预热
    │       └─ 失败：HostPathDenied / CapabilityMismatch / Unavailable → ToolError
    │
    ├─► handle = sandbox.execute(spec, ctx).await?         ◄── 返回 ProcessHandle
    │       │
    │       ├─ stdout / stderr → 转 ToolStream::Partial    ◄── 经主流水线 Hook + budget
    │       ├─ cwd_marker      → 业务侧记账，不进 ToolStream
    │       └─ activity        → heartbeat / kill 由 ProcessHandle 自管
    │
    ├─► outcome = handle.activity.wait().await?
    │
    ├─► sandbox.after_execute(&outcome, &ctx).await?       ◄── 反向同步：拉回 / 回收
    │       └─ 失败仅打 trace + Event::SandboxBackendError，不覆盖原始 outcome
    │
    └─► ToolEvent::Final(ToolResult::from(outcome))
```

**契约要点**：

- **指纹必须由 Tool 在 `check_permission` 阶段生成**：Orchestrator 不替 Tool 计算指纹，避免 Orchestrator 反向依赖 sandbox。Tool 把 `(scope_hint, fingerprint)` 一并塞进 `PermissionCheck`，Broker 与 AllowList 据此做匹配。
- **`before_execute` 与 `execute` 之间不允许插入 Hook**：Hook 的窗口在主流水线（`PreToolUse` / `PostToolUse`），不进入 sandbox 子流；这避免业务层在 sandbox 已部分预热（如 rsync 上传完成）后再被 Hook 覆盖造成状态裂痕。
- **`after_execute` 不可改写 outcome**：仅做反向同步与资源回收；任何"成功/失败"语义保持来自 `outcome.exit_status`，便于事件 `SandboxExecutionCompleted` 与 `ToolUseCompleted` 一致。
- **CWD marker 流不进 `ToolStream`**：`cwd_marker` 是 Shell 类工具内部的"会话状态记账通道"，与给业务/UI 看的 `ToolStream::Partial` 解耦；业务若需展示当前 cwd，由 Tool 自己在 `Final` 中携带。
- **失败原子性**：`before_execute` 失败 → 不发出 `SandboxExecutionStarted`；`execute` 失败 → 仍发 `Started` + `Completed { BackendError }`；`after_execute` 失败 → 已发的 `Completed` 不回滚，仅追加 `SandboxBackendError`。这条规则与 ADR-001 事件追加性配套。
- **指纹失效处理**：若 Tool 在 `check_permission` 阶段命中 `lookup_by_fingerprint`，但执行前 `policy` 字段被 Hook 在 `PreToolUse` 中改写（参见 §2.8），Tool 必须**重新计算指纹并重走匹配**，而不是沿用旧指纹的 Allow 状态——否则会让 Hook 成为绕过审批的通道。

> 与 `harness-engine.md` §3 主循环、`harness-permission.md` §2.3、`harness-sandbox.md` §2.1 / §2.2 / §9 形成闭环；任何对本子流的修改需同步回查上述四处。

### 2.8 Hook 介入点（P0-5）

`harness-tool` 与 `harness-hook` 的协议固定为以下五个介入点（详见 `harness-hook.md`）：

| Hook 事件 | 上下文 | 允许动作 |
|---|---|---|
| `PreToolUse` | `tool_name`, `input`, `descriptor` | `Continue` / `Block(reason)` / `PreToolUse(PreToolUseOutcome { rewrite_input?, override_permission?, additional_context?, block? })` —— 三件套唯一入口；权威定义在 `harness-hook.md §2.3 / §2.4.1` |
| `TransformToolResult` | `tool_name`, `result` | `Continue` / `Transform(value)`：替换 `ToolResult.content` / 添加 `MessagePart` |
| `TransformTerminalOutput` | `tool_name`, `partial bytes` | `Continue` / `Transform(bytes)`：实时改写流（仅对 Bash/SubagentStream 类） |
| `PostToolUse` | `tool_name`, `final result` | `Continue` / `AddContext`：只触发副作用（写日志、推 IM）或注入下一轮上下文，**不能改 result** |
| `PostToolUseFailure` | `tool_name`, `error` | `Continue` / `AddContext`：只触发告警或注入解释性上下文 |

约束：

- Hook **不允许**直接调用 `tool.execute`，避免循环；
- 任一 hook 抛 panic → Orchestrator 捕获并降级为 `Continue`，发 `Event::HookPanicked`（结构定义见 `event-schema.md §3.7.2`）；
- handler 失败的兜底语义（fail-open / fail-closed）由声明的 `failure_mode` 决定（详见 `harness-hook.md §2.6.1`）；UserControlled 来源 hook 强制 `FailOpen`；
- Hook 只能通过 `ToolCapability::HookEmitter` 发布自己定义的事件，不能直接写入 Journal。

### 2.9 `coerce_tool_args`（参数强制转换，对齐 HER-010）

```rust
pub fn coerce_tool_args(
    schema: &JsonSchema,
    raw: Value,
) -> Result<Value, CoerceError> {
    // 根据 schema 把字符串化的 number/bool/json 自动反序列化
    // 处理 LLM 常见的"把 bool 写成字符串"问题
}
```

## 3. `DenyPattern` 库（P1-10）

```rust
/// 内置的"危险动作模式库"，用于工具的 check_permission 与 Hook PreToolUse 默认值。
/// 也作为管控配置 deny_extras 的合并基线。
pub struct DenyPatternLibrary {
    pub bash_command_patterns: Vec<DenyPattern<CommandMatch>>,
    pub path_patterns:         Vec<DenyPattern<PathMatch>>,
    pub url_patterns:          Vec<DenyPattern<UrlMatch>>,
}

pub struct DenyPattern<M> {
    pub id: &'static str,
    pub matcher: M,
    pub severity: Severity,
    pub message: &'static str,
}
```

内置示例（节选；完整列表写入 `harness-tool/data/deny-patterns.toml`）：

| id | 类型 | 模式 | severity | 含义 |
|---|---|---|---|---|
| `bash.rm-root` | Command | `^rm\s+(-rf?|--no-preserve-root).*\s/(?:\s|$)` | Critical | 删根 |
| `bash.dd-zero` | Command | `^dd\s+.*of=/dev/(sda|nvme0n1)` | Critical | 直写裸盘 |
| `bash.curl-pipe-bash` | Command | `curl[^\|]+\|\s*(sudo\s+)?bash` | High | 远程脚本直跑 |
| `path.system-etc` | Path | starts-with `/etc/` 写入 | High | 改系统配置 |
| `url.localhost-private` | Url | localhost / 169.254.169.254 / 私网 | Medium | 元数据/私网 |

工具实现可在 `check_permission` 里用 `ctx.deny_patterns.match_command(...)` 命中后返回 `PermissionCheck::Deny { reason, severity }` 或 `AskUser { severity: matched.severity, ... }`。

## 4. 内置 Toolset（M0 全集）

下表是 M0 阶段必须交付的内置工具清单。每一项的 `descriptor()` 通过表内字段固定。

| 工具 | group | concurrency | budget(metric/limit) | trust | required caps | 说明 |
|---|---|---|---|---|---|---|
| `Bash` | Shell | false | Chars / 256k | Builtin | — | 执行 shell；命中 deny 模式必询问 |
| `FileRead` | FileSystem | true | Chars / 64k | Builtin | — | UTF-8 文件读，支持行范围 |
| `FileEdit` | FileSystem | false | Chars / 64k | Builtin | — | 字符串替换；细粒度按 path 审批 |
| `FileWrite` | FileSystem | false | Chars / 64k | Builtin | — | 覆盖写；高破坏性 |
| `Grep` | Search | true | Chars / 64k | Builtin | — | ripgrep 包装 |
| `Glob` | Search | true | Chars / 32k | Builtin | — | glob 匹配 |
| `ListDir` | FileSystem | true | Chars / 32k | Builtin | — | 列目录（深度可控） |
| `WebFetch` | Network | true | Chars / 64k（建议溢出落盘）| Builtin | — | URL 内容抓取 |
| `WebSearch` | Network | true | Chars / 32k | Builtin | — | Web 搜索；只读；摘要型 |
| `Todo` | Memory | false | Chars / 32k | Builtin | `TodoStore` | Run 内 TODO 列 |
| `Agent` | Agent | false | Chars / 16k（仅 announcement） | Builtin | `SubagentRunner` | spawn subagent，等结构化结果 |
| `TaskStop` | Agent | false | Chars / 1k | Builtin | `RunCanceller` | 主动结束当前 Run/Subagent |
| `Clarify` | Clarification | false | Chars / 8k | Builtin | `ClarifyChannel` | 向用户结构化提问 |
| `SendMessage` | Network | false | Chars / 4k | Builtin | `UserMessenger` | 异步向用户/IM 推送 |
| `ReadBlob` | Meta | true | Chars / 64k（递归受限） | Builtin | `BlobReader` | 取回 ToolResultOffloaded 落盘 |
| `ToolSearch` | Meta | true | Chars / 16k | Builtin | — | ADR-009 元工具，详见 `harness-tool-search.md` |

### 4.1 工具骨架（节选）

```rust
pub struct BashTool { default_shell: ShellKind }

impl Tool for BashTool {
    fn descriptor(&self) -> &ToolDescriptor { /* 字面量定义 */ }

    async fn check_permission(
        &self,
        input: &Value,
        ctx: &ToolContext,
    ) -> PermissionCheck {
        let command = input["command"].as_str().unwrap_or("");
        if let Some(hit) = ctx.deny_patterns.match_command(command) {
            return PermissionCheck::AskUser {
                subject: format!("执行命令: {}", command),
                detail: Some(hit.message.into()),
                severity: hit.severity,
                scope: DecisionScope::ExactCommand {
                    command: command.into(),
                    cwd: input["cwd"].as_str().map(Into::into),
                },
            };
        }
        PermissionCheck::AskUser {
            subject: format!("执行命令: {}", command),
            detail: None,
            severity: Severity::Medium,
            scope: DecisionScope::ExactCommand { /* ... */ },
        }
    }

    async fn execute(&self, input: Value, ctx: ToolContext)
        -> Result<ToolStream, ToolError>
    { /* ... */ }
}
```

### 4.2 `WebSearchTool`（M0 新增）

```rust
pub struct WebSearchTool {
    backends: Vec<Arc<dyn WebSearchBackend>>, // brave / serpapi / 自建
    default_max_results: u32,
}
```

入参：`{ query: string, max_results?: u32, region?: string, recency?: enum }`
出参：`Vec<{ title, url, snippet, score }>`，命中预算时 offload 整页 raw json。
`provider_restriction = ProviderRestriction::All`，但实际可用性受 `cap_registry` 中是否注入 `WebSearchBackend` 影响（不注入则注册期失败 `RegistrationError::CapabilityMissing`）。

### 4.3 `ClarifyTool`（M0 新增）

```rust
pub struct ClarifyTool;
```

入参：`{ prompt: string, choices?: [{ id, label, hint? }], multiple?: bool, timeout_seconds?: u32 }`
出参：`{ answer: string, chosen_ids: [string], answered_at: timestamp }`
机制：通过 `ToolCapability::ClarifyChannel`，由 `harness-session` 把请求转成 `Event::ClarifyRequested` 推到 UI；用户回答前工具阻塞（受 `interrupt` 与 `timeout_seconds` 中断）。

### 4.4 `SendMessageTool`（M0 新增）

```rust
pub struct SendMessageTool;
```

入参：`{ channel: enum, body: string, attachments?: [BlobRef] }`
出参：`{ message_id, delivered_at }`
机制：通过 `ToolCapability::UserMessenger`，对接 `harness-session` 的 outbound 通道（IM / Webhook / 桌面通知）。**异步**，不阻塞主循环。

### 4.5 `ListDirTool`（M0 新增）

```rust
pub struct ListDirTool;
```

入参：`{ path: string, max_depth?: u32, include_hidden?: bool }`
出参：`Vec<{ path, kind, size, modified }>`，命中预算时按目录摘要 offload。

### 4.6 `BuiltinToolset`

```rust
pub enum BuiltinToolset {
    Default,            // 全部 M0 工具
    ReadOnly,           // FileRead / Grep / Glob / ListDir / WebFetch / WebSearch / ReadBlob
    FileSystem,         // FileRead / FileEdit / FileWrite / Grep / Glob / ListDir
    Coordinator,        // Agent / TaskStop / SendMessage
    Conversation,       // Clarify / SendMessage / Todo
    Minimal,            // TaskStop + Todo（不含 ToolSearch）
    Custom(Vec<String>),
}

impl ToolRegistryBuilder {
    pub fn with_builtin_toolset(self, toolset: BuiltinToolset) -> Self;
}
```

`Default` / `ReadOnly` / `FileSystem` / `Coordinator` 默认包含 `ToolSearchTool`（`defer_policy = AlwaysLoad`）；`Minimal` 不含。

> **关于 `execute_code` 元工具**：当 `feature_flags.programmatic_tool_calling = on`
> 时，`BuiltinToolset::Default` / `Coordinator` 自动追加 `ExecuteCodeTool`（§4.7）。
> `ReadOnly` / `FileSystem` / `Conversation` / `Minimal` 一律 **不** 包含——避免破坏
> 这些 toolset 的"约束面"语义。`Custom(Vec<String>)` 中显式列出 `"execute_code"`
> 时也需 feature flag 同时开启，否则注册期 fail-closed
> `RegistrationError::FeatureDisabled { tool: "execute_code" }`。

### 4.7 `ExecuteCodeTool`（M1，feature `programmatic_tool_calling`）

> 元工具：让主 Agent 用一次推理发起多步、有依赖的工具编排，详见 ADR-0016。
> 默认对 Subagent 不可见（`harness-subagent §2.5 default blocklist` 已含 `execute_code`，
> 不在本节修改）。本节给出本 crate 内的描述子、流水线、白名单规则与与 §5 注册期校验
> 的对接点，**不**重复 ADR-0016 的决策正文。

#### 4.7.1 工具描述子

```rust
pub struct ExecuteCodeTool;

impl Tool for ExecuteCodeTool {
    fn descriptor(&self) -> &ToolDescriptor { &EXECUTE_CODE_DESC }
    async fn check_permission(&self, input: &Value, ctx: &ToolContext) -> PermissionCheck;
    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolStream, ToolError>;
}

static EXECUTE_CODE_DESC: std::sync::LazyLock<ToolDescriptor> = std::sync::LazyLock::new(|| {
    ToolDescriptor {
        name: "execute_code".into(),
        display_name: "Programmatic Tool Calling".into(),
        description: include_str!("descriptions/execute_code.md").into(),
        category: "meta".into(),
        group: ToolGroup::Meta,
        version: semver::Version::new(1, 0, 0),
        input_schema: schema_from_str(include_str!("schemas/execute_code.input.json")),
        output_schema: Some(schema_from_str(include_str!("schemas/execute_code.output.json"))),
        dynamic_schema: false,
        properties: ToolProperties {
            is_concurrency_safe: false,
            is_read_only: false,       // 元工具按"破坏性"归口，不因嵌入工具集 read-only 而下调
            is_destructive: false,
            defer_policy: DeferPolicy::AlwaysLoad,
            ..PROPERTIES_DEFAULTS
        },
        trust_level: TrustLevel::AdminTrusted,
        required_capabilities: vec![
            ToolCapability::CodeRuntime,
            ToolCapability::EmbeddedToolDispatcher,
        ],
        budget: ResultBudget {
            metric: BudgetMetric::Chars,
            limit: 30_000,
            on_overflow: OverflowAction::Offload,
            preview_head_chars: 2_000,
            preview_tail_chars: 2_000,
        },
        provider_restriction: ProviderRestriction::All,
        origin: ToolOrigin::Builtin,
        search_hint: None,
    }
});
```

`ToolDescriptor` 内容冻结（ADR-0003）；`programmatic_tool_calling` flag 只控制
"是否被装配进 `ToolPool`"，descriptor 内容本身不随 flag 漂移。

#### 4.7.2 执行流水线（与 ADR-0016 §2.4 对齐）

```text
ExecuteCodeTool::execute(input, ctx)
  ├─ [1] script_validator（语法+长度+静态扫描；mini-lua 子集 §4.7.4）
  │       失败 → ToolEvent::Error(ScriptInvalid { reason })
  ├─ [2] ctx.cap::<CodeRuntimeCap>().spawn(script_handle, sandbox_spec)
  ├─ [3] CodeSandbox 内执行：每次 `emb.tool(name, args)`
  │       └─ EmbeddedToolDispatcher::dispatch
  │             ├─ §4.7.3 白名单校验
  │             ├─ 转译为合法 ToolUse → ToolOrchestrator::single_use_pipeline
  │             │   （复用 PreToolUse / Permission / TransformToolResult / PostToolUse 全套 5 介入点）
  │             └─ Event::ExecuteCodeStepInvoked { parent_tool_use_id, ... }
  ├─ [4] 脚本退出（return / 异常 / instruction quota / wall_clock）
  └─ [5] Orchestrator 拼装 final ToolResult（含每步的 args_summary / result_summary）
        → ToolEvent::Final(envelope)
```

**强制约束**（与 ADR-0016 §2.4 同款；本节按"运行期 fail-closed"列出）：

- 脚本不得反射调用 `execute_code` 自身（递归直接 deny；事件
  `ExecuteCodeStepInvoked.refused_reason = SelfReentrant`）
- 脚本不得直接发起 `model.infer`；任何"对模型说话"必须走嵌入工具
- 嵌入调用每一次都仍走 `PermissionBroker`；`DedupGate` 自然吸收"同脚本 8 次相同 grep"

#### 4.7.3 嵌入工具白名单（按 ADR-0016 §2.6）

```rust
pub struct EmbeddedToolWhitelist {
    pub names: BTreeSet<String>,
}

impl EmbeddedToolWhitelist {
    /// 默认（ADR-0016 q4）：仅 7 个 read-only built-in 工具。
    pub const DEFAULT_READONLY_BUILTIN: &'static [&'static str] = &[
        "Grep", "Glob", "FileRead", "ListDir",
        "WebSearch", "ReadBlob", "ToolSearch",
    ];

    /// 业务在 `team_config.toml [execute_code.embedded_tools]` 中显式扩展时调用。
    pub fn from_team_config(cfg: &TeamConfig) -> Result<Self, ConfigError>;
}
```

扩展校验规则（注册期 + reload_with 期均执行，见 §5 矩阵）：

| 校验项 | 通过条件 |
|---|---|
| `properties.is_destructive` | 必须 `== false`，否则 `ConfigError::EmbeddedToolNotPermitted` |
| `properties.is_read_only` | 必须 `== true`，否则 `ConfigError::EmbeddedToolNotPermitted` |
| `origin + trust_level` | 必须满足 `origin == ToolOrigin::Builtin`，或 `origin == ToolOrigin::Plugin { trust: AdminTrusted }`；user-controlled / mcp 一律拒绝 |
| `is_concurrency_safe` | 不强制；orchestrator 在脚本内仍按 `false` 串行执行嵌入调用 |
| 命中 `ToolDescriptor.search_hint = None` | 视为 `ToolSearchTool` 自身，强制保留（业务无法剔除 ToolSearch） |

通过的扩展会写出 `Event::ExecuteCodeWhitelistExtended { added, source: "team_config" }`，
便于审计跟踪"谁扩了白名单、扩了什么"。

#### 4.7.4 受限脚本语法（mini-lua 子集，M0 实现）

> 详细允许 / 禁止条目见 ADR-0016 §2.5；本节不重复，仅给出 SDK 内 Rust 类型映射：

```rust
pub struct CompiledScript {
    pub bytecode: Vec<u8>,
    pub constants: Vec<LuaValue>,
    pub instruction_count_estimate: u64,
    pub max_call_depth: u32,
    pub script_hash: [u8; 32],            // 用于 DecisionScope::ExecuteCodeScript
}

pub trait ScriptCompiler: Send + Sync + 'static {
    fn language(&self) -> ScriptLanguage;
    fn compile(&self, src: &str) -> Result<CompiledScript, ScriptCompileError>;
}

pub enum ScriptLanguage {
    MiniLua,                              // M0 默认实现
}
```

业务侧若要扩展为 Python / JS，必须走 ADR + capability + sandbox 三联评审
（参考 `extensibility.md §3.x`），不得在 SDK 默认 `MiniLuaCodeSandbox` 上偷加。

#### 4.7.5 与 §2 / §3 / §5 的接入点

| 接入点 | 行为 |
|---|---|
| `ToolRegistry::register`（§5） | 校验 `required_capabilities` 必含 `CodeRuntime + EmbeddedToolDispatcher`；feature flag off 时直接拒绝 |
| `ToolPool::pool_for_session`（§3） | feature flag on 时把 `ExecuteCodeTool` 注入 `Default / Coordinator` toolset |
| `ToolOrchestrator::single_use_pipeline`（§3） | 嵌入调用复用本入口；不另开新流水线，保证 5 个 hook 介入点不被绕过 |
| `harness-subagent::DefaultSubagentRunner::filter_for`（参考 §2.5） | 默认 blocklist 已含 `execute_code`；Subagent 装配期直接剔除 |

## 5. 注册时校验（与 ADR-006 + ADR-011 协同）

```rust
impl ToolRegistry {
    pub fn register(&self, tool: Box<dyn Tool>) -> Result<(), RegistrationError> {
        let desc = tool.descriptor().clone();

        // 1. Trust × Capability 矩阵
        CapabilityPolicy::default_locked()
            .check(&desc.trust_level, &desc.required_capabilities)?;

        // 2. Trust × is_destructive 矩阵
        if desc.properties.is_destructive
            && !matches!(
                desc.origin,
                ToolOrigin::Builtin
                    | ToolOrigin::Plugin { trust: TrustLevel::AdminTrusted, .. }
                    | ToolOrigin::Mcp(_)
                    | ToolOrigin::Skill(_)
            )
        {
            return Err(RegistrationError::TrustViolation { /* ... */ });
        }

        // 3. ForceDefer × ToolSearchMode::Disabled（ADR-009 §2.2）
        // 4. 名字冲突裁决（§2.5.1 矩阵）
        // 5. 通过 → 写入 inner.tools，bump generation
    }
}
```

## 6. Feature Flags

```toml
[features]
default = ["builtin-toolset", "deny-patterns"]
builtin-toolset = ["dep:regex", "dep:grep-searcher"]
deny-patterns   = ["dep:regex"]
testing         = []                                # MockTool / MockCapabilityRegistry 暴露
```

## 7. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("validation: {0}")]
    Validation(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("sandbox: {0}")]
    Sandbox(#[from] SandboxError),

    #[error("timeout")]
    Timeout,

    #[error("interrupted")]
    Interrupted,

    #[error("result too large: {original} {metric:?} > {limit} {metric:?}")]
    ResultTooLarge { original: u64, limit: u64, metric: BudgetMetric },

    #[error("offload failed: {0}")]
    OffloadFailed(String),

    #[error("required capability missing: {0:?}")]
    CapabilityMissing(ToolCapability),

    #[error("dynamic schema resolution failed: {0}")]
    SchemaResolution(String),

    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("duplicate tool name: {0}")]
    Duplicate(String),

    #[error("trust violation: required {required:?}, got {provided:?}")]
    TrustViolation { required: TrustLevel, provided: TrustLevel },

    #[error("capability not permitted for trust level {trust:?}: {cap:?}")]
    CapabilityNotPermitted { trust: TrustLevel, cap: ToolCapability },

    #[error("required capability missing in CapabilityRegistry: {0:?}")]
    CapabilityMissing(ToolCapability),

    #[error("invalid descriptor: {0}")]
    InvalidDescriptor(String),

    /// `DeferPolicy::ForceDefer` 工具在 `ToolSearchMode::Disabled` Session 中注册失败（ADR-009 §2.2）
    #[error("deferral required but tool search is disabled: {0}")]
    DeferralRequired(String),
}
```

## 8. 使用示例

### 8.1 基本注册

```rust
let registry = ToolRegistry::builder()
    .with_builtin_toolset(BuiltinToolset::Default)
    .with_tool(Box::new(MyBusinessTool::new()))
    .build()?;

let snapshot = registry.snapshot();
```

### 8.2 创建 Pool

```rust
let pool = ToolPool::assemble(
    &snapshot,
    &ToolPoolFilter {
        allowlist: None,
        denylist: HashSet::from_iter(["bash".into()]),
        mcp_included: true,
        plugin_included: true,
        group_allowlist: None,
        group_denylist: HashSet::new(),
        ..Default::default()
    },
    &ToolSearchMode::Auto,
    &tool_pool_model_profile,
    &schema_resolver_ctx,
).await?;
```

### 8.3 Orchestrator 分发

```rust
let orchestrator = ToolOrchestrator::new(
    /* concurrency_limit */ 10,
    blob_store,
    event_emitter,
);
let results = orchestrator.dispatch(
    vec![
        ToolCall { tool_use_id: ToolUseId::new(), tool_name: "grep".into(),  input: /* ... */ },
        ToolCall { tool_use_id: ToolUseId::new(), tool_name: "glob".into(),  input: /* ... */ },
    ],
    ctx,
).await;
```

### 8.4 工具实现样板（流式 + capability + budget 自动应用）

```rust
struct AgentTool;

#[async_trait]
impl Tool for AgentTool {
    fn descriptor(&self) -> &ToolDescriptor { &AGENT_TOOL_DESCRIPTOR }

    async fn validate(&self, input: &Value, _ctx: &ToolContext)
        -> Result<(), ValidationError>
    { /* ... */ }

    async fn check_permission(&self, _input: &Value, _ctx: &ToolContext)
        -> PermissionCheck
    {
        PermissionCheck::AllowIfMode(PermissionMode::AcceptEdits)
    }

    async fn execute(&self, input: Value, ctx: ToolContext)
        -> Result<ToolStream, ToolError>
    {
        let runner: Arc<dyn SubagentRunnerCap> =
            ctx.capability(ToolCapability::SubagentRunner)?;
        let spec: SubagentSpec = serde_json::from_value(input)?;
        let parent = ctx.parent_for_subagent();
        let handle = runner.spawn(spec, parent).await?;

        let stream = async_stream::stream! {
            yield ToolEvent::Progress(ToolProgress::start("subagent spawned"));
            match handle.wait().await {
                Ok(ann) => {
                    let part = MessagePart::Text(format!("{:?}", ann));
                    yield ToolEvent::Final(ToolResult::single_text(part));
                }
                Err(err) => yield ToolEvent::Error(ToolError::Internal(err.to_string())),
            }
        };
        Ok(Box::pin(stream))
    }
}
```

## 9. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 每个内建 Tool 的 validate / check_permission / execute 起始与结束事件 |
| Mock | `MockTool::returning(value)` / `MockTool::failing(error)` / `MockTool::stream(events)` |
| 并发 | 100 个 concurrency_safe Tool 并行；100 个不安全 Tool 串行不重叠 |
| 预算 | 流式输出超过 budget → ToolResult.overflow 已填，BlobStore 写入字节数与原文一致；`Reject` 模式抛 ResultTooLarge |
| 心跳 | `LongRunningPolicy.stall_threshold` 命中 → 自动注入 Progress；`hard_timeout` 命中 → Timeout |
| 契约 | Pool 排序稳定（相同 snapshot + filter 产生相同顺序）；ProviderRestriction 过滤正确 |
| Trust | User-controlled Plugin 注册 destructive Tool 失败；UserControlled 申请 SubagentRunner cap 失败 |
| 裁决 | Builtin 与同名 Plugin 共存 → 保留 Builtin，emit ShadowedRegistration |
| Hook | PreToolUse(PreToolUseOutcome) 三件套生效（rewrite_input / override_permission / additional_context 互不冲突）；TransformToolResult 生效；PostToolUse 异常不污染 result |
| Coerce | 字符串化 number/bool/json 自动反序列化通过 |

## 10. 可观测性

| 指标 | 单位 | 说明 |
|---|---|---|
| `octopus.tool.invocations_total` | counter | 按 `tool_name` × `outcome` 分桶 |
| `octopus.tool.duration_ms` | histogram | 执行耗时 |
| `octopus.tool.permission_denials_total` | counter | 被拒次数 |
| `octopus.tool.orchestrator_parallel_depth` | gauge | 当前并行度 |
| `octopus.tool.bytes_offloaded_total` | counter | 溢出落盘字节（ADR-010） |
| `octopus.tool.budget_hit_rate` | ratio | 命中预算的请求占比 |
| `octopus.tool.long_running_stall_total` | counter | 触发心跳的次数 |
| `octopus.tool.registration_shadowed_total` | counter | 被遮蔽注册次数（按 reason 分桶） |
| `octopus.tool.dynamic_schema_resolution_ms` | histogram | dynamic_schema 解析耗时 |

## 11. 反模式

- 在 Tool 里直接 import UI 框架（违反 ADR-002）
- `is_concurrency_safe` 不评估直接设 `true`（破坏 Fail-Closed）
- Tool 维持全局可变状态（应 stateless，state 走 Event Journal）
- 直接返回大 blob 而不走 ResultBudget offload（影响 Prompt Cache + 上下文）
- 在工具实现里直接 `Arc<dyn SubagentRunner>` 等内部结构（应通过 `ToolCapability`）
- Plugin 工具用 `name` 覆盖 builtin（被裁决矩阵拒绝）
- Hook 在 `PostToolUse` 里再次调用 `tool.execute`（被禁，会引发循环检测）

## 12. 相关

- D3 · `api-contracts.md` §Tool
- D7 · `extensibility.md` §3 Tool
- ADR-002 Tool 不含 UI
- ADR-003 Prompt Cache 硬约束（Pool 分区根据）
- ADR-006 插件信任域
- ADR-009 Deferred Tool Loading（Pool 三分区 + ToolSearchTool 元工具）
- ADR-010 Tool 结果预算与溢出落盘（`ResultBudget` / `OverflowMetadata` / `read_blob`）
- ADR-011 Tool Capability Handle（`ToolCapability` / `CapabilityRegistry` / 矩阵）
- `crates/harness-tool-search.md`（`ToolSearchTool` 实体所在 crate）
- `crates/harness-hook.md`（Hook 介入点完整清单）
- `crates/harness-permission.md`（PermissionBroker / DecisionScope）
- Evidence: CC-02, CC-03, CC-04, CC-07, CC-33, OC-16, OC-20, OC-21, HER-005, HER-008, HER-010, HER-015
