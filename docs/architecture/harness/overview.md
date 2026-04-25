# D1 · Octopus Agent Harness SDK · 架构总览（SAD）

> 版本：1.0（定稿） · 状态：Accepted · 所有模块层面的决策与本文一致
> 证据基线：`docs/architecture/reference-analysis/{claude-code-sourcemap,hermes-agent,openclaw,evidence-index,comparison-matrix}.md`

---

## 1. 愿景与定位

### 1.1 目标

打造**生产级** Agent 基础设施 SDK，把"模型推理 ↔ 外界副作用"解耦成**可组合、可观测、可审计**的管线，汲取 Claude Code / Hermes Agent / OpenClaw 三者经验，规避各自债务。

### 1.2 定义

- **Agent Harness**：承载各种 Agent 运行的基础设施，它**不是 Agent 本身**
- **SDK**：业务侧通过调用方式使用的稳定 API 契约
- **Octopus**：本仓库的产品品牌名
- **业务层**：SDK 之外的形态实现（Desktop/Server/CLI/IDE/...）

### 1.3 非目标

本 SDK **不承担**：

- **前端渲染**（无 UI / 无 Ink / 无 React / 无 Tauri 依赖）
- **具体产品形态**（不提供 HTTP Server、TUI、CLI 可执行）
- **数据库迁移脚本**（`data/main.db` 结构属业务层职责）
- **业务领域逻辑**（业务 Tool 由业务层通过 `trait Tool` 注册）
- **定时任务 / 后台作业 / 定时触发**（cron / schedule / dream 等）
  - SDK 不提供 `Task` / `Scheduler` / `Dream` 等抽象；这些由业务层自行实现或集成外部调度器（K8s CronJob / `tokio-cron-scheduler` / `apalis` 等）
  - SDK 只承诺：若业务层触发一次 Session 执行（通过 `session.run_turn(...)`），SDK 会完整承载本次 Session 的生命周期 + 事件
  - 业务层的长跑/监控任务如要与 Harness 协作，应通过 `Harness::create_session` 拉起 Session 并订阅 EventStream，而非让 SDK 承担调度
- **跨进程 Team 拓扑**（分布式多 Agent）
  - `harness-team` 仅支持**单进程内**的 Team（基于 `tokio::sync::broadcast`）；跨进程/跨机的 Agent 协作需要业务层通过 MCP Server Adapter（参见 `harness-mcp`）或自建消息总线实现
- **终端 UI 组件 / 渲染器 / Shell**（Bash REPL 形式的交互外壳）

### 1.4 术语表（必读）

| 术语 | 定义 | 关键点 |
|---|---|---|
| **Harness** | SDK 实例；一个进程里可以有多个 Harness（不同租户/配置） | 工厂入口，聚合所有能力 |
| **Tenant** | 多租户边界；默认 `TenantId::SINGLE` | 所有 ID 隔离、Permission / Memory / Journal 按 tenant 分区 |
| **Workspace** | 工作空间（通常对应一个项目/代码库） | 持有 bootstrap 文件 + 默认 SessionOptions；Session 可绑定到 workspace |
| **Session** | 一次对话容器（持有 memory / 消息历史 / 工具快照） | **路由键 + 上下文容器**，不是 auth token；生命周期从 create 到 end |
| **Run** | Session 内一次 `run_turn` 的执行周期 | 每个 Run 以 `RunStarted` 开始、`RunEnded` 结束；多 Run 组成 Session |
| **Agent** | 概念：按 role/prompt 定义的"虚拟智能体" | `AgentId` 在 Team 里代表成员角色句柄，不是实例；真正执行单元是 Session |
| **Subagent** | 动态 spawn 的短生命周期子任务 Agent | 父 Session 通过 `AgentTool` 触发；独立 Session；用 `SubagentAnnouncement` 结构化回父 |
| **Team Member** | Team 里的长驻成员 | 每成员对应一个独立 Session；AgentId 长期稳定 |
| **Coordinator** | Team 的调度 Agent | 特殊 Toolset（只含 dispatch/message/stop_team）；不能直接执行任务 |
| **Engine** | Agent 运行内核（主循环） | 被 Session / Subagent / Team 共享；通过 `trait EngineRunner` 解耦 |
| **ToolPool** | 当前 Session 可见的工具集合 | 分固定集（创建期）+ 追加集（reload 追加）；固定集按名字排序，追加集按加入序 |
| **Broker** | PermissionBroker；审批决策者 | 两种形态：DirectBroker（同步回调） / StreamBasedBroker（事件驱动） |
| **Hook** | 业务侧 plug-in，监听 Agent 生命周期事件 | 20 类事件 + 受限能力矩阵；不能 UI 渲染 |
| **Event** | 状态变化的持久化记录 | Event-Sourcing 的真相源；Projection 由 Event 流重建 |

