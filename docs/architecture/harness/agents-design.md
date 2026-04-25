# D6 · Subagent + Team · 多 Agent 协作设计

> 依赖 ADR：ADR-003（Prompt Cache 硬约束）, ADR-004（Agent Team 拓扑抽象）
> 状态：Accepted · `harness-subagent` 与 `harness-team` 独立 crate

## 1. 总体定位

Octopus Harness SDK 区分**两种多 Agent 形态**：

| 形态 | 核心诉求 | Crate |
|---|---|---|
| **Subagent（委派）** | 父 Agent 临时委派有界子任务 | `harness-subagent` |
| **Team（团队）** | 多 Agent 长期协同 + 消息路由 + 共享记忆 | `harness-team` |

两者共享底层基础设施（Engine / Session / Journal），差异在**编排器**与**语义约束**。

## 2. Subagent vs Team · 语义对比

| 维度 | Subagent | Team |
|---|---|---|
| **生命周期** | 任务级（秒～分钟） | 长驻（分钟～天） |
| **关系** | 父子层级 | 对等或角色化 |
| **会话** | 独立 session（fork 或 isolated） | 每成员独立 session |
| **触发** | 父 Agent 通过 `AgentTool::execute` | 外部 `Team::dispatch` 或成员互发 |
| **通信** | 单向（父→子→父 summary） | 双向 / 多向消息总线 |
| **结果交付** | `SubagentAnnouncement` 结构化回父 | `AgentMessage` 在 Team 内流转 |
| **上下文** | 隔离或 fork 父 | 可配置可见性 |
| **共享记忆** | 否 | 可选（Team 级 Memory） |
| **典型用途** | 并行探索、子问题求解 | 多角色协作、长流水线 |
| **参考来源** | Hermes `delegate_task`（HER-014）、CC `AgentTool`（CC-08）、OC `sessions_spawn isolated`（OC-27） | CC Coordinator Mode（CC-12）、OC Multi-Agent（OC-08） |

---

## 3. Subagent 设计（`harness-subagent`）

### 3.1 核心抽象

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

    /// MCP 子代理可用的连接集合（命名引用 + 内联）；trust 校验由
    /// `harness-mcp.md §5.2` 负责。
    pub mcp_servers: Vec<McpServerRef>,

    /// 装配期必须满足的 MCP 依赖；任一 pattern 不命中即 spawn fail-closed。
    pub required_mcp_servers: Vec<McpServerPattern>,

    /// 子代理在审批链中的可交互性；默认 `DeferredInteractive`，决策请求
    /// 镜像到父 Session 的 EventStream。详见 `harness-subagent.md §6.2`。
    pub interactivity: InteractivityLevel,

    /// 单次子代理运行的资源配额；`None` 沿用父 RunBudget。
    /// 形状定义在本文 §7.1，与 `TeamMemberSpec.quota` 共享。
    pub quota: Option<ResourceQuota>,

    /// 父→子上下文协调旋钮（与 `harness-context.md §11.1` 表格对应）：
    /// - `memory_scope`：子 Memdir 装配范围（默认 Inherit，不得越权扩展）
    /// - `input_strategy`：父 transcript 默认裁剪策略（hook 缺位的兜底）
    /// - `system_header_extra`：声明式 system 段附加（不影响父 cache）
    /// - `bootstrap_filter`：Workspace Bootstrap 文件继承策略（默认 ExcludeAll，
    ///   对齐 Claude Code 的 omitClaudeMd）
    /// 字段权威定义见 `harness-subagent.md §2.2`。
    pub memory_scope: SubagentMemoryScope,
    pub input_strategy: SubagentInputStrategy,
    pub system_header_extra: Option<String>,
    pub bootstrap_filter: BootstrapFilter,
}

pub enum SubagentContextMode {
    Isolated,
    ForkFromParent { include_tool_results: bool },
}

pub enum SandboxInheritance {
    Inherit,
    Require(RequiredSandboxCapabilities),
    Override(SandboxPolicy),
}

pub enum AnnounceMode {
    StructuredOnly,
    SummaryText,
    FullTranscript,
}
```

> **权威定义在 `crates/harness-subagent.md §2.2`**：本节展示对照视图，便于和 §4 Team
> 横向对齐。字段语义、错误类型、并发模型、`AnnouncementRenderer` 抽象、
> `SubagentAdmin` 管理面 API 等细节以 crate spec 为准；本文本节仅在两份发生
> 矛盾时由 crate spec 反向修正。

### 3.2 执行流

```text
父 Engine 遇到 AgentTool::execute(spec)
    │
    ▼
