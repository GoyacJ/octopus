# Novai · 数据模型文档（DATA_MODEL.md）

**版本**: v0.1.0 | **状态**: 正式版 | **日期**: 2026-03-10
**依赖文档**: PRD v0.1.0 · ARCHITECTURE v0.1.0 · DOMAIN v0.1.0

---

## 目录

1. [存储架构总览](#1-存储架构总览)
2. [ER 关系全景图](#2-er-关系全景图)
3. [关系型 Schema（按 Bounded Context）](#3-关系型-schema按-bounded-context)
    - [3.1 Identity & Access](#31-identity--access)
    - [3.2 Agent Context](#32-agent-context)
    - [3.3 Team Context](#33-team-context)
    - [3.4 Capability Context](#34-capability-context)
    - [3.5 Task Context](#35-task-context)
    - [3.6 Discussion Context](#36-discussion-context)
4. [向量存储 Schema（Agent Memory）](#4-向量存储-schemaagent-memory)
5. [SQLite vs PostgreSQL 差异映射](#5-sqlite-vs-postgresql-差异映射)
6. [索引策略](#6-索引策略)
7. [迁移规范（sqlx migrate）](#7-迁移规范sqlx-migrate)
8. [关键设计决策](#8-关键设计决策)
9. [表目录速查](#9-表目录速查)

---

## 1. 存储架构总览

### 1.1 双层存储模型

Novai Hub 使用两类存储，职责严格分离：

```
┌──────────────────────────────────────────────────────────────────────┐
│  关系型数据库（结构化业务数据）                                          │
│                                                                      │
│  本地 Hub（Desktop）         远程 Hub（Server/Docker）                 │
│  SQLite（嵌入式，零部署）     PostgreSQL 16                            │
│                                                                      │
│  存储：tenants · users · agents · teams · skills ·                   │
│        mcp_servers · tasks · subtasks · decisions ·                  │
│        task_log_entries · discussion_sessions ·                      │
│        discussion_participants · discussion_turns                    │
└──────────────────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────────────────┐
│  向量数据库（Agent 语义记忆）                                            │
│                                                                      │
│  本地 Hub                    远程 Hub                                 │
│  LanceDB（编译进二进制）       Qdrant（独立服务）                        │
│                                                                      │
│  存储：MemoryEntry（每 Agent 私有 Collection/Table）                    │
│  访问：仅通过 AgentService.recall() / memorize()                      │
│        对外不暴露 MemoryStore 接口（ADR-11）                           │
└──────────────────────────────────────────────────────────────────────┘
```

### 1.2 Client 侧存储（不在 Hub DB 中）

客户端本地存储仅保存连接配置，不存储业务数据：

| 存储位置 | 内容 | 实现 |
|---------|------|------|
| OS Keychain | Hub Access/Refresh Token | macOS Keychain / Windows Credential Store / Android Keystore |
| Tauri 本地配置 | Hub 列表（名称、URL、类型） | `tauri-plugin-store`，非敏感配置 |
| 内存缓存 | 当前活跃 Hub 数据快照 | Pinia Store，会话级，重启清空 |

### 1.3 多租户隔离策略

所有业务表均含 `tenant_id` 列，应用层（`AgentService` / `TaskEngine` / `DiscussionEngine`）在每个数据库访问入口强制过滤。Phase 1 不启用 PostgreSQL Row-Level Security（RLS），Phase 2 引入后作为深度防御层。

---

## 2. ER 关系全景图

```
tenants ──────────────────────────────────────────────────────────────┐
    │                                                                  │
    ├──< users                                                         │
    │       └──< user_roles                                            │
    │                                                                  │
    ├──< agents ──< agent_skills >── skills                            │
    │       │  └──< agent_mcp_bindings >── mcp_servers                │
    │       │                                                          │
    │       └── memory_store_id ──→ [向量DB: MemoryEntry集合]          │
    │                                                                  │
    ├──< teams ─────────── leader_id ──→ agents                       │
    │       ├──< team_members >── agents                               │
    │       └──< routing_rules                                         │
    │                                                                  │
    ├──< skills（builtin: tenant_id IS NULL）                           │
    │                                                                  │
    ├──< mcp_servers                                                   │
    │                                                                  │
    ├──< tasks ────────── mode_target_id ──→ agents | teams           │
    │       ├──< subtasks ─── agent_id ──→ agents                     │
    │       ├──< decisions ── source_agent_id ──→ agents              │
    │       └──< task_log_entries                                      │
    │                                                                  │
    └──< discussion_sessions ─── moderator_id ──→ agents              │
            ├──< discussion_participants >── agents                    │
            └──< discussion_turns                                      │
                                                                       │
（所有业务表均含 tenant_id FK ────────────────────────────────────────┘）
```

---

## 3. 关系型 Schema（按 Bounded Context）

> **约定**：DDL 以 PostgreSQL 为规范形式。SQLite 差异见 [第 5 节](#5-sqlite-vs-postgresql-差异映射)。
> ID 全部使用 `TEXT`（UUID v4 字符串），由应用层生成，不依赖 DB 自增。

---

### 3.1 Identity & Access

#### `tenants`

```sql
CREATE TABLE tenants (
    id          TEXT        PRIMARY KEY,
    name        TEXT        NOT NULL,
    -- JSON: { max_agents: int|null, max_teams: int|null, allowed_providers: string[] }
    settings    JSONB       NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

> **说明**：本地单用户 Hub 固定写入一个 `tenant_id`（启动时 seed），不对外开放多租户入口。远程 Hub 由 `hub_admin` 创建多租户。

#### `users`

```sql
CREATE TABLE users (
    id              TEXT        PRIMARY KEY,
    tenant_id       TEXT        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    username        TEXT        NOT NULL,
    email           TEXT,
    password_hash   TEXT,           -- NULL 表示 SSO 用户（Phase 2）
    status          TEXT        NOT NULL DEFAULT 'active'
                                    CHECK (status IN ('active', 'suspended', 'locked')),
    last_seen_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tenant_id, username)
);
```

#### `user_roles`

```sql
CREATE TABLE user_roles (
    user_id     TEXT        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id   TEXT        NOT NULL,   -- 反范式冗余，避免 JOIN，快速权限检查
    role        TEXT        NOT NULL,
    -- Phase 1 预置角色:
    --   'hub_admin', 'tenant_admin', 'member', 'viewer'
    --   'discussion.create', 'discussion.view', 'discussion.conclude'
    -- Phase 2 新增: 'team_manager'（需配合 resource_id 扩展）
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, role)
);
```

> **Phase 2 扩展点**：`team_manager` 角色需要绑定具体 `team_id` 作用域时，在此表增加 `resource_type TEXT` + `resource_id TEXT` 列，并放宽 PRIMARY KEY 为 `(user_id, role, resource_id)`。

---

### 3.2 Agent Context

#### `agents`

```sql
CREATE TABLE agents (
    id               TEXT        PRIMARY KEY,
    tenant_id        TEXT        NOT NULL REFERENCES tenants(id),
    created_by       TEXT        NOT NULL REFERENCES users(id),

    -- Identity 维度
    name             TEXT        NOT NULL,
    avatar_url       TEXT,
    role             TEXT        NOT NULL,                  -- 自然语言角色描述
    persona          JSONB       NOT NULL DEFAULT '[]',     -- string[]，最多 10 个标签
    system_prompt    TEXT        NOT NULL CHECK (LENGTH(system_prompt) > 0),
    prompt_version   INTEGER     NOT NULL DEFAULT 1,        -- 单调递增，Prompt 修改时 +1

    -- Capability 维度
    -- JSON: { provider, model, temperature, max_tokens, top_p, api_key_ref }
    model_config     JSONB       NOT NULL,
    tools_whitelist  JSONB       NOT NULL DEFAULT '[]',     -- string[]，显式授权的内置工具名

    -- Memory 维度（引用，实际内容在向量库）
    memory_store_id  TEXT        NOT NULL UNIQUE,           -- Agent 私有，生命周期内不可更改（I-02）

    -- Status
    status           TEXT        NOT NULL DEFAULT 'idle'
                                     CHECK (status IN ('idle', 'busy', 'error')),

    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- model_config 字段结构：
-- {
--   "provider":    "openai" | "anthropic" | "gemini" | "ollama" | "compatible",
--   "model":       "gpt-4o",
--   "temperature": 0.7,
--   "max_tokens":  4096,
--   "top_p":       null,
--   "api_key_ref": "OPENAI_KEY_1"   -- 引用标识符，非明文
-- }
```

> **不变量映射**：
> - `system_prompt` CHECK 保证非空（I-01）
> - `memory_store_id` UNIQUE + 应用层禁止 UpdateAgentRequest 携带此字段（I-02）
> - `persona` JSON 最大长度由应用层校验（≤ 10 个标签）

#### `agent_skills`

```sql
CREATE TABLE agent_skills (
    agent_id    TEXT    NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    skill_id    TEXT    NOT NULL REFERENCES skills(id),
    position    INTEGER NOT NULL DEFAULT 0,  -- 合并到 effective_prompt 时的叠加顺序

    PRIMARY KEY (agent_id, skill_id)
    -- 不变量：同一 Agent 的 skill_ids 不重复（PK 保证，I-SKILL-01）
);
```

> **为何用 junction table 而非 JSON 列**：`skill_ids` 需要反向查询（"哪些 Agent 附加了某 Skill"），且 Skill 是独立聚合根，引用完整性由 FK 保证。见 [8.1 节](#81-json-列-vs-规范化表)。

#### `agent_mcp_bindings`

```sql
CREATE TABLE agent_mcp_bindings (
    agent_id        TEXT    NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    mcp_server_id   TEXT    NOT NULL REFERENCES mcp_servers(id),
    -- NULL = 启用该服务所有工具；非 NULL = 仅启用指定工具列表
    enabled_tools   JSONB,  -- string[] | null

    PRIMARY KEY (agent_id, mcp_server_id)
);
```

---

### 3.3 Team Context

#### `teams`

```sql
CREATE TABLE teams (
    id              TEXT        PRIMARY KEY,
    tenant_id       TEXT        NOT NULL REFERENCES tenants(id),
    name            TEXT        NOT NULL,
    description     TEXT        NOT NULL DEFAULT '',
    leader_id       TEXT        NOT NULL REFERENCES agents(id),   -- 必填（I-11）
    leader_source   TEXT        NOT NULL
                                    CHECK (leader_source IN ('auto_generated', 'user_selected')),
    score           REAL,           -- Phase 2：基于历史任务的综合评分

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `team_members`

```sql
CREATE TABLE team_members (
    team_id     TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    agent_id    TEXT NOT NULL REFERENCES agents(id),

    PRIMARY KEY (team_id, agent_id)
    -- 应用层保证 agent_id ≠ leader_id（I-12）：
    -- Leader 单独通过 teams.leader_id 管理，不出现在此表
);
```

> **不变量 I-12 执行点**：`TeamService.add_member()` 入口检查 `agent_id != team.leader_id`，如相等则拒绝并静默忽略（保证语义清晰，不报错）。

#### `routing_rules`

```sql
CREATE TABLE routing_rules (
    id          TEXT        PRIMARY KEY,
    team_id     TEXT        NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    priority    INTEGER     NOT NULL,       -- 数字越小优先级越高；同 team 内唯一
    -- 条件（判别联合类型，存为 JSON）：
    -- {"type":"keyword_contains","keywords":["deploy","urgent"]}
    -- {"type":"label_equals","label":"finance"}
    -- {"type":"tool_required","tool_name":"run_bash"}
    -- {"type":"always"}
    condition   JSONB       NOT NULL,
    -- 动作（判别联合类型，存为 JSON）：
    -- {"type":"assign_to","agent_id":"..."}
    -- {"type":"skip_plan_approval"}
    -- {"type":"force_approval"}
    -- {"type":"notify","message":"..."}
    action      JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

### 3.4 Capability Context

#### `skills`

```sql
CREATE TABLE skills (
    id              TEXT        PRIMARY KEY,
    -- builtin skill: tenant_id IS NULL（跨租户共享）
    tenant_id       TEXT        REFERENCES tenants(id),
    name            TEXT        NOT NULL,
    description     TEXT        NOT NULL DEFAULT '',
    source          TEXT        NOT NULL DEFAULT 'user_defined'
                                    CHECK (source IN ('builtin', 'user_defined', 'imported')),
    prompt_addon    TEXT        NOT NULL DEFAULT '',   -- 追加到 Agent system_prompt 末尾
    tool_grants     JSONB       NOT NULL DEFAULT '[]', -- string[]，此 Skill 授权的工具名
    mcp_grants      JSONB       NOT NULL DEFAULT '[]', -- McpServerId[]，此 Skill 自动绑定的 MCP 服务

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
    -- 不变量 I-14：source='builtin' 时应用层拒绝 UPDATE/DELETE
);
```

> **内置 Skill 列表（Phase 1 seed）**：`coding`、`research`、`data_analysis`、`content_writing`、`system_ops`。

#### `mcp_servers`

```sql
CREATE TABLE mcp_servers (
    id              TEXT        PRIMARY KEY,
    tenant_id       TEXT        NOT NULL REFERENCES tenants(id),
    name            TEXT        NOT NULL,
    -- 传输协议（判别联合类型）：
    -- {"type":"http","url":"https://mcp.example.com"}
    -- {"type":"stdio","command":"npx","args":["-y","@mcp/server-name"]}
    transport       JSONB       NOT NULL,
    status          TEXT        NOT NULL DEFAULT 'offline'
                                    CHECK (status IN ('online', 'offline', 'error')),
    status_reason   TEXT,           -- 错误原因，status='error' 时填写
    -- 从服务器发现的工具清单（Hub 启动或重连后更新）
    -- JSON 结构见 ToolSpec 定义
    tool_manifest   JSONB,
    last_seen_at    TIMESTAMPTZ,
    registered_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

### 3.5 Task Context

#### `tasks`

```sql
CREATE TABLE tasks (
    id                      TEXT        PRIMARY KEY,
    tenant_id               TEXT        NOT NULL REFERENCES tenants(id),
    created_by              TEXT        NOT NULL REFERENCES users(id),

    input                   TEXT        NOT NULL,    -- 用户原始自然语言任务描述
    mode                    TEXT        NOT NULL
                                            CHECK (mode IN ('single_agent', 'team')),
    mode_target_id          TEXT        NOT NULL,    -- agent_id（单 Agent）或 team_id（团队）
    -- 注意：FK 无法同时约束两个表，由应用层校验 mode_target_id 存在于对应表

    status                  TEXT        NOT NULL DEFAULT 'pending'
                                            CHECK (status IN (
                                                'pending', 'planning', 'waiting_plan_approval',
                                                'running', 'waiting_approval',
                                                'completed', 'failed', 'terminated'
                                            )),
    pipeline_approval_mode  TEXT        NOT NULL DEFAULT 'per_stage'
                                            CHECK (pipeline_approval_mode IN (
                                                'per_stage', 'auto_approve', 'custom'
                                            )),
    result                  TEXT,           -- 最终汇总结果，completed 后写入

    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ,
    terminated_at           TIMESTAMPTZ,
    terminated_by           TEXT        REFERENCES users(id)
);
```

#### `subtasks`

```sql
CREATE TABLE subtasks (
    id                  TEXT        PRIMARY KEY,
    task_id             TEXT        NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    agent_id            TEXT        NOT NULL REFERENCES agents(id),

    description         TEXT        NOT NULL,   -- Leader 发给该 Agent 的执行描述
    -- DAG 依赖关系，SubtaskId 数组：["subtask-id-1", "subtask-id-2"]
    -- 不变量 I-05：不能成环，由 LeaderPlanningService 在创建时拓扑排序校验
    depends_on          JSONB       NOT NULL DEFAULT '[]',

    status              TEXT        NOT NULL DEFAULT 'pending'
                                        CHECK (status IN (
                                            'pending', 'running', 'waiting_approval',
                                            'completed', 'failed', 'skipped'
                                        )),
    result              TEXT,
    approval_required   BOOLEAN     NOT NULL DEFAULT FALSE,
    position            INTEGER     NOT NULL DEFAULT 0,  -- 用于 DAG 可视化和顺序恢复

    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

> **单 Agent 模式约束**：`mode='single_agent'` 时 `tasks` 对应且仅对应一条 `subtask`，`depends_on=[]`。由 `TaskEngine` 创建时保证。

#### `decisions`

```sql
CREATE TABLE decisions (
    id               TEXT        PRIMARY KEY,
    task_id          TEXT        NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    source_agent_id  TEXT        NOT NULL REFERENCES agents(id),

    decision_type    TEXT        NOT NULL
                                     CHECK (decision_type IN (
                                         'deliverable_review', 'clarification', 'risk_alert'
                                     )),
    content          TEXT        NOT NULL,   -- 问题描述或成果物摘要
    artifact_ref     TEXT,                  -- 关联成果物的引用（文档路径、代码片段等）

    status           TEXT        NOT NULL DEFAULT 'pending'
                                     CHECK (status IN (
                                         'pending', 'auto_resolved',
                                         'user_confirmed', 'user_rejected'
                                     )),
    resolution       TEXT,                  -- 最终决策内容（用户输入或 Leader 自动处理结果）
    memory_ref       TEXT,                  -- MemoryEntryId（向量库引用，Leader 自动处理时填写）

    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at      TIMESTAMPTZ
);
```

#### `task_log_entries`（追加只写，Trace Log）

```sql
CREATE TABLE task_log_entries (
    id          TEXT        PRIMARY KEY,
    task_id     TEXT        NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    subtask_id  TEXT,       -- 可为 NULL（任务级别日志）
    agent_id    TEXT,       -- 可为 NULL（系统级日志）

    entry_type  TEXT        NOT NULL
                                CHECK (entry_type IN (
                                    'llm_request', 'llm_response',
                                    'tool_call', 'tool_result',
                                    'decision_point', 'status_change'
                                )),
    content     TEXT        NOT NULL,   -- 已脱敏（不含 API Key 等敏感信息）
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW()

    -- ⚠️ 仅 INSERT，无 UPDATE。Repository 接口不提供 update_log 方法。
);
```

---

### 3.6 Discussion Context

#### `discussion_sessions`

```sql
CREATE TABLE discussion_sessions (
    id              TEXT        PRIMARY KEY,
    tenant_id       TEXT        NOT NULL REFERENCES tenants(id),
    created_by      TEXT        NOT NULL REFERENCES users(id),

    topic           TEXT        NOT NULL,
    mode            TEXT        NOT NULL
                                    CHECK (mode IN ('roundtable', 'brainstorm', 'debate')),
    status          TEXT        NOT NULL DEFAULT 'active'
                                    CHECK (status IN ('active', 'paused', 'concluded')),
    turn_strategy   TEXT        NOT NULL DEFAULT 'sequential'
                                    CHECK (turn_strategy IN ('sequential', 'moderated')),
    moderator_id    TEXT        REFERENCES agents(id),
    -- 不变量 I-10：turn_strategy='moderated' 时 moderator_id 不能为 NULL（应用层校验）

    -- config JSON 结构：
    -- {
    --   "max_turns_per_agent": 10,
    --   "context_window_size": 10,
    --   "tool_augmented": false,
    --   "debate_positions": {           -- 仅 mode='debate' 时存在
    --     "agent-id-1": "支持方",
    --     "agent-id-2": "反对方"
    --   }
    -- }
    config          JSONB       NOT NULL DEFAULT '{}',

    conclusion      TEXT,           -- 结束时合成的结论（用户可编辑后重新触发记忆写入，PRD #12）
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    concluded_at    TIMESTAMPTZ
);
```

> **不变量执行点**：
> - I-08：参与者数量 [2, 8]，在 `DiscussionService.create()` 入口校验
> - I-09：`Debate` 模式下 `debate_positions` 必须覆盖所有参与者，未指定时系统自动分配对立立场
> - I-10：`Moderated` 策略时 `moderator_id NOT NULL`

#### `discussion_participants`

```sql
CREATE TABLE discussion_participants (
    session_id       TEXT    NOT NULL REFERENCES discussion_sessions(id) ON DELETE CASCADE,
    agent_id         TEXT    NOT NULL REFERENCES agents(id),
    position         INTEGER NOT NULL,  -- 发言顺序（0 起），Sequential 策略按此轮转
    -- 辩论立场（mode='debate' 时使用，与 config.debate_positions 同步）
    -- 此处冗余存储便于查询，以 config.debate_positions 为权威源
    debate_position  TEXT,

    PRIMARY KEY (session_id, agent_id)
);
```

> **moderator 处理**：`Moderated` 策略下，`moderator_id` 若同时是参与者，则在此表也有一行。若是"专职主持人"（不参与讨论只调度），则不在此表中，仅通过 `discussion_sessions.moderator_id` 引用。`DiscussionEngine` 在调度时区分两种情况。

#### `discussion_turns`（追加只写）

```sql
CREATE TABLE discussion_turns (
    id           TEXT        PRIMARY KEY,
    session_id   TEXT        NOT NULL REFERENCES discussion_sessions(id) ON DELETE CASCADE,
    turn_number  INTEGER     NOT NULL,   -- 全局轮次，从 1 开始，单调递增

    speaker_type TEXT        NOT NULL
                                 CHECK (speaker_type IN ('agent', 'user', 'system')),
    speaker_id   TEXT,       -- agent_id 或 user_id；speaker_type='system' 时为 NULL
    content      TEXT        NOT NULL,   -- 完整发言内容（流式结束后持久化）

    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()

    -- ⚠️ 仅 INSERT，无 UPDATE。不变量 I-17：DiscussionTurn 写入后不可修改。
    -- Repository 接口不提供 update_turn 方法。
);
```

---

## 4. 向量存储 Schema（Agent Memory）

向量存储不属于关系型 DB，通过 `MemoryStore` trait 抽象，有两种实现。

### 4.1 Collection / Table 命名规则

每个 Agent 拥有独立的向量集合（Collection），以 `memory_store_id` 命名：

| 实现 | 集合单位 | 命名规则 |
|-----|---------|---------|
| LanceDB（本地） | Table | `mem_{memory_store_id}` |
| Qdrant（远程） | Collection | `mem_{memory_store_id}` |

一个 `memory_store_id` 对应且仅对应一个 Agent（`agents.memory_store_id` UNIQUE）。删除 Agent 时同步删除对应集合。

### 4.2 MemoryEntry 字段结构

```
Field           Type            Description
─────────────────────────────────────────────────────────────────────
id              string          UUID v4，MemoryEntryId
agent_id        string          AgentId（冗余存储，便于跨集合调试查询）
content         string          记忆文本内容（语义检索的原始文本）
vector          float[N]        嵌入向量（维度取决于 Embedding 模型）
                                text-embedding-3-small → 1536 维
                                nomic-embed-text（Ollama）→ 768 维
source_type     string          "task" | "discussion" | "manual"
source_id       string          TaskId 或 DiscussionSessionId 或 "manual"
created_at      string          ISO 8601 时间戳
metadata        JSON object     扩展字段，如 {"topic": "deployment", "lang": "zh"}
```

### 4.3 检索与写入接口

```
MemoryStore.search(store_id, query, top_k)
    → 语义相似度检索，返回 top_k 条最相关 MemoryEntry

MemoryStore.insert(store_id, NewMemoryEntry)
    → 先调用 Embedding API 生成向量，再写入

MemoryStore.list(store_id)
    → 按 created_at DESC 列出所有条目（UI 管理用）

MemoryStore.delete(store_id, entry_id)
    → 按 ID 精确删除（用户手动管理记忆 UI）

MemoryStore.create_store(store_id)
    → Agent 创建时同步初始化集合

MemoryStore.delete_store(store_id)
    → Agent 删除时同步清除集合（级联保证数据一致性）
```

### 4.4 Embedding 模型配置策略

Embedding 模型与 Agent 的推理模型解耦，由 Hub 级别统一配置（不是每个 Agent 单独配）：

- 本地 Hub 默认：Ollama `nomic-embed-text`（无需网络）
- 远程 Hub 默认：OpenAI `text-embedding-3-small`（性价比最优）
- 可通过 Hub 配置文件覆盖（`HUB_EMBEDDING_PROVIDER` / `HUB_EMBEDDING_MODEL` 环境变量）

> **注意**：同一 Agent 的所有 MemoryEntry 必须使用相同维度的 Embedding，切换模型需重建该 Agent 的向量集合。

---

## 5. SQLite vs PostgreSQL 差异映射

本地 Hub（SQLite）和远程 Hub（PostgreSQL）使用独立的 migration 文件目录，Schema 逻辑等价但语法有差异：

| 概念 | PostgreSQL | SQLite | 备注 |
|-----|-----------|--------|------|
| 时间戳类型 | `TIMESTAMPTZ` | `TEXT`（ISO 8601：`2026-03-10T12:00:00Z`） | sqlx `DateTime<Utc>` 两边都能正确映射 |
| 布尔类型 | `BOOLEAN` | `INTEGER`（0/1） | sqlx `bool` 自动转换 |
| JSON 类型 | `JSONB`（二进制，支持操作符查询） | `TEXT`（无原生 JSON 索引） | Phase 1 SQLite 不依赖 JSON 操作符，无影响 |
| CHECK 约束 | 完整支持 | 支持，但不验证 FK（需 `PRAGMA foreign_keys = ON`） | migration 脚本开头加 PRAGMA |
| UNIQUE 约束 | 完整支持 | 完整支持 | 无差异 |
| ON DELETE CASCADE | 完整支持 | 需 `PRAGMA foreign_keys = ON` | 同上 |
| 默认值函数 | `DEFAULT NOW()` | `DEFAULT (datetime('now'))` | Migration 脚本各自处理 |
| 部分索引 | `CREATE INDEX ... WHERE ...` | 不支持 | 仅在 PostgreSQL migration 中创建 |

**SQLite migration 文件开头模板**：

```sql
PRAGMA journal_mode = WAL;     -- 写性能优化
PRAGMA foreign_keys = ON;      -- 启用外键约束
PRAGMA synchronous = NORMAL;   -- WAL 模式下安全
```

---

## 6. 索引策略

### 6.1 必须创建的索引（性能关键路径）

```sql
-- ── tenants / users ──────────────────────────────────────────────────
-- username 登录检索（已有 UNIQUE 约束，覆盖）

-- ── agents ───────────────────────────────────────────────────────────
CREATE INDEX idx_agents_tenant     ON agents(tenant_id);
CREATE INDEX idx_agents_created_by ON agents(created_by);
CREATE INDEX idx_agents_status     ON agents(status);
-- 用途：AgentService.list(tenant_id)，LeaderPlanningService 检查 Agent 空闲状态

-- ── teams ────────────────────────────────────────────────────────────
CREATE INDEX idx_teams_tenant      ON teams(tenant_id);
CREATE INDEX idx_teams_leader      ON teams(leader_id);

-- ── tasks ────────────────────────────────────────────────────────────
CREATE INDEX idx_tasks_tenant      ON tasks(tenant_id);
CREATE INDEX idx_tasks_created_by  ON tasks(created_by);
CREATE INDEX idx_tasks_status      ON tasks(status);
-- 断点续跑关键索引：Hub 启动时 TaskRecoveryService 扫描 status IN ('running','planning')

-- ── subtasks ─────────────────────────────────────────────────────────
CREATE INDEX idx_subtasks_task     ON subtasks(task_id);
CREATE INDEX idx_subtasks_agent    ON subtasks(agent_id);
CREATE INDEX idx_subtasks_status   ON subtasks(status);

-- ── decisions ────────────────────────────────────────────────────────
CREATE INDEX idx_decisions_task    ON decisions(task_id);
CREATE INDEX idx_decisions_status  ON decisions(status);
-- 用途：任务看板拉取待处理决策队列

-- ── task_log_entries ─────────────────────────────────────────────────
CREATE INDEX idx_logs_task         ON task_log_entries(task_id);
CREATE INDEX idx_logs_timestamp    ON task_log_entries(task_id, timestamp);
-- 用途：Trace 视图按时间序拉取日志；断点续跑状态恢复

-- ── discussion_sessions ──────────────────────────────────────────────
CREATE INDEX idx_discussion_tenant ON discussion_sessions(tenant_id);
CREATE INDEX idx_discussion_status ON discussion_sessions(status);
-- 用途：讨论列表、活跃会话检测

-- ── discussion_participants ──────────────────────────────────────────
CREATE INDEX idx_dp_session        ON discussion_participants(session_id);
CREATE INDEX idx_dp_agent          ON discussion_participants(agent_id);
-- 用途：某 Agent 参与的所有讨论（记忆关联）

-- ── discussion_turns ─────────────────────────────────────────────────
CREATE INDEX idx_turns_session          ON discussion_turns(session_id);
CREATE INDEX idx_turns_session_number   ON discussion_turns(session_id, turn_number);
-- 用途：按顺序拉取发言历史（DiscussionContextBuilder 构建上下文）
```

### 6.2 PostgreSQL 专属索引（本地 SQLite 不适用）

```sql
-- JSONB 字段内部索引（仅在访问频率高时添加）
-- tasks.mode_target_id 如需跨 mode 查询某 Agent 的所有任务：
CREATE INDEX idx_tasks_target_agent
    ON tasks(mode_target_id)
    WHERE mode = 'single_agent';

CREATE INDEX idx_tasks_target_team
    ON tasks(mode_target_id)
    WHERE mode = 'team';
```

### 6.3 索引原则

- `task_log_entries` 和 `discussion_turns` 是高写入追加表，**不加过多索引**，仅保留按 `task_id` / `session_id` 批量拉取的复合索引
- `agents.status` 索引支持断点续跑时批量查找 `status='busy'` 的 Agent
- 不在 JSON 列内部创建 GIN 索引（Phase 1 规模不需要；Phase 2 按需评估）

---

## 7. 迁移规范（sqlx migrate）

### 7.1 目录结构

```
crates/novai-hub/src/db/
├── migrations/
│   ├── sqlite/
│   │   ├── 0001_init_iam.sql
│   │   ├── 0002_init_agents.sql
│   │   ├── 0003_init_capability.sql
│   │   ├── 0004_init_teams.sql
│   │   ├── 0005_init_tasks.sql
│   │   └── 0006_init_discussions.sql
│   └── postgres/
│       ├── 0001_init_iam.sql
│       ├── 0002_init_agents.sql
│       ├── 0003_init_capability.sql
│       ├── 0004_init_teams.sql
│       ├── 0005_init_tasks.sql
│       └── 0006_init_discussions.sql
└── mod.rs
```

### 7.2 运行时选择 Migration 目录

```rust
// db/mod.rs
pub async fn run_migrations(pool: &AnyPool, db_type: DbType) -> Result<()> {
    let migrator = match db_type {
        DbType::Sqlite   => sqlx::migrate!("./src/db/migrations/sqlite"),
        DbType::Postgres => sqlx::migrate!("./src/db/migrations/postgres"),
    };
    migrator.run(pool).await?;
    Ok(())
}
```

### 7.3 Migration 编写规范

- **命名**：`{序号}_{描述}.sql`，序号四位零填充，描述使用下划线，无空格
- **幂等性**：使用 `CREATE TABLE IF NOT EXISTS`，`CREATE INDEX IF NOT EXISTS`
- **原子性**：单文件内所有 DDL 包裹在一个事务中（sqlx migrate 默认开启）
- **只有向前迁移**：不提供 `.down.sql`（简化运维复杂度，回滚通过新 migration 修正）
- **Seed 数据**：内置 Skills（5 个）和默认租户在 Hub 启动逻辑中检查后插入，不写入 migration 文件（避免与生产数据混淆）

### 7.4 启动时自动迁移

```rust
// novai-tauri/main.rs 和 novai-server/main.rs
HubCore::new(config).await?
// 内部 → db::run_migrations(&pool, config.db_type).await?
// sqlx migrate 检测版本表 _sqlx_migrations，增量执行未运行的迁移
```

---

## 8. 关键设计决策

### 8.1 JSON 列 vs 规范化表

| 字段 | 选择 | 理由 |
|-----|------|------|
| `agents.model_config` | **JSON 列** | 复杂嵌套对象（7 个字段），始终作为整体读写，不单独查询内部字段 |
| `agents.persona` | **JSON 列** | 简单字符串数组，无需单独查询"某个性格标签被哪些 Agent 使用" |
| `agents.tools_whitelist` | **JSON 列** | 字符串列表，校验在应用层（ToolRegistry.get_for_agent），无 FK 需求 |
| `agents.skill_ids` | **Junction Table（agent_skills）** | 需要 FK 完整性保证，需反向查询（哪些 Agent 用了某 Skill），有排序需求 |
| `agents.mcp_bindings` | **Junction Table（agent_mcp_bindings）** | 需要 FK 完整性，需额外属性（enabled_tools），需反向查询 |
| `tasks.mode_target_id` | **单列 + mode 判别** | 避免可空的 `agent_id` / `team_id` 双列设计；FK 由应用层在 TaskEngine 入口校验 |
| `subtasks.depends_on` | **JSON 列** | DAG 依赖始终作为整体序列化读写，拓扑排序校验在内存中（LeaderPlanningService）完成 |
| `routing_rules.condition/action` | **JSON 列** | 判别联合类型（Rust enum），规范化需要多表且无法穷举未来扩展 |
| `discussion_sessions.config` | **JSON 列** | 包含可选的 `debate_positions` Map，结构随 mode 不同变化，不值得规范化 |
| `discussion_participants.debate_position` | **关系列** | 辩论立场需要按 agent 快速查询，与 session_id+agent_id 的 PK 共存 |

### 8.2 追加只写表设计

`task_log_entries` 和 `discussion_turns` 是系统中两张追加只写表：

- **无 UPDATE 接口**：Repository trait 不提供 `update_log` / `update_turn` 方法，编译期防止误操作
- **删除策略**：仅随父实体（Task / DiscussionSession）级联删除，不支持单条删除
- **索引策略**：只需要按 `task_id` / `session_id` + 时间顺序检索，索引尽量少（写放大）
- **不变量支持**：直接对应领域不变量 I-17（DiscussionTurn 写入后不可修改）

### 8.3 `tasks.mode_target_id` 无 FK 约束处理

`mode_target_id` 指向 `agents` 或 `teams` 中的一条记录，取决于 `mode` 值，无法在 DB 层建立单一 FK。

**执行策略**：
1. `TaskEngine.create_task()` 入口根据 `mode` 查询对应表验证目标存在
2. `mode_target_id` 对应表记录删除时（Agent / Team 删除），`TaskService` 在删除前检查是否有 `running` 状态的 Task 引用，有则拒绝删除
3. 已完成 / 终止的 Task 不受约束（历史记录保留）

### 8.4 `memory_store_id` 不可变性保证

- DB 层：`agents.memory_store_id` 有 UNIQUE 约束
- 应用层：`UpdateAgentRequest` 结构体中不包含 `memory_store_id` 字段（编译期保证）
- 不变量 I-02 在 API 层和领域层双重保证

### 8.5 租户隔离的防御深度

| 层次 | 机制 |
|-----|------|
| 应用层（主要防线） | 所有 Service 方法接收 `tenant_id` 参数并作为 WHERE 条件 |
| Repository 层 | `find_by_tenant()` 系列方法封装 tenant 过滤，不提供无 tenant 过滤的批量查询 |
| DB 层（Phase 2） | PostgreSQL Row-Level Security（多租户 SaaS 场景） |
| 向量 DB 层 | 每 Agent 独立 Collection，store_id 名称隔离 |

### 8.6 讨论历史的存储与检索性能（PRD 开放问题 #13）

**问题**：长讨论（100+ 轮次）的历史存储和 Context 构建性能。

**方案**：
- `discussion_turns` 使用复合索引 `(session_id, turn_number)`，按轮次范围高效分页
- `DiscussionContextBuilder` 只拉取近 `context_window_size`（默认 10）轮完整记录
- 更早的历史由 `ConclusionSummarizer` 的中间摘要（存于内存/临时缓存）替代，不重复查询全量
- Phase 2 评估是否对 `discussion_turns.content` 建立全文搜索索引（PostgreSQL `tsvector`）

---

## 9. 表目录速查

| 表名 | Bounded Context | 类型 | 行数量级（Phase 1） | 关键 FK |
|-----|----------------|------|-----------------|---------|
| `tenants` | IAM | 配置 | < 10 | — |
| `users` | IAM | 实体 | < 1K | tenant_id |
| `user_roles` | IAM | 关联 | < 10K | user_id |
| `agents` | Agent | 聚合根 | < 10K | tenant_id, created_by |
| `agent_skills` | Agent/Capability | Junction | < 50K | agent_id, skill_id |
| `agent_mcp_bindings` | Agent/Capability | Junction | < 5K | agent_id, mcp_server_id |
| `skills` | Capability | 聚合根 | < 1K | tenant_id (nullable) |
| `mcp_servers` | Capability | 聚合根 | < 100 | tenant_id |
| `teams` | Team | 聚合根 | < 1K | tenant_id, leader_id |
| `team_members` | Team | Junction | < 10K | team_id, agent_id |
| `routing_rules` | Team | 实体 | < 5K | team_id |
| `tasks` | Task | 聚合根 | < 100K | tenant_id, created_by |
| `subtasks` | Task | 实体 | < 1M | task_id, agent_id |
| `decisions` | Task | 实体 | < 100K | task_id, source_agent_id |
| `task_log_entries` | Task | 追加只写 | < 10M | task_id |
| `discussion_sessions` | Discussion | 聚合根 | < 10K | tenant_id, created_by |
| `discussion_participants` | Discussion | Junction | < 50K | session_id, agent_id |
| `discussion_turns` | Discussion | 追加只写 | < 1M | session_id |

**向量存储**：

| 集合命名规则 | 存储系统 | 数量级 | 隔离单位 |
|-----------|---------|-------|---------|
| `mem_{memory_store_id}` | LanceDB（本地）/ Qdrant（远程） | < 10K 条/Agent | Agent 级别 |

---

*本文档由 PRD v0.1.0、ARCHITECTURE v0.1.0、DOMAIN v0.1.0 推导生成，描述 Novai Hub 的完整数据存储模型。*
*Phase 2 新增实体（TeamGroup、团队评分、多用户协作扩展）在对应版本更新本文档。*