### 1.5 拓扑关系图

```text
Harness
  │
  ├─ Tenant (多个;默认 TenantId::SINGLE)
  │    │
  │    ├─ Workspace (多个)
  │    │    ├─ bootstrap files (AGENTS.md / SOUL.md / ...)
  │    │    └─ default SessionOptions
  │    │
  │    ├─ Session (可独立 OR 绑定到 Workspace)
  │    │    ├─ Engine (主循环实例)
  │    │    ├─ ContextEngine
  │    │    ├─ Memory (builtin Memdir + 可选 external provider)
  │    │    ├─ ToolPool (snapshot)
  │    │    ├─ Hooks (snapshot)
  │    │    ├─ MCP servers (snapshot)
  │    │    └─ Run[] (一次次的 run_turn 产生)
  │    │         └─ Subagent[] (Run 内 spawn,独立 Session)
  │    │              └─ 子 Run[]
  │    │
  │    └─ Team
  │         ├─ Topology (CoordinatorWorker / P2P / RoleRouted)
  │         ├─ MessageBus (JournalPlusBroadcast)
  │         ├─ SharedMemory (可选)
  │         └─ Team Member[]
  │              ├─ 每成员对应一个独立 Session
  │              │    └─ Subagent[] (成员 Session 内也可 spawn subagent)
  │              └─ Coordinator (特殊成员;仅 dispatch/message/stop_team 工具)
  │
  ├─ Journal (EventStore + BlobStore,全局共享)
  └─ Observability (Tracer / Usage / Redactor,全局共享)
```

**关键 Ownership 规则**：

1. **Harness 直属**：Journal / Observability / PluginRegistry / ModelCatalog / CredentialPool
2. **Tenant 隔离**：Session / Team / PermissionRules / Memdir / Pricing 由 tenant 隔离
3. **Session 聚合**：一次 Session 拥有 Engine / ContextEngine / Tool snapshot / Memory / Hooks；**三件套创建期冻结**（ADR-003）
4. **Team Member 独立 Session**：每成员自成一个 Session；Team 只持有 member 的引用（AgentId → SessionId）
5. **Subagent 短生命周期**：spawn 时创建独立 Session，end 后 Session 保留在 Journal 但不再活跃
6. **Event 反向追踪**：所有 Event 均带 `tenant_id + session_id + run_id?`，可从 Journal 反查到所属 Tenant/Session/Run

---

## 2. 六条不可妥协的设计原则

| 编号 | 原则 | 正向要求 | 反例（对照） |
|---|---|---|---|
| **P1** | **内核纯净** | SDK 绝不包含 UI / CLI / HTTP Server / Tauri 或任何业务形态代码 | Claude Code 把 `React.ReactNode` 塞进 Tool 接口（CC-37） |
| **P2** | **依赖倒置** | 所有外部资源（LLM、存储、沙箱、审批、内存后端）皆为 `trait`，业务侧提供实现 | — |
| **P3** | **事件源持久化** | 所有状态变化先以 Event 形式写入 Append-Only Journal，Projection 派生视图 | OpenClaw 事件不回放（OC-05） |
| **P4** | **契约优先** | Rust 类型为主契约源，向外派生 OpenAPI/TypeScript/JSON Schema | — |
| **P5** | **Prompt Cache 硬约束** | Session 运行期内禁止修改 system prompt / toolset / memory 三件套 | Hermes 在 AGENTS.md 明确要求（HER-027）|
| **P6** | **Fail-Closed 默认值** | Tool 默认不并发安全、不只读、破坏性未知；Broker 默认 Deny-All；Hook 层为受控例外（详见 `crates/harness-hook.md §2.6.1` 与 `security-trust.md §10.W`） | Claude Code `buildTool` fail-closed（CC-03）|

