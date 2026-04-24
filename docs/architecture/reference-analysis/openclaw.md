# OpenClaw 参考架构分析

> 范围限定：本文仅基于 `docs/references/openclaw-main/` 目录下的**开源代码与文档**进行抽象归纳，不复制源码、不照搬目录结构，也不据此推导 Octopus 设计。所有结论均附带本地文件路径引用；无法从材料中直接验证的判断标注为 **Unverified**。

---

## 1. 总览与产品形态（Observed）

- OpenClaw 是“**个人 AI 助手**”形态：一个**常驻的本地 Gateway 守护进程**负责接入多种消息通道，再由通道路由到一个或多个隔离的 Agent 实例。参考 `docs/references/openclaw-main/README.md`（§"OpenClaw is a personal AI assistant you run on your own devices"、§"Local-first Gateway — single control plane for sessions, channels, tools, and events"）。
- 产品定位是**单运营者信任边界**：文档明确声明 OpenClaw 不是面向互相不信任多租户的安全边界。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Scope first: personal assistant security model"）。
- 核心由 TypeScript 单进程构建，目录分工在根 AGENTS 中有明确声明：`src/`（核心）、`extensions/`（内置插件）、`src/plugin-sdk/*`（公共 SDK）、`src/channels/*`（通道核心）、`src/plugins/*`（加载与注册表）、`src/gateway/protocol/*`（协议）、`apps/`（Mac/iOS/Android）。参考 `docs/references/openclaw-main/AGENTS.md`（§Repo Map）。

---

## 2. 核心架构分层（Observed）

OpenClaw 可以被抽象成五个相对清晰的层级，而不是按目录平摊：

1. **控制平面（Gateway）**
   - 长存守护进程，承载 WebSocket API、HTTP API、Control UI、Canvas Host。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§"Gateway (daemon)"、§"canvas host is served by the Gateway HTTP server"）。
   - 集中挂管 WS+HTTP 于同一端口，默认 `127.0.0.1:18789`，并强制 `first frame MUST be connect`。参考 `docs/references/openclaw-main/docs/gateway/protocol.md`（§Transport, §Handshake）。
2. **通道层（Channels）**
   - 负责与每种消息平台的协议适配、认证、消息入站/出站、群组/DM 语义。核心部分 `src/channels/**` 被明确划为“内部实现，插件作者不应直接 import”；对外通过 SDK（`openclaw/plugin-sdk/*`）暴露。参考 `docs/references/openclaw-main/src/channels/AGENTS.md`（§"Channels Boundary", §Boundary Rules）。
3. **Agent 运行时层（embedded agent runtime）**
   - 一个嵌入式单 Agent 运行回路，封装 intake → context → inference → tool exec → streaming → 持久化。参考 `docs/references/openclaw-main/docs/concepts/agent-loop.md`（§Entry points、§"How it works (high-level)"）。
   - 明确声明基于 “Pi agent core”，OpenClaw 只负责会话管理、发现、工具连线与通道投递。参考 `docs/references/openclaw-main/docs/concepts/agent.md`（§Runtime boundaries）。
4. **插件 / 能力 / 工具 / 技能层（Extensions）**
   - 由独立的 `extensions/<provider>` 包和 `skills/<skill>` 目录构成扩展宇宙；通过 Plugin SDK 注册能力。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Architecture overview" 四层：manifest+discovery / enablement+validation / runtime loading / surface consumption）。
5. **节点与伴生应用（Nodes / Apps）**
   - Mac、iOS、Android 三类应用以 **WebSocket Node** 形式接入 Gateway，声明 `role: node` + capabilities + commands。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§"Nodes (macOS / iOS / Android / headless)"）和 `docs/references/openclaw-main/docs/gateway/protocol.md`（§"Node example"）。

每层的**入/出口**都以协议或 SDK 定义，而不是直接共享内存对象。根仓 AGENTS.md 中明确“Core must stay extension-agnostic”和“Extensions cross into core only via `openclaw/plugin-sdk/*`”。参考 `docs/references/openclaw-main/AGENTS.md`（§Architecture）。

---

## 3. 模块边界：控制平面 vs 运行时平面（Observed）

OpenClaw 反复强调“**control-plane vs runtime-plane / data-plane**”的分离：

