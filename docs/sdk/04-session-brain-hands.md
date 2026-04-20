# 04 · 三分架构：Brain / Hands / Session

> "We virtualized the components of an agent: a **session**, a **harness**, and a **sandbox**. This allows the implementation of each to be swapped without disturbing the others."
> — [Anthropic · Managed Agents: Decoupling the brain from the hands](https://www.anthropic.com/engineering/managed-agents)（2026）

这是本 SDK 最高层的**架构决策**：将一个 Agent 解构为三个可独立演进的组件。

## 4.1 Why：解耦的根本动机

### 4.1.1 Harness 编码的假设会 staleness

> "Harnesses encode assumptions about what Claude can't do on its own. When Claude evolves past those limitations, those assumptions start to feel obsolete."

例子（Anthropic 自己踩过的坑）：

- Sonnet 4.5 接近 context 上限会"提前收工"→ 在 harness 里加了 **context reset** → Opus 4.5 不再有此问题 → reset 变 dead weight。
- 早期模型不擅长 git → harness 教它一整套 git 命令 → 新模型自主搞定 → 教学变噪音。

**结论**：harness 内的每一段"补偿"代码都应该**可拆可换**。

### 4.1.2 "Pet vs Cattle" 问题

早期 Managed Agents 给每个会话分配一个"我的容器"：

- 管理员会**手动 nurse** 这个容器
- 容器健康变成长期心智负担
- 随容器数量变大，运维爆炸

**解决**：让所有容器都是 cattle；挂了就重建；harness 把错误当工具输出反馈给模型。

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"The pet-and-cattle problem"。

## 4.2 三个组件 + 四条接口

### 4.2.1 组件职责

```
┌──────────────────────────────────────────────────────────────┐
│                         Session                                │
│   Append-only event log; durable; the single source of truth   │
│   Events: messages, tool_use, tool_result, plans, checkpoints  │
└──────────────────────────────────────────────────────────────┘
               ▲                                     ▲
               │  emitEvent / getEvents              │
               │                                     │
┌──────────────┴────────────────┐  ┌─────────────────┴─────────────────┐
│         Brain (Harness)        │  │            Hands (Sandbox)        │
│                                │◀─│                                   │
│  - Prompt builder              │  │  - Bash / FS / Edit / ...         │
│  - Tool dispatcher             │──▶  - Sandbox runtime (OS-level)    │
│  - Context engineer            │  │  - MCP servers                    │
│  - Permission gate             │  │  - execute(name, input) → string  │
│  - Subagent orchestrator       │  │  - provision(resources) lazily    │
│                                │  │                                   │
│  wake(sessionId) = rebuild     │  │  (cattle; crash → tool error)     │
│    itself from Session         │  │                                   │
└────────────────────────────────┘  └───────────────────────────────────┘
```

### 4.2.2 四条接口（窄而稳定）

| 接口 | 方向 | 签名 | 关键不变量 |
|---|---|---|---|
| `execute(name, input)` | Brain → Hands | `(toolName, jsonInput) → {tool_use_id, output:string, is_error:boolean}` | **永远返回**，错误被包装进 `is_error`（与 Anthropic API `tool_result` 边界对齐，见 03 §3.1.3） |
| `provision(spec)` | Hands 内部 | 懒分配新 sandbox | 只在第一次使用时；失败 → 作为 `tool_result` 反馈 |
| `emitEvent(id, event)` | Brain → Session | append-only | **幂等**（由 event_id 去重） |
| `getEvents(id, range)` | Brain → Session | 支持区间、tail、按类型过滤 | **时间排序**；支持回放 |

可选 ops：

| 接口 | 作用 |
|---|---|
| `wake(sessionId)` | Brain 自启动；重建 context、tool registry、permissions |
| `fork(sessionId, fromEvent)` | 分叉一条 session（用于 rewind / what-if） |
| `terminate(sandboxId)` | 回收 Hands |

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Building on decoupled interfaces"。

## 4.3 Brain（Harness）详解

### 4.3.1 内部组件

```
Brain
├─ PromptBuilder        (see 02-context-engineering.md)
├─ ToolDispatcher       (see 03-tool-system.md)
├─ ContextEngineer      (compaction / jit-fetch / note-taking)
├─ PermissionGate       (see 06-permissions-sandbox.md)
├─ BudgetTracker        (max_turns / token budget)
├─ HookRuntime          (see 07-hooks-lifecycle.md)
├─ SubagentOrchestrator (see 05-sub-agents.md)
└─ SessionClient        (emitEvent / getEvents)
```

### 4.3.2 "Brain is cattle"

Brain 进程随时可被杀；重建流程：

1. 从 Session 读 header（`config_snapshot_id`, `agent_definition`）
2. 根据 header 装回 tool registry、system prompt、permission rules
3. 调 `getEvents(latest)` 取一定窗口（或压缩摘要）
4. 继续下一轮

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Brain is cattle"。

### 4.3.3 Brain 不持有凭据

凭据（Git token、OAuth tokens、API keys）**不进入 Brain 的工作内存**：

- Git：在 clone 时注入到 `.git/config`；之后 push/pull 自动用，不经过 Brain
- MCP OAuth：由"MCP 代理"层在网络请求路径上注入 `Authorization` header
- 用户输入的敏感值：走专用 `AskUserQuestion` + `secret` 字段 + vault 写入

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Credentials never enter the sandbox"；[Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)。

## 4.4 Hands（Sandbox）详解

### 4.4.1 组件

```
Hands
├─ SandboxBackend      (bubblewrap | seatbelt | docker | firecracker | in-process)
├─ ToolHandlers        (FS / Shell / Process / Network / ...)
├─ MCPHost             (本地 MCP servers; stdio / sse)
├─ ResourceLimits      (cpu / mem / fd / 网络)
└─ NetworkProxy        (egress allowlist + auth injection)
```

### 4.4.2 Lazy Provisioning

- 会话启动时**不**创建沙箱
- 首个需要沙箱的工具调用触发 `provision(spec)`
- Anthropic 报告 TTFT（首 token 延迟）p50 ↓60%, p95 ↓90%

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Lazy provisioning"。

### 4.4.3 Sandbox 错误即工具错误

```
brain: execute('shell_exec', {cmd: 'npm install'})
hands: (sandbox crashed)
hands: → returns { output: 'sandbox unavailable', is_error: true }
brain: Model sees error, picks alternative strategy (e.g., retry / use Read tool)
```

**绝对不做**：把 sandbox crash 变成 Brain 的 unhandled exception。

### 4.4.4 Many Brains, Many Hands

一个 Brain 可以同时拥有多个 Sandbox：

```
brain ─── execute('remote_env_a', {cmd:...})
      ├── execute('remote_env_b', {cmd:...})   # reasoning across envs
      └── execute('local_fs', {path:...})
```

这是 Managed Agents "many brains, many hands" 模式的核心：Brain 本身不绑定到某一个 Hands。

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Many brains, many hands"。

## 4.5 Session 详解

### 4.5.1 存储层

本 SDK 的 Session 存储必须同时满足 Octopus 的持久化治理约束（本仓 `AGENTS.md` §"Persistence Governance"）：

- 结构化、可查询字段（session meta、消息索引、todo 状态）→ **SQLite (`data/main.db`)**
- Append-only 事件流 → **JSONL (`runtime/events/*.jsonl`)**
- 大 blob（长工具输出、附件）→ **`data/blobs/` 文件，DB 只存 hash/path**

### 4.5.2 事件表 + JSONL 双写

- SQLite 投影：便于 UI 分页、检索、关联查询
- JSONL：便于审计、回放、灾备、流式订阅

启动时从 JSONL 重建 SQLite 投影是幂等操作（event_id 去重）。

### 4.5.3 基础事件表（SQLite DDL 示意）

```sql
CREATE TABLE session (
  id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  agent_definition_id TEXT NOT NULL,
  config_snapshot_id TEXT NOT NULL,
  effective_config_hash TEXT NOT NULL,
  started_from_scope_set TEXT NOT NULL,   -- JSON array
  model_id TEXT NOT NULL,
  status TEXT NOT NULL                    -- active | paused | done | aborted | errored
);

CREATE TABLE event (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  type TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  payload_ref TEXT NOT NULL,              -- points to JSONL offset OR inline
  -- 索引
  UNIQUE (session_id, seq),
  FOREIGN KEY (session_id) REFERENCES session(id)
);
```

### 4.5.4 Session 不是上下文窗口

- Session 可以有 **数百 MB / 数十万事件**
- 上下文窗口 **每轮只含**模型当前能看到的那一截

两者的桥梁是 Brain 的 `ContextEngineer`：**按需**从 Session 拉取、压缩、投喂。

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Session is durable context"。

### 4.5.5 Rewind / Checkpoints

- 每个 tool_use 前自动记录 checkpoint：`{fs_snapshot_ref, session_seq}`
- 用户按 `Esc Esc`：
  1. 找到 checkpoint
  2. 回滚工作目录（或 ephemeral working copy）
  3. `fork` 新 session，从 checkpoint 之后重开

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Rewind with checkpoints"。

## 4.6 Config Snapshot（Octopus 专有约束）

每次 session 启动必须：

1. 读 effective runtime config（合并 user < workspace < project）
2. 计算 `effective_config_hash`
3. 持久化一份 `config_snapshot`（不存绝对路径；存 `sourceRefs` + hashes）
4. session 记录 `config_snapshot_id`

**运行中 session 不跟随磁盘 config 变更**（本仓 `AGENTS.md` §"Live session behavior"）。

## 4.7 "Many Brains, Many Hands" 示例场景

### 场景 A：跨多机器调试

```
Brain
  ├── execute('sandbox_linux_x86',  {cmd:'...'})
  ├── execute('sandbox_macos_arm64',{cmd:'...'})
  └── execute('local_fs',           {read:'report.md'})
```

Brain 对三个 Hands 同时推理，无需把它们的输出汇总到某个"老大"sandbox。

### 场景 B：Fan-out 研究

Lead Brain → spawn 5 Research Brain（各自独立 Session + Hands）→ 并行 web_search → 返回 summary。详见 [`05-sub-agents.md`](./05-sub-agents.md)。

### 场景 C：Host 抽象

同一 Brain 在 **Tauri 本地宿主** 和 **Browser 宿主** 下需要一致接口：

- 本地：走 `apps/desktop/src/tauri/shell.ts`（IPC 到 Rust 后端）
- 浏览器：走 `apps/desktop/src/tauri/workspace-client.ts`（HTTP 到服务端）

对 Brain 都是 `execute()`；这契合本仓 `AGENTS.md` §"Host consistency rule"。

## 4.8 可替换性（Swap-without-Disturb）矩阵

| 想替换 | 影响面 | 是否需重建 Brain | 是否需重建 Sandbox | 是否需重建 Session |
|---|---|---|---|---|
| 底层模型版本 | Brain 内部 | 是（动态 switch 也可） | 否 | 否 |
| Compaction 策略 | Brain 内部 | 是 | 否 | 否 |
| Sandbox 后端（bubblewrap → docker） | Hands 内部 | 否 | 是 | 否 |
| 存储引擎（SQLite → PG） | Session 内部 | 否 | 否 | 是（数据迁移） |
| Tool 实现 | Hands | 否 | 是（或局部） | 否 |
| Tool schema | Tool contract | 是（Brain 需重启以刷新提示词缓存） | 是 | 否 |

这是本 SDK"每个组件都可独立演进"的验证性表格。

## 4.9 失败域（Failure Domains）

| 失败 | 策略 |
|---|---|
| Brain OOM | 重启；`wake(session)` 恢复 |
| Sandbox crash | 下次 provision；当前调用作为 tool error 反馈 |
| Session 存储暂时不可用 | Brain 暂停循环，缓存事件，可用后补写；**不丢消息** |
| 模型 API 长时间不可达 | fallback model；超时则挂起 session 并通知用户 |
| Clock skew / eventual consistency | 事件 id = `ULID`（带单调时间戳），避免排序歧义 |

## 4.10 对 Octopus 实施的 Checklist

- [ ] Brain / Hands / Session 作为独立 package（可发布独立版本）
- [ ] `execute()` 契约固定；任何 Hands 后端只需实现它
- [ ] Session 事件持久化双写（SQLite 投影 + JSONL）
- [ ] `wake(session)` 幂等可重复；集成测试覆盖"Brain 随机 kill"场景
- [ ] Config snapshot 在 session 启动时生成，落入事件流
- [ ] 凭据不经过 Brain 与 Sandbox，由代理层注入
- [ ] Lazy provisioning：第一次工具调用前不创建 sandbox
- [ ] 新 sandbox 后端实现只需通过 8–10 个契约测试即可接入

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Managed Agents](https://www.anthropic.com/engineering/managed-agents) | **本章主干**：三分架构、lazy provisioning、cattle 原则、many-brains-many-hands |
| [Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) | Sandbox + 凭据隔离 |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) | Rewind / checkpoints |
| [Claude Agent SDK – Sessions](https://docs.claude.com/en/api/agent-sdk/sessions) | resume / fork 语义 |
| 本仓 `AGENTS.md` §Persistence Governance, §Runtime Config | Octopus 落地约束：文件 + SQLite + JSONL 分工，config snapshot |
| Claude Code restored src `history.ts` | Session append 实现示例 |
| Hermes `hermes_state.py` | SQLite+FTS session store |
| OpenClaw `~/.openclaw/agents/<id>/sessions/*.jsonl` | 多 agent session 目录约定 |