> 以上原则是本架构的"宪法"。任何 PR/ADR 不得违反；如需突破，必须先在 ADR 中论证并获得明确批准。

---

## 3. 分层架构

### 3.1 五层结构总图

```text
┌────────────────────────────────────────────────────────────────────────┐
│   Business Layer（业务层 · 不在 SDK 范畴）                                 │
│   octopus-desktop │ octopus-server │ octopus-cli │ apps/* │ 第三方集成  │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ 仅依赖 harness-sdk
┌────────────────────────────────────▼───────────────────────────────────┐
│   L4 · SDK 门面（Facade）                                                 │
│   octopus-harness-sdk                                                   │
│   - Harness · HarnessBuilder · Session · Event · EventStream            │
│   - ext:: (外部 trait) · builtin:: (默认实现) · testing:: (mock)        │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │
┌────────────────────────────────────▼───────────────────────────────────┐
│   L3 · 运行引擎与协作层                                                    │
│   harness-engine      ← 单 Agent 主循环                                  │
│   harness-subagent    ← 父→子 任务委派                                   │
│   harness-team        ← 多 Agent 长期协同                                │
│   harness-plugin      ← 插件宿主                                         │
│   harness-observability ← Trace/Metrics/Replay/Redactor                │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │
┌────────────────────────────────────▼───────────────────────────────────┐
│   L2 · 复合能力层                                                         │
│   harness-tool   harness-tool-search   harness-skill   harness-mcp      │
│   harness-hook   harness-context   harness-session                      │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │
┌────────────────────────────────────▼───────────────────────────────────┐
│   L1 · 原语层                                                             │
│   harness-model   harness-journal   harness-sandbox                     │
│   harness-permission   harness-memory                                    │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │
┌────────────────────────────────────▼───────────────────────────────────┐
│   L0 · 契约层（所有人都可依赖）                                            │
│   harness-contracts                                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 依赖约束

| 约束 | 说明 |
|---|---|
| 单向向下 | L4 → L3 → L2 → L1 → L0；严禁反向或同层深度耦合 |
| 契约层开放 | `harness-contracts` 对所有 crate 可见，不得依赖任何其他 crate |
| 门面聚合 | 任何跨 L2 / L3 crate 的组合能力必须通过 `harness-sdk` 暴露；业务层不得多 crate 直接 import |
| 依赖倒置 | 跨 crate 耦合一律通过 `trait`（多数场景 `dyn` dispatch），不透传具体类型 |

详见 `module-boundaries.md`（D2）。

---

## 4. 19 个 Crate 总览

### 4.1 清单

| # | Crate | 层 | 一句话职责 |
|---|---|---|---|
| 1 | `harness-contracts` | L0 | 公共类型 / Event / Id / Schema / 错误枚举 |
| 2 | `harness-model` | L1 | LLM Provider 抽象 + 凭证池 + Prompt-Cache 策略 |
| 3 | `harness-journal` | L1 | Append-Only Event Store + Projection + Snapshot |
| 4 | `harness-sandbox` | L1 | 执行环境抽象（local/docker/ssh）+ 心跳 |
| 5 | `harness-permission` | L1 | 权限模型 + DirectBroker + StreamBroker + 规则引擎 |
| 6 | `harness-memory` | L1 | Memdir（MEMORY.md/USER.md）+ 外部 Provider Slot + 威胁扫描 |
| 7 | `harness-tool` | L2 | Tool trait + Registry + Pool + 并发编排 |
| 8 | `harness-tool-search` | L2 | Deferred Tool Loading + ToolSearchTool + 物化 backend + scorer |
| 9 | `harness-skill` | L2 | Skill Loader + 多源优先级 + Agent allowlist + SkillTool 三件套（list/view/invoke）+ 内容威胁扫描 |
| 10 | `harness-mcp` | L2 | MCP Client（入站）+ Server Adapter（出站）+ OAuth |
| 11 | `harness-hook` | L2 | Hook Dispatcher + Registry + 多 transport |
| 12 | `harness-context` | L2 | Context 管线（`assemble` 含 ingest 决策 → compact → `after_turn`）|
| 13 | `harness-session` | L2 | Session 生命周期 + Projection + Fork + Hot Reload |
| 14 | `harness-engine` | L3 | 单 Agent 主循环（turn 编排、中断、预算）|
| 15 | `harness-subagent` | L3 | 父→子 任务委派（有界搜索、sidechain）|
| 16 | `harness-team` | L3 | 多 Agent 长期协同（Coordinator/P2P/RoleRouted）|
| 17 | `harness-plugin` | L3 | 插件宿主 + 清单校验 + 信任域二分 |
| 18 | `harness-observability` | L3 | Tracer + Usage + Replay + Redactor |
| 19 | **`harness-sdk`** | **L4** | **对外门面**（业务层唯一直接依赖） |

### 4.2 Crate 数量选择理由

- **向下拆分**：L1/L2 细分到 "单一职责"，便于独立 feature gate、独立演化（如新增 LLM Provider 只改 `harness-model`）
- **向上聚合**：L4 门面只有一个，避免业务层感知内部拆分；业务方只 `use octopus_harness_sdk::prelude::*;` 就能覆盖 95% 场景
- **Subagent 与 Team 分离**：两种多 Agent 语义差异显著（生命周期 / 通信 / 场景），独立 crate 便于各自演化（详见 `agents-design.md`）

---

## 5. 14 大功能域

| 编号 | 功能域 | 承载 Crate | 关键能力 |
|---|---|---|---|
| **A** | Agent 执行 | `engine` | 主循环、中断、iteration budget、grace call、状态机 |
| **B** | 模型接入 | `model` | 7+ Provider、凭证池（fill/round/random/least）、API Mode、Prompt-Cache 策略 |
| **C** | 工具系统 | `tool` | Tool trait、Registry、Pool 稳定排序、并发分桶、内置工具集、参数强制转换 |
| **D** | 技能系统 | `skill` | Markdown + frontmatter、多源优先级、per-agent allowlist、模板展开 |
| **E** | MCP 双向 | `mcp` | Client（stdio/http/ws/sse/in-proc）+ Server Adapter + OAuth + Elicitation |
| **F** | 钩子系统 | `hook` | 20 类事件（核心 11 + LLM/API 4 + 转换层 2 + MCP 1 + Tool Search 2）+ PreToolUse 三件套 + Transform* + 多 transport（in-process / Exec / HTTP）+ failure_mode + replay 幂等 |
| **G** | 上下文工程 | `context` | 固定序管线 tool-result-budget→snip→microcompact→collapse→autocompact |
| **H** | 记忆系统 | `memory` | Memdir（`<memory-context>` 栅栏）+ 外部 Provider Slot + 威胁扫描 |
| **I** | 权限与审批 | `permission` | 6 Mode + DirectBroker + StreamBroker + 规则分层 + 危险命令库 |
| **J** | 沙箱 | `sandbox` | Local/Docker/SSH/Noop + 心跳 + CWD marker + Spawn-per-call |
| **K** | 会话与事件 | `journal` + `session` | Event Journal + Projection + Snapshot + Fork + Compact 血缘 |
| **L** | 多 Agent 协作 | `subagent` + `team` | Subagent 委派 + Team（Star/P2P/RoleRouted）+ InterAgentBus |
| **M** | 插件系统 | `plugin` | 四源发现 + Manifest 校验 + Admin/User 二分 + Capability Slot |
| **N** | 观测性 | `observability` | Tracer（OTel）+ Usage + Replay + Redactor |

---

## 6. 关键设计模式

### 6.1 Builder + Type-State（编译期约束）

```rust
pub struct HarnessBuilder<ModelState, StoreState, SandboxState> { /* ... */ }
pub struct Unset;
pub struct Set<T>(pub T);