- 插件系统：Manifest（控制面）可以在**不执行插件代码**的前提下完成发现、配置校验、安装/下发、UI 提示；只有真正要跑时才进入 runtime。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"The important design boundary"：discovery + config validation should work from manifest/schema metadata without executing plugin code）。
- 通道层：`src/plugins/AGENTS.md` 明确“Keep control-plane and runtime-plane concerns separate”，并要求“preserve laziness in discovery and activation flows”。参考 `docs/references/openclaw-main/src/plugins/AGENTS.md`（§Boundary Rules）。
- Gateway 启动：“Gateway Hot Paths”指南要求 Gateway 的 HTTP/server 代码**不要为了静态问题加载完整通道 registry**，而应使用轻量 artifact。参考 `docs/references/openclaw-main/src/gateway/AGENTS.md`（§Guardrails）。

这个贯穿的思想（“元数据优先、运行时按需加载”）是 OpenClaw 架构中一个显著的工程取舍：用复杂性换启动成本与可调试性。

---

## 4. Gateway 协议与客户端模型（Observed）

- 单一 WebSocket 协议同时承担**控制面和节点传输**。所有客户端（CLI / macOS app / web admin / iOS / Android / headless nodes）都说同一套协议，通过 handshake 时的 `role` 和 `scopes` 区分身份。参考 `docs/references/openclaw-main/docs/gateway/protocol.md`（§"single control plane + node transport"、§"Roles + scopes"）。
- 帧结构三类：`req` / `res` / `event`；侧效方法（如 `send`、`agent`）要求 **idempotency key**，服务端保留短期去重缓存。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§"Wire protocol"）。
- 事件**不会回放**：客户端在断开后必须刷新，不能依赖服务端重发。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§Invariants: "Events are not replayed; clients must refresh on gaps"）。
- 协议 schema 使用 **TypeBox**，由此派生 JSON Schema，再生成 Swift 模型给 iOS/macOS。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§"Protocol typing and codegen"）。
- 身份与配对：所有 WS 客户端在 `connect` 中带 `device` 标识，首次需要“设备配对”；本机 loopback 可自动批准，tailnet/LAN 仍需显式批准。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§"Pairing + local trust"）。

抽象上 OpenClaw 把“控制 UI / CLI / 手机节点 / Canvas”这些异构客户端**统一为同一份握手+事件流 API**，用 role + capability + command 声明来区分权限与能力，而不是按客户端类型分叉服务端。

---

## 5. Agent 运行时与嵌入式回路（Observed）

- **单入口**：对外暴露 `agent` RPC 与 `agent.wait`，CLI 侧有 `agent` 命令。参考 `docs/references/openclaw-main/docs/concepts/agent-loop.md`（§Entry points）。
- **运行拓扑**：
  - `agent` RPC 持久化会话元数据后，立刻返回 `{ runId, acceptedAt }`，异步进入 `agentCommand`。
  - `agentCommand` 解析模型/思考级、加载 skills、调用 `runEmbeddedPiAgent`，并补齐缺失的 lifecycle 事件。
  - `runEmbeddedPiAgent` 通过 **per-session + global lanes** 串行化执行、订阅 pi-agent-core 事件、enforced timeout、abort 机制。
  - `subscribeEmbeddedPiSession` 把 pi-agent-core 的事件桥接到 OpenClaw 的 `agent` 流：`tool` / `assistant` / `lifecycle`。
  - 参考 `docs/references/openclaw-main/docs/concepts/agent-loop.md`（§"How it works (high-level)"）。
- **工作区契约**：每个 Agent 有唯一 `workspace` 目录作为 **cwd**；启动时注入 `AGENTS.md` / `SOUL.md` / `TOOLS.md` / `BOOTSTRAP.md` / `IDENTITY.md` / `USER.md` 等 Markdown bootstrap 文件作为系统提示。参考 `docs/references/openclaw-main/docs/concepts/agent.md`（§Workspace、§"Bootstrap files (injected)"）。
- **多 Agent 隔离**：每个 Agent 有自己的 workspace、`agentDir`、auth 档案、会话存储，路径 `~/.openclaw/agents/<agentId>/...`。参考 `docs/references/openclaw-main/docs/concepts/multi-agent.md`（§"What is 'one agent'?"、§Paths）。
- **Session = 路由键 + 持久上下文**：
  - `session.dmScope` 决定 DM 是共享 `main` 还是按 `per-peer / per-channel-peer / per-account-channel-peer` 隔离。参考 `docs/references/openclaw-main/docs/concepts/session.md`（§"DM isolation"）。
  - 会话存储按 `~/.openclaw/agents/<agentId>/sessions/<sessionId>.jsonl`（append-only transcript）+ `sessions.json`（索引）。参考 `docs/references/openclaw-main/docs/concepts/session.md`（§"Where state lives"）。
  - 会话既有**每日重置**又有**空闲重置**，谁先触发谁生效。参考 `docs/references/openclaw-main/docs/concepts/session.md`（§"Session lifecycle"）。

