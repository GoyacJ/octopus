# Octopus · 技术架构文档（ARCHITECTURE.md）

**版本**: v0.1.0 | **状态**: 正式版 | **日期**: 2026-03-10
**对应 PRD**: v0.1.0

---

## 目录

1. [技术选型总览](#1-技术选型总览)
2. [目标态仓库蓝图（非当前仓库事实）](#2-目标态仓库蓝图非当前仓库事实)
3. [整体架构图](#3-整体架构图)
4. [核心架构决策：合并模式](#4-核心架构决策合并模式)
5. [Client 层（Vue 3）](#5-client-层vue-3)
6. [Hub 层（Rust）](#6-hub-层rust)
7. [内置工具系统（Built-in Tools）](#7-内置工具系统built-in-tools)
8. [Tool Search：工具自主发现](#8-tool-search工具自主发现)
9. [MCP Gateway（自实现协议）](#9-mcp-gateway自实现协议)
10. [Skills 系统](#10-skills-系统)
11. [Agent 运行时设计](#11-agent-运行时设计)
12. [Agent 记忆系统](#12-agent-记忆系统)
13. [Discussion Engine：圆桌会议与头脑风暴](#13-discussion-engine圆桌会议与头脑风暴)
14. [任务调度引擎与断点续跑](#14-任务调度引擎与断点续跑)
15. [数据层](#15-数据层)
16. [Transport 抽象层](#16-transport-抽象层)
17. [权限与安全实现](#17-权限与安全实现)
18. [部署模型实现](#18-部署模型实现)
19. [关键技术决策记录（ADR）](#19-关键技术决策记录adr)

---

## 1. 技术选型总览

> 本节描述的是目标态技术蓝图，不代表当前仓库已经存在对应 manifest、目录或运行时代码。

所有决策已确认，无待决项。

| 层次 | 技术 | 版本要求 | 备注 |
|------|------|---------|------|
| **Desktop Shell** | Tauri 2 | 2.x | Hub 逻辑直接内嵌，无 sidecar |
| **Hub 核心** | Rust + tokio + axum | Rust stable, tokio 1.x, axum 0.7 | 本地走 Tauri invoke，远程走 HTTP |
| **数据库** | sqlx | 0.7+ | async，编译期 SQL 检查 |
| **向量 DB（本地）** | LanceDB | lancedb crate 0.x | 编译进二进制，零外部依赖 |
| **向量 DB（远程）** | Qdrant | qdrant-client 1.x | 官方 Rust client |
| **Frontend** | Vue 3 + TypeScript | Vue 3.4+, TS 5 | Composition API + `<script setup>` |
| **构建工具** | Vite | 5.x | Tauri 官方推荐 |
| **状态管理** | Pinia | 2.x | Vue 官方 |
| **UI 组件** | self-built UI components + design tokens + UnoCSS | — | 与 `AGENTS.md` 的当前前端基线保持一致 |
| **实时推送（本地）** | Tauri Event System | built-in | emit/listen，零网络开销 |
| **实时推送（远程）** | SSE | — | axum 内置支持 |
| **认证** | JWT（jsonwebtoken crate）| — | 仅远程 Hub 模式 |
| **DB 迁移** | sqlx migrate | built-in | 嵌入二进制，启动时自动执行 |
| **序列化** | serde + serde_json | — | 全链路统一 |
| **HTTP 客户端** | reqwest | 0.12+ | 调 LLM API + MCP HTTP transport |
| **MCP 协议** | 自实现（reqwest + serde）| — | JSON-RPC 2.0，不依赖任何 SDK |
| **部署容器化** | Docker + Docker Compose | — | 远程 Hub 分发 |

---

## 2. 目标态仓库蓝图（非当前仓库事实）

以下结构只表达后续实现阶段的目标态蓝图。除非某目录已经出现在 tracked tree 中，否则不得把这些目录、crate、app、manifest 或命令入口描述为当前事实。

```
Octopus/
├── Cargo.toml                          # Cargo workspace root
├── package.json                        # pnpm workspace root
│
├── crates/
│   ├── Octopus-hub/                      # Hub 核心业务逻辑（纯 library crate）
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── agent/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── service.rs          # AgentService：Agent CRUD + 记忆行为入口
│   │   │   │   ├── runner.rs           # AgentRunner：执行循环（ReAct）
│   │   │   │   ├── context.rs          # AgentRuntimeContext：运行时有效配置合并
│   │   │   │   ├── experience.rs       # 经验提取：任务/讨论结束后从对话中提取关键信息
│   │   │   │   ├── memory/
│   │   │   │   │   ├── mod.rs          # MemoryStore trait + MemoryEntry
│   │   │   │   │   ├── lancedb.rs      # 本地向量存储实现
│   │   │   │   │   └── qdrant.rs       # 远程向量存储实现
│   │   │   │   └── llm/
│   │   │   │       ├── mod.rs          # LlmClient trait
│   │   │   │       ├── openai.rs
│   │   │   │       ├── anthropic.rs
│   │   │   │       ├── gemini.rs
│   │   │   │       └── ollama.rs
│   │   │   ├── discussion/             #  Discussion Engine
│   │   │   │   ├── mod.rs
│   │   │   │   ├── engine.rs           # DiscussionEngine：驱动发言循环
│   │   │   │   ├── session.rs          # DiscussionService：会话 CRUD
│   │   │   │   ├── scheduler.rs        # TurnScheduler：Sequential / Moderated 策略
│   │   │   │   ├── context.rs          # DiscussionContext：上下文构建 + 滚动摘要
│   │   │   │   └── summarizer.rs       # ConclusionSummarizer：结论合成
│   │   │   ├── tools/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── executor.rs
│   │   │   │   ├── search.rs
│   │   │   │   ├── builtin/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── filesystem.rs
│   │   │   │   │   ├── execution.rs
│   │   │   │   │   ├── network.rs
│   │   │   │   │   ├── data.rs
│   │   │   │   │   ├── system.rs
│   │   │   │   │   └── coordination.rs
│   │   │   │   └── mcp/
│   │   │   │       └── proxy.rs
│   │   │   ├── skills/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── builtin.rs
│   │   │   │   └── loader.rs
│   │   │   ├── task/
│   │   │   │   ├── engine.rs
│   │   │   │   ├── dag.rs
│   │   │   │   ├── leader.rs
│   │   │   │   ├── approval.rs
│   │   │   │   └── recovery.rs
│   │   │   ├── mcp/
│   │   │   │   ├── gateway.rs
│   │   │   │   ├── client.rs
│   │   │   │   └── protocol.rs
│   │   │   ├── db/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── models.rs
│   │   │   │   ├── queries.rs
│   │   │   │   └── migrations/
│   │   │   ├── event/
│   │   │   │   └── bus.rs
│   │   │   └── config.rs
│   │   └── Cargo.toml
│   │
│   ├── Octopus-server/
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── api/v1/
│   │   │       ├── agents.rs
│   │   │       ├── discussions.rs      #  Discussion API routes
│   │   │       ├── teams.rs
│   │   │       ├── tasks.rs
│   │   │       ├── skills.rs
│   │   │       ├── tools.rs
│   │   │       ├── auth.rs
│   │   │       ├── events.rs
│   │   │       └── middleware/
│   │   └── Cargo.toml
│   │
│   └── Octopus-tauri/
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands.rs
│       │   ├── events.rs
│       │   └── keychain.rs
│       └── Cargo.toml
│
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── components/
│       │   │   ├── chat/
│       │   │   ├── agent/
│       │   │   │   └── memory/
│       │   │   ├── discussion/         #  Discussion UI 组件
│       │   │   │   ├── DiscussionRoom.vue      # 讨论主界面
│       │   │   │   ├── TurnFeed.vue            # 发言流
│       │   │   │   ├── ParticipantBar.vue      # 参与者状态条
│       │   │   │   ├── UserInjection.vue       # 用户插话输入
│       │   │   │   └── ConclusionPanel.vue     # 结论展示
│       │   │   ├── team/
│       │   │   ├── skills/
│       │   │   └── tools/
│       │   ├── views/
│       │   ├── stores/
│       │   │   ├── hub.ts
│       │   │   ├── task.ts
│       │   │   ├── agent.ts
│       │   │   ├── discussion.ts       #  Discussion store
│       │   │   ├── skill.ts
│       │   │   └── decision.ts
│       │   └── lib/
│       │       ├── transport.ts
│       │       └── events.ts
│       └── src-tauri/
│
├── deploy/
└── docs/
```

**关键设计原则**（新增）：
- `discussion/` 是与 `task/` 平行的独立模块，不复用 TaskEngine
- `DiscussionEngine` 通过 `AgentService.recall()` 注入记忆，通过 `ExperienceHarvester` 写回记忆，与记忆系统集成路径和 Task 完全一致
- `DiscussionEngine` 持有 `LlmClient`，直接驱动 LLM 调用，不经过 `AgentRunner`（Runner 是 ReAct 模式，不适合讨论场景）

---

## 3. 整体架构图

```
┌────────────────────────────────────────────────────────────────────┐
│                      桌面应用（本地模式）                             │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                  Vue 3 Frontend (WebView)                    │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ ┌───────┐  │   │
│  │  │Chat 视图 │ │Agent管理│ │ 团队管理 │ │Skills  │ │Tools  │  │   │
│  │  │         │ │ +记忆UI │ │        │ │        │ │       │  │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └────────┘ └───────┘  │   │
│  │  ┌───────────────────────┐                                   │   │
│  │  │ 圆桌讨论视图     │                                   │   │
│  │  │ DiscussionRoom +      │                                   │   │
│  │  │ TurnFeed + Conclusion │                                   │   │
│  │  └───────────────────────┘                                   │   │
│  │  ┌─────────────────────────────────────────────────────┐    │   │
│  │  │      Pinia Stores  +  Transport Layer (invoke/HTTP) │    │   │
│  │  └─────────────────────────────────────────────────────┘    │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             │ Tauri IPC                            │
│  ┌──────────────────────────▼──────────────────────────────────┐   │
│  │           Octopus-tauri（Tauri Rust Core）                     │   │
│  │   commands / events relay / keychain                        │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             │ 同进程函数调用                         │
│  ┌──────────────────────────▼──────────────────────────────────┐   │
│  │                  Octopus-hub（业务核心）                        │   │
│  │                                                             │   │
│  │  ┌─────────────────────────────────────────────────────┐   │   │
│  │  │              Agent System（核心）                    │   │   │
│  │  │  AgentService（Identity/Capability/Memory 入口）     │   │   │
│  │  │  AgentRunner（ReAct 执行循环）                       │   │   │
│  │  │  ExperienceHarvester（事件驱动记忆写入）              │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  │                         ↑ 共享                              │   │
│  │  ┌──────────────────────┴──────────────────────────────┐   │   │
│  │  │           Discussion System                    │   │   │
│  │  │                                                     │   │   │
│  │  │  ┌──────────────────────────────────────────────┐   │   │   │
│  │  │  │  DiscussionEngine                            │   │   │   │
│  │  │  │  ├── 发言循环驱动（TurnScheduler 调度）       │   │   │   │
│  │  │  │  ├── 上下文构建（DiscussionContext）          │   │   │   │
│  │  │  │  │   ├── 调用 AgentService.recall()          │   │   │   │
│  │  │  │  │   └── 滚动摘要 + 近 K 轮历史              │   │   │   │
│  │  │  │  ├── LLM 流式调用（直接持有 LlmClient）       │   │   │   │
│  │  │  │  └── 发言存储 + 事件发布                     │   │   │   │
│  │  │  └──────────────────────────────────────────────┘   │   │   │
│  │  │                                                     │   │   │
│  │  │  ┌──────────────────────────────────────────────┐   │   │   │
│  │  │  │  ConclusionSummarizer                        │   │   │   │
│  │  │  │  触发条件：用户主动 / 轮次上限 / 主持人宣布   │   │   │   │
│  │  │  │  → 合成结论 → 发布 DiscussionConcluded 事件  │   │   │   │
│  │  │  └──────────────────────────────────────────────┘   │   │   │
│  │  │              ↓ DiscussionConcluded 事件              │   │   │
│  │  │  ExperienceHarvester.on_discussion_concluded()      │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  │                                                             │   │
│  │  ┌─────────────────────┐  ┌──────────────────────────────┐  │   │
│  │  │   Task System       │  │   Capability System          │  │   │
│  │  │  TaskEngine(DAG)    │  │  SkillService                │  │   │
│  │  │  LeaderPlanning     │  │  ToolRegistry + Executor     │  │   │
│  │  │  ApprovalGate       │  │  McpGateway                  │  │   │
│  │  │  Recovery           │  │                              │  │   │
│  │  └─────────────────────┘  └──────────────────────────────┘  │   │
│  │                                                             │   │
│  │  SQLite + LanceDB（嵌入式，同进程）                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────┐
│                远程 Hub（Octopus-server）                              │
│   axum + SSE + JWT  →  Octopus-hub（同一代码）  →  PostgreSQL + Qdrant│
└────────────────────────────────────────────────────────────────────┘
                        ↕ HTTPS
          外部 LLM API（OpenAI / Anthropic / Gemini / Ollama）
          外部 MCP Servers（通过 MCP Gateway 接入）
```
---

## 4. 核心架构决策：合并模式

### 4.1 本质

本地模式下，`Octopus-hub`（业务核心）与 `Octopus-tauri`（Tauri Shell）编译进同一进程。Vue 前端通过 Tauri IPC（`invoke`）直接调用 Rust 函数，绕过网络栈。

远程模式下，同一套 `Octopus-hub` 代码被 `Octopus-server` 包装为 axum HTTP 服务。

### 4.2 Transport 抽象

```typescript
// lib/transport.ts
interface Transport {
  call<T>(command: string, args?: Record<string, unknown>): Promise<T>
  subscribe(event: string, handler: (payload: unknown) => void): () => void
}

class LocalTransport implements Transport {
  async call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return await invoke<T>(command, args)
  }
  subscribe(event: string, handler: (payload: unknown) => void) {
    const unlisten = listen(event, (e) => handler(e.payload))
    return () => unlisten.then(f => f())
  }
}

class RemoteTransport implements Transport {
  constructor(private baseUrl: string, private token: string) {}
  async call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const res = await fetch(`${this.baseUrl}/api/v1/${commandToPath(command)}`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${this.token}` },
      body: JSON.stringify(args),
    })
    return res.json()
  }
  subscribe(event: string, handler: (payload: unknown) => void) {
    const es = new EventSource(`${this.baseUrl}/api/v1/events?token=${this.token}`)
    es.addEventListener(event, (e) => handler(JSON.parse((e as MessageEvent).data)))
    return () => es.close()
  }
}

export function createTransport(hub: HubConfig): Transport {
  return hub.type === 'local'
    ? new LocalTransport()
    : new RemoteTransport(hub.url, hub.token)
}
```

---

## 5. Client 层（Vue 3）

### 5.1 Pinia Store 设计

```typescript
// stores/hub.ts
export const useHubStore = defineStore('hub', () => {
  const hubs = ref<HubConfig[]>([])
  const activeHubId = ref<string | null>(null)
  const transport = shallowRef<Transport | null>(null)

  async function switchHub(hubId: string) {
    const hub = hubs.value.find(h => h.id === hubId)
    if (!hub) return
    transport.value = createTransport(hub)
    activeHubId.value = hubId
    await Promise.all([
      useAgentStore().fetchAll(),
      useSkillStore().fetchAll(),
    ])
  }
  return { hubs, activeHubId, transport, switchHub }
})

// stores/agent.ts
// Agent Store 统一管理 Agent 的三个维度：Identity / Capability / Memory
export const useAgentStore = defineStore('agent', () => {
  const agents = ref<Agent[]>([])

  async function fetchAll() {
    const { transport } = useHubStore()
    agents.value = await transport.value!.call<Agent[]>('list_agents')
  }

  // ── Memory 相关操作（经由 Agent，不直接调用 memory service）──────────
  async function listMemories(agentId: string): Promise<MemoryEntry[]> {
    const { transport } = useHubStore()
    // 路由：GET /api/v1/agents/:id/memories
    return transport.value!.call<MemoryEntry[]>('list_agent_memories', { agent_id: agentId })
  }

  async function deleteMemory(agentId: string, entryId: string): Promise<void> {
    const { transport } = useHubStore()
    // 路由：DELETE /api/v1/agents/:id/memories/:entry_id
    await transport.value!.call('delete_agent_memory', { agent_id: agentId, entry_id: entryId })
  }

  async function addMemory(agentId: string, content: string): Promise<void> {
    const { transport } = useHubStore()
    // 路由：POST /api/v1/agents/:id/memories
    await transport.value!.call('add_agent_memory', { agent_id: agentId, content })
  }

  return { agents, fetchAll, listMemories, deleteMemory, addMemory }
})

// stores/skill.ts
export const useSkillStore = defineStore('skill', () => {
  const skills = ref<Skill[]>([])

  async function fetchAll() {
    const { transport } = useHubStore()
    skills.value = await transport.value!.call<Skill[]>('list_skills')
  }

  const grouped = computed(() => ({
    builtin:     skills.value.filter(s => s.source === 'builtin'),
    userDefined: skills.value.filter(s => s.source === 'user_defined'),
    imported:    skills.value.filter(s => s.source === 'imported'),
  }))

  return { skills, grouped, fetchAll }
})
```


```typescript
// stores/discussion.ts
export const useDiscussionStore = defineStore('discussion', () => {
  const sessions = ref<DiscussionSession[]>([])
  const activeSessionId = ref<string | null>(null)
  const turns = ref<Map<string, DiscussionTurn[]>>(new Map())

  // 活跃会话的实时发言流（流式 token 缓冲）
  const streamingTurn = ref<{ agentId: string; content: string } | null>(null)

  async function createSession(req: CreateDiscussionRequest): Promise<DiscussionSession> {
    const { transport } = useHubStore()
    const session = await transport.value!.call<DiscussionSession>('create_discussion', req)
    sessions.value.push(session)
    return session
  }

  async function startSession(sessionId: string): Promise<void> {
    const { transport } = useHubStore()
    await transport.value!.call('start_discussion', { session_id: sessionId })
    activeSessionId.value = sessionId
  }

  // 用户插话：注入一条用户发言到讨论流
  async function injectMessage(sessionId: string, content: string): Promise<void> {
    const { transport } = useHubStore()
    await transport.value!.call('inject_discussion_message', { session_id: sessionId, content })
  }

  // 请求生成结论（用户主动触发）
  async function requestConclusion(sessionId: string): Promise<void> {
    const { transport } = useHubStore()
    await transport.value!.call('conclude_discussion', { session_id: sessionId })
  }

  async function loadTurns(sessionId: string): Promise<void> {
    const { transport } = useHubStore()
    const data = await transport.value!.call<DiscussionTurn[]>('list_discussion_turns', { session_id: sessionId })
    turns.value.set(sessionId, data)
  }

  // 订阅讨论事件（发言流 + 状态变更）
  function subscribeToSession(sessionId: string): () => void {
    const { transport } = useHubStore()
    const unsubToken  = transport.value!.subscribe('discussion.token_stream', (e: any) => {
      if (e.session_id !== sessionId) return
      if (!streamingTurn.value || streamingTurn.value.agentId !== e.agent_id) {
        streamingTurn.value = { agentId: e.agent_id, content: e.token }
      } else {
        streamingTurn.value.content += e.token
      }
    })
    const unsubTurn = transport.value!.subscribe('discussion.turn_completed', (e: any) => {
      if (e.session_id !== sessionId) return
      const sessionTurns = turns.value.get(sessionId) ?? []
      sessionTurns.push(e.turn)
      turns.value.set(sessionId, sessionTurns)
      streamingTurn.value = null
    })
    const unsubConcluded = transport.value!.subscribe('discussion.concluded', (e: any) => {
      if (e.session_id !== sessionId) return
      const session = sessions.value.find(s => s.id === sessionId)
      if (session) {
        session.status = 'concluded'
        session.conclusion = e.conclusion
      }
    })
    return () => { unsubToken(); unsubTurn(); unsubConcluded() }
  }

  return { sessions, activeSessionId, turns, streamingTurn, createSession, startSession, injectMessage, requestConclusion, loadTurns, subscribeToSession }
})
```

### 5.2 Agent 配置界面：Skills 与有效工具预览

```typescript
// 在 Agent 配置 UI 中，展示有效配置预览
const agentSkillIds = ref<string[]>(props.agent.skill_ids)

// 附加 Skill 后，UI 侧实时预览有效工具集合（Identity + Capability 合并视图）
const effectiveTools = computed(() => {
  const base = new Set(agentForm.tools_whitelist)
  for (const skillId of agentSkillIds.value) {
    const skill = useSkillStore().skills.find(s => s.id === skillId)
    skill?.tool_grants.forEach(t => base.add(t))
  }
  return [...base]
})
```

---

## 6. Hub 层（Rust）

### 6.1 HubCore 入口（更新）

新增 `discussion_service` 字段，`ExperienceHarvester` 同时监听 `DiscussionConcluded` 事件：

```rust
pub struct HubCore {
    pub db: DbPool,
    pub vector_store: Arc<dyn MemoryStore>,
    pub event_bus: Arc<EventBus>,
    pub task_engine: Arc<TaskEngine>,
    pub agent_service: Arc<AgentService>,
    pub model_registry: Arc<ModelRegistry>,
    pub discussion_service: Arc<DiscussionService>,   
    pub skill_service: Arc<SkillService>,
    pub tool_registry: Arc<ToolRegistry>,
    pub mcp_gateway: Arc<McpGateway>,
    pub config: HubConfig,
}

impl HubCore {
    pub async fn new(config: HubConfig) -> Result<Self> {
        // ... 省略已有初始化逻辑 ...

        let model_registry = Arc::new(ModelRegistry::new(db.clone(), config.clone()));
        let llm_factory = Arc::new(LlmClientFactory::new(model_registry.clone()));

        // DiscussionService 持有 AgentService 用于记忆注入
        let discussion_service = Arc::new(DiscussionService::new(
            db.clone(),
            agent_service.clone(),
            llm_factory.clone(),
            event_bus.clone(),
        ));

        // ExperienceHarvester 同时监听 TaskCompleted 和 DiscussionConcluded
        let harvester = Arc::new(ExperienceHarvester::new(agent_service.clone(), llm_factory.clone()));

        event_bus.subscribe(DomainEvent::TaskCompleted, {
            let h = harvester.clone();
            move |event| tokio::spawn(async move { let _ = h.on_task_completed(event).await; })
        });

        event_bus.subscribe(DomainEvent::DiscussionConcluded, {   
            let h = harvester.clone();
            move |event| tokio::spawn(async move { let _ = h.on_discussion_concluded(event).await; })
        });

        Ok(Self { db, vector_store, event_bus, task_engine, agent_service, model_registry, discussion_service, skill_service, tool_registry, mcp_gateway, config })
    }
}
```

### 6.2 ModelRegistry 与 LLM Client 工厂

```rust
// Octopus-hub/src/agent/llm/mod.rs

pub enum StreamChunk {
    Token(String),
    ToolCall(ToolCall),
    Done { finish_reason: FinishReason },
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSpec]>,
    ) -> Result<LlmResponse>;

    async fn complete_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSpec]>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;
}

pub struct ResolvedInferenceModel {
    pub profile_id:         ModelProfileId,
    pub provider_type:      LlmProvider,
    pub endpoint:           Option<String>,
    pub model:              String,
    pub temperature:        f32,
    pub max_tokens:         u32,
    pub top_p:              Option<f32>,
    pub secret_binding_ref: Option<String>,
}

pub struct ModelRegistry { /* 省略仓储依赖 */ }

impl ModelRegistry {
    pub fn resolve_profile(&self, tenant_id: &TenantId, profile_id: &ModelProfileId)
        -> Result<ResolvedInferenceModel>;

    pub fn resolve_default_summary_profile(&self, tenant_id: &TenantId)
        -> Result<ResolvedInferenceModel>;
}

pub fn create_llm_client(config: &ResolvedInferenceModel) -> Arc<dyn LlmClient> {
    match config.provider_type {
        LlmProvider::OpenAi    => Arc::new(OpenAiClient::new(config)),
        LlmProvider::Anthropic => Arc::new(AnthropicClient::new(config)),
        LlmProvider::Gemini    => Arc::new(GeminiClient::new(config)),
        LlmProvider::Ollama    => Arc::new(OllamaClient::new(config)),
        LlmProvider::Compatible => Arc::new(OpenAiCompatibleClient::new(config)),
    }
}
```

设计说明：

1. Agent 只持有 `model_profile_id`，不直接持有 Provider 密钥或裸模型配置。
2. `ModelRegistry` 负责把 `ModelProfile + ModelProvider + TenantModelPolicy` 解析成可调用的配置。
3. 本轮 `ModelRegistry` 只负责解析和默认值绑定，不承担自动 fallback、成本路由或健康探测。

---

## 7. 内置工具系统（Built-in Tools）

### 7.1 工具规格定义

```rust
// Octopus-hub/src/tools/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,   // JSON Schema
    pub risk_level: RiskLevel,
    pub category: ToolCategory,
    pub source: ToolSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel { Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    FileSystem, CodeExecution, Network, Data, System, Coordination, Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    Builtin,
    Mcp { server_id: String, server_name: String },
}
```

### 7.2 内置工具全表,参考[Claude_Hidden_Toolkit.md](references/Claude_Hidden_Toolkit.md)

```rust
// Octopus-hub/src/tools/builtin/mod.rs

pub fn all_builtin_specs() -> Vec<ToolSpec> {
    vec![
        // ── 文件系统（7）
        // read_file / write_file / edit_file / list_dir / search_files / grep_files / delete_file

        // ── 代码执行（3）
        // run_bash / run_python / run_nodejs

        // ── 网络（4）
        // web_search / web_fetch / http_get / http_post

        // ── 数据处理（3）
        // json_format / csv_read / csv_write

        // ── 系统（2）
        // read_env / process_info

        // ── 协调（2）
        // search_tools（工具自主发现）/ spawn_agent（协同编排扩展）
    ]
}
```

完整 ToolSpec 定义参见 v0.1.0 中的详细代码，此处省略以避免重复。风险等级分配规则见第 16 章。

### 7.3 ToolRegistry

```rust
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, ToolSpec>>,
}

impl ToolRegistry {
    pub fn register_builtins(&self) { /* ... */ }
    pub fn register_mcp_tool(&self, spec: ToolSpec) { /* ... */ }

    /// 返回 Agent 有权使用的 ToolSpecs（effective_whitelist 过滤后）
    pub fn get_for_agent(&self, effective_whitelist: &[String]) -> Vec<ToolSpec> { /* ... */ }

    /// search_tools 底层查询
    pub fn search(&self, query: &str, category: Option<&str>, whitelist: &[String]) -> Vec<ToolSpec> { /* ... */ }
}
```

### 7.4 ToolExecutor

```rust
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    mcp_gateway: Arc<McpGateway>,
}

impl ToolExecutor {
    pub async fn execute(&self, call: &ToolCall, agent: &AgentRuntimeContext) -> Result<ToolResult> {
        if !agent.effective_whitelist.contains(&call.name) {
            return Err(HubError::ToolNotAllowed { tool: call.name.clone() });
        }
        let spec = self.registry.get(&call.name)
            .ok_or_else(|| HubError::ToolNotFound(call.name.clone()))?;
        match &spec.source {
            ToolSource::Builtin => self.execute_builtin(call).await,
            ToolSource::Mcp { server_id, .. } => {
                self.mcp_gateway.call_tool(server_id, &call.name, &call.arguments).await
            }
        }
    }
}
```

---

## 8. Tool Search：工具自主发现

当 Agent 绑定大量工具时，`search_tools` 采用**懒加载策略**：LLM 上下文中默认只注入少量核心工具，Agent 主动调用 `search_tools` 按需发现其余工具。

### 8.1 两种注入模式

```rust
pub enum ToolInjectionMode {
    /// 精简模式（默认）：只注入 Low 风险内置工具 + search_tools
    Minimal,
    /// 全量模式：注入 effective_whitelist 中所有工具
    Full,
}

impl AgentRuntimeContext {
    pub fn tools_for_llm(&self) -> Vec<ToolSpec> {
        match self.tool_injection_mode {
            ToolInjectionMode::Minimal => {
                let mut tools = vec![SEARCH_TOOLS_SPEC.clone()];
                tools.extend(
                    self.registry.get_for_agent(&self.effective_whitelist)
                        .into_iter()
                        .filter(|t| t.risk_level == RiskLevel::Low && t.source == ToolSource::Builtin)
                );
                tools
            }
            ToolInjectionMode::Full => {
                self.registry.get_for_agent(&self.effective_whitelist)
            }
        }
    }
}
```

工具数 > 15 时自动切换到 `Minimal` 模式（在 `AgentRuntimeContext::build` 中判断）。

---

## 9. MCP Gateway（自实现协议）

不依赖任何 MCP SDK。MCP 协议本质是 JSON-RPC 2.0，支持 HTTP/SSE 和 stdio 两种 transport，均用 `reqwest` 和 `tokio::process` 自行实现。

MCP 工具注册到 `ToolRegistry` 时使用命名空间前缀 `mcp__{server_id}__{tool_name}`，避免与内置工具冲突。

Hub 启动时通过 `restore_registered_servers()` 自动恢复已持久化的 MCP Server 连接，连接失败标记为 `offline` 状态，不阻塞启动。

完整 protocol / client / gateway 实现参见 v0.1.0 详细代码，设计无变更。

---

## 10. Skills 系统

**Skill** 是可组合的能力模块，可叠加附加到 Agent，运行时合并到 `AgentRuntimeContext`。

| 对比点 | Agent 模板 | Skill |
|--------|-----------|-------|
| 本质 | 预配置好的完整 Agent | 可叠加的能力包 |
| 使用方式 | 一键创建 Agent | 附加到已有 Agent |
| 组合性 | 独立 | 多个可同时叠加 |
| 包含内容 | 完整 Agent 配置 | prompt 片段 + 工具授权 + MCP 绑定 |

内置 Skill（5 个）：`builtin:coding` / `builtin:research` / `builtin:data_analysis` / `builtin:content_writing` / `builtin:system_ops`

Skill 的 `tool_grants` 授权的工具继承标准风险规则，Skill 无法绕过高风险工具的审批要求。

完整 Skill 实体定义和内置实现参见 v0.1.0 详细代码，设计无变更。

---

## 11. Agent 运行时设计

### 11.1 AgentService：Agent 的统一入口

```rust
// Octopus-hub/src/agent/service.rs

/// AgentService 是外部访问 Agent 所有能力的唯一入口
/// MemoryStore 是其内部依赖，不对外暴露
pub struct AgentService {
    db: DbPool,
    memory_store: Arc<dyn MemoryStore>,   // 私有，外部不可直接访问
}

impl AgentService {
    // ── Identity / Capability CRUD ────────────────────────────────────
    pub async fn create(&self, req: CreateAgentRequest) -> Result<Agent> { /* ... */ }
    pub async fn get(&self, agent_id: &AgentId) -> Result<Agent> { /* ... */ }
    pub async fn update(&self, agent_id: &AgentId, req: UpdateAgentRequest) -> Result<Agent> { /* ... */ }
    pub async fn delete(&self, agent_id: &AgentId) -> Result<()> { /* ... */ }
    pub async fn list(&self, tenant_id: &TenantId) -> Result<Vec<Agent>> { /* ... */ }

    // ── Memory 行为（体现 Agent 主体性）──────────────────────────────────

    /// Agent 主动回忆与当前任务相关的经验
    /// 调用方：AgentRunner（执行任务前检索上下文）
    pub async fn recall(
        &self,
        agent_id: &AgentId,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<MemoryEntry>> {
        // 校验 agent_id 存在，确保隔离
        let agent = self.get(agent_id).await?;
        self.memory_store.search(&agent.memory_store_id, query, top_k).await
    }

    /// Agent 将经验写入自己的记忆
    /// 调用方：ExperienceHarvester（任务完成后，由 TaskCompleted 事件触发）
    pub async fn memorize(
        &self,
        agent_id: &AgentId,
        entries: Vec<RawExperience>,
    ) -> Result<()> {
        let agent = self.get(agent_id).await?;
        for entry in entries {
            let memory = MemoryEntry {
                id: Uuid::new_v4().to_string(),
                store_id: agent.memory_store_id.clone(),
                agent_id: agent_id.clone(),
                content: entry.content,
                source_task_id: entry.source_task_id,
                created_at: Utc::now(),
            };
            self.memory_store.add(memory).await?;
        }
        Ok(())
    }

    /// 用户手动管理 Agent 记忆（UI 操作）
    pub async fn list_memories(
        &self,
        agent_id: &AgentId,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<MemoryEntry>> {
        let agent = self.get(agent_id).await?;
        self.memory_store.list(&agent.memory_store_id, offset, limit).await
    }

    pub async fn add_memory_manual(
        &self,
        agent_id: &AgentId,
        content: String,
    ) -> Result<MemoryEntry> {
        let agent = self.get(agent_id).await?;
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            store_id: agent.memory_store_id.clone(),
            agent_id: agent_id.clone(),
            content,
            source_task_id: None,   // 手动添加，无来源任务
            created_at: Utc::now(),
        };
        self.memory_store.add(entry.clone()).await?;
        Ok(entry)
    }

    pub async fn delete_memory(
        &self,
        agent_id: &AgentId,
        entry_id: &str,
    ) -> Result<()> {
        let agent = self.get(agent_id).await?;
        self.memory_store.delete(&agent.memory_store_id, entry_id).await
    }
}
```

### 11.2 AgentRunner（集成 Skills + Memory）

```rust
// Octopus-hub/src/agent/runner.rs

pub struct AgentRunner {
    agent: Agent,
    agent_service: Arc<AgentService>,
    skill_service: Arc<SkillService>,
    tool_registry: Arc<ToolRegistry>,
    tool_executor: Arc<ToolExecutor>,
    approval_gate: Arc<ApprovalGateService>,
    llm: Arc<dyn LlmClient>,
    event_bus: Arc<EventBus>,
}

impl AgentRunner {
    pub async fn run(
        &self,
        subtask: &Subtask,
        context: &TaskContext,
        cancel: CancellationToken,
    ) -> Result<SubtaskResult> {
        // 1. 构建运行时上下文（合并 Skills → effective config）
        let skills = self.skill_service.get_for_agent(&self.agent.id).await?;
        let runtime_ctx = AgentRuntimeContext::build(&self.agent, &skills, &self.tool_registry).await;

        // 2. Agent 主动回忆（通过 AgentService，不直接访问 MemoryStore）
        let memories = self.agent_service
            .recall(&self.agent.id, &subtask.description, 5)
            .await?;

        // 3. 构建初始 messages（使用有效 system_prompt + 记忆上下文）
        let mut messages = self.build_messages(&runtime_ctx, subtask, context, &memories);

        // 4. ReAct 循环
        for _ in 0..MAX_ITERATIONS {
            if cancel.is_cancelled() {
                return Ok(SubtaskResult::cancelled());
            }

            let tools_for_llm = runtime_ctx.tools_for_llm();
            let mut stream = self.llm.complete_stream(&messages, Some(&tools_for_llm)).await?;
            let mut response = LlmResponse::default();

            while let Some(chunk) = stream.next().await {
                match chunk? {
                    StreamChunk::Token(t) => {
                        self.event_bus.emit(AgentTokenEvent {
                            subtask_id: subtask.id.clone(),
                            agent_id: self.agent.id.clone(),
                            token: t.clone(),
                        });
                        response.content.push_str(&t);
                    }
                    StreamChunk::ToolCall(tc) => response.tool_call = Some(tc),
                    StreamChunk::Done { finish_reason } => response.finish_reason = finish_reason,
                }
            }

            match response.finish_reason {
                FinishReason::ToolCall => {
                    let call = response.tool_call.unwrap();
                    // 高风险工具强制审批（优先级高于 pipeline 模式）
                    if self.tool_registry.is_high_risk(&call.name) {
                        let approved = self.approval_gate.request_sync(&call, &self.agent).await?;
                        if !approved { return Err(HubError::ToolDenied(call.name)); }
                    }
                    let result = self.tool_executor.execute(&call, &runtime_ctx).await?;
                    messages.push(LlmMessage::tool_result(result));
                }
                FinishReason::Stop => {
                    return Ok(SubtaskResult::completed(response.content));
                }
            }
        }
        Ok(SubtaskResult::failed("Max iterations reached"))
    }
}
```

### 11.3 AgentRuntimeContext 构建

```rust
// Octopus-hub/src/agent/context.rs

#[derive(Debug, Clone)]
pub struct AgentRuntimeContext {
    pub agent_id: AgentId,
    pub model_profile_id: ModelProfileId,
    pub resolved_model: ResolvedInferenceModel,
    /// 有效 System Prompt = agent.identity.system_prompt + skill.prompt_addon (x N)
    pub effective_system_prompt: String,
    /// 有效工具白名单 = agent.capability.tools_whitelist ∪ skill.tool_grants (x N)
    pub effective_whitelist: Vec<String>,
    /// 有效 MCP 绑定 = agent.capability.mcp_bindings.server_id ∪ skill.mcp_grants (x N)
    pub effective_mcp_bindings: Vec<String>,
    pub tool_injection_mode: ToolInjectionMode,
    pub registry: Arc<ToolRegistry>,
}

impl AgentRuntimeContext {
    pub async fn build(
        agent: &Agent,
        skills: &[Skill],
        registry: &Arc<ToolRegistry>,
        model_registry: &Arc<ModelRegistry>,
    ) -> Self {
        let mut prompt = agent.identity.system_prompt.clone();
        for skill in skills {
            if !skill.prompt_addon.is_empty() {
                prompt.push_str("\n\n---\n");
                prompt.push_str(&skill.prompt_addon);
            }
        }

        let mut whitelist: HashSet<String> = agent
            .capability
            .tools_whitelist
            .iter()
            .cloned()
            .collect();
        for skill in skills { whitelist.extend(skill.tool_grants.iter().cloned()); }
        whitelist.insert("search_tools".into()); // 始终可用

        let mut mcp: HashSet<String> = agent
            .capability
            .mcp_bindings
            .iter()
            .map(|binding| binding.server_id.to_string())
            .collect();
        for skill in skills {
            mcp.extend(skill.mcp_grants.iter().map(ToString::to_string));
        }

        let mode = if whitelist.len() > 15 { ToolInjectionMode::Minimal } else { ToolInjectionMode::Full };

        Self {
            agent_id: agent.id.clone(),
            model_profile_id: agent.capability.model_profile_id.clone(),
            resolved_model: model_registry
                .resolve_profile(&agent.tenant_id, &agent.capability.model_profile_id)?,
            effective_system_prompt: prompt,
            effective_whitelist: whitelist.into_iter().collect(),
            effective_mcp_bindings: mcp.into_iter().collect(),
            tool_injection_mode: mode,
            registry: registry.clone(),
        }
    }
}
```

---

## 12. Agent 记忆系统

### 12.1 设计原则

记忆系统围绕 Agent 主体性设计：

- **归属**：每个 Agent 拥有独立的记忆库，`MemoryStore` 对外不可见
- **访问路径**：所有记忆的读写必须经由 `AgentService`，不允许跨 Agent 访问
- **写入触发**：任务完成后，由 `ExperienceHarvester` 监听 `TaskCompleted` 事件，异步驱动各参与 Agent 写入记忆；Execution 层不直接操作记忆
- **读取时机**：`AgentRunner` 在每次任务开始前，调用 `AgentService.recall()` 检索相关记忆，注入 LLM 上下文

### 12.2 MemoryStore Trait（Agent 模块内部）

```rust
// Octopus-hub/src/agent/memory/mod.rs
// 注意：此文件是 agent 模块的内部实现，不在 lib.rs 中 pub use

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub store_id: String,               // 对应 Agent.memory_store_id
    pub agent_id: AgentId,              // 冗余存储，便于查询
    pub content: String,
    pub source_task_id: Option<String>, // 来源任务（手动添加时为 None）
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RawExperience {
    pub content: String,
    pub source_task_id: String,
}

#[async_trait]
pub(crate) trait MemoryStore: Send + Sync {
    // pub(crate)：只在 agent 模块内可见
    async fn search(&self, store_id: &str, query: &str, top_k: usize) -> Result<Vec<MemoryEntry>>;
    async fn add(&self, entry: MemoryEntry) -> Result<()>;
    async fn delete(&self, store_id: &str, entry_id: &str) -> Result<()>;
    async fn list(&self, store_id: &str, offset: u64, limit: u64) -> Result<Vec<MemoryEntry>>;
}

pub(crate) fn create_memory_store(config: &HubConfig) -> Arc<dyn MemoryStore> {
    match config.vector_db {
        VectorDbType::LanceDb => Arc::new(LanceDbStore::new(&config.vector_db_path)),
        VectorDbType::Qdrant  => Arc::new(QdrantStore::new(&config.qdrant_url)),
    }
}
```

### 12.3 ExperienceHarvester：事件驱动的记忆写入

```rust
// Octopus-hub/src/agent/experience.rs

pub struct ExperienceHarvester {
    agent_service: Arc<AgentService>,
}

impl ExperienceHarvester {
    /// 监听 TaskCompleted 事件，为每个参与 Agent 提取并写入经验
    /// 异步执行，fire-and-forget，不阻塞任务完成流程
    pub async fn on_task_completed(&self, event: TaskCompletedEvent) -> Result<()> {
        let conversation = load_task_conversation(&event.task_id).await?;

        for agent_id in &event.participating_agents {
            let agent_messages = filter_messages_by_agent(&conversation, agent_id);
            let experiences = self.extract_experiences(&agent_messages, agent_id, &event.task_id).await;

            if !experiences.is_empty() {
                // 通过 AgentService 写入，保持访问路径一致
                if let Err(e) = self.agent_service.memorize(agent_id, experiences).await {
                    tracing::warn!("Failed to memorize for agent {}: {}", agent_id, e);
                    // 记忆写入失败不影响任务完成状态
                }
            }
        }
        Ok(())
    }

    /// 从对话中提取关键经验
    /// 使用 LLM 提取：重要结论、用户偏好、领域知识、历史决策依据
    async fn extract_experiences(
        &self,
        messages: &[LlmMessage],
        agent_id: &AgentId,
        task_id: &str,
    ) -> Vec<RawExperience> {
        // 调用 LLM 从对话中提取结构化经验
        // 提取失败时返回空 vec，不 panic
        extract_key_experiences_from_conversation(messages, agent_id, task_id)
            .await
            .unwrap_or_default()
    }
}
```

### 12.4 记忆隔离策略

**LanceDB（本地模式）**：
- 每个 Agent 独立 table，命名规则：`mem_{memory_store_id}`
- `memory_store_id` 在 Agent 创建时生成，与 `agent_id` 一一对应
- 所有查询强制传入 `store_id`，存储层不提供跨 store 查询接口

**Qdrant（远程模式）**：
- 每个 Agent 独立 collection，命名规则：`{tenant_id}_{memory_store_id}`
- tenant_id 前缀确保多租户隔离
- 所有查询强制传入 collection name，存储层不提供跨 collection 查询接口

---


## 13. Discussion Engine：圆桌会议与头脑风暴

### 13.1 设计原则

**为什么不复用 TaskEngine**：

| 维度 | TaskEngine | DiscussionEngine |
|-----|-----------|-----------------|
| 执行结构 | DAG，有依赖关系，状态机驱动 | 线性发言循环，调度器驱动 |
| Agent 入口 | AgentRunner（ReAct，有工具循环） | 直接调用 LlmClient（单次 complete） |
| 工具使用 | 核心能力 | 默认禁用，可选开启低风险工具 |
| 状态持久化 | 子任务粒度持久化，支持断点续跑 | 发言粒度持久化，暂停后可续会 |
| 结束条件 | 目标完成 | 用户主动 / 轮次上限 / 主持人宣布 |

**DiscussionEngine 不是 AgentRunner 的多实例**：Runner 内置 ReAct 循环（Thought → Action → Observation），讨论场景中 Agent 只需一次 LLM 调用生成发言，不存在 tool-use 循环。复用 Runner 会引入不必要的复杂度。

### 13.2 核心实体定义

```rust
// Octopus-hub/src/discussion/session.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionSession {
    pub id: String,
    pub tenant_id: String,
    pub topic: String,
    pub mode: DiscussionMode,
    pub status: DiscussionStatus,
    pub participant_ids: Vec<AgentId>,
    pub moderator_id: Option<AgentId>,
    pub turn_strategy: TurnStrategy,
    pub max_turns_per_agent: u32,   // 每个 Agent 最大发言轮数
    pub context_window: u32,        // 完整保留的最近 N 条发言
    pub current_round: u32,         // 当前已完成轮次（一轮 = 所有参与者各发言一次）
    pub conclusion: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub concluded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscussionMode {
    Roundtable,  // 圆桌：平等发言，寻求共识
    Brainstorm,  // 头脑风暴：发散思维，不批判
    Debate,      // 辩论：对立立场，充分论辩
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscussionStatus {
    Active,
    Paused,
    Concluded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnStrategy {
    Sequential,   // 顺序轮流（agent[0] → agent[1] → ... → agent[n] → agent[0] → ...）
    Moderated,    // 主持人指定（moderator_id 必须存在）
    // Reactive,  // 按相关性自主决定是否发言（可作为扩展策略）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionTurn {
    pub id: String,
    pub session_id: String,
    pub turn_number: u32,
    pub speaker_type: SpeakerType,
    pub speaker_id: Option<String>,   // agent_id or user_id
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpeakerType {
    Agent,
    User,
    System,  // 系统提示（如"讨论开始"、"进入下一轮"）
}
```

### 13.3 TurnScheduler

```rust
// Octopus-hub/src/discussion/scheduler.rs

pub trait TurnScheduler: Send + Sync {
    /// 返回下一个发言的 agent_id，None 表示讨论自然结束
    async fn next_speaker(
        &self,
        session: &DiscussionSession,
        history: &[DiscussionTurn],
    ) -> Result<Option<AgentId>>;
}

/// Sequential：按 participant_ids 顺序轮流，超出 max_turns_per_agent 后返回 None
pub struct SequentialScheduler;

impl TurnScheduler for SequentialScheduler {
    async fn next_speaker(
        &self,
        session: &DiscussionSession,
        history: &[DiscussionTurn],
    ) -> Result<Option<AgentId>> {
        // 统计每个 Agent 的发言次数
        let mut counts: HashMap<&AgentId, u32> = session.participant_ids.iter()
            .map(|id| (id, 0u32))
            .collect();
        for turn in history {
            if turn.speaker_type == SpeakerType::Agent {
                if let Some(sid) = &turn.speaker_id {
                    if let Some(c) = counts.get_mut(&AgentId::from(sid.as_str())) {
                        *c += 1;
                    }
                }
            }
        }

        // 检查是否所有 Agent 都达到上限
        if counts.values().all(|&c| c >= session.max_turns_per_agent) {
            return Ok(None);  // 自然结束
        }

        // 按轮次顺序找下一个未达上限的 Agent
        let agent_turns: u32 = history.iter()
            .filter(|t| t.speaker_type == SpeakerType::Agent)
            .count() as u32;
        let n = session.participant_ids.len() as u32;
        let idx = (agent_turns % n) as usize;
        let candidate = &session.participant_ids[idx];

        if *counts.get(candidate).unwrap_or(&0) >= session.max_turns_per_agent {
            // 当前候选已达上限，跳过（取下一个未达上限的）
            let next = session.participant_ids.iter()
                .find(|id| *counts.get(id).unwrap_or(&0) < session.max_turns_per_agent);
            Ok(next.cloned())
        } else {
            Ok(Some(candidate.clone()))
        }
    }
}

/// Moderated：由 moderator agent 通过 LLM 输出决定下一发言者
pub struct ModeratedScheduler {
    llm: Arc<dyn LlmClient>,
    agent_service: Arc<AgentService>,
}

impl TurnScheduler for ModeratedScheduler {
    async fn next_speaker(
        &self,
        session: &DiscussionSession,
        history: &[DiscussionTurn],
    ) -> Result<Option<AgentId>> {
        let moderator_id = session.moderator_id.as_ref()
            .ok_or(HubError::MissingModerator)?;
        let moderator = self.agent_service.get(moderator_id).await?;

        // 构建主持人决策 prompt
        let participants_desc = self.build_participants_desc(session).await?;
        let recent_history = summarize_recent_turns(history, 5);

        let prompt = format!(
            "你是会议主持人{}。\n\n参与者：\n{}\n\n最近讨论：\n{}\n\n\
             请决定下一个发言者（从参与者 ID 中选择一个），并简短说明选择原因。\
             如果讨论已充分、可以总结，返回 \"CONCLUDE\"。\
             仅返回 JSON：{{\"next\": \"agent_id_or_CONCLUDE\", \"reason\": \"...\"}}\
            ",
            moderator.name, participants_desc, recent_history
        );

        let response = self.llm.complete(
            &[LlmMessage::user(prompt)],
            None,
        ).await?;

        // 解析主持人决策
        #[derive(Deserialize)]
        struct ModeratorDecision { next: String, reason: String }
        let decision: ModeratorDecision = serde_json::from_str(&response.content)
            .map_err(|_| HubError::InvalidModeratorResponse)?;

        if decision.next == "CONCLUDE" {
            Ok(None)
        } else {
            let agent_id = AgentId::from(decision.next.as_str());
            // 校验该 agent_id 确实在参与者列表中
            if session.participant_ids.contains(&agent_id) {
                Ok(Some(agent_id))
            } else {
                // 主持人输出非法 ID，降级为顺序模式
                SequentialScheduler.next_speaker(session, history).await
            }
        }
    }
}
```

### 13.4 DiscussionContext：上下文构建与滚动摘要

```rust
// Octopus-hub/src/discussion/context.rs

pub struct DiscussionContext {
    llm: Arc<dyn LlmClient>,
}

impl DiscussionContext {
    /// 为指定 Agent 构建发言所需的完整上下文
    /// 包含：Agent 的身份 + 讨论模式引导 + 历史（滚动摘要 + 近 K 轮全文）
    pub async fn build_for_agent(
        &self,
        agent: &Agent,
        session: &DiscussionSession,
        all_turns: &[DiscussionTurn],
        memories: &[MemoryEntry],
        agent_name_map: &HashMap<AgentId, String>,  // id → name 用于历史显示
    ) -> Vec<LlmMessage> {
        let system_prompt = self.build_system_prompt(agent, session, memories);
        let history_messages = self.build_history_messages(all_turns, session.context_window, agent_name_map).await;

        let mut messages = vec![LlmMessage::system(system_prompt)];
        messages.extend(history_messages);
        messages
    }

    fn build_system_prompt(
        &self,
        agent: &Agent,
        session: &DiscussionSession,
        memories: &[MemoryEntry],
    ) -> String {
        let mode_instruction = match session.mode {
            DiscussionMode::Roundtable =>
                "本次是圆桌会议。请从你的专业角度发表观点，可以回应他人的观点，但必须提供独立的分析和论据。不要简单重复他人已说过的内容。",
            DiscussionMode::Brainstorm =>
                "本次是头脑风暴。请尽情发散思维，提出你独特的想法和可能性。不要批判他人的想法，而是在已有想法基础上继续发散或提出全新方向。",
            DiscussionMode::Debate =>
                "本次是辩论。请坚持自己的专业立场，用事实和逻辑论证观点，也要直接指出他人论述中的薄弱之处。保持专业，对事不对人。",
        };

        let memory_context = if memories.is_empty() {
            String::new()
        } else {
            let mem_lines = memories.iter()
                .map(|m| format!("- {}", m.content))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\n\n你过去积累的相关经验：\n{}", mem_lines)
        };

        format!(
            "{}\n\n{}\n\n议题：{}{}\n\n请用简洁、有观点的方式发言，通常 150-300 字为宜。",
            agent.identity.system_prompt,
            mode_instruction,
            session.topic,
            memory_context,
        )
    }

    /// 构建历史消息：近 context_window 条完整保留，更早的滚动摘要
    async fn build_history_messages(
        &self,
        all_turns: &[DiscussionTurn],
        context_window: u32,
        agent_name_map: &HashMap<AgentId, String>,
    ) -> Vec<LlmMessage> {
        let window = context_window as usize;

        if all_turns.len() <= window {
            // 全量保留，直接格式化
            return all_turns.iter().map(|t| self.format_turn(t, agent_name_map)).collect();
        }

        let (older, recent) = all_turns.split_at(all_turns.len() - window);

        // 对 older 部分生成摘要
        let older_text = older.iter()
            .map(|t| self.turn_to_text(t, agent_name_map))
            .collect::<Vec<_>>()
            .join("\n");

        let summary = self.summarize_older_turns(&older_text).await
            .unwrap_or_else(|_| "（早期讨论内容摘要不可用）".to_string());

        let mut messages = vec![
            LlmMessage::user(format!("【早期讨论摘要】\n{}", summary))
        ];
        messages.extend(recent.iter().map(|t| self.format_turn(t, agent_name_map)));
        messages
    }

    fn format_turn(&self, turn: &DiscussionTurn, names: &HashMap<AgentId, String>) -> LlmMessage {
        let label = match turn.speaker_type {
            SpeakerType::Agent => {
                let name = turn.speaker_id.as_ref()
                    .and_then(|id| names.get(&AgentId::from(id.as_str())))
                    .map(|s| s.as_str())
                    .unwrap_or("未知 Agent");
                format!("[{}]", name)
            }
            SpeakerType::User => "[用户]".to_string(),
            SpeakerType::System => "[系统]".to_string(),
        };
        LlmMessage::user(format!("{} {}", label, turn.content))
    }

    async fn summarize_older_turns(&self, text: &str) -> Result<String> {
        let prompt = format!(
            "以下是一段多人讨论的记录，请用 2-3 段话总结各方的核心观点和已形成的共识：\n\n{}",
            text
        );
        let resp = self.llm.complete(&[LlmMessage::user(prompt)], None).await?;
        Ok(resp.content)
    }
}
```

### 13.5 DiscussionEngine：主驱动循环

```rust
// Octopus-hub/src/discussion/engine.rs

pub struct DiscussionEngine {
    db: DbPool,
    agent_service: Arc<AgentService>,
    llm_factory: Arc<LlmClientFactory>,
    event_bus: Arc<EventBus>,
    context_builder: DiscussionContext,
    active_sessions: DashMap<String, CancellationToken>,
}

impl DiscussionEngine {
    /// 启动讨论驱动循环（在独立 tokio task 中运行）
    pub async fn run_session(
        &self,
        session_id: &str,
        cancel: CancellationToken,
    ) -> Result<()> {
        self.active_sessions.insert(session_id.to_string(), cancel.clone());

        loop {
            if cancel.is_cancelled() { break; }

            // 1. 加载当前会话状态
            let session = queries::get_discussion_session(&self.db, session_id).await?;
            if session.status != DiscussionStatus::Active { break; }

            let all_turns = queries::list_discussion_turns(&self.db, session_id).await?;

            // 2. 调度下一个发言者
            let scheduler = self.get_scheduler(&session);
            let next_agent_id = match scheduler.next_speaker(&session, &all_turns).await? {
                Some(id) => id,
                None => {
                    // 自然结束：触发结论生成
                    self.trigger_conclusion(&session, &all_turns).await?;
                    break;
                }
            };

            // 3. 通知 UI：某 Agent 开始发言
            self.event_bus.emit(DomainEvent::DiscussionTurnStarted {
                session_id: session_id.to_string(),
                agent_id: next_agent_id.clone(),
            });

            // 4. 构建该 Agent 的发言上下文
            let agent = self.agent_service.get(&next_agent_id).await?;
            let memories = self.agent_service
                .recall(&next_agent_id, &session.topic, 5)
                .await
                .unwrap_or_default();
            let agent_name_map = self.build_agent_name_map(&session).await?;

            let resolved_model = self.model_registry
                .resolve_profile(&agent.tenant_id, &agent.capability.model_profile_id)?;
            let llm = self.llm_factory.create(&resolved_model);
            let messages = self.context_builder.build_for_agent(
                &agent, &session, &all_turns, &memories, &agent_name_map
            ).await;

            // 5. 流式调用 LLM，实时推送 token
            let mut stream = llm.complete_stream(&messages, None).await?;
            let mut full_content = String::new();

            while let Some(chunk) = stream.next().await {
                if cancel.is_cancelled() { break; }
                match chunk? {
                    StreamChunk::Token(t) => {
                        self.event_bus.emit(DomainEvent::DiscussionTokenStream {
                            session_id: session_id.to_string(),
                            agent_id: next_agent_id.clone(),
                            token: t.clone(),
                        });
                        full_content.push_str(&t);
                    }
                    StreamChunk::Done { .. } => break,
                    _ => {}  // 讨论中不处理 ToolCall
                }
            }

            if cancel.is_cancelled() { break; }

            // 6. 持久化发言
            let turn_number = all_turns.len() as u32 + 1;
            let turn = DiscussionTurn {
                id: Uuid::new_v4().to_string(),
                session_id: session_id.to_string(),
                turn_number,
                speaker_type: SpeakerType::Agent,
                speaker_id: Some(next_agent_id.to_string()),
                content: full_content,
                created_at: Utc::now(),
            };
            queries::insert_discussion_turn(&self.db, &turn).await?;

            // 7. 通知 UI：发言完成
            self.event_bus.emit(DomainEvent::DiscussionTurnCompleted {
                session_id: session_id.to_string(),
                turn: turn.clone(),
            });
        }

        self.active_sessions.remove(session_id);
        Ok(())
    }

    /// 用户注入消息（插话）
    pub async fn inject_user_message(
        &self,
        session_id: &str,
        user_id: &str,
        content: String,
    ) -> Result<DiscussionTurn> {
        let all_turns = queries::list_discussion_turns(&self.db, session_id).await?;
        let turn = DiscussionTurn {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            turn_number: all_turns.len() as u32 + 1,
            speaker_type: SpeakerType::User,
            speaker_id: Some(user_id.to_string()),
            content,
            created_at: Utc::now(),
        };
        queries::insert_discussion_turn(&self.db, &turn).await?;

        // 用户插话后，如果讨论处于暂停等待状态，自动恢复
        self.event_bus.emit(DomainEvent::DiscussionUserInjected {
            session_id: session_id.to_string(),
            turn: turn.clone(),
        });
        Ok(turn)
    }

    /// 用户主动触发结论生成
    pub async fn request_conclusion(&self, session_id: &str) -> Result<()> {
        // 取消当前运行的发言循环
        if let Some((_, token)) = self.active_sessions.get(session_id).map(|e| e.clone()) {
            token.cancel();
        }
        let session = queries::get_discussion_session(&self.db, session_id).await?;
        let all_turns = queries::list_discussion_turns(&self.db, session_id).await?;
        self.trigger_conclusion(&session, &all_turns).await
    }

    async fn trigger_conclusion(
        &self,
        session: &DiscussionSession,
        all_turns: &[DiscussionTurn],
    ) -> Result<()> {
        let summarizer = ConclusionSummarizer::new(self.llm_factory.clone());
        let conclusion = summarizer.summarize(session, all_turns).await?;

        queries::set_discussion_concluded(&self.db, &session.id, &conclusion).await?;

        self.event_bus.emit(DomainEvent::DiscussionConcluded {
            session_id: session.id.clone(),
            conclusion: conclusion.clone(),
            participating_agents: session.participant_ids.clone(),
        });
        Ok(())
    }

    fn get_scheduler(&self, session: &DiscussionSession) -> Box<dyn TurnScheduler> {
        match session.turn_strategy {
            TurnStrategy::Sequential => Box::new(SequentialScheduler),
            TurnStrategy::Moderated => Box::new(ModeratedScheduler::new(
                self.llm_factory.clone(),
                self.agent_service.clone(),
            )),
        }
    }
}
```

### 13.6 ConclusionSummarizer：结论合成

```rust
// Octopus-hub/src/discussion/summarizer.rs

pub struct ConclusionSummarizer {
    llm_factory: Arc<LlmClientFactory>,
}

impl ConclusionSummarizer {
    pub async fn summarize(
        &self,
        session: &DiscussionSession,
        turns: &[DiscussionTurn],
    ) -> Result<String> {
        // 使用功能最强的可用模型生成结论（或使用 Hub 全局配置的摘要模型）
        let llm = self.llm_factory.create_default();

        let discussion_text = turns.iter()
            .map(|t| format!("[{}] {}", t.speaker_type_label(), t.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let mode_hint = match session.mode {
            DiscussionMode::Roundtable => "这是一场圆桌会议，请整合各方专业观点",
            DiscussionMode::Brainstorm => "这是一场头脑风暴，请归纳所有创意方向",
            DiscussionMode::Debate     => "这是一场辩论，请梳理各方立场、主要争议和可能的折中方案",
        };

        let prompt = format!(
            "以下是一场关于「{}」的多人讨论记录。{}\n\n\
             请生成一份结构化的讨论结论，包含：\n\
             1. 各参与者的核心观点摘要\n\
             2. 主要分歧与争议点\n\
             3. 形成的共识（如有）\n\
             4. 推荐的行动方向或下一步建议\n\n\
             讨论记录：\n{}",
            session.topic, mode_hint, discussion_text
        );

        let response = llm.complete(&[LlmMessage::user(prompt)], None).await?;
        Ok(response.content)
    }
}
```

### 13.7 ExperienceHarvester 扩展

```rust
// Octopus-hub/src/agent/experience.rs（扩展）

impl ExperienceHarvester {
    // 已有方法不变，新增：
    pub async fn on_discussion_concluded(&self, event: DiscussionConcludedEvent) -> Result<()> {
        let turns = load_discussion_turns(&event.session_id).await?;

        for agent_id in &event.participating_agents {
            // 筛选该 Agent 的发言 + 他人对其观点的回应（完整上下文）
            let relevant_turns = extract_relevant_turns(&turns, agent_id);

            let experiences = self.extract_discussion_experiences(
                &relevant_turns,
                agent_id,
                &event.session_id,
                &event.conclusion,
            ).await;

            if !experiences.is_empty() {
                if let Err(e) = self.agent_service.memorize(agent_id, experiences).await {
                    tracing::warn!("Failed to memorize discussion for agent {}: {}", agent_id, e);
                }
            }
        }
        Ok(())
    }

    async fn extract_discussion_experiences(
        &self,
        turns: &[DiscussionTurn],
        agent_id: &AgentId,
        session_id: &str,
        conclusion: &str,
    ) -> Vec<RawExperience> {
        // 从讨论记录中提取：
        // - 本次讨论形成的关键结论（对该 Agent 视角最相关的部分）
        // - 他人提出的有价值观点（可供未来参考）
        // - 本次讨论的议题背景（便于相似议题检索）
        extract_discussion_key_experiences(turns, agent_id, session_id, conclusion)
            .await
            .unwrap_or_default()
    }
}
```

### 13.8 DiscussionService：会话 CRUD

```rust
// Octopus-hub/src/discussion/session.rs

pub struct DiscussionService {
    db: DbPool,
    agent_service: Arc<AgentService>,
    engine: Arc<DiscussionEngine>,
}

impl DiscussionService {
    pub async fn create(&self, req: CreateDiscussionRequest, user_id: &str) -> Result<DiscussionSession> {
        // 校验：participant_ids 2-8 个，均存在且属于同一租户
        self.validate_participants(&req.participant_ids, &req.tenant_id).await?;

        let session = DiscussionSession {
            id: Uuid::new_v4().to_string(),
            tenant_id: req.tenant_id,
            topic: req.topic,
            mode: req.mode,
            status: DiscussionStatus::Active,
            participant_ids: req.participant_ids,
            moderator_id: req.moderator_id,
            turn_strategy: req.turn_strategy.unwrap_or(TurnStrategy::Sequential),
            max_turns_per_agent: req.max_turns_per_agent.unwrap_or(10),
            context_window: req.context_window.unwrap_or(10),
            current_round: 0,
            conclusion: None,
            created_by: user_id.to_string(),
            created_at: Utc::now(),
            concluded_at: None,
        };
        queries::insert_discussion_session(&self.db, &session).await?;

        // 自动启动（也可以由 start_discussion 命令手动触发）
        let cancel = CancellationToken::new();
        let engine = self.engine.clone();
        let session_id = session.id.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.run_session(&session_id, cancel).await {
                tracing::error!("Discussion session {} failed: {}", session_id, e);
            }
        });

        Ok(session)
    }

    pub async fn pause(&self, session_id: &str) -> Result<()> {
        self.engine.cancel_session(session_id);
        queries::set_discussion_status(&self.db, session_id, DiscussionStatus::Paused).await
    }

    pub async fn resume(&self, session_id: &str) -> Result<()> {
        queries::set_discussion_status(&self.db, session_id, DiscussionStatus::Active).await?;
        let cancel = CancellationToken::new();
        let engine = self.engine.clone();
        let sid = session_id.to_string();
        tokio::spawn(async move { let _ = engine.run_session(&sid, cancel).await; });
        Ok(())
    }

    pub async fn list(&self, tenant_id: &str) -> Result<Vec<DiscussionSession>> {
        queries::list_discussion_sessions(&self.db, tenant_id).await
    }

    pub async fn get_turns(&self, session_id: &str) -> Result<Vec<DiscussionTurn>> {
        queries::list_discussion_turns(&self.db, session_id).await
    }
}
```

---

## 14. 任务调度引擎与断点续跑

### 14.1 任务状态机

```
Task:    pending → planning → running → completed / failed / terminated
                                      → waiting_approval → running

Subtask: pending → running → completed / failed / cancelled / waiting_approval
```

所有状态变更通过统一函数，确保 DB 写入 + 事件发布同步：

```rust
async fn transition_task(&self, task_id: &str, new_status: TaskStatus) -> Result<()> {
    queries::update_task_status(&self.db, task_id, new_status).await?;
    self.event_bus.emit(DomainEvent::TaskStatusChanged {
        task_id: task_id.into(),
        status: new_status,
    });
    Ok(())
}
```

`TaskCompleted` 事件同时携带 `participating_agents`，供 `ExperienceHarvester` 使用：

```rust
self.event_bus.emit(DomainEvent::TaskCompleted {
    task_id: task_id.into(),
    result: final_result.clone(),
    participating_agents: subtasks.iter().map(|s| s.agent_id.clone()).collect(),
});
```

### 14.2 断点续跑

Hub 启动时扫描 `running` / `planning` 状态的任务并恢复执行。`DagExecutor` 恢复时跳过已完成的子任务，只重跑未完成部分。

```rust
impl TaskEngine {
    pub async fn recover_crashed_tasks(&self) -> Result<()> {
        let crashed = queries::find_tasks_by_status(
            &self.db, &[TaskStatus::Running, TaskStatus::Planning]
        ).await?;
        for task in crashed {
            tracing::warn!("Recovering crashed task: {}", task.id);
            self.resume_task(task).await?;
        }
        Ok(())
    }
}
```

### 14.3 任务终止

```rust
running_tasks: DashMap<String, CancellationToken>,

pub async fn terminate_task(&self, task_id: &str, user_id: &str) -> Result<()> {
    if let Some((_, token)) = self.running_tasks.remove(task_id) {
        token.cancel();   // 广播取消信号，已完成子任务结果不丢失
    }
    self.transition_task(task_id, TaskStatus::Terminated).await?;
    queries::set_terminated_by(&self.db, task_id, user_id).await?;
    Ok(())
}
```

---

## 15. 数据层

### 15.1 核心 Schema

```sql
-- agents 表
CREATE TABLE agents (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    -- Identity 维度
    name            TEXT NOT NULL,
    avatar          TEXT,
    role            TEXT,
    persona         TEXT,               -- JSON array of strings
    system_prompt   TEXT NOT NULL DEFAULT '',
    -- Capability 维度
    model_profile_id TEXT NOT NULL,     -- 引用 model_profiles.id
    tools_whitelist TEXT NOT NULL DEFAULT '[]',
    mcp_bindings    TEXT NOT NULL DEFAULT '[]',
    skill_ids       TEXT NOT NULL DEFAULT '[]',
    -- Memory 维度（存储引用，不存内容）
    memory_store_id TEXT NOT NULL,      -- 对应向量 DB 中的 store/collection ID
    -- 元数据
    status          TEXT NOT NULL DEFAULT 'idle',
    created_by      TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX idx_agents_tenant ON agents(tenant_id);

-- memory_entries 表（关系型索引，向量内容在向量 DB 中）
-- 用于支持 UI 列表展示、来源追溯、全文搜索等非向量操作
CREATE TABLE memory_entries (
    id              TEXT PRIMARY KEY,
    store_id        TEXT NOT NULL,       -- 对应 agents.memory_store_id
    agent_id        TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    tenant_id       TEXT NOT NULL,       -- 冗余，便于租户级查询
    content_summary TEXT NOT NULL,       -- 内容摘要（UI 展示用）
    source_task_id  TEXT,               -- 来源任务（手动添加时为 NULL）
    created_at      INTEGER NOT NULL
);
CREATE INDEX idx_memory_agent ON memory_entries(agent_id);
CREATE INDEX idx_memory_store  ON memory_entries(store_id);
-- 注意：向量检索在向量 DB 中进行，此表只做元数据管理

-- skills 表
CREATE TABLE skills (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT,               -- NULL 表示系统内置（跨租户可见）
    name            TEXT NOT NULL,
    description     TEXT NOT NULL,
    icon            TEXT,
    version         TEXT NOT NULL DEFAULT '1.0.0',
    source          TEXT NOT NULL,      -- builtin|user_defined|imported
    tags            TEXT NOT NULL DEFAULT '[]',
    prompt_addon    TEXT NOT NULL DEFAULT '',
    tool_grants     TEXT NOT NULL DEFAULT '[]',
    mcp_grants      TEXT NOT NULL DEFAULT '[]',
    workflow_hints  TEXT,
    author          TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX idx_skills_source ON skills(source);
CREATE INDEX idx_skills_tenant ON skills(tenant_id);

-- mcp_servers 表
CREATE TABLE mcp_servers (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    name            TEXT NOT NULL,
    transport       TEXT NOT NULL,      -- http|stdio
    endpoint        TEXT,
    command         TEXT,
    args            TEXT NOT NULL DEFAULT '[]',
    status          TEXT NOT NULL DEFAULT 'offline',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- tasks 表
CREATE TABLE tasks (
    id                     TEXT PRIMARY KEY,
    tenant_id              TEXT NOT NULL,
    team_id                TEXT REFERENCES teams(id),
    agent_id               TEXT REFERENCES agents(id),
    input                  TEXT NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'pending',
    mode                   TEXT NOT NULL,
    plan                   TEXT,
    pipeline_approval_mode TEXT NOT NULL DEFAULT 'auto_approve',
    result                 TEXT,
    created_by             TEXT NOT NULL,
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    completed_at           INTEGER,
    terminated_at          INTEGER,
    terminated_by          TEXT
);
CREATE INDEX idx_tasks_status ON tasks(status) WHERE status IN ('running', 'planning');
CREATE INDEX idx_tasks_tenant_status ON tasks(tenant_id, status);

-- subtasks、decisions、trace_entries 表结构不变，省略


```sql
-- discussion_sessions 表
CREATE TABLE discussion_sessions (
    id                  TEXT PRIMARY KEY,
    tenant_id           TEXT NOT NULL,
    topic               TEXT NOT NULL,
    mode                TEXT NOT NULL,          -- roundtable|brainstorm|debate
    status              TEXT NOT NULL DEFAULT 'active', -- active|paused|concluded
    participant_ids     TEXT NOT NULL,           -- JSON array of agent IDs
    moderator_id        TEXT,                    -- optional agent ID
    turn_strategy       TEXT NOT NULL DEFAULT 'sequential', -- sequential|moderated
    max_turns_per_agent INTEGER NOT NULL DEFAULT 10,
    context_window      INTEGER NOT NULL DEFAULT 10,
    current_round       INTEGER NOT NULL DEFAULT 0,
    conclusion          TEXT,                    -- 结论摘要（结束后填入）
    created_by          TEXT NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    concluded_at        INTEGER
);
CREATE INDEX idx_discussion_tenant ON discussion_sessions(tenant_id);
CREATE INDEX idx_discussion_status ON discussion_sessions(status) WHERE status = 'active';

-- discussion_turns 表
CREATE TABLE discussion_turns (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES discussion_sessions(id) ON DELETE CASCADE,
    turn_number     INTEGER NOT NULL,
    speaker_type    TEXT NOT NULL,          -- agent|user|system
    speaker_id      TEXT,                   -- agent_id or user_id（system 时为 NULL）
    content         TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);
CREATE INDEX idx_turns_session ON discussion_turns(session_id, turn_number);
-- 注意：discussion_turns 按 session_id 分组查询，不需要全局索引

```

**Schema 设计说明**：
- `memory_entries` 表作为向量内容的关系型索引，用于 UI 展示、来源追溯、全文搜索。向量检索在向量 DB 中进行，两者通过 `store_id` + `id` 关联
- `ON DELETE CASCADE` 确保 Agent 删除时，关联的记忆元数据也一并清理（向量 DB 侧需单独清理对应 store/collection）
- `participant_ids` 存 JSON 数组，不做关联表，因为参与者在会话创建后不变
- `discussion_turns` 的 `ON DELETE CASCADE` 确保会话删除时发言记录联级清理
- 讨论记忆通过已有 `memory_entries` 表记录（`source_task_id` 改为支持 `source_session_id`，或统一用 `source_ref` 字段）

---

## 16. Transport 抽象层

### 16.1 Command → REST 映射

| Command | HTTP | 说明 |
|---------|------|------|
| `create_agent` | POST /api/v1/agents | |
| `list_agents` | GET /api/v1/agents | |
| `update_agent` | PUT /api/v1/agents/:id | |
| `delete_agent` | DELETE /api/v1/agents/:id | |
| `list_agent_memories` | GET /api/v1/agents/:id/memories | 记忆归属于 Agent |
| `add_agent_memory` | POST /api/v1/agents/:id/memories | |
| `delete_agent_memory` | DELETE /api/v1/agents/:id/memories/:entry_id | |
| `submit_task` | POST /api/v1/tasks | |
| `terminate_task` | POST /api/v1/tasks/:id/terminate | |
| `resolve_decision` | POST /api/v1/tasks/:task_id/decisions/:id/resolve | |
| `list_skills` | GET /api/v1/skills | |
| `create_skill` | POST /api/v1/skills | |
| `list_tools` | GET /api/v1/tools | |
| `search_tools_api` | GET /api/v1/tools/search?q=... | |
| `register_mcp_server` | POST /api/v1/mcp/servers | |
| `create_discussion` | POST /api/v1/discussions | 创建并自动启动讨论会话 |
| `list_discussions` | GET /api/v1/discussions | 列表（支持 status 过滤） |
| `get_discussion` | GET /api/v1/discussions/:id | 获取会话详情 + 发言列表 |
| `inject_discussion_message` | POST /api/v1/discussions/:id/inject | 用户插话 |
| `conclude_discussion` | POST /api/v1/discussions/:id/conclude | 主动触发结论生成 |
| `pause_discussion` | POST /api/v1/discussions/:id/pause | 暂停 |
| `resume_discussion` | POST /api/v1/discussions/:id/resume | 续会 |
| `list_discussion_turns` | GET /api/v1/discussions/:id/turns | 获取发言记录 |
| `delete_discussion` | DELETE /api/v1/discussions/:id | 删除（仅已结束的会话） |
| `create_agent` | POST /api/v1/agents | （不变） |
| `list_agent_memories` | GET /api/v1/agents/:id/memories | （不变） |

**设计原则**：记忆相关路由挂载在 `/agents/:id/` 下，体现记忆归属于 Agent。系统中没有独立的 `/memory` 顶级路由。

### 16.2 SSE 事件类型


```typescript
type HubEvent =
  // ── Task 相关（不变）──────────────────────────────────────────
  | { type: 'task.status_changed';     data: { task_id: string; status: TaskStatus } }
  | { type: 'subtask.progress';        data: { task_id: string; subtask_id: string } }
  | { type: 'agent.token_stream';      data: { task_id: string; agent_id: string; token: string } }
  | { type: 'decision.pending';        data: { task_id: string; decisions: Decision[] } }
  | { type: 'decision.auto_resolved';  data: { task_id: string; decision_id: string; memory_ref: string } }
  | { type: 'task.completed';          data: { task_id: string; result: string } }
  | { type: 'task.failed';             data: { task_id: string; error: string } }
  | { type: 'task.terminated';         data: { task_id: string } }
  | { type: 'agent.memorized';         data: { agent_id: string; entry_count: number; source_id: string } }
  | { type: 'mcp.server_status';       data: { server_id: string; status: 'online' | 'offline' | 'error' } }

  // ── Discussion 相关 ─────────────────────────────────────
  | { type: 'discussion.turn_started';    data: { session_id: string; agent_id: string } }
  // Agent 开始思考/发言，UI 显示"正在发言..."

  | { type: 'discussion.token_stream';    data: { session_id: string; agent_id: string; token: string } }
  // 发言内容流式 token，实时渲染

  | { type: 'discussion.turn_completed';  data: { session_id: string; turn: DiscussionTurn } }
  // 单次发言完成，turn 包含完整内容，UI 可替换流式缓冲

  | { type: 'discussion.user_injected';   data: { session_id: string; turn: DiscussionTurn } }
  // 用户插话写入成功，广播给同一会话的其他 Client（多端场景）

  | { type: 'discussion.paused';          data: { session_id: string } }
  | { type: 'discussion.resumed';         data: { session_id: string } }

  | { type: 'discussion.concluded';       data: { session_id: string; conclusion: string } }
  // 讨论结束，附带结论全文

  | { type: 'discussion.memorized';       data: { session_id: string; agent_summaries: { agent_id: string; entry_count: number }[] } }
  // 各 Agent 记忆写入完成，UI 可在参与者列表上展示"已学习"标记
```

新增 `agent.memorized` 事件：当 Agent 完成记忆写入后，UI 侧可在 Agent 卡片上展示"刚刚学到了新经验"的成长提示。

---

## 17. 权限与安全实现

### 17.1 工具风险等级与审批规则

| 风险等级 | 工具示例 | 默认授权 | 执行时审批 |
|---------|---------|---------|---------|
| Low | `read_file` `list_dir` `grep_files` `search_tools` | 可默认勾选 | 否 |
| Medium | `write_file` `edit_file` `run_python` `web_search` `http_get` | 需显式勾选 | 否 |
| High | `run_bash` `http_post` `delete_file` | 需显式勾选 | **始终强制审批** |

高风险工具审批不受 `pipeline_approval_mode` 影响，`auto_approve` 模式下高风险工具依然触发审批。

**讨论模式补充**：
- 讨论中 Agent 默认不调用任何工具（`tools: None` 传给 LLM）
- 若用户开启"工具增强讨论"，仅允许 `RiskLevel::Low` 的工具（如 `web_search`、`read_file`）
- `run_bash`、`http_post` 等高风险工具在讨论模式下**硬编码禁用**，不受用户配置影响

### 17.2 记忆访问权限控制

- `AgentService` 的记忆方法在入口处校验 `agent_id` 的归属（确保 `agent.tenant_id` 与当前请求者一致）
- Hub API 层在 `/agents/:id/memories` 路由上校验调用者是否有权访问该 `agent_id`
- 向量 DB 层通过 `store_id` / collection 名称隔离，不提供跨 Agent 查询接口

### 17.3 代码执行沙箱

```rust
async fn run_code_subprocess(code: &str, lang: &str) -> Result<ToolResult> {
    let output = timeout(
        Duration::from_secs(30),
        Command::new(lang_interpreter(lang))
            .arg("-c").arg(code)
            .kill_on_drop(true)
            .output()
    ).await
    .map_err(|_| HubError::ExecutionTimeout)?
    .map_err(|e| HubError::ExecutionFailed(e.to_string()))?;
    Ok(ToolResult::from_output(output))
}
// 远程 Hub：强制 Docker 容器隔离
```

---

## 17. 部署模型实现

### 17.1 本地模式（Tauri 合并进程）

```
App 启动 → Octopus-tauri/main.rs
  → HubCore::new(local_config)
      → sqlx migrate
      → tool_registry.register_builtins()
      → mcp_gateway.restore_registered_servers()
      → skill_service.register_builtin_skills()
      → agent_service 初始化（内部初始化 LanceDB）
      → task_engine 初始化
      → event_bus 注册 ExperienceHarvester 监听器
      → task_engine.recover_crashed_tasks()
  → start_event_relay(app, hub)
  → Tauri WebView 启动 → Vue App
      → switchHub('local') → LocalTransport
```

### 17.2 远程 Hub（Docker Compose）

```yaml
services:
  hub:
    image: Octopus/hub-server:latest
    environment:
      DATABASE_URL: postgres://Octopus:${DB_PASSWORD}@postgres:5432/Octopus
      QDRANT_URL: http://qdrant:6333
      JWT_SECRET: ${JWT_SECRET}
    ports:
      - "8080:8080"
    depends_on:
      postgres: { condition: service_healthy }
      qdrant:   { condition: service_started }
    restart: unless-stopped

  postgres:
    image: postgres:16-alpine
    environment: { POSTGRES_USER: Octopus, POSTGRES_PASSWORD: "${DB_PASSWORD}", POSTGRES_DB: Octopus }
    volumes: [pgdata:/var/lib/postgresql/data]
    healthcheck: { test: ["CMD-SHELL", "pg_isready -U Octopus"], interval: 5s, retries: 5 }

  qdrant:
    image: qdrant/qdrant:latest
    volumes: [qdrant_data:/qdrant/storage]

volumes: { pgdata: {}, qdrant_data: {} }
```

---

## 19. 关键技术决策记录（ADR）

| # | 决策 | 结论 | 关键理由 |
|---|------|------|---------|
| ADR-01 | Desktop 框架 | **Tauri 2** ✅ | Rust 合并模式，包体积 ~30MB |
| ADR-02 | Hub 语言 | **Rust** ✅ | 全栈同语言，LLM 主要调 API，代价可控 |
| ADR-03 | 向量 DB | **LanceDB（本地）+ Qdrant（远程）** ✅ | MemoryStore trait 统一接口 |
| ADR-04 | 并发模型 | **tokio + JoinSet** ✅ | 当前不依赖外部队列，断点续跑靠 DB 状态机 |
| ADR-05 | 记忆写入 | **TaskCompleted / DiscussionConcluded 事件驱动** ✅ | 体现 Agent 主体性；异步不阻塞链路 |
| ADR-06 | 本地 Hub | **Tauri 合并进程** ✅ | 零 IPC 开销，App 与 Hub 同生命周期 |
| ADR-07 | 断点续跑 | **实时状态持久化 + 启动扫表恢复** ✅ | 无外部队列依赖，子任务粒度恢复 |
| ADR-08 | MCP 协议 | **自实现 JSON-RPC 2.0（reqwest）** ✅ | 不依赖任何 SDK，协议完全自控 |
| ADR-09 | 工具上下文 | **Minimal/Full 双模式 + search_tools** ✅ | 工具多时避免 context 爆炸 |
| ADR-10 | Skills | **可组合能力模块，运行时合并到 AgentRuntimeContext** ✅ | 与 Agent 模板正交，可多 Skill 叠加 |
| ADR-11 | 记忆归属 | **Memory 内聚于 AgentService，不对外暴露 MemoryStore** ✅ | API 语义清晰；防止跨 Agent 误操作 |
| ADR-12 | 讨论引擎独立 | **DiscussionEngine 不复用 TaskEngine** ✅ | 执行模型根本不同：讨论是线性发言循环，Task 是 DAG 状态机；强行复用会导致两者都复杂化 |
| ADR-13 | 发言调度策略 | **Sequential + Moderated（可选）** ✅ | Sequential 足够覆盖 90% 用例，实现简单、行为可预期；Moderated 满足需要主持人的正式讨论；Reactive（按相关性自主发言）保留为扩展策略 |
| ADR-14 | 讨论上下文裁剪 | **滚动摘要（older turns）+ 完整保留（近 K 轮）** ✅ | 纯截断会丢失关键早期共识；全量保留会导致长讨论 context 爆炸；滚动摘要是可控的折中，与 RAG 记忆注入正交 |
| ADR-15 | 讨论中工具使用 | **默认禁用，可选开启低风险工具** ✅ | 讨论是观点碰撞，不是任务执行；默认禁用工具保持讨论焦点，可选开启给需要引用实时数据的场景（如投资分析）提供灵活性 |