[SubagentRunner::spawn(spec, input)]
    │
    ├─ 创建子 Session（fork or isolated）
    ├─ 冻结 renderedSystemPrompt（共享父 prompt cache；对齐 CC-08）
    ├─ 计算子 toolset = 父 toolset - tool_blocklist
    ├─ 创建子 Engine（max_depth - 1）
    ├─ 继承 sandbox 或创建新 sandbox
    │
    ▼
[子 Engine 运行 input]
    │
    ▼
[子 Session 结束]
    │
    ▼
[SubagentAnnouncement 生成]
    │
    ▼
父 Engine 以 user-role `<task-notification>` 注入结果（对齐 CC-08）
```

### 3.3 硬约束

**默认 blocklist**（对齐 HER-014）：

```rust
impl Default for SubagentPolicy {
    fn default() -> Self {
        Self {
            blocklist: ["delegate", "clarify", "memory_write",
                        "send_user_message", "execute_code"].into_iter()
                .map(String::from).collect(),
            max_depth: 1,
            depth_cap: 3,
            max_concurrent_children: 3,
            announce_mode: AnnounceMode::StructuredOnly,
            shared_prompt_cache: true,
        }
    }
}
```

- `delegate_task`：防止无限递归委派；`max_depth` 是业务可调软上限，`depth_cap` 是
  系统硬上限（与 Hermes `_MAX_SPAWN_DEPTH_CAP=3` 同款），任何业务覆盖最终都满足
  `effective = min(max_depth, depth_cap)`，运行期不可突破。
- `clarify` / `send_user_message`：子 Agent 不能直接对用户说话（对齐所有三个参考项目）
- `memory_write`：子 Agent 不得污染全局 Memory
- `execute_code`：PTC 本身就是一层 sandbox，不允许嵌套

### 3.4 并发控制

```rust
pub struct ConcurrentSubagentPool {
    limit: Semaphore,
    running: DashMap<SubagentId, SubagentHandle>,
}
```

- 默认 `max_concurrent_children = 3`（对齐 HER-014）
- 超限时父 Engine 进入等待状态（不抛错）
- 父被中断时，所有子 agent 级联取消

### 3.5 结果注入

```rust
pub struct SubagentAnnouncement {
    pub subagent_id: SubagentId,
    pub status: SubagentStatus,
    pub summary: String,
    pub result: Option<Value>,
    pub usage: UsageSnapshot,
    pub transcript_ref: Option<TranscriptRef>,
}

pub enum SubagentStatus {
    Completed,
    InterruptedByParent,
    MaxIterationsReached,
    Failed(String),
}
```

父 Engine 在下一轮把 announcement 序列化为 user message。默认渲染器
`XmlTaskNotificationRenderer` 输出：

```xml
<task-notification subagent-id="01J..." status="completed">
  <rewrite-hint>This is an internal subagent result, not a direct user message. Rewrite naturally before responding to the end user; do not echo XML, subagent-id, or structured-result verbatim.</rewrite-hint>
  <summary>研究结果摘要（约 500 字以内）</summary>
  <structured-result>{...JSON...}</structured-result>
</task-notification>
```

`<rewrite-hint>` 是父代理"必须改写、不得直接外露"的硬约束承载点（对齐 CC-08 / OC-27）。
渲染器抽象与替换语义见 `crates/harness-subagent.md §7`。

### 3.6 Event 轨迹

```text
SubagentSpawned
    │
    ▼
[子 Session 的完整 Event 流：RunStarted / AssistantDelta / ToolUse / ...]
    │
    ▼