一个关键抽象：**session 是路由选择器而非鉴权边界**。文档明确反对把 `sessionKey` 当 auth token。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Trust boundary matrix"）。

---

## 6. 消息路由与多 Agent 绑定（Observed）

- 路由规则是**确定性 + 最具体优先**。OpenClaw 定义了 8 级匹配顺序：peer / parentPeer / guildId+roles / guildId / teamId / accountId / 通道级通配 / 默认 agent 回退。参考 `docs/references/openclaw-main/docs/concepts/multi-agent.md`（§"Routing rules"）。
- **`agent` / `accountId` / `binding`** 三元抽象：
  - `agentId` = 一个独立 “大脑”（workspace + auth + sessions）
  - `accountId` = 同通道里的一个账户实例（例如两个 WhatsApp 号码）
  - `binding` = “通道账户 + peer / 群 / 主题 → Agent”的路由规则
  - 参考 `docs/references/openclaw-main/docs/concepts/multi-agent.md`（§Concepts）。
- **DM 访问策略**：每个通道有 `dmPolicy = pairing | allowlist | open | disabled`；`open` 还必须显式在 `allowFrom` 中包含 `"*"`，防止误开公网。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"DM access model"）。
- **群组访问控制**：群层 allowlist + `requireMention` + `groupPolicy` + `groupAllowFrom` 四件套；群策略先于 mention/回复激活判断。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Allowlists for DMs and groups"）。

在抽象上，OpenClaw 把“**触发授权**（谁能让 Agent 回）”和 “**上下文可见性**（Agent 能看到什么）”拆成两个正交旋钮，引入 `contextVisibility: "all" | "allowlist" | "allowlist_quote"`。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Context visibility model"）。

---

## 7. 插件与能力（Capability）模型（Observed）

OpenClaw 的扩展机制围绕“**能力合约 + 实现插件**”展开：

- 能力由核心定义，插件只做实现：列出 12 种公开 capability 类型（text / speech / realtime transcription / realtime voice / media understanding / image / music / video generation / web fetch / web search / channel 等）和对应的 `api.registerXxx(...)` 方法。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Public capability model"）。
- **插件是“公司/特性”的所有权边界，不是功能杂货篓**：一个公司的所有面对 OpenClaw 的能力应尽量由一个插件承担。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Capability ownership model"）。
- 插件 **shape 分类**：plain-capability / hybrid-capability / hook-only / non-capability；这是根据运行期实际注册行为，不是静态 manifest。参考同文（§"Plugin shapes"）。
- **加载流水线**（四步）：discover → validate/enablement → runtime load（jiti 动态加载或 native loader）→ registry materialization。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Load pipeline"）。
- **注册表为共享单写源**：插件模块 → registry 注册；Core 运行时 → registry 消费。Core 不直接调用插件模块。参考同文（§"Registry model"）。
- **独占 slot**：部分能力是“只能一选一”的 slot，典型例子 `memory` 与 `contextEngine`。参考 `docs/references/openclaw-main/docs/concepts/context-engine.md`（§"Configuration reference"）。
- **信任模型**：原生插件**与 Gateway 进程同信任域**，一个恶意原生插件等价于任意代码执行；compatible bundle（Codex/Claude/Cursor 等布局）相对安全，目前主要是元数据/skills。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Execution model"）。
- **配置驱动 + 允许/拒绝列表 + 路径发现**：`plugins.enabled` / `plugins.allow` / `plugins.deny` / `plugins.entries.<id>` / `plugins.slots` / `plugins.load.paths` 构成插件策略。参考 `docs/references/openclaw-main/docs/tools/plugin.md`（§Configuration）。