impl<S, SB> HarnessBuilder<Unset, S, SB> {
    pub fn with_model<M: ModelProvider>(self, m: M)
        -> HarnessBuilder<Set<M>, S, SB> { /* ... */ }
}

impl<M: ModelProvider, S: EventStore, SB: SandboxBackend>
    HarnessBuilder<Set<M>, Set<S>, Set<SB>>
{
    pub async fn build(self) -> Result<Harness> { /* ... */ }
}
```

**效果**：缺少必填依赖时**编译失败**，不会产生运行时 panic。

### 6.2 Event Stream + 业务侧自渲染

SDK 仅暴露 `EventStream: Stream<Item = Result<Event>>`。业务层用 Vue / React / Ink / TUI / Webhook 自行消费渲染事件。

**对比**：Claude Code 把 `React.ReactNode` 塞进 Tool 接口（CC-37），非 Ink 消费者必须重实现。我们杜绝此耦合（ADR-002）。

### 6.3 Permission Broker 双形态

- **DirectBroker**：业务侧实现 `PermissionBroker`，SDK `broker.decide(req).await` 直接回调
- **StreamBasedBroker**：业务侧使用默认实现，SDK 把审批请求作为 `PermissionRequested` 事件推到 stream，业务侧处理后 `harness.resolve_permission(id, decision)` 回调

**推荐选用**：CLI 场景用 Direct；Desktop/Web UI 场景用 Stream。两种并存（ADR-007）。

### 6.4 Event Sourcing + Projection

所有状态变化都写 `Event`，不直接改 projection。`SessionProjection::replay(events)` 构造视图。

**效益**：
- Replay 测试天然支持
- 审计无死角
- Debug：给定问题 session，可重放到任意节点

对齐 Octopus 既有 `runtime/events/*.jsonl` 规范（`AGENTS.md` §Persistence Governance）。详见 ADR-001 / D4。

### 6.5 Context 管线（固定顺序）

`tool-result-budget → snip → microcompact → collapse → autocompact`

每个阶段可插拔 `ContextProvider`，但**顺序固定**以保证 prompt cache 稳定性（CC-06, HER-027）。详见 D8。

### 6.6 Registry + Snapshot

`ToolRegistry / HookRegistry / PluginRegistry / McpRegistry / SkillRegistry` 皆为 `Arc<RwLock<...>>` 结构，对外 `snapshot() -> RegistrySnapshot` —— **业务方拿到的是不可变视图**，避免中途篡改（支撑 P5 Prompt Cache 硬约束）。

### 6.7 RAII Handle 管理

- `SessionHandle` drop 时 flush event + release lock
- `McpClient` drop 时断开 transport
- `SubagentContext` drop 时回收 inline MCP servers、释放 sandbox

---

## 7. 核心流程

### 7.1 一次 Turn 的执行流

```text
Business: session.run_turn(input)
    │
    ▼
[Engine] RunStarted  ──────────►  Journal
    │
    ▼
[Hook] UserPromptSubmit ── 可改写 / 拦截
    │
    ▼
[Context] assemble（含 ingest 决策：recall / skill / hook AddContext）→ budget 检查 → 序列化 prompt
    │
    ▼
[Model] stream inference
    │    ├─ AssistantDeltaProduced   ──► EventStream
    │    └─ ToolUseRequested          ──► 
    │
    ▼
[Orchestrator] 分桶（bool 二档）：is_concurrency_safe=true → 并行；false → 串行
    │
    ▼
[Permission] check → (allow / ask → broker → resolve / deny)
    │
    ▼
[Hook] PreToolUse（可改写 input / 返回 permission decision）
    │
    ▼
[Tool] execute（sandbox + activity heartbeat）
    │
    ▼
[Hook] PostToolUse（可改写 result / transform）
    │
    ▼
[Context] 工具结果注入 + budget 检查 → 触发 compact?
    │
    ▼
循环 until 非工具响应 / iteration budget / interrupt
    │
    ▼
RunEnded ──► Journal + UsageProjection 更新
```

### 7.2 Subagent 委派

父 Agent 遇到 `AgentTool::execute` 时，Engine 按 `SubagentPolicy` 生成子 Engine 实例：

- **工具集**：父集 − `DelegationBlocklist`（默认屏蔽 `delegate / clarify / memory_write / send_user_message / execute_code`，对齐 HER-014；详见 `crates/harness-subagent.md` §2.5）
- **系统提示**：frozen snapshot（对齐 CC-08），共享父 prompt cache
- **沙箱**：`Inherit` 或 `Require(policy)` 策略
- **MCP servers**：引用型复用父连接，inline 型结束时断开
- **结果**：以 `SubagentAnnouncement` 结构化回父 session，不允许直接对用户说话
- **深度**：`max_depth = 1`（默认），可调至 3
- **并发**：`max_concurrent_children = 3`（默认）

详见 D6 · agents-design.md 与 crates/harness-subagent.md。

### 7.3 Agent Team 协同

多个 Agent 作为长期成员形成 Team，通过消息总线协作：

| 拓扑 | 说明 | 典型场景 |
|---|---|---|
| **Coordinator-Worker** | 一个 Coordinator 调度多个 Worker | 企业任务分派、研发流水线 |
| **Peer-to-Peer** | Agent 之间直接通信，无中心 | 协作编辑、多视角评审 |
| **Role-Routed** | 消息按角色规则自动路由 | 客服分流、领域专家分诊 |

- Message Bus 基于 `tokio::sync::broadcast` + 持久化到 Journal，支持 replay
- Shared Memory 作为 `harness-memory` 的特殊 provider（Team 级共享）
- Coordinator 本身即一个 Engine 实例，其 toolset 限定为 `{DispatchTool, MessageTool, StopTeamTool}`

详见 D6 与 crates/harness-team.md。

### 7.4 Prompt Cache 硬约束（编译期表达）

```rust
impl Session {
    // 运行期禁写
    pub fn set_system_prompt(&mut self, _: String) -> Result<()> {
        Err(Error::PromptCacheLocked)
    }
}

impl SessionBuilder {
    // 创建期可写
    pub fn set_system_prompt(mut self, v: String) -> Self {
        self.system_prompt = Some(v); self
    }
}
```

系统提示、toolset、memory 三件套只在 Session 创建阶段可写；运行期只读。**热重载**通过 `Session::reload_with(ConfigDelta)` 实现：

- **就地应用**（加工具、加 Skill、加 MCP server、permission rule 扩展）→ `AppliedInPlace { cache_impact }`
  - 其中 `permission_rule_patch` 等纯 SDK 侧变更 → `CacheImpact::NoInvalidation`
  - 其他（加工具/加 Skill/加 MCP server）→ `CacheImpact::OneShotInvalidation`（**下一 turn 产生一次 cache miss，此后重新建立 cache**；不 fork session）
- **破坏性修改**（改 system prompt、删工具、切 model、Memdir 内容变更）→ `ForkedNewSession { parent, child, cache_impact: FullReset }`
- **违禁操作**（跨租户迁移、删除正在使用的 Tool）→ `Rejected`

> **诚实声明**：`AppliedInPlace` ≠ 零成本。SDK 保证 Session 对象不 fork，但 LLM 层面 Anthropic `system_and_3` / OpenAI auto-cache 对 tools / skills / mcp 段的任何变动均会产生**一次 cache 重建**。业务方应把它理解为"一次性代价后续恢复命中"而非"完全无代价"。

详见 ADR-003 + crates/harness-session.md + D8 context-engineering §9。

---

## 8. 可扩展性矩阵

| 扩展点 | 机制 | 典型业务示例 |
|---|---|---|
| 新 LLM Provider | 实现 `trait ModelProvider` | 接入 Qwen / 自家代理 |
| 新 Tool | 实现 `trait Tool` | 业务专属 `SendInvoiceTool` |
| 新 Skill | Markdown + frontmatter，注册到 `SkillLoader`；内置 `skills_list` / `skills_view` / `skills_invoke` 渐进披露 | 无代码扩展 |
| 新 Hook | 实现 `trait HookHandler`；亦可由 Skill frontmatter 声明（生命周期与 skill 绑定） | pre-tool-use 审计日志 |
| 新 MCP Server | 实现 `trait McpTransport` | 自定义传输 |
| 新 Sandbox | 实现 `trait SandboxBackend` | Kubernetes Pod 后端 |
| 新 Permission Broker | 实现 `trait PermissionBroker` | 企业 SSO 审批 |
| 新 Memory Provider | 实现 `trait MemoryProvider` | 向量库召回 |
| 新 EventStore | 实现 `trait EventStore` | Postgres / Redis |
| 新 Plugin | `plugin.yaml` 声明 + `trait Plugin` | 批量捆绑上述所有 |

扩展规则详见 D7 · extensibility.md。

---

## 9. 业务层调用示例

```rust
use octopus_harness_sdk::prelude::*;
use octopus_harness_sdk::builtin::*;

async fn bootstrap() -> Result<Harness> {
    HarnessBuilder::new()
        .with_model(OpenAiProvider::from_env()?)
        .with_store(JsonlEventStore::open("runtime/events").await?)
        .with_sandbox(LocalSandbox::default())
        .with_permission_broker(MyInteractiveBroker::new(tx))
        .with_memory(BuiltinMemory::at("data/memdir"))
        .with_tool_registry(ToolRegistry::builder()
            .with_builtin_toolset(BuiltinToolset::Default)
            .with_tool(Box::new(MyBusinessTool::new()))
            .build())
        .with_skill_loader(SkillLoader::default()
            .with_source(SkillSource::Workspace("data/skills"))
        )
        .with_hook_registry(HookRegistry::builder()
            .with_hook(Box::new(AuditHook::new()))
            .build())
        .with_mcp_config(McpConfig::default()
            .with_server(McpServerSpec::stdio("filesystem", /* ... */))
        )
        .with_observability(ObservabilityBuilder::new()
            .with_otel_endpoint("http://localhost:4317")
            .with_replay_enabled(true)
            .build())
        .build()
        .await
}