SubagentAnnounced → 父 Session 注入 user message → 父 AssistantMessage 继续
```

### 3.7 主 Agent 与 `execute_code` 的协作（ADR-0016）

> `execute_code` 是**主 Agent 元工具**，对 Subagent 默认不可见（§3.3 default
> blocklist 已含 `execute_code`，由 `harness-subagent §2.5` 双层闸门强制延续）。
> 本节给出主 Agent 何时优先选用 `execute_code` 的语义指引；具体决策见 ADR-0016。

#### 3.7.1 何时优先用 `execute_code`

| 场景 | 是否推荐 | 理由 |
|---|---|---|
| N 个目录各跑一次 grep，比较结果 | ✅ 强烈 | 一次推理替代 N 次 round-trip；与 ADR-0010 ResultBudget 自然集成 |
| 多步、有依赖（step1 命中 0 行 → 跳过 step2） | ✅ 强烈 | 普通批量 `tool_use` 无法表达条件分支 |
| 单次、独立工具调用 | ❌ | 直接发普通 `tool_use` 即可，PTC 反而引入解释器开销 |
| 写文件 / 改配置 / 调网络 | ❌ | 嵌入工具默认白名单仅含 read-only built-in（ADR-0016 §2.6） |
| 跨 Subagent 协作 | ❌ | Subagent 默认看不到 `execute_code`；改走 `delegate(...)` 工具链 |

#### 3.7.2 系统提示模板片段（业务侧 SystemPrompt 增量）

```text
你可以使用 `execute_code` 工具一次性发起多步、有依赖的工具调用。
- 仅当一组工具调用之间存在数据依赖或条件分支时才考虑使用
- 嵌入式工具仅含 read-only：Grep / Glob / FileRead / ListDir / WebSearch / ReadBlob / ToolSearch
- 任何写操作 / 网络发起 / shell 都必须**继续走单独的 tool_use**
- 若该轮只需一个工具，直接调用普通 tool_use 即可，不要为之编排脚本
```

业务侧若开启 `feature_flags.programmatic_tool_calling`，应把上面提示**追加**到
`SessionOptions.system_prompt_addendum` 或 Skill 系统提示，使主 Agent 能正确调度。
**不**默认装入 SDK 内核的 system prompt——SDK 不假设业务系统提示形态。

#### 3.7.3 Event 轨迹示例

```text
ToolUseRequested { name: "execute_code", input.script: "for d in dirs do ..." }
    │
    ▼
[script 内每次 emb.tool(...) 调用]
    ├─ ExecuteCodeStepInvoked { step_seq: 1, embedded_tool: "Grep" }
    ├─ ExecuteCodeStepInvoked { step_seq: 2, embedded_tool: "FileRead" }
    └─ ...
    │
    ▼
ToolUseCompleted { output: <脚本汇总; 命中 ResultBudget 时 ToolResultOffloaded> }
```

事件 schema 详见 `event-schema.md §3.5.2`。

---

## 4. Team 设计（`harness-team`）

### 4.1 核心抽象

```rust
pub struct TeamSpec {
    pub id: TeamId,
    pub members: Vec<TeamMemberSpec>,
    pub topology: TeamTopology,
    pub routing: RoutingPolicy,
    pub message_bus: MessageBusSpec,
    pub shared_memory: Option<SharedMemorySpec>,
    pub lifecycle: TeamLifecycle,
    pub observability: Option<TeamObservability>,

    /// 单次 dispatch 的回合硬上限；详见 `crates/harness-team.md §2.2`。
    /// `PeerToPeer` 拓扑下未设此值视为反模式（§9）。
    pub max_turns_per_goal: Option<u32>,
    /// 同 correlation_id 路由环防御阈值；详见 `crates/harness-team.md §2.2`。
    pub max_messages_per_correlation: Option<u32>,
}

pub struct TeamMemberSpec {
    pub agent_id: AgentId,
    pub role: String,
    pub engine_config: EngineConfig,
    pub visibility: ContextVisibility,
    pub quota: ResourceQuota,
}

pub enum TeamTopology {
    CoordinatorWorker {
        coordinator: AgentId,
        workers: Vec<AgentId>,
    },
    PeerToPeer,
    RoleRouted(RoleRoutingTable),
    Custom(Arc<dyn TopologyStrategy>),
}
```

### 4.2 三种拓扑

#### 4.2.1 Coordinator-Worker

```text
              Coordinator
              /    |    \
             v     v     v
         Worker1 Worker2 Worker3
```

- Coordinator 的 toolset 限定为 `{DispatchTool, MessageTool, StopTeamTool}`（对齐 CC-12）
- Worker 结果以 user-role `<task-notification>` 回 Coordinator
- 适合企业任务分派、研发流水线

#### 4.2.2 Peer-to-Peer

```text
        AgentA ◄────► AgentB
          │     /        │
          │    /         │
          ▼   /          ▼
        AgentC ◄────────┘