其中 **Provider Plugin 有 40+ 可选 hook**（catalog / normalize / prepareRuntimeAuth / refreshOAuth / buildReplayPolicy 等），构成一个“按需重写 inference 行为”的开放网格。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Hook order and usage"）。

---

## 8. 工具组织、组与策略（Observed）

- **工具是唯一“做事”的途径**：Agent 生成文本之外的一切行为都走工具。参考 `docs/references/openclaw-main/docs/tools/index.md`（前言段落）。
- **工具分组抽象**（`group:*`）：runtime / fs / sessions / memory / web / ui / automation / messaging / nodes / agents / media / openclaw；允许/拒绝都可用组简写。参考同文（§"Tool groups"）。
- **工具档位（profile）**：`full / coding / messaging / minimal` 作为快速预设，再叠加 `allow` / `deny`；deny 始终压倒 allow。参考同文（§"Tool profiles"）。
- **按 provider 定向限制**：`tools.byProvider` 允许针对特定模型提供商缩窄工具集（例子：给 `google-antigravity` 发 `minimal`）。参考同文（§"Provider-specific restrictions"）。
- **关键工具语义分层**：
  - `exec` / `process` / `code_execution` — shell/进程/沙箱计算
  - `read` / `write` / `edit` / `apply_patch` — 文件系统
  - `browser` / `canvas` — UI 驱动
  - `message` — 统一的跨通道发送（核心保留）
  - `cron` / `gateway` — **控制面工具**，可以写配置、排任务（被视为高危）
  - `sessions_*` / `subagents` / `agents_list` — 会话管理与子代理编排
  - 参考 `docs/references/openclaw-main/docs/tools/index.md`（§"Built-in tools"）。
- **统一 message 工具 + 通道特化**：Core 拥有 `message` 工具的宿主、会话/线程记账和 dispatch，通道插件通过 `describeMessageTool(...)` 返回自己 scoped action、capability 与 schema 片段。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Channel plugins and the shared message tool"）。

---

## 9. Skills 层（Observed）

- Skills 是**向模型注入使用方法的 Markdown 文档**，结构是 `SKILL.md + YAML frontmatter`。参考 `docs/references/openclaw-main/docs/tools/index.md`（§"Skills teach the agent when and how"）。
- **六级 skill 加载优先级**（高到低）：workspace/skills > workspace/.agents/skills > ~/.agents/skills > ~/.openclaw/skills > bundled > extraDirs。参考 `docs/references/openclaw-main/docs/tools/skills.md`（§"Locations and precedence"）。
- **位置与可见性分离**：加载优先级只决定同名 skill 哪份胜出；每个 Agent 另有 `agents.defaults.skills` / `agents.list[].skills` 白名单决定**能用哪些**。参考同文（§"Agent skill allowlists"）。
- 插件可以**携带 skills**，并通过 manifest 的 `metadata.openclaw.requires.config` 门控加载。参考同文（§"Plugins + skills"）。
- 注册表 ClawHub（`https://clawhub.ai`）是专属技能市场，CLI 提供 install/update/sync 流。参考同文（§"ClawHub"）。

这一层把 “Agent 提示工程” 从代码里剥离到**可版本控制的文本 + 受控仓库**，并允许按 Agent 切分可见集。

---

## 10. 沙箱 / 工具策略 / 提权的三层信任模型（Observed）

OpenClaw 把“可以跑什么”拆成三个独立的控制旋钮：

1. **Tool policy**（`tools.allow/deny/profile/byProvider`）：决定工具**是否存在**于 Agent 可见集。
2. **Sandbox**（`agents.defaults.sandbox.*`）：决定工具**在哪里跑**（host vs docker/ssh/openshell container）。
3. **Elevated**（`tools.elevated`）：显式“跳出沙箱”的后门，绑定 allowFrom。

参考 `docs/references/openclaw-main/docs/gateway/sandboxing.md`（§"Tool policy + escape hatches"）。

沙箱模式进一步细分：

- `mode`: `off | non-main | all`
- `scope`: `agent | session | shared`
- `backend`: `docker | ssh | openshell`
- `workspaceAccess`: `none | ro | rw`

参考同文（§Modes、§Scope、§Backend、§"Workspace access"）。

Exec 审批进一步叠加在上：

