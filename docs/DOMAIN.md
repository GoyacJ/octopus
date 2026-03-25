# octopus · 领域模型文档（DOMAIN.md）

**版本**: v0.1.0 | **状态**: 正式版 | **日期**: 2026-03-10
**依赖文档**: PRD v0.1.0 · ARCHITECTURE v0.1.0

---

## 目录

1. [领域边界概览](#1-领域边界概览)
2. [核心设计原则](#2-核心设计原则)
3. [Bounded Context：Agent](#3-bounded-contextagent)
4. [Bounded Context：Task](#4-bounded-contexttask)
5. [Bounded Context：Discussion](#5-bounded-contextdiscussion)
6. [Bounded Context：Capability](#6-bounded-contextcapability)
7. [Bounded Context：Team](#7-bounded-contextteam)
8. [Bounded Context：Identity & Access](#8-bounded-contextidentity--access)
9. [跨上下文集成与领域事件](#9-跨上下文集成与领域事件)
10. [Repository 接口定义](#10-repository-接口定义)
11. [领域不变量汇总](#11-领域不变量汇总)
12. [类型系统与强类型 ID](#12-类型系统与强类型-id)

---

## 1. 领域边界概览

### 1.1 Bounded Context 全景图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Identity & Access Context                                                   │
│  Tenant · User · Role · Permission                                           │
│  （多租户隔离的边界守卫，所有其他 Context 的前置依赖）                              │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │ TenantId / UserId
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
          ▼                     ▼                     ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│  Agent Context  │   │  Team Context   │   │ Capability Ctx  │
│                 │   │                 │   │                 │
│  Agent          │   │  Team           │   │  Skill          │
│  MemoryEntry    │◄──│  (Leader+Members│   │  ToolSpec       │
│                 │   │   RoutingRule)  │   │  McpServer      │
│  «CORE DOMAIN»  │   │                 │   │                 │
└────────┬────────┘   └────────┬────────┘   └────────┬────────┘
         │                    │                      │
         │ AgentService.recall()  LeaderPlanningService   SkillService
         │                    │                      │
         ▼                    ▼                      ▼
┌────────────────────────────────────────────────────────┐
│  Task Context                                          │
│  Task · Subtask · Decision · TraceLog                  │
│  TaskEngine(DAG) · ApprovalGate · ExperienceHarvester  │
└────────────────────────────────────────────────────────┘
         │
         │ ExperienceHarvester（共享路径）
         ▼
┌────────────────────────────────────────────────────────┐
│  Discussion Context                                    │
│  DiscussionSession · DiscussionTurn                    │
│  DiscussionEngine · TurnScheduler · ConclusionSummarizer│
└────────────────────────────────────────────────────────┘
```

### 1.2 Context 职责摘要

| Context | 核心职责 | Aggregate Root | 关键服务 |
|---------|---------|----------------|---------|
| **Agent** | Agent 的身份、能力定义与记忆积累 | `Agent` | `AgentService`, `ExperienceHarvester` |
| **Task** | 任务下发、DAG 执行、审批、断点续跑 | `Task` | `TaskEngine`, `LeaderPlanningService`, `ApprovalGateService` |
| **Discussion** | 多 Agent 圆桌讨论驱动与结论合成 | `DiscussionSession` | `DiscussionEngine`, `TurnScheduler`, `ConclusionSummarizer` |
| **Capability** | 工具注册与执行、Skill 组合、MCP 接入 | `Skill`, `McpServer` | `ToolRegistry`, `ToolExecutor`, `SkillService`, `McpGateway` |
| **Team** | 团队组建、Leader 机制、路由规则 | `Team` | `TeamService` |
| **Identity & Access** | 租户隔离、用户认证、RBAC 权限 | `Tenant` | `AuthService`, `PermissionGuard` |

---

## 2. 核心设计原则

### 2.1 Agent 是"人"，不是"模块"

octopus 最核心的设计理念：Agent 是对现实世界中"人"的数字化抽象。这个比喻不只是产品层面的叙事，而是直接影响领域建模决策：

- **三个不可分割的维度**：`Identity`（他是谁）、`Capability`（他能做什么）、`Memory`（他经历过什么）构成 Agent 完整存在。
- **记忆是 Agent 的私有财产**：`MemoryStore` 作为 `AgentService` 的内部依赖，不对外暴露，外部只能通过 `AgentService.recall()` 和 `AgentService.memorize()` 访问。
- **成长性**：Agent 通过 `ExperienceHarvester` 在任务/讨论完成后自主写入记忆，不需要用户干预。

### 2.2 白名单能力模型（Allowlist）

所有 Agent 能力默认为空，必须显式授权。能力授权通过三个正交维度叠加：
1. `tools_whitelist`：直接授权的内置工具
2. `skill_ids`：通过 Skill 间接授权的工具 + Prompt 增强
3. `mcp_bindings`：通过 MCP 接入的外部工具

运行时将三个维度合并为 `AgentRuntimeContext.effective_whitelist`，这是最终生效的权限集合。

### 2.3 Task 与 Discussion 是平行的执行模型

- **Task**：DAG 状态机，Leader 协调多 Agent 并行/串行执行，核心是"做事"。
- **Discussion**：线性发言循环，TurnScheduler 驱动 Agent 轮流表达观点，核心是"讨论"。
- 两者共享 `AgentService`（记忆注入和写回），但不共享执行引擎。强行复用会导致两者都复杂化（ADR-12）。

### 2.4 高风险操作不可绕过审批

无论 `pipeline_approval_mode` 如何配置，`RiskLevel::High` 工具始终触发审批节点。讨论模式下 `RiskLevel::High` 工具硬编码禁用。这是系统安全边界，不可由用户配置覆盖。

---

## 3. Bounded Context：Agent

### 3.1 Aggregate Root：Agent

`Agent` 是系统的核心聚合根，承载三个维度的完整状态。

```rust
pub struct Agent {
    // ── 系统标识 ───────────────────────────────────────────────────────
    pub id:               AgentId,
    pub tenant_id:        TenantId,
    pub created_by:       UserId,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
    pub status:           AgentStatus,

    // ── Identity 维度 ──────────────────────────────────────────────────
    pub identity:         AgentIdentity,

    // ── Capability 维度 ────────────────────────────────────────────────
    pub capability:       AgentCapability,

    // ── Memory 维度（引用，实际内容在向量库中）────────────────────────────
    pub memory_store_id:  MemoryStoreId,   // Agent 私有，外部不可直接访问
}

pub enum AgentStatus {
    Idle,   // 空闲，可接受新任务或参与讨论
    Busy,   // 正在执行任务或参与讨论中
    Error,  // 执行出错，需要用户介入
}
```

#### 3.1.1 值对象：AgentIdentity

```rust
pub struct AgentIdentity {
    pub name:          String,           // Agent 名称，如 "Alex · 市场分析师"
    pub avatar_url:    Option<String>,   // 头像 URL（上传或 AI 生成）
    pub role:          String,           // 角色描述（自然语言）
    pub persona:       Vec<String>,      // 性格标签，如 ["严谨", "逻辑性强"]
    pub system_prompt: String,           // 核心 System Prompt，支持 {{变量}} 语法
    pub prompt_version: u32,             // Prompt 版本号，单调递增
}
```

**不变量**：
- `name` 长度 1–100 字符，在同一租户内不强制唯一（同名 Agent 合法，靠 ID 区分）
- `system_prompt` 不能为空字符串（必须有身份定义）
- `persona` 最多 10 个标签

#### 3.1.2 值对象：AgentCapability

```rust
pub struct AgentCapability {
    pub model_config:     ModelConfig,    // 绑定的 LLM
    pub tools_whitelist:  Vec<String>,    // 显式授权的工具名列表
    pub skill_ids:        Vec<SkillId>,   // 附加的 Skill（运行时展开）
    pub mcp_bindings:     Vec<McpBindingConfig>, // 绑定的 MCP 服务
}

pub struct ModelConfig {
    pub provider:     LlmProvider,     // openai / anthropic / gemini / ollama / compatible
    pub model:        String,          // 具体模型版本，如 "gpt-4o"
    pub temperature:  f32,             // 默认 0.7
    pub max_tokens:   u32,             // 默认 4096
    pub top_p:        Option<f32>,
    pub api_key_ref:  Option<String>,  // 引用密钥标识符（非明文存储）
}

pub struct McpBindingConfig {
    pub server_id:        McpServerId,
    pub enabled_tools:    Option<Vec<String>>, // None = 启用该服务所有工具
}
```

**不变量**：
- `tools_whitelist` 中的工具名必须存在于 `ToolRegistry`，或 MCP 服务注册后自动追加
- 同一 Agent 的 `skill_ids` 不能重复
- `model_config.temperature` 范围 [0.0, 2.0]

#### 3.1.3 实体：MemoryEntry

`MemoryEntry` 是 Agent 记忆系统中的最小可检索单元，存储于向量数据库，不属于关系型 DB 的主表。

```rust
pub struct MemoryEntry {
    pub id:           MemoryEntryId,
    pub agent_id:     AgentId,         // 归属 Agent
    pub content:      String,          // 记忆文本内容（已嵌入向量化）
    pub source_type:  MemorySource,    // 来源类型
    pub source_id:    String,          // 来源 Task ID 或 DiscussionSession ID
    pub created_at:   DateTime<Utc>,
    pub metadata:     HashMap<String, String>, // 扩展字段（如 topic 标签）
}

pub enum MemorySource {
    Task,       // 任务完成后自动提取
    Discussion, // 讨论结束后自动提取
    Manual,     // 用户手动添加
}
```

**不变量**：
- `content` 不能为空
- `MemoryEntry` 只能通过 `AgentService` 操作，不对外暴露 `MemoryStore`

### 3.2 值对象：AgentRuntimeContext（运行时合并快照）

`AgentRuntimeContext` 在任务/讨论执行开始时由 `AgentService` 构建，是运行时生效的能力快照，不持久化。

```rust
pub struct AgentRuntimeContext {
    pub agent_id:            AgentId,
    pub identity:            AgentIdentity,
    pub model_config:        ModelConfig,
    pub effective_whitelist: Vec<String>,  // tools_whitelist ∪ skill grants ∪ mcp tools
    pub effective_prompt:    String,       // system_prompt + skill prompt 片段合并
    pub tool_injection_mode: ToolInjectionMode, // Minimal(default) / Full
    pub memory_entries:      Vec<MemoryEntry>,  // recall() 检索结果注入
}

// 工具数 > 15 时自动切换 Minimal 模式
pub enum ToolInjectionMode { Minimal, Full }
```

### 3.3 领域服务：AgentService

`AgentService` 是外部访问 Agent 所有能力的唯一入口，内部持有 `MemoryStore`。

```rust
impl AgentService {
    // ── CRUD ────────────────────────────────────────────────────────────
    pub async fn create(&self, req: CreateAgentRequest) -> Result<Agent>;
    pub async fn get(&self, id: &AgentId) -> Result<Agent>;
    pub async fn update(&self, id: &AgentId, req: UpdateAgentRequest) -> Result<Agent>;
    pub async fn delete(&self, id: &AgentId) -> Result<()>;
    pub async fn list(&self, tenant_id: &TenantId) -> Result<Vec<Agent>>;

    // ── 运行时上下文构建 ──────────────────────────────────────────────────
    // 供 AgentRunner / DiscussionEngine 在执行前调用
    pub async fn build_runtime_context(
        &self,
        agent_id: &AgentId,
        skill_service: &SkillService,
        tool_registry: &ToolRegistry,
    ) -> Result<AgentRuntimeContext>;

    // ── Memory 行为（体现 Agent 主体性）──────────────────────────────────
    pub async fn recall(&self, agent_id: &AgentId, query: &str, top_k: usize)
        -> Result<Vec<MemoryEntry>>;
    pub async fn memorize(&self, agent_id: &AgentId, entry: NewMemoryEntry)
        -> Result<MemoryEntryId>;

    // ── Memory 管理 UI 接口 ──────────────────────────────────────────────
    pub async fn list_memories(&self, agent_id: &AgentId) -> Result<Vec<MemoryEntry>>;
    pub async fn delete_memory(&self, agent_id: &AgentId, entry_id: &MemoryEntryId)
        -> Result<()>;
    pub async fn add_memory_manual(&self, agent_id: &AgentId, content: &str)
        -> Result<MemoryEntryId>;
}
```

### 3.4 领域服务：ExperienceHarvester

监听领域事件，在任务/讨论完成后异步提取关键经验并写入 Agent 记忆。

```rust
pub struct ExperienceHarvester {
    agent_service: Arc<AgentService>,
    llm_client:    Arc<dyn LlmClient>,  // 用于从对话日志中提取结构化记忆
}

impl ExperienceHarvester {
    // 响应 TaskCompleted 事件
    pub async fn on_task_completed(&self, event: TaskCompletedEvent) -> Result<()>;
    // 响应 DiscussionConcluded 事件
    pub async fn on_discussion_concluded(&self, event: DiscussionConcludedEvent) -> Result<()>;
}
```

**设计决策（ADR-05）**：记忆写入由领域事件触发，异步执行，不阻塞任务/讨论主链路。体现 Agent 的主体性——是 Agent 在"自主学习"，不是系统在"强制写入"。

---

## 4. Bounded Context：Task

### 4.1 Aggregate Root：Task

`Task` 是用户下发的完整工作单元，从创建到完成/失败/终止的全生命周期状态机。

```rust
pub struct Task {
    pub id:                     TaskId,
    pub tenant_id:              TenantId,
    pub created_by:             UserId,
    pub input:                  String,           // 用户原始自然语言描述
    pub mode:                   TaskMode,
    pub status:                 TaskStatus,
    pub plan:                   Vec<Subtask>,      // Leader 生成的执行计划
    pub pipeline_approval_mode: PipelineApprovalMode,
    pub pending_decisions:      Vec<Decision>,     // 待确认决策队列
    pub result:                 Option<String>,    // 最终汇总结果
    pub trace_log:              Vec<LogEntry>,     // 完整执行日志
    pub created_at:             DateTime<Utc>,
    pub completed_at:           Option<DateTime<Utc>>,
    pub terminated_at:          Option<DateTime<Utc>>,
    pub terminated_by:          Option<UserId>,
}

pub enum TaskMode {
    SingleAgent { agent_id: AgentId },
    Team        { team_id: TeamId },
}

pub enum TaskStatus {
    Pending,          // 已创建，等待 Leader 规划
    Planning,         // Leader 正在生成执行计划
    WaitingPlanApproval, // 计划生成完毕，等待用户确认（超阈值时触发）
    Running,          // 子任务执行中
    WaitingApproval,  // 有高风险操作或阶段成果，等待用户审批
    Completed,        // 成功完成
    Failed,           // 执行失败
    Terminated,       // 用户主动终止
}

pub enum PipelineApprovalMode {
    PerStage,    // 每阶段成果确认后再继续
    AutoApprove, // 全链路自动放行（高风险工具仍强制审批）
    Custom,      // 每个 Subtask 独立配置策略
}
```

#### 4.1.1 实体：Subtask

`Subtask` 是 Leader 将 Task 分解后分配给具体 Agent 的执行单元，组成 DAG。

```rust
pub struct Subtask {
    pub id:               SubtaskId,
    pub task_id:          TaskId,
    pub agent_id:         AgentId,
    pub description:      String,        // 发给 Agent 的任务描述
    pub depends_on:       Vec<SubtaskId>,// DAG 依赖关系
    pub status:           SubtaskStatus,
    pub result:           Option<String>,
    pub approval_required: bool,         // 本 Subtask 完成时是否需要用户审批
}

pub enum SubtaskStatus {
    Pending,    // 等待依赖完成
    Running,    // 执行中
    WaitingApproval, // 等待用户审批结果
    Completed,
    Failed,
    Skipped,    // 前置依赖失败时跳过（不中断其他并行分支）
}
```

**不变量**：
- `depends_on` 不能形成环（DAG 约束，创建时拓扑排序校验）
- 单 Agent 模式下 `plan` 有且仅有一个 `Subtask`，`depends_on` 为空

#### 4.1.2 实体：Decision

`Decision` 是 Agent 执行过程中提交给 Leader（或直接给用户）的决策请求，异步处理。

```rust
pub struct Decision {
    pub id:              DecisionId,
    pub task_id:         TaskId,
    pub source_agent_id: AgentId,
    pub decision_type:   DecisionType,
    pub content:         String,           // 问题描述或成果物摘要
    pub artifact_ref:    Option<String>,   // 关联成果物引用
    pub status:          DecisionStatus,
    pub resolution:      Option<String>,   // 最终决策结果
    pub memory_ref:      Option<MemoryEntryId>, // Leader 自动处理时引用的历史记忆
    pub created_at:      DateTime<Utc>,
}

pub enum DecisionType {
    DeliverableReview, // 阶段成果物审核
    Clarification,     // 执行疑问
    RiskAlert,         // 风险提示（高风险工具触发）
}

pub enum DecisionStatus {
    Pending,       // 等待 Leader 或用户处理
    AutoResolved,  // Leader 基于历史记忆自动处理
    UserConfirmed, // 用户确认
    UserRejected,  // 用户拒绝（附原因）
}
```

#### 4.1.3 值对象：LogEntry（TraceLog）

```rust
pub struct LogEntry {
    pub id:          LogEntryId,
    pub task_id:     TaskId,
    pub subtask_id:  Option<SubtaskId>,
    pub agent_id:    Option<AgentId>,
    pub entry_type:  LogEntryType,
    pub content:     String,           // 内容（已脱敏，不含 API Key 等敏感信息）
    pub timestamp:   DateTime<Utc>,
}

pub enum LogEntryType {
    LlmRequest,    // LLM 输入
    LlmResponse,   // LLM 输出（含工具调用）
    ToolCall,      // 工具调用入参
    ToolResult,    // 工具调用结果
    DecisionPoint, // 决策节点
    StatusChange,  // 状态变更
}
```

### 4.2 领域服务：LeaderPlanningService

```rust
pub struct LeaderPlanningService {
    agent_service: Arc<AgentService>,
    llm_client:    Arc<dyn LlmClient>,
}

impl LeaderPlanningService {
    /// 调用 Leader Agent 生成结构化执行计划
    /// Leader 会先 recall() 自身记忆（历史分配经验）再规划
    pub async fn plan(
        &self,
        task: &Task,
        team: &Team,
        leader: &Agent,
    ) -> Result<Vec<Subtask>>;

    /// Leader 规划失败时，降级为用户手动分配模式（PRD 决策 #4）
    pub async fn fallback_to_manual(&self, task_id: &TaskId) -> Result<TaskStatus>;
}
```

### 4.3 领域服务：ApprovalGateService

```rust
impl ApprovalGateService {
    /// 判断当前操作是否需要触发审批节点
    /// 高风险工具：始终触发，不受 pipeline_approval_mode 影响
    pub fn requires_approval(
        &self,
        tool_spec: &ToolSpec,
        task: &Task,
        subtask: &Subtask,
    ) -> bool;

    /// 提交决策请求，Leader 先查记忆，记忆命中则自动处理
    pub async fn submit_decision(&self, req: NewDecision) -> Result<Decision>;

    /// 用户处理决策（确认/拒绝），Leader 将结果写入自身记忆
    pub async fn resolve_decision(
        &self,
        decision_id: &DecisionId,
        resolution: DecisionResolution,
        resolver: &UserId,
    ) -> Result<()>;
}
```

### 4.4 领域服务：TaskRecoveryService

```rust
impl TaskRecoveryService {
    /// Hub 启动时扫描所有状态为 Running 的 Task，恢复断点续跑
    /// 子任务粒度恢复：已完成的 Subtask 不重跑（ADR-07）
    pub async fn recover_crashed_tasks(&self) -> Result<Vec<TaskId>>;
}
```

---

## 5. Bounded Context：Discussion

### 5.1 Aggregate Root：DiscussionSession

`DiscussionSession` 是圆桌/头脑风暴/辩论的核心聚合根，独立于 Task 存在。

```rust
pub struct DiscussionSession {
    pub id:              DiscussionSessionId,
    pub tenant_id:       TenantId,
    pub created_by:      UserId,
    pub topic:           String,              // 讨论议题
    pub mode:            DiscussionMode,
    pub status:          DiscussionStatus,
    pub participant_ids: Vec<AgentId>,        // 2–8 个参与 Agent
    pub moderator_id:    Option<AgentId>,     // 主持人 Agent（Moderated 策略时必填）
    pub turn_strategy:   TurnStrategy,
    pub config:          DiscussionConfig,
    pub turns:           Vec<DiscussionTurn>, // 按 turn_number 有序
    pub conclusion:      Option<String>,      // 结束时合成的结论
    pub created_at:      DateTime<Utc>,
    pub concluded_at:    Option<DateTime<Utc>>,
}

pub enum DiscussionMode {
    Roundtable,  // 圆桌：各方平等，从专业视角分析
    Brainstorm,  // 头脑风暴：鼓励发散，不批判
    Debate,      // 辩论：参与者持对立立场（用户指定或自动分配）
}

pub enum DiscussionStatus {
    Active,    // 进行中
    Paused,    // 暂停（用户暂停或系统等待）
    Concluded, // 已结束（含结论）
}

pub enum TurnStrategy {
    Sequential,  // 顺序轮流（Phase 1 默认）
    Moderated,   // 主持人指定（需要 moderator_id，Phase 1 可选）
    // Reactive  // 按相关性自主发言（Phase 3）
}

pub struct DiscussionConfig {
    pub max_turns_per_agent: u32,      // 每 Agent 最大发言轮数，默认 10
    pub context_window_size: u32,      // 近 K 轮完整保留，更早滚动摘要，默认 10（ADR-14）
    pub tool_augmented:      bool,     // 是否开启工具增强讨论（默认 false）
    pub debate_positions:    Option<HashMap<AgentId, String>>, // 辩论模式下各 Agent 的立场
}
```

**不变量**：
- `participant_ids` 数量 2–8 个（含 moderator 如果 moderator 也是参与者）
- `moderator_id` 必须在 `participant_ids` 中，或是额外创建的专职主持人 Agent
- `DiscussionMode::Debate` 时，所有 participant 必须有明确立场（`debate_positions` 非空，未指定时系统自动分配对立立场）
- `turn_strategy == Moderated` 时 `moderator_id` 不能为 None

#### 5.1.1 实体：DiscussionTurn

```rust
pub struct DiscussionTurn {
    pub id:           DiscussionTurnId,
    pub session_id:   DiscussionSessionId,
    pub turn_number:  u32,            // 全局轮次，从 1 开始
    pub speaker_type: SpeakerType,
    pub speaker_id:   Option<String>, // agent_id 或 user_id
    pub content:      String,         // 发言内容（完整，流式结束后持久化）
    pub created_at:   DateTime<Utc>,
}

pub enum SpeakerType {
    Agent,   // Agent 发言
    User,    // 用户插话（不占 Agent 轮次计数）
    System,  // 系统提示（如"讨论已到达轮次上限"）
}
```

### 5.2 领域服务：DiscussionEngine

`DiscussionEngine` 直接持有 `LlmClient`，驱动发言循环。不经过 `AgentRunner`（Runner 是 ReAct 模式，不适合讨论场景，ADR-12）。

```rust
pub struct DiscussionEngine {
    agent_service:   Arc<AgentService>,
    scheduler:       Box<dyn TurnScheduler>,
    context_builder: DiscussionContextBuilder,
    llm_client:      Arc<dyn LlmClient>,
    event_bus:       Arc<EventBus>,
}

impl DiscussionEngine {
    /// 启动讨论循环，驱动 TurnScheduler 轮流调度 Agent 发言
    pub async fn run(&self, session_id: &DiscussionSessionId) -> Result<()>;

    /// 注入用户插话，写入 DiscussionTurn（speaker_type = User），影响后续上下文
    pub async fn inject_user_message(
        &self,
        session_id: &DiscussionSessionId,
        content: &str,
        user_id: &UserId,
    ) -> Result<DiscussionTurn>;

    /// 触发结论合成（用户主动 / 轮次上限 / 主持人宣布）
    pub async fn conclude(&self, session_id: &DiscussionSessionId) -> Result<String>;

    pub async fn pause(&self,  session_id: &DiscussionSessionId) -> Result<()>;
    pub async fn resume(&self, session_id: &DiscussionSessionId) -> Result<()>;
}
```

### 5.3 领域服务：TurnScheduler（接口）

```rust
#[async_trait]
pub trait TurnScheduler: Send + Sync {
    /// 返回下一个发言的 AgentId，None 表示讨论应自然结束
    async fn next_speaker(
        &self,
        session: &DiscussionSession,
    ) -> Option<AgentId>;
}

/// Phase 1：顺序轮流（完整覆盖 90% 用例，ADR-13）
pub struct SequentialScheduler;

/// Phase 1（可选）：主持人指定下一个发言者
pub struct ModeratedScheduler {
    moderator_agent_id: AgentId,
    llm_client:         Arc<dyn LlmClient>,
}
```

**轮次终止条件**（满足任一）：
1. 所有 Agent 累计发言轮数达到 `config.max_turns_per_agent`
2. 用户主动点击"生成结论"
3. Moderated 策略下主持人宣布讨论结束

### 5.4 值对象：DiscussionContextBuilder

构建每次 Agent 发言前的上下文（系统提示 + 历史 + 记忆），实现滚动摘要策略。

```rust
pub struct DiscussionContextBuilder {
    agent_service: Arc<AgentService>,
    llm_client:    Arc<dyn LlmClient>,  // 用于生成滚动摘要
}

impl DiscussionContextBuilder {
    /// 构建发言上下文：
    /// 1. 读取近 context_window_size 轮完整历史
    /// 2. 更早的轮次生成滚动摘要（ADR-14）
    /// 3. 调用 AgentService.recall() 注入 Agent 个人记忆
    /// 4. 合并 mode-specific 提示（辩论/头脑风暴/圆桌 prompt 模板）
    pub async fn build(
        &self,
        session: &DiscussionSession,
        speaker_agent: &Agent,
    ) -> Result<DiscussionPromptContext>;
}

pub struct DiscussionPromptContext {
    pub system_prompt:      String,   // Agent 身份 + 讨论角色 + 议题 + 立场（辩论模式）
    pub memory_injection:   String,   // recall() 结果注入
    pub recent_turns:       Vec<DiscussionTurn>, // 近 K 轮完整历史
    pub history_summary:    Option<String>,      // 更早历史的滚动摘要
    pub tools:              Option<Vec<ToolSpec>>, // 工具增强讨论时注入（仅 Low 风险）
}
```

### 5.5 领域服务：ConclusionSummarizer

```rust
impl ConclusionSummarizer {
    /// 从完整 turns 中合成结论，包含：
    /// - 各方核心观点摘要
    /// - 主要风险点
    /// - 推荐方案（如果可收敛）
    /// - 后续行动建议
    pub async fn summarize(&self, session: &DiscussionSession) -> Result<String>;

    /// 用户编辑后的结论重新触发记忆写入（PRD 决策 #12：允许用户编辑结论后写入）
    pub async fn finalize_conclusion(
        &self,
        session_id: &DiscussionSessionId,
        conclusion: String, // 用户编辑后的版本
    ) -> Result<()>;  // 内部发布 DiscussionConcluded 事件
}
```

---

## 6. Bounded Context：Capability

### 6.1 Aggregate Root：Skill

`Skill` 是可叠加的能力模块，附加到 Agent 后运行时合并生效，与 Agent 配置正交（ADR-10）。

```rust
pub struct Skill {
    pub id:           SkillId,
    pub name:         String,
    pub description:  String,
    pub source:       SkillSource,
    pub prompt_addon: String,         // 追加到 Agent system_prompt 末尾的提示片段
    pub tool_grants:  Vec<String>,    // 授权的工具名（仍遵循风险规则，无法绕过高风险审批）
    pub mcp_grants:   Vec<McpServerId>, // 自动绑定的 MCP 服务
    pub created_at:   DateTime<Utc>,
}

pub enum SkillSource {
    Builtin,          // 系统内置（5 个：coding/research/data_analysis/content_writing/system_ops）
    UserDefined,      // 用户创建
    Imported,         // 从模板导入
}
```

**不变量**：
- `Builtin` Skill 不可修改、不可删除
- `tool_grants` 中的工具即使通过 Skill 授权，`RiskLevel::High` 工具仍强制触发审批

### 6.2 值对象：ToolSpec

```rust
pub struct ToolSpec {
    pub name:        String,
    pub description: String,
    pub parameters:  serde_json::Value, // JSON Schema
    pub risk_level:  RiskLevel,
    pub category:    ToolCategory,
    pub source:      ToolSource,
}

pub enum RiskLevel { Low, Medium, High }

pub enum ToolCategory {
    FileSystem,      // read_file / write_file / edit_file / list_dir / search_files / grep_files / delete_file
    CodeExecution,   // run_bash / run_python / run_nodejs
    Network,         // web_search / web_fetch / http_get / http_post
    Data,            // json_format / csv_read / csv_write
    System,          // read_env / process_info
    Coordination,    // search_tools / spawn_agent(Phase 2)
    Mcp,             // MCP 接入的外部工具（命名空间前缀 mcp__{server_id}__）
}

pub enum ToolSource {
    Builtin,
    Mcp { server_id: McpServerId, server_name: String },
}
```

### 6.3 Aggregate Root：McpServer

```rust
pub struct McpServer {
    pub id:         McpServerId,
    pub tenant_id:  TenantId,
    pub name:       String,
    pub transport:  McpTransport,
    pub status:     McpServerStatus,
    pub tools:      Vec<ToolSpec>,  // 启动时从 MCP 服务发现
    pub registered_at: DateTime<Utc>,
    pub last_seen:  Option<DateTime<Utc>>,
}

pub enum McpTransport {
    Http  { url: String },
    Stdio { command: String, args: Vec<String> },
}

pub enum McpServerStatus { Online, Offline, Error { reason: String } }
```

**不变量**：
- MCP 工具名格式：`mcp__{server_id}__{tool_name}`，避免与内置工具冲突
- Hub 启动时通过 `restore_registered_servers()` 恢复连接，失败不阻塞启动（标记 Offline）

### 6.4 领域服务：ToolRegistry

```rust
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, ToolSpec>>,
}

impl ToolRegistry {
    pub fn register_builtins(&self);
    pub fn register_mcp_tool(&self, spec: ToolSpec);
    pub fn unregister_mcp_server(&self, server_id: &McpServerId);

    /// 返回 Agent 有权使用的 ToolSpecs（effective_whitelist 过滤）
    pub fn get_for_agent(&self, effective_whitelist: &[String]) -> Vec<ToolSpec>;

    /// search_tools 工具的底层实现（供 Agent 自主发现）
    pub fn search(&self, query: &str, category: Option<ToolCategory>, whitelist: &[String])
        -> Vec<ToolSpec>;
}
```

### 6.5 领域服务：ToolExecutor

```rust
impl ToolExecutor {
    /// 执行工具调用前：
    /// 1. 校验工具在 effective_whitelist 中
    /// 2. 校验 RiskLevel（High 工具必须已通过 ApprovalGate）
    /// 3. 根据 ToolSource 路由到内置实现或 McpGateway
    pub async fn execute(
        &self,
        call: &ToolCall,
        agent: &AgentRuntimeContext,
        approved: bool,  // 高风险工具是否已通过审批
    ) -> Result<ToolResult>;
}
```

---

## 7. Bounded Context：Team

### 7.1 Aggregate Root：Team

```rust
pub struct Team {
    pub id:            TeamId,
    pub tenant_id:     TenantId,
    pub name:          String,
    pub description:   String,
    pub leader_id:     AgentId,           // 必填，Leader 是必要角色
    pub leader_source: LeaderSource,
    pub member_ids:    Vec<AgentId>,      // 不含 Leader（Leader 单独字段管理）
    pub routing_rules: Vec<RoutingRule>,  // 用户自定义规则，优先级高于 LLM 决策
    pub task_history:  Vec<TaskId>,       // 历史任务引用
    pub score:         Option<f32>,       // Phase 2：综合评分
    pub created_at:    DateTime<Utc>,
}

pub enum LeaderSource {
    AutoGenerated, // 系统基于团队描述自动生成 Leader Agent
    UserSelected,  // 用户从现有 Agent 中选择担任 Leader
}
```

**不变量**：
- 每个 Team 有且仅有一个 Leader（`leader_id` 必填）
- `member_ids` 不包含 `leader_id`（Leader 和 Member 角色分离）
- `member_ids` 最少 1 个（Team 至少有 Leader + 1 Member）
- Leader 本质是一个 `Agent`，拥有完整的 Identity/Capability/Memory

#### 7.1.1 值对象：RoutingRule

路由规则是用户定义的任务分配规则，匹配优先级高于 Leader 的 LLM 规划决策。

```rust
pub struct RoutingRule {
    pub id:          RoutingRuleId,
    pub priority:    u32,                  // 数字越小优先级越高
    pub condition:   RoutingCondition,
    pub action:      RoutingAction,
}

pub enum RoutingCondition {
    KeywordContains { keywords: Vec<String> },  // 任务描述包含关键词
    LabelEquals     { label: String },          // 任务来源标签匹配
    ToolRequired    { tool_name: String },       // 涉及特定工具
    Always,                                     // 始终匹配（兜底规则）
}

pub enum RoutingAction {
    AssignTo        { agent_id: AgentId },       // 分配给指定 Agent
    SkipPlanApproval,                            // 跳过计划确认，直接执行
    ForceApproval,                               // 强制触发用户审批
    Notify          { message: String },         // 通知用户（不阻塞执行）
}
```

---

## 8. Bounded Context：Identity & Access

### 8.1 Aggregate Root：Tenant

```rust
pub struct Tenant {
    pub id:         TenantId,
    pub name:       String,
    pub created_at: DateTime<Utc>,
    pub settings:   TenantSettings,
}

pub struct TenantSettings {
    pub max_agents:        Option<u32>,   // None = 不限制
    pub max_teams:         Option<u32>,
    pub allowed_providers: Vec<LlmProvider>, // 限制可用的 LLM Provider
}
```

### 8.2 实体：User

```rust
pub struct User {
    pub id:          UserId,
    pub tenant_id:   TenantId,
    pub username:    String,
    pub email:       Option<String>,
    pub roles:       Vec<RoleId>,
    pub status:      UserStatus,
    pub created_at:  DateTime<Utc>,
    pub last_seen:   Option<DateTime<Utc>>,
}

pub enum UserStatus { Active, Suspended, Locked }
```

### 8.3 值对象：Role & Permission

系统预置角色，租户管理员可基于预置角色创建自定义角色。

```rust
pub enum SystemRole {
    HubAdmin,       // Hub 全部管理权限（含租户创建）
    TenantAdmin,    // 本租户内全部管理权限
    TeamManager,    // 管理指定团队（Agent 配置、任务下发、成员管理）
    Member,         // 创建 Agent、下发任务、查看自己的结果
    Viewer,         // 只读（查看任务结果、Agent 状态）
}

// 讨论模块细粒度权限（可单独授予）
pub enum DiscussionPermission {
    Create,    // 创建讨论会话
    View,      // 查看讨论记录
    Conclude,  // 结束讨论
}
```

### 8.4 值对象：HubConfig（客户端侧）

```rust
pub struct HubConfig {
    pub id:           HubConfigId,
    pub name:         String,          // 用户自定义，如"公司内网 Hub"
    pub hub_type:     HubType,
    pub url:          Option<String>,  // 远程 Hub 地址（本地模式为 None）
    pub status:       HubConnectionStatus,
}

pub enum HubType { Local, Remote }

pub enum HubConnectionStatus {
    Online,
    Offline,
    AuthExpired,
    Connecting,
}
```

**不变量**：
- 单 Client 可注册多个 Hub，不同 Hub 的数据完全隔离（Agent / Team / Memory 不跨 Hub 共享）
- `HubType::Local` 时 Hub 进程与 Tauri Shell 同生命周期（合并模式，ADR-06）

---

## 9. 跨上下文集成与领域事件

### 9.1 领域事件总表

所有领域事件由 `EventBus` 异步分发，事件消费者不阻塞事件发布者。

```rust
pub enum DomainEvent {
    // ── Agent ────────────────────────────────────────────────────────────
    AgentCreated       { agent_id: AgentId, tenant_id: TenantId },
    AgentUpdated       { agent_id: AgentId },
    AgentDeleted       { agent_id: AgentId },
    AgentMemorized     { agent_id: AgentId, entry_count: usize, source_id: String },
    // Agent 完成记忆写入，UI 可展示"刚刚学到了新经验"

    // ── Task ─────────────────────────────────────────────────────────────
    TaskCreated        { task_id: TaskId },
    TaskStatusChanged  { task_id: TaskId, status: TaskStatus },
    SubtaskProgress    { task_id: TaskId, subtask_id: SubtaskId, progress: SubtaskStatus },
    AgentTokenStream   { task_id: TaskId, agent_id: AgentId, token: String },
    DecisionPending    { task_id: TaskId, decisions: Vec<Decision> },
    DecisionAutoResolved { task_id: TaskId, decision_id: DecisionId, memory_ref: String },
    TaskCompleted      { task_id: TaskId, result: String, agent_ids: Vec<AgentId> },
    // ↑ ExperienceHarvester 监听此事件，触发参与 Agent 的记忆写入
    TaskFailed         { task_id: TaskId, error: String },
    TaskTerminated     { task_id: TaskId, terminated_by: UserId },

    // ── Discussion ───────────────────────────────────────────────────────
    DiscussionTurnStarted   { session_id: DiscussionSessionId, agent_id: AgentId },
    DiscussionTokenStream   { session_id: DiscussionSessionId, agent_id: AgentId, token: String },
    DiscussionTurnCompleted { session_id: DiscussionSessionId, turn: DiscussionTurn },
    DiscussionUserInjected  { session_id: DiscussionSessionId, turn: DiscussionTurn },
    DiscussionPaused        { session_id: DiscussionSessionId },
    DiscussionResumed       { session_id: DiscussionSessionId },
    DiscussionConcluded     { session_id: DiscussionSessionId, conclusion: String, participant_ids: Vec<AgentId> },
    // ↑ ExperienceHarvester 监听此事件，触发所有参与 Agent 的记忆写入
    DiscussionMemorized     { session_id: DiscussionSessionId, agent_summaries: Vec<AgentMemorySummary> },

    // ── MCP ──────────────────────────────────────────────────────────────
    McpServerStatusChanged  { server_id: McpServerId, status: McpServerStatus },
}
```

### 9.2 跨 Context 事件流

```
TaskCompleted / DiscussionConcluded
    │
    ▼ (EventBus 异步分发)
ExperienceHarvester.on_*()
    │
    ├── 调用 LlmClient 从完整对话/发言日志中提取结构化关键信息
    │
    └── 对每个参与 Agent 调用 AgentService.memorize()
            │
            ▼
        MemoryStore（LanceDB / Qdrant）写入向量
            │
            ▼
        发布 AgentMemorized 事件
            │
            ▼
        UI：Agent 卡片展示"刚刚学到了新经验" ✨
```

### 9.3 Context 集成规则

| 集成点 | 调用方 | 被调用方 | 集成方式 | 说明 |
|-------|--------|---------|---------|------|
| 执行前注入记忆 | `TaskEngine` / `DiscussionEngine` | `AgentService.recall()` | 同进程函数调用 | 执行前检索相关记忆 |
| 运行时能力构建 | `TaskEngine` / `DiscussionEngine` | `AgentService.build_runtime_context()` | 同进程函数调用 | 合并 Skill + Tool 有效集合 |
| 经验写入 | `ExperienceHarvester` | `AgentService.memorize()` | 事件驱动异步 | TaskCompleted/DiscussionConcluded 触发 |
| Skill 展开 | `AgentService` | `SkillService` | 同进程函数调用 | build_runtime_context 时展开 skill_ids |
| MCP 工具执行 | `ToolExecutor` | `McpGateway` | 同进程函数调用 | 按 ToolSource 路由 |
| 团队信息 | `LeaderPlanningService` | `TeamService` | 同进程函数调用 | 读取成员 Agent 信息辅助规划 |

---

## 10. Repository 接口定义

所有 Repository 定义为 trait，便于本地（SQLite/LanceDB）和远程（PostgreSQL/Qdrant）实现切换。

```rust
// ── Agent ────────────────────────────────────────────────────────────────────
#[async_trait]
pub trait AgentRepository: Send + Sync {
    async fn save(&self, agent: &Agent) -> Result<()>;
    async fn find_by_id(&self, id: &AgentId) -> Result<Option<Agent>>;
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Agent>>;
    async fn delete(&self, id: &AgentId) -> Result<()>;
    async fn update_status(&self, id: &AgentId, status: AgentStatus) -> Result<()>;
}

// ── Memory（向量存储）────────────────────────────────────────────────────────
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn search(&self, store_id: &MemoryStoreId, query: &str, top_k: usize)
        -> Result<Vec<MemoryEntry>>;
    async fn insert(&self, store_id: &MemoryStoreId, entry: NewMemoryEntry)
        -> Result<MemoryEntryId>;
    async fn list(&self, store_id: &MemoryStoreId) -> Result<Vec<MemoryEntry>>;
    async fn delete(&self, store_id: &MemoryStoreId, entry_id: &MemoryEntryId)
        -> Result<()>;
    async fn create_store(&self, store_id: &MemoryStoreId) -> Result<()>;
    async fn delete_store(&self, store_id: &MemoryStoreId) -> Result<()>;
}
// 实现：LanceDbMemoryStore（本地）/ QdrantMemoryStore（远程）

// ── Task ─────────────────────────────────────────────────────────────────────
#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn save(&self, task: &Task) -> Result<()>;
    async fn find_by_id(&self, id: &TaskId) -> Result<Option<Task>>;
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Task>>;
    async fn find_running(&self) -> Result<Vec<Task>>;  // 断点续跑扫表
    async fn update_status(&self, id: &TaskId, status: TaskStatus) -> Result<()>;
    async fn append_log(&self, id: &TaskId, entry: LogEntry) -> Result<()>;
    async fn save_decision(&self, decision: &Decision) -> Result<()>;
    async fn update_decision(&self, id: &DecisionId, status: DecisionStatus, resolution: Option<String>) -> Result<()>;
}

// ── DiscussionSession ─────────────────────────────────────────────────────────
#[async_trait]
pub trait DiscussionRepository: Send + Sync {
    async fn save(&self, session: &DiscussionSession) -> Result<()>;
    async fn find_by_id(&self, id: &DiscussionSessionId) -> Result<Option<DiscussionSession>>;
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<DiscussionSession>>;
    async fn append_turn(&self, turn: &DiscussionTurn) -> Result<()>;
    async fn list_turns(&self, session_id: &DiscussionSessionId) -> Result<Vec<DiscussionTurn>>;
    async fn update_status(&self, id: &DiscussionSessionId, status: DiscussionStatus) -> Result<()>;
    async fn save_conclusion(&self, id: &DiscussionSessionId, conclusion: &str) -> Result<()>;
}

// ── Team ─────────────────────────────────────────────────────────────────────
#[async_trait]
pub trait TeamRepository: Send + Sync {
    async fn save(&self, team: &Team) -> Result<()>;
    async fn find_by_id(&self, id: &TeamId) -> Result<Option<Team>>;
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Team>>;
    async fn delete(&self, id: &TeamId) -> Result<()>;
}

// ── Skill ─────────────────────────────────────────────────────────────────────
#[async_trait]
pub trait SkillRepository: Send + Sync {
    async fn find_by_id(&self, id: &SkillId) -> Result<Option<Skill>>;
    async fn find_by_ids(&self, ids: &[SkillId]) -> Result<Vec<Skill>>;
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Skill>>;
    async fn save(&self, skill: &Skill) -> Result<()>;
}

// ── McpServer ─────────────────────────────────────────────────────────────────
#[async_trait]
pub trait McpServerRepository: Send + Sync {
    async fn save(&self, server: &McpServer) -> Result<()>;
    async fn find_all_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<McpServer>>;
    async fn update_status(&self, id: &McpServerId, status: McpServerStatus) -> Result<()>;
    async fn delete(&self, id: &McpServerId) -> Result<()>;
}
```

---

## 11. 领域不变量汇总

不变量是系统必须在任何情况下保证的业务规则。违反不变量的操作应在领域层返回错误，不应到达基础设施层。

| # | 不变量 | 归属 Context | 违反时行为 |
|---|-------|-------------|----------|
| I-01 | `Agent.system_prompt` 不能为空字符串 | Agent | `DomainError::InvalidSystemPrompt` |
| I-02 | `Agent.memory_store_id` 在 Agent 生命周期内不可更改 | Agent | 禁止该字段出现在 `UpdateAgentRequest` |
| I-03 | `MemoryEntry` 只能通过 `AgentService` 操作，不可直接访问 `MemoryStore` | Agent | 架构层强制（MemoryStore 不对外暴露） |
| I-04 | `MemoryEntry.content` 不能为空 | Agent | `DomainError::EmptyMemoryContent` |
| I-05 | `Task.plan` 中的 `depends_on` 不能形成环（DAG 约束） | Task | 创建时拓扑排序校验，失败返回错误 |
| I-06 | `RiskLevel::High` 工具始终触发审批，`AutoApprove` 模式不可绕过 | Task | `ApprovalGateService` 强制拦截 |
| I-07 | 讨论模式下 `RiskLevel::High` 工具硬编码禁用 | Discussion | `ToolExecutor` 直接拒绝，不经过白名单检查 |
| I-08 | `DiscussionSession.participant_ids` 数量必须在 [2, 8] 范围内 | Discussion | `DomainError::InvalidParticipantCount` |
| I-09 | `DiscussionMode::Debate` 下所有参与者必须有明确立场 | Discussion | 创建时校验，未指定立场时系统自动分配对立立场 |
| I-10 | `TurnStrategy::Moderated` 时 `moderator_id` 不能为 None | Discussion | `DomainError::MissingModerator` |
| I-11 | 每个 `Team` 有且仅有一个 Leader（`leader_id` 必填） | Team | `DomainError::TeamMissingLeader` |
| I-12 | `Team.member_ids` 不包含 `leader_id` | Team | 保存时自动剔除，保证语义清晰 |
| I-13 | MCP 工具名格式必须为 `mcp__{server_id}__{tool_name}` | Capability | 注册时格式校验 |
| I-14 | `Builtin` Skill 不可修改、不可删除 | Capability | `DomainError::CannotModifyBuiltinSkill` |
| I-15 | Agent 记忆在租户内隔离：`memory_store_id` 与 `tenant_id` 绑定，不跨租户查询 | Agent / IAM | `AgentService.recall()` 入口校验 tenant 归属 |
| I-16 | 不同 Hub 的数据完全隔离，Agent/Team/Memory 不跨 Hub 共享 | All | 架构层（多 Hub 各自独立数据库实例） |
| I-17 | `DiscussionTurn` 写入后不可修改（仅可追加） | Discussion | Repository 不提供 `update_turn` 接口 |
| I-18 | `AgentStatus::Busy` 的 Agent 在任务完成前不应再被分配到新任务 | Agent / Task | `LeaderPlanningService` 规划时检查 Agent 状态 |

---

## 12. 类型系统与强类型 ID

所有领域 ID 使用 newtype 模式，避免跨领域 ID 误用（如把 `AgentId` 传给需要 `TeamId` 的函数）。

```rust
// 所有 ID 均为 UUID 字符串包装
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self { Self(uuid::Uuid::new_v4().to_string()) }
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(AgentId);
define_id!(TenantId);
define_id!(UserId);
define_id!(TeamId);
define_id!(TaskId);
define_id!(SubtaskId);
define_id!(DecisionId);
define_id!(DiscussionSessionId);
define_id!(DiscussionTurnId);
define_id!(MemoryStoreId);
define_id!(MemoryEntryId);
define_id!(SkillId);
define_id!(McpServerId);
define_id!(RoutingRuleId);
define_id!(LogEntryId);
define_id!(HubConfigId);

// LLM Provider 枚举（字符串序列化保持兼容性）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    OpenAi,
    Anthropic,
    Gemini,
    Ollama,
    Compatible, // 自定义 OpenAI 兼容接口
}
```

---

*本文档由 PRD v0.1.0 和 ARCHITECTURE v0.1.0 推导生成，描述 octopus Hub 的核心领域模型。*
*后续迭代：Phase 2 引入 `TeamGroup`、团队评分、多用户协作等新实体时同步更新本文档。*