async fn single_turn(harness: &Harness) -> Result<()> {
    let session = harness.create_session(SessionOptions::default()).await?;
    let mut events = session
        .run_turn(TurnInput::user("请把 README 翻译成英文"))
        .await?;

    while let Some(event) = events.next().await {
        match event? {
            Event::AssistantDeltaProduced { delta, .. } => print!("{delta}"),
            Event::ToolUseRequested { tool_name, .. } => ui.on_tool_start(tool_name),
            Event::PermissionRequested { request_id, subject, .. } => {
                let d = ui.ask_user(subject).await;
                harness.resolve_permission(request_id, d).await?;
            }
            _ => {}
        }
    }
    Ok(())
}
```

更丰富示例（Team / Subagent / Hot Reload）见 `crates/harness-sdk.md`。

---

## 10. 关键设计决策一览

| 维度 | 决策 | 证据/推荐来源 | 反例 |
|---|---|---|---|
| Tool 接口是否含 UI | **否**，仅 schema + 元数据 | ADR-002 | Claude Code `React.ReactNode`（CC-37） |
| Session 中段能否改系统面 | **否**（编译期禁止） | ADR-003, HER-027 | — |
| 审批决策是否事件化 | **是**，进入 Event Journal | ADR-007 | Hermes `_session_approved` 进程字典（HER-040 末段） |
| 沙箱能否替代审批 | **否**（正交闸门） | ADR-007 | Hermes 容器环境跳审批（HER-041） |
| Subagent 能否直接对用户说话 | **否**（通过 announce） | ADR-004 | — |
| 插件信任域 | **二分**：admin-trusted / user-controlled | ADR-006 | OpenClaw 同信任（OC-18） |
| Skill 注入位置 | user message（不碰 system）；Eager / `SkillsInvokeTool` / Hook 三种路径同终点 | HER-038, CC-26 | — |
| Skill 第三方分发 | 走 Plugin 信任链（ADR-006），不自建 marketplace | ADR-006 | — |
| Hook 能力上限 | 可改写 input / permission / system msg | CC-23 | Hermes 仅 block（HER-034） |
| 配置来源 | 业务层负责，SDK 只收 Options 结构 | — | — |
| 事件回放语义 | 支持完整 replay（Projection 派生） | ADR-001 | OpenClaw 明确不回放（OC-05） |
| Multi-Agent 语义拆分 | Subagent + Team 独立 | ADR-004 | — |
| MCP 方向 | Client（入站）+ Server Adapter（出站） | ADR-005 | — |
| 多租户 | 单租户默认（`TenantId::SINGLE`）| OC-01 参考 | — |
| 凭证池 | 池在 SDK、读取 trait 化 | HER-048 | — |
| Event 流传输 | Stream + broadcast 双形态 | — | — |
| Hot Reload | 三档：AppliedInPlace(带 CacheImpact) / ForkedNewSession / Rejected | ADR-003 | — |

---

## 11. 与 Octopus 现有治理的对齐

| 治理规则 | 本架构映射 |
|---|---|
| `runtime/events/*.jsonl` append-only | `harness-journal` 的 `JsonlEventStore` 直接写到该路径，作为 Octopus 产品事件真相源 |
| `data/main.db` 结构化投影 | 业务层 projection / session current-state / 索引写入 SQLite；不得把 `data/main.db` 当作产品 EventStore 真相源 |
| `config/runtime/*.json` 分层配置 | 业务层读取后映射为 `HarnessOptions` 注入 SDK |
| `config_snapshot_id / effective_config_hash` | SDK 的 `Session::snapshot_id()` 对外暴露 |
| 运行中 session 绑定启动时配置 | Builder 模式 + Session 不可变配置副本 |
| `contracts/openapi/` | 门面 SDK 的 public types 可由 `schemars` 反向派生 OpenAPI |
| 配置严格校验 + last-known-good 回退 | `HarnessOptions::parse` 拒绝未知字段；业务层启动走 `LastKnownGoodConfig`（详见 security-trust.md §9.X） |
| 密钥与业务配置物理分离 | 业务 `*.options.*` 不含明文密钥；`CredentialSource` trait 提供 Vault/1Password/AWS SM 接入 |
| `apps/desktop` adapter-first | Tauri 层调用 `harness-sdk`，与 Browser host 共用契约 |
| `@octopus/schema` 约束 | SDK 的事件 schema 可通过 `schemars` 导出供前端复用 |

---

## 12. 本 SAD 的权威性

- 本文档若与任一 ADR 冲突，**以 ADR 为准**（ADR 层级更低、变更流程更严）
- 本文档若与具体 crate SPEC 冲突，**以 crate SPEC 为准**（SPEC 直接映射代码）
- 本文档若与 `docs/architecture/reference-analysis/*.md` 冲突，那是**事实陈述错误**，请修订本 SAD

---

## 13. 文档索引

完整文档地图见 `README.md`。关键跳转：

- **module 边界** → [module-boundaries.md](./module-boundaries.md)
- **trait 签名总表** → [api-contracts.md](./api-contracts.md)
- **Event / Replay** → [event-schema.md](./event-schema.md)
- **权限** → [permission-model.md](./permission-model.md)
- **多 Agent** → [agents-design.md](./agents-design.md)
- **扩展规则** → [extensibility.md](./extensibility.md)
- **上下文工程** → [context-engineering.md](./context-engineering.md)
- **安全与信任** → [security-trust.md](./security-trust.md)
- **Feature Flag** → [feature-flags.md](./feature-flags.md)
- **ADR 全集** → [adr/](./adr/)
- **Crate SPEC 全集** → [crates/](./crates/)