- 在 Gateway host 或 node host 上本地强制执行；只有当“policy + allowlist + 用户审批”三者都同意时才执行。参考 `docs/references/openclaw-main/docs/tools/exec-approvals.md`（前言、§"Where it applies"）。
- 审批会**绑定请求上下文**（canonical cwd / argv / env / exec 路径）以及**一个具体本地文件操作数**；如果无法精确绑定，拒绝放行而不是假装覆盖。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Node execution (system.run)"）。

---

## 11. 状态与持久化布局（Observed）

OpenClaw 的磁盘布局是**文件 + JSONL + 配置驱动**，没有中央数据库：

- `~/.openclaw/openclaw.json` — JSON5 配置（主文件，必须是 regular file，非 symlink）。参考 `docs/references/openclaw-main/docs/gateway/configuration.md`（前言段落）。
- `~/.openclaw/agents/<agentId>/` — 每个 Agent 的隔离状态目录。
- `~/.openclaw/agents/<agentId>/sessions/<sessionId>.jsonl` — 会话 transcript 是 **append-only JSONL**。参考 `docs/references/openclaw-main/docs/concepts/session.md`（§"Where state lives"）。
- `~/.openclaw/agents/<agentId>/agent/auth-profiles.json` — per-agent 模型 auth 档案。参考 `docs/references/openclaw-main/docs/concepts/multi-agent.md`（§"What is 'one agent'?"）。
- `~/.openclaw/credentials/` — 通道凭据、allowlist store、legacy OAuth。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Credential storage map"）。
- `~/.openclaw/sandboxes/` — 沙箱工作区拷贝。
- `~/.openclaw/workspace` 或 `~/.openclaw/workspace-<agentId>` — Agent 工作区（也是 Agent 的 `cwd`）。

**Session 锁语义**：transcript 写入受一个**进程感知、文件级**的 session write lock 保护，覆盖“in-process 队列之外”和“其他进程”两种路径；非重入，除非显式 `allowReentrant: true`。参考 `docs/references/openclaw-main/docs/concepts/agent-loop.md`（§"Session + workspace preparation"）。

---

## 12. 上下文、压缩与记忆（Observed）

- **Context Engine** 是一个**可插拔的上下文管道**，在一次运行中有四个生命周期钩点：`ingest` / `assemble` / `compact` / `afterTurn`，外加可选 `prepareSubagentSpawn` / `onSubagentEnded`。参考 `docs/references/openclaw-main/docs/concepts/context-engine.md`（§"How it works"）。
- **`ownsCompaction` 语义**：Engine 可以**独占压缩**，也可以委托给 runtime 内置流程（`delegateCompactionToRuntime`）。参考同文（§ownsCompaction）。
- **Compaction（压缩）** 是自动 + 手动 (`/compact`) 的两路触发，会在拆分点保持“assistant tool call ↔ toolResult”配对；transcript 本体保留在磁盘，压缩只影响下次喂给模型的视图。参考 `docs/references/openclaw-main/docs/concepts/compaction.md`（§"How it works"、§"Auto-compaction"）。
- **Memory** 是用**workspace 里的 Markdown 文件**承载：`MEMORY.md`（长期）、`memory/YYYY-MM-DD.md`（每日）、`DREAMS.md`（可选）。参考 `docs/references/openclaw-main/docs/concepts/memory.md`（§"How it works"）。
- Memory 是**独立的插槽**（`plugins.slots.memory`，默认 `memory-core`），与 context engine 正交但可协作。参考 `docs/references/openclaw-main/docs/concepts/context-engine.md`（§"Relationship to compaction and memory"）。
- “Dreaming” 是**后台巩固 / 促升** pass，只有通过分数、召回频次、查询多样性门槛的条目才能升入 `MEMORY.md`；输出走 `DREAMS.md` 供人工复核。参考 `docs/references/openclaw-main/docs/concepts/memory.md`（§Dreaming）。

抽象上，“**记忆 = 文件**”使 Agent 的“可记忆部分”本身具备可读、可 diff、可备份的属性。

---

## 13. 工作流：一次 Agent 运行的端到端序列（Observed）

综合 `docs/references/openclaw-main/docs/concepts/agent-loop.md` 与 `docs/references/openclaw-main/docs/concepts/queue.md`，典型流转：