```

- 任意 Agent 可 `@mention` 其他 Agent
- 无中心协调者
- 适合协作编辑、多视角评审

#### 4.2.3 Role-Routed

```text
User Message → RoleRouter
                   │
                   ├─ "bug" → BugFixer Agent
                   ├─ "feat" → Planner Agent
                   └─ "review" → Reviewer Agent
```

- 基于 `RoleRoutingTable`（正则 / 分类器 / 关键词）路由
- 适合客服分流、领域专家分诊

### 4.3 消息模型

```rust
pub struct AgentMessage {
    pub message_id: MessageId,
    pub team_id: TeamId,
    pub from: AgentId,
    pub to: Recipient,
    pub payload: MessagePayload,
    pub sent_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}

pub enum Recipient {
    Agent(AgentId),
    Role(String),
    Broadcast,
    Coordinator,
}

pub enum MessagePayload {
    Text(String),
    Structured(Value),
    Request { reply_to: MessageId },
    Response { in_reply_to: MessageId, body: Value },
    Handoff { to: AgentId, summary: String },
}
```

### 4.4 路由策略

```rust
pub enum RoutingPolicy {
    Mention,
    RoleBased,
    Explicit,
    Broadcast,
    Coordinator,
}
```

- **Mention**：消息里 `@agent_id` 被解析为路由目标
- **RoleBased**：`Role("coder")` 路由到所有角色=coder 的成员
- **Explicit**：消息显式指定 `to: Recipient::Agent(id)`
- **Broadcast**：所有成员接收
- **Coordinator**：强制走 Coordinator

### 4.5 Context Visibility（对齐 OC-26）

```rust
pub enum ContextVisibility {
    All,
    Allowlist(Vec<AgentId>),
    AllowlistQuote(Vec<AgentId>),
    Private,
}
```

**关键设计**：触发授权 ↔ 上下文可见性**正交**

- 触发授权：谁能让某 Agent 响应？（由 Topology + RoutingPolicy 决定）
- 上下文可见性：Agent 能看见哪些消息？（由 `ContextVisibility` 决定）

两个独立旋钮，避免 OC-08 里的常见陷阱。

### 4.6 Shared Memory

```rust
pub struct SharedMemorySpec {
    pub provider: Arc<dyn MemoryProvider>,
    pub scopes: HashMap<AgentId, MemoryScope>,
    pub write_policy: SharedWritePolicy,
}

pub enum SharedWritePolicy {
    Unrestricted,
    CoordinatorOnly,
    PerMemberQuota,
    RoleGated(Vec<String>),
}
```

Shared Memory 是 `harness-memory` 的一个特殊 Provider，Team 级共享，访问隔离可配。

### 4.7 生命周期与动态成员

```rust
pub enum TeamLifecycle {
    OneShot { goal: String },
    Persistent { max_idle: Duration },
    ExplicitTerminate,
}
```

- `OneShot`：完成目标后自动终止
- `Persistent`：长驻，空闲超时终止
- `ExplicitTerminate`：需显式 `Team::terminate`

**动态成员**（`crates/harness-team.md §2.1`）：

- `Team::add_member(spec)` / `Team::remove_member(agent, reason)` 允许运行期
  调整 Team 组成；Coordinator 也可通过 `spawn_worker / pause_worker /
  resume_worker` 工具触发同样路径。
- 每次成员变更都会写出 `TeamMemberJoinedEvent`（含 `spec_snapshot_id`）或
  `TeamMemberLeftEvent`，replay 据此重建任意时刻的成员快照——
  `TeamCreatedEvent.member_specs_hash` 不再需要覆盖运行期加入者。
- 长驻 `Persistent` Team 必须配合 watchdog（详见 §8 可观测性 + 反模式
  `team_member_stalled_total`）；停留过久或卡死会触发 `TeamMemberStalled`
  事件并由 watchdog 选择 `Reported` / `Interrupted` / `Removed` 处置。

### 4.8 Event 轨迹

```text
TeamCreated
    │
    ├─ AgentMessageSent (from=User → to=Coordinator)
    │      │
    │      ▼
    ├─ AgentMessageRouted (recipients=[Coordinator])
    │
    ├─ [Coordinator 执行 AgentTool → DispatchTool]
    │   │
    │   ▼
    ├─ AgentMessageSent (from=Coordinator → to=Worker1)
    │      │
    │      ▼
    ├─ AgentMessageRouted (recipients=[Worker1])
    │
    ├─ [Worker1 的 Session Event 流]
    │
    ├─ AgentMessageSent (from=Worker1 → to=Coordinator, payload=Response)
    │
    ├─ TeamTurnCompleted
    │
    ▼