1. 入站消息进入某个通道适配器；
2. 经过 DM/群策略、allowlist、mention gating、`contextVisibility` 过滤；
3. 路由层按 binding 规则挑出 `agentId`，并生成 sessionKey；
4. 消息进入**按 session 的执行车道**（`session:<key>`）和**全局车道**（`main` / `subagent` 等），由 concurrency cap 决定是否等待；
5. 触发 typing 指示（如果通道支持），然后拿到写锁、加载 skills 快照、注入 bootstrap 文件；
6. Context engine 的 `ingest → assemble` 决定系统提示与历史；
7. 调用嵌入式 Pi agent 回路，订阅 lifecycle/assistant/tool 事件，按需 stream 到通道；
8. 工具调用经过 tool policy → sandbox → exec approvals 三层闸门；
9. 结束时依据 queue mode（`collect` / `steer` / `followup` / `steer-backlog`）决定是否把等待中的消息合并进下一轮；
10. 可能触发 `compact` / auto-compaction，把旧 transcript 归约；
11. 写回 transcript、发送 chat `final`、释放写锁、持久化 usage/meta。

其中 **queue mode** 是 OpenClaw 特有的“streaming + 消息积攒”语义，允许把 steering 消息注入**下一个工具边界**之后、下一次 LLM 调用之前，避免中断 tool call 的语义一致性。参考 `docs/references/openclaw-main/docs/concepts/agent.md`（§"Steering while streaming"）。

---

## 14. 子 Agent 与跨会话编排（Observed）

- `sessions_spawn` 是**启动子 Agent 运行**的工具入口，使用独立 lane（`subagent`）；默认不把会话工具给子 Agent，强调“小接口面不易误用”。参考 `docs/references/openclaw-main/docs/tools/subagents.md`（§Primary goals、§Tool）。
- 子 Agent 有两种上下文模式：
  - `isolated`（默认）：全新 transcript；
  - `fork`：把父会话当前 transcript 分叉给子 Agent。
  - 参考同文（§"Tool params"）。
- `sandbox: "inherit" | "require"`：子 Agent 可以强制要求沙箱，父沙箱的子如果不沙箱会被拒绝。参考同文（§Allowlist）。
- **Thread-bound session**：在支持的通道（当前文档列出 Discord），子 Agent 可以被“钉”在一个帖子上，后续该帖消息一律路由回同一子会话；带 `idleHours` / `maxAgeHours` 的自动解绑策略。参考同文（§"Thread-bound sessions"）。
- 完成后的**announce**：子 Agent 完成时会把结果 push 回父通道，携带 `Result` / `Status` / stats，并明确指示父 Agent 用正常语气重写（避免泄露内部元数据）。参考同文（§"Spawn behavior"）。
- `sessions_send` / `sessions_yield` / `sessions_history` 构成**跨会话通信与召回**工具集；`sessions_history` 是**有界、净化后的召回视图**，不是原始 transcript dump。参考 `docs/references/openclaw-main/docs/concepts/session-tool.md`（§"Available tools"、§"Listing and reading sessions"）。

抽象上，OpenClaw 把“多 Agent 协作”设计为**显式 session 图 + 显式消息**，而不是黑箱 orchestrator。

---

## 15. 钩子系统（Observed）

OpenClaw 有两套互补的钩子系统：

1. **Internal / Gateway hooks**：由 `HOOK.md + handler.ts` 结构描述，响应 `command:*` / `session:compact:*` / `message:*` / `agent:bootstrap` / `gateway:startup` 等事件；可以向 `event.messages` push 回用户可见文本。参考 `docs/references/openclaw-main/docs/automation/hooks.md`（§"Quick start"、§"Event types"、§"Handler implementation"）。
2. **Plugin hooks**：更细粒度，嵌入 Agent 回路与 Gateway pipeline，例如 `before_model_resolve` / `before_prompt_build` / `before_agent_reply` / `agent_end` / `before_compaction` / `after_compaction` / `before_tool_call` / `after_tool_call` / `before_install` / `tool_result_persist` / `message_received|sending|sent` / `session_start|end` / `gateway_start|stop`。参考 `docs/references/openclaw-main/docs/concepts/agent-loop.md`（§"Plugin hooks"）。

两套钩子各自表达**静态策略**（`before_install` block 终止）与**动态 side effect**（`message_received` 采集事件）的不同语义，并明确“`block: false` 是 no-op 不能清除之前的 block”这样的优先级合成规则。参考同文（§"Hook decision rules"）。