TeamTerminated
```

### 4.9 Coordinator 作为特殊 Engine

Coordinator 本身就是一个 Engine 实例，特点：

- **Toolset 限制**：只有 `DispatchTool`（派发给 Worker）、`MessageTool`（发送消息）、`StopTeamTool`（终止 Team）
- **System prompt**：强调"你是协调者，不能直接执行任务，只能派发"
- **Worker 响应格式**：`<task-notification>` XML（对齐 CC-08）

---

## 5. Subagent 与 Team 如何组合

Team 中的任何成员自身可以再 `spawn` Subagent：

```text
Team
├─ Coordinator
│   └─ [派发任务给 Coder]
├─ Coder (Team 成员)
│   └─ [遇复杂子问题，spawn Subagent 探索]
│       └─ ExploringSubagent  ← 临时、有界
└─ Reviewer
```

- Team 级 Event（`AgentMessageSent`）由 Team Bus 路由
- Subagent 级 Event 仅在 Subagent 自己的 Session 内
- Subagent 的 `SubagentAnnouncement` **不会**自动广播到 Team

## 6. 业务层调用示例

### 6.1 单 Subagent

```rust
let session = harness.create_session(opts).await?;
let mut events = session
    .run_turn(TurnInput::user("研究一下这个代码库的架构"))
    .await?;

while let Some(event) = events.next().await {
    match event? {
        Event::SubagentSpawned { subagent_id, agent_ref, .. } => {
            tracing::info!(%subagent_id, %agent_ref, "子 agent 启动");
        }
        Event::SubagentAnnounced { parent_session_id, summary, .. } => {
            tracing::info!(%parent_session_id, "子 agent 回报");
        }
        _ => {}
    }
}
```

### 6.2 Team

```rust
let team = harness
    .create_team(
        TeamBuilder::new("code-review-team")
            .topology(TeamTopology::CoordinatorWorker {
                coordinator: "orchestrator".into(),
                workers: vec!["coder".into(), "reviewer".into()],
            })
            .member(
                TeamMemberBuilder::new("orchestrator")
                    .role("Planner")
                    .toolset(ToolsetSelector::Preset("coordinator"))
                    .build(),
            )
            .member(
                TeamMemberBuilder::new("coder")
                    .role("Coder")
                    .toolset(ToolsetSelector::Preset("fs-edit"))
                    .sandbox_policy(SandboxPolicy::Docker)
                    .build(),
            )
            .member(
                TeamMemberBuilder::new("reviewer")
                    .role("Reviewer")
                    .toolset(ToolsetSelector::Preset("read-only"))
                    .build(),
            )
            .build(),
    )
    .await?;

let mut events = team
    .dispatch(TeamInput::goal("为 auth 模块添加单元测试并通过代码评审"))
    .await?;

while let Some(event) = events.next().await {
    match event? {
        Event::AgentMessageSent { from, to, payload, .. } => {
            ui.render(from, to, payload);
        }
        Event::TeamTurnCompleted { .. } => {}
        _ => {}
    }
}
```

## 7. 性能与资源限额

### 7.1 配额

```rust
pub struct ResourceQuota {
    pub max_tokens: Option<u64>,
    pub max_tool_calls: Option<u64>,
    pub max_duration: Option<Duration>,
    pub max_cost_cents: Option<u64>,
}
```

每个 Subagent / Team Member 可独立配置。**跨 crate 共享形态**：本类型同时被
`SubagentSpec.quota`（`crates/harness-subagent.md §2.2`）与 `TeamMemberSpec.quota`
（`crates/harness-team.md`，已去重并 cross-ref 本节）引用，是 Subagent / Team
两套语义在资源维度的统一旋钮。

#### 为什么不上抬到 `harness-contracts §3.4`

> 这是经过多轮评估保留的**显式拒绝**——记录决策依据，避免后续被反复重提。

| 评估维度 | 上抬到 contracts | 留在 SDK 共享层（当前选择） |
|---|---|---|
| 事件层引用 | 否（事件只引用 `BudgetKind` 这一维度判别量） | 否（同样不引用） |
| 跨 crate 共享 | subagent / team 两处 | subagent / team 两处（cross-ref agents-design §7.1） |
| L0 契约稳定性压力 | **高**：contracts 定义即视为破坏性边界，运行期参数（如新增 `max_gpu_seconds`）会触发 minor bump 涟漪 | **低**：本节是高层声明式 spec，演化无 contracts 兼容包袱 |
| 文档心智 | 阅读 contracts 时混入"运行期配额配置"，扩大 L0 范围认知 | contracts 仅保留事件 + 决策必要类型，与"L0 = 跨 crate 通用契约"心智一致 |

**结论**：`BudgetKind` 因为出现在 `Event::ContextBudgetExceeded / SubagentStatus::MaxBudget` 等事件结构中，必须落 contracts；`ResourceQuota` 仅在装配期被读取、运行期被检查、不直接进事件载荷，**留在 SDK 共享层是更小的耦合面**。任何提议把它上抬都需要先证明：(a) 至少一条新事件直接消费 `ResourceQuota` 字段，且 (b) 替代方案（即在事件层用 `BudgetKind` + 数值字段拆开）在审计可读性上明显更差。两条都不成立时，维持现状。

### 7.2 预算耗尽

- 耗尽时 Engine 回退到 **StopReason::MaxIterations / MaxBudget**
- 父 Agent / Coordinator 收到 `SubagentAnnouncement`：
  - 命中 `max_turns` 走 `SubagentStatus::MaxIterationsReached`；
  - 命中 `quota` 任一维度走 `SubagentStatus::MaxBudget(BudgetKind)`，
    `BudgetKind` 标识具体维度（token / tool_calls / wall-clock / cost）。
- 不抛错，让父 Agent 决定是否继续（如升级 quota 重新委派）

## 8. 可观测性

| 维度 | 指标 |
|---|---|
| **Subagent** | 委派深度直方图、子任务时长分布、blocklist 命中率 |
| **Team** | 消息数 / 路由命中数 / Coordinator 决策时长 |
| **共享** | Token 使用、成本、失败率 |

详见 `crates/harness-observability.md`。

## 9. 反模式

| 反模式 | 原因 |
|---|---|
| Subagent 直接调用 `send_user_message` | 违反"子 agent 不对用户说话"硬约束 |
| Team 没有 `ContextVisibility` 配置 | 默认全透明可能导致敏感信息泄漏 |
| Coordinator 被赋予 `Bash` 等执行工具 | 违反 CC-12 设计；Coordinator 应只调度 |
| 无限委派（未设 `max_depth`） | 可能引发指数级资源消耗 |
| 在 Subagent 中修改父的 `renderedSystemPrompt` | 破坏 prompt cache 共享 |
| `PeerToPeer` 未设 `max_turns_per_goal` / `max_messages_per_correlation` | 等同放任路由环；命中 `CyclicRouting` 时已经太晚 |
| 长驻 `Persistent` Team 缺少 watchdog（不监听 `TeamMemberStalled`） | 卡死成员会无声占用资源 |
| 运行期热改成员的 `EngineConfig`（model_ref / toolset / sandbox_policy） | 违反 `TeamMemberJoined.spec_snapshot_id` 不可变约定；正确做法是 `remove_member` + 新 spec 的 `add_member` |
| Coordinator 用 `spawn_worker` 频繁 add/remove 同名 Worker 做"一次性子任务" | 应改用 `harness-subagent::AgentTool`：Subagent 才是临时子任务的语义 |
| Coordinator 把 `<task-notification>` XML 原样回吐给终端用户 | 与 Subagent 同款反模式：渲染器中的 `<rewrite-hint>` 是父代理"必须改写"的承载点（CC-12 / OC-27） |
| 引入跨成员共享的 sandbox 实例 | Team 不提供 `TeamSandboxScope`；每成员 sandbox 独立是默认契约，跨成员共享需 ADR 显式记录 |

## 10. 测试策略

| 测试类型 | 覆盖点 |
|---|---|
| Subagent 单测 | Blocklist 生效、Depth 限制、Announcement 格式 |
| Team 单测 | Routing 各策略；Visibility 隔离；消息总线顺序 |
| 集成测试 | Subagent 嵌套 Team、Team 中 Subagent、混合场景 |
| 回放测试 | 给定 Event 序列，重建 Team Projection = 原状态 |
| 性能测试 | 1000 消息 Team、50 并发 Subagent |