---

## 16. 通道插件：统一外观、独占内部（Observed）

- 通道用户看到“统一的 `message` 工具”，但每种通道内部仍拥有自己的：
  - 账户与认证适配
  - 会话/线程语法（conversation id 如何编码 thread id）
  - 自定义 action（如 Discord poll、Slack 原生 button）
  - native schema contributions
  - 参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Channel plugins and the shared message tool"）。
- 输出呈现使用 **`MessagePresentation` 抽象合约**（text / context / divider / buttons / select）作为通用表达，由 Core 决定是否“原生渲染”或“降级为文本”；不允许通道插件绕过暴露 provider-native UI 原语。参考同文（§"Message tool schemas"）。
- 通道插件 manifest 中可声明 `openclaw.channel` 目录元数据（label / docsPath / blurb / aliases / markdownCapable / exposure 等），使 Core 不必存储通道硬编码就能做发现、setup、catalog 合并（支持**外部目录文件**合并）。参考同文（§"Channel catalog metadata"）。

抽象来说，OpenClaw 的通道架构等同于“**适配器模式 + 能力协商**”，用合约和 runtime scope 参数（`accountId` / `currentChannelId` / `currentThreadTs` / `sessionKey` / `agentId` / `requesterSenderId`）把通道差异注入到一个统一的 message tool host。

---

## 17. 安全与信任边界的总结（Observed）

OpenClaw 的安全模型是**分层的显式闸门**，不是一条大锁：

| 闸门 | 目的 | 失败模式 |
|---|---|---|
| `gateway.auth.mode`（token/password/trusted-proxy/none） | 鉴权控制面接入 | 默认 fail-closed，loopback 也要 token |
| Device pairing + nonce 签名 | 防止未知客户端接入 | loopback 可自动批准，其他需显式批准 |
| `dmPolicy` / group allowlist / mention gating | 谁能**触发** Agent | open 必须配合 `allowFrom: "*"` 显式打开 |
| `contextVisibility` | 谁的内容能**进入**模型上下文 | allowlist/allowlist_quote 分级过滤 |
| tool policy + byProvider | 工具**是否可见** | deny 压倒 allow |
| sandbox mode/scope/workspaceAccess | 工具**在哪儿跑** | workspaceAccess=none 默认不挂载本机 |
| exec approvals + bound argv/cwd/file | 具体命令是否允许 | 无法精确绑定则拒绝 |
| elevated.allowFrom | 跳出沙箱的后门 | 必须显式白名单 |
| external-content sanitization | 防止 prompt injection 从 web/email/doc 进入 | 自动抽取 `<\|...\|>` 等控制 token |
| `openclaw security audit` / `--deep` / `--fix` | 定期扫描漂移 | checkId-structured 报告，可 auto-fix |

依据：`docs/references/openclaw-main/docs/gateway/security/index.md`（§"Trust boundary matrix"、§"Hardened baseline"、§"Command authorization model"、§"External content special-token sanitization"、§"What the audit checks"），`docs/references/openclaw-main/docs/gateway/sandboxing.md`（§"Tool policy + escape hatches"），`docs/references/openclaw-main/docs/tools/exec-approvals.md`（§"Inspecting the effective policy"）。

一个值得记录的设计取舍：**默认接受“sessionKey 不是鉴权 token”“plugin 与进程同信任”“trusted-single-operator”** 的假设，用显式文档和 audit 承认自己的边界，而不是假装是一个多租户平台。

---

## 18. 值得注意的横切约束（Observed）

- **Manifest-first / lazy runtime**：Control UI、schema 校验、doctor、status 都应从 manifest 得到答案，避免 cold-load 插件运行时。参考 `docs/references/openclaw-main/docs/plugins/architecture.md`（§"Manifest-first behavior"）。
- **配置严格模式**：未知 key / 类型错 / 值非法都会让 Gateway **拒绝启动**；只允许 `$schema` 作为根异常；启动后保留 last-known-good 配置用于自恢复。参考 `docs/references/openclaw-main/docs/gateway/configuration.md`（§"Strict validation"）。
- **通道 env 文件硬屏蔽**：workspace 本地 `.env` 不能覆盖 `OPENCLAW_*` 或通道 endpoint key，防止克隆仓库的 env 篡改通道方向。参考 `docs/references/openclaw-main/docs/gateway/security/index.md`（§"Workspace .env files"）。
- **Proxy header 安全**：只信任 `gateway.trustedProxies` 列出的代理发来的 `X-Forwarded-For`；非信任来源带 forwarding header 的连接不会被视为本地客户端。参考同文（§"Reverse proxy configuration"）。
- **事件不回放**：这是一个**显式的简化决策**，迫使客户端在断开后走 snapshot 重建而不是依赖历史。参考 `docs/references/openclaw-main/docs/concepts/architecture.md`（§Invariants）。

---

## 19. 受限与未验证（Unverified）

以下几项在本次只读本地文件的范围内**没有得到直接代码级验证**，仅作为方向性判断：

- **Pi agent core 的内部实现**（如具体的 tool dispatch、LLM stream 抽象、ReAct 形态）。文档只声明 OpenClaw “embedded” 了 pi-agent-core 而未暴露其实现细节，本仓只有 OpenClaw 对接层；需要另外查阅 pi-agent-core 仓库才能确定。参考 `docs/references/openclaw-main/docs/concepts/agent.md`（§"Runtime boundaries"）。— **Unverified**
- **Canvas Host / A2UI 的内部渲染模型**：文档与路径存在（`src/canvas-host/`、`/__openclaw__/canvas/`、`/__openclaw__/a2ui/`），但本次未打开 `src/canvas-host/a2ui.ts` 内部实现。— **Unverified**
- **sandbox `openshell` backend 的资源回收语义与 `mirror` vs `remote` 的一致性保证**：文档给出行为描述，但同步点/冲突分辨的具体实现细节未核对。— **Unverified**
- **Skill Workshop 自动写入的审核队列与 quarantine 详细流程**：`docs/references/openclaw-main/docs/tools/skills.md`（§"Skill Workshop"）只做了抽象描述。— **Unverified**
- **`memory-wiki` 与 `memory-core` 的数据兼容 / 迁移路径**：`docs/references/openclaw-main/docs/concepts/memory.md`（§"Memory Wiki companion plugin"）仅说明“不替代，而是并存”，具体数据一致性保证未核对。— **Unverified**

---

## 20. 分析可直接复用的抽象要点（Observation only）

> 本节只总结 OpenClaw 在**架构抽象**层面可复用的模式，不据此推导 Octopus。

1. **控制平面 = 一条 WebSocket + 角色+能力声明**：用单协议承载 CLI、桌面、手机、headless node，避免每种客户端一套服务端。
2. **Agent / Session / Binding 的三元拆分**：`agent` = 大脑与持久化根；`session` = 路由与上下文容器（不是鉴权）；`binding` = (channel, account, peer) → agent 的确定性路由表。
3. **Capability 合约 + Plugin 所有权**：核心定义典型 capability（text / speech / image / video / channel / context-engine 等），插件只做实现；独占型能力走 slot。
4. **Tool policy、Sandbox、Elevated、Exec Approvals 四重正交闸门**，可叠加可分别配置。
5. **Manifest-first / lazy runtime**：让 `doctor`、status、UI、schema lookup 尽可能从元数据回答。
6. **Transcript + Markdown Memory 的“文件即真相”**持久化模型，把可记忆部分做成 diff 友好的文本。
7. **Queue-based Agent Loop**：per-session lane + global lane + queue modes（collect/steer/followup/steer-backlog），在“单会话串行”与“多会话并行”之间取得可配置的平衡。
8. **Sub-agent 是显式 session + announce 桥接**，不用隐式 orchestrator；`context: isolated|fork` 保留显式分叉语义。
9. **安全由“显式文档 + 工具化 audit”支撑**：每个 checkId 有 severity 与 fix key，并有 `--fix` 自动收敛少量配置漂移。
10. **默认信任模型是“单运营者”**：诚实的边界声明让设计可以简单，把多租户留给“开多个 Gateway 实例”的上层编排。

---

## 21. 文档更新钩点

- 本文档与 `docs/architecture/reference-analysis/evidence-index.md` 中 **OpenClaw** 段保持同步；若后续再次阅读 `docs/references/openclaw-main/` 产生新观察，优先更新 evidence-index 中的行，再回填本文相应章节。
