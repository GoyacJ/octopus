# 参考项目横向比较矩阵

> 范围：本文基于 `docs/architecture/reference-analysis/` 下四个已有产出（`claude-code-sourcemap.md`、`hermes-agent.md`、`openclaw.md`、`evidence-index.md`）做横向比较，**不重新扫描 `docs/references/`**。所有事实声明都回指 evidence-index 中的 ID（`HER-xxx` / `OC-xx` / `CC-xx`）。
>
> 声明强度约定：
>
> - **Directly observed**：evidence-index 中 Confidence ∈ {High, Medium} 的 claim，或三个分析文件正文已直接叙述。
> - **Inferred**：基于 observed 证据合成的跨项目判断（矩阵里的「对比差异」多属此类）。
> - **Recommended for Octopus**：面向本仓架构（Rust core + schema-first + M0/M1/M2 发布边界）的设计建议。每条推荐都显式引用支撑它的 evidence ID 组合，但推荐本身不是 observed 事实。
>
> 三个参考项目形态差异极大，矩阵只抽取「对 Octopus 设计有映射意义」的维度；其他差异（UI 实现、具体通道名单等）不在本表内。

---

## 0. 项目一句话定位（Directly observed）

| 项目 | 语言 / 形态 | 信任模型 | Evidence |
|---|---|---|---|
| **Hermes Agent** | 单一 Python 包，多前端共享 `AIAgent` 内核（CLI / TUI / Gateway / ACP / Cron / MCP‑serve / Batch / RL） | 单用户多会话；Gateway 自保护；容器型 env 默认跳审批 | HER-001, HER-041, HER-051 |
| **OpenClaw** | TypeScript 单进程 Gateway + 嵌入式 `pi-agent-core`；Mac/iOS/Android/headless 客户端作为 WS Node 接入 | 明确「单运营者个人助手」、不是多租户安全边界 | OC-01, OC-02, OC-06 |
| **Claude Code** | 单 Node/Ink 进程，REPL/SDK/coordinator/daemon/bridge/env-runner 等多入口共享 `Tool`/`query` 机器 | 构建期 `feature()` DCE + 运行期 GrowthBook/Statsig + `USER_TYPE === 'ant'` 双层 gating | CC-01, CC-36 |

关键 **Inferred** 差异：

- Hermes 用「一个 AIAgent + Toolset 差异化」承载多表面；OpenClaw 用「一个 Gateway 协议 + 角色/能力声明」承载多客户端；Claude Code 用「一个 bootstrap 快速分发 + 一个 Tool 契约」承载多模式。三者共同点是**单二进制 / 单进程，但把差异下推到配置、声明、与运行期 gating**（HER-001, OC-03, CC-01）。

---

## 1. 总览矩阵（Directly observed 为主）

| 维度 | Hermes Agent | OpenClaw | Claude Code | Evidence |
|---|---|---|---|---|
| 进程模型 | 单进程 / 多表面；Gateway 内每会话 LRU `AIAgent` | 单 Gateway 守护进程 + 嵌入式 agent core；per-session + global lane | 单进程，多 entrypoint 路由；coordinator workers 同机器 | HER-028 / OC-02, OC-11 / CC-01, CC-12 |
| 主传输 | CLI=stdin；TUI=stdio JSON-RPC；Gateway=平台适配器；ACP=stdio | 单一 WS（多路复用 WS+HTTP+Canvas）| CLI/REPL（Ink）+ SDK + 各 MCP transport | HER-045 / OC-03 / CC-19 |
| 协议类型源 | Python 手写；消息统一 OpenAI chat schema | TypeBox → JSON Schema → Swift 代码生成 | 内部 Zod schema；对外 SDK `controlSchemas` + `coreSchemas` | HER-004 / OC-04 / CC-02 |
| 事件语义 | 同步循环 + 线程级中断；工具返回 JSON 串 | `req/res/event` 三类帧；**事件不回放**，断连需 snapshot | async generator stream events + tool-use summaries | HER-004, HER-013 / OC-05 / CC-06 |
| Agent 运行内核 | 自研 `AIAgent.run_conversation` 同步循环 | 嵌入 `pi-agent-core`；OpenClaw 只负责 session/channel/discovery | 自研 `query → queryLoop` async generator | HER-004 / OC-06 / CC-06 |
| 工具抽象 | `ToolEntry`：schema + handler + check_fn + requires_env；必返回 JSON 字符串 | 插件通过 `api.register<Capability>(...)` 注册；12 种公开 capability | `Tool` 接口同时含 schema + render + permission + concurrency 标志 | HER-005, HER-010 / OC-16, OC-19, OC-21 / CC-02, CC-03 |
| 动态工具面 | `get_tool_definitions` 每轮重算；`execute_code/discord_server/browser_navigate` 动态改 schema | Channel 插件 `describeMessageTool` 贡献 action 分支；provider 40+ hook | `assembleToolPool` + 名字稳定排序保护 prompt cache | HER-009 / OC-21, OC-19 / CC-04 |
| Agent 级工具（主循环拦截）| `{todo, memory, session_search, delegate_task}` sentinel | `sessions_*` / `subagents` / `agents_list` 由 core 持有 | AgentTool / TaskStop / SendMessage（coordinator 专用） | HER-008 / OC-27, OC-28 / CC-12 |
| Toolset / 档位 | `toolsets.py` 领域/场景/平台三层 + `_HERMES_CORE_TOOLS` | `tools.allow/deny/profile/byProvider/group:*`，deny 压倒 allow | `TOOL_PRESETS=['default']`；靠 deny rule + feature flag 裁剪；`CLAUDE_CODE_SIMPLE` bare mode | HER-007 / OC-20 / CC-05, CC-17 |
| 权限模式 | `off / interactive / smart(Aux LLM) / cron_mode` | `tools.elevated` + exec approvals（绑定 argv/cwd/文件操作数）| 6 模式：`default/plan/acceptEdits/bypassPermissions/dontAsk/auto(ant)` | HER-040 / OC-24 / CC-13 |
| 沙箱 | Terminal env 六后端（local/docker/ssh/modal/daytona/singularity）同一 `_wrap_command+snapshot+CWD marker` | 三旋钮 `mode × scope × backend`（docker/ssh/openshell），另有 `workspaceAccess` | 外部 `@anthropic-ai/sandbox-runtime` + `shouldUseSandbox`；`excludedCommands` 仅为 UX | HER-011 / OC-23 / CC-18 |
| 沙箱⇄审批耦合 | 容器型 env 早退跳审批 | 沙箱 × exec approvals 三闸门 `tools.elevated` 是后门 | 沙箱与 permission 解耦，permission 是控制点 | HER-041 / OC-23, OC-24 / CC-18 |
| 子 Agent | `delegate_task`：blocklist `{delegate,clarify,memory,send_message,execute_code}`，depth ≤3，并发 ≤3 | `sessions_spawn`：独立 `subagent` lane，`isolated/fork` 上下文，`inherit/require` 沙箱，完成 announce | AgentTool → subagent context（`setAppState` no-op、`renderedSystemPrompt` 冻结共享 prompt cache） | HER-014 / OC-27 / CC-08 |
| 会话持久化 | 单 WAL SQLite：sessions + messages + FTS5 + state_meta；`_execute_write` BEGIN IMMEDIATE + 抖动重试 | 无中央 DB：append-only JSONL + 配置 JSON5 + 文件锁 | 多源持久化：settings 分层 + 权限规则源 + task output 文件 + sidechain transcript + append-only hook events | HER-020, HER-021, HER-022 / OC-09, OC-30 / CC-14, CC-30 |
| 记忆模型 | 双层：Builtin `MEMORY.md/USER.md`（§ 分隔、字符数限制） + 至多 1 个外部 provider | Markdown 文件即记忆（`MEMORY.md / memory/YYYY-MM-DD.md / DREAMS.md`）；`plugins.slots.memory` 独占槽 | 四类 memory 枚举 `user/feedback/project/reference`，各自 scope | HER-016, HER-018 / OC-14, OC-33 / CC-31 |
| 上下文压缩 | 辅助 LLM 摘要，保头尾预算，预裁旧 tool 输出，`_SUMMARY_RATIO=0.20` | Context Engine 四 hook `ingest/assemble/compact/afterTurn`；`ownsCompaction` 可独占或委托 | `query.ts` 固定序：tool-result-budget → snip → microcompact → collapse → autocompact | HER-026 / OC-13, OC-15 / CC-06, CC-32 |
| Prompt caching 策略 | Anthropic `system_and_3`：system + 最近 3 条非 system；**session 中绝不改 system/toolset/memory** | 未在 evidence 内直接描述（文件级压缩保留 assistant/toolResult 对齐） | 跨 fork 共享 `renderedSystemPrompt`；built-ins 在 MCP 之前稳定排序；server-side global-system-caching | HER-027 / OC-13（拆分点对齐）/ CC-04, CC-08 |
| Tool-result 预算 | Registry 注册时声明 `max_result_size_chars` | 未在 evidence 内显式单独列（隐含在 transcript 写盘） | `maxResultSizeChars` 每工具单独设置；溢出落盘 + `ContentReplacementState` 记账，`Infinity` 用于 Read 等自限工具 | HER-005（声明）/ —— / CC-33 |
| MCP | 双向：入站 stdio+HTTP，每服务器一个 Task；反向 `mcp serve` 暴露 9+ 工具 | 暂未作为专章（channel/provider 插件覆盖能力组合） | 7 transport + 7 scope + XAA(SEP-990) + agent-scoped 继承 + 元素级 elicitation（-32042） | HER-042 / —— / CC-19, CC-20, CC-21 |
| 扩展来源 | 4 源：仓内 `plugins/`、`~/.hermes/plugins/`、项目 `./.hermes/plugins/`、pip entry point `hermes_agent.plugins` | manifest-first / lazy runtime：discover → validate → jiti load → registry；`plugins.slots` 独占 | 独立 loader：built-in agents / custom / plugin / skills(6 kinds) / hooks(4 sources) / MCP(7 scopes) | HER-033 / OC-16, OC-17 / CC-38 |
| Hook 事件 | `VALID_HOOKS` 13 项（pre/post tool、pre/post llm、pre/post api、session 五件套、subagent_stop、transform_*）+ gateway 独立 hooks.py | 两套：Internal `HOOK.md+handler.ts` + Plugin hook（嵌 Agent 循环、40+ typed 事件）| Zod union 观察到 11 项（PreToolUse/PostToolUse/PostToolUseFailure/UserPromptSubmit/SessionStart/Setup/SubagentStart/PermissionDenied/Notification/PermissionRequest/Elicitation），外加 agent SDK 的 `HOOK_EVENTS` | HER-034, HER-036 / OC-29 / CC-22, CC-23, CC-25 |
| Skills 系统 | 内建 `skills/` + 显式安装 `optional-skills/`；frontmatter `metadata.hermes`；作为 **user message** 注入以保 prompt caching；模板 `${HERMES_SKILL_DIR}`、`${HERMES_SESSION_ID}`、内联 shell `` !`…` `` 上限 4000 | `SKILL.md + frontmatter` Markdown；六级加载优先级；per-agent allowlist；ClawHub 市场 | `loadSkillsDir` 6 源 `commands_DEPRECATED/skills/plugin/managed/bundled/mcp`；`SkillTool` 一等；skill discovery 在 query 内预取 | HER-037, HER-038 / OC-22 / CC-26 |
| 插件信任面 | 「插件不得修改核心文件」；需要能力必须扩 hook/ctx 接口 | 原生插件与 Gateway **同信任域**；compatible bundle 被视为元数据/内容包 | 插件来源划分 admin-trusted vs user-controlled；`strictPluginOnlyCustomization` 可阻止 frontmatter MCP 来自用户 | HER-035 / OC-18 / CC-20, CC-27 |
| Slash 命令 | `COMMAND_REGISTRY` 单一源派生 CLI/Gateway/Telegram/Slack/autocomplete/help | `sessions_list/history/send/yield/spawn` 是显式跨会话工具族；有 `/compact`；斜杠命令路径与 session-tool 路径并存 | 未作为独立章节（命令由 plugin/skill/hook 注入） | HER-032 / OC-28 / CC-38 |
| 配置 | `~/.hermes/config.yaml` 设置 + `~/.hermes/.env` 仅密钥；三 loader 并存；`_config_version` 仅在结构迁移升 | JSON5 `openclaw.json`；严格模式，未知 key 拒绝启动，保 last-known-good 自恢复；workspace `.env` 不能覆盖 channel endpoint | 设置分层 `userSettings < projectSettings < localSettings < flagSettings < policySettings`；permission-rule 源再叠加 `cliArg/command/session` | HER-050 / OC-31 / CC-14 |
| 跨进程并发 | SQLite WAL + `BEGIN IMMEDIATE` + 抖动重试；cron `.tick.lock`；platform lock / session lock | Gateway 单进程内：per-session lane + global lane + queue modes；文件级 session write lock 跨进程感知 | 进程内 async generator 顺序；`PermissionContext` 用 resolve-once 处理并发 resolution | HER-021, HER-046 / OC-11 / CC-06 |
| 定时任务 / 后台 | `cron/scheduler.py`：文件锁、并发 executor、wake-gate 脚本、`[SILENT]` 静音、基于 inactivity 超时 | （文档未作为独立章节；通过 sandbox backend + sessions_spawn 覆盖长任务） | `Task` 7 类：`local_bash/local_agent/remote_agent/in_process_teammate/local_workflow/monitor_mcp/dream`；stdout 落盘 + `outputOffset` 增量尾追 | HER-046, HER-047 / —— / CC-29, CC-30 |
| 身份/路由 | Gateway 的 `SessionSource`（平台 + chat/user）；双 guard（adapter 级 `_pending_messages` + runner 级命令拦截）必须同步绕过 | 三元 `agent/accountId/binding`；8 级路由优先级；`dmPolicy` + `requireMention` + group allowlist；`contextVisibility` 与触发授权正交 | 无等价（本地 REPL 为主） | HER-029, HER-031 / OC-08, OC-26, OC-32 / —— |

---

## 2. 分项深入（Directly observed vs. Inferred）

### 2.1 执行表面 & 入口分发

- **Observed**
  - Hermes：六大「超大文件」（`run_agent.py` 12k+ 行、`cli.py` 11k+、`gateway/run.py` 11k+）承载多表面；AGENTS.md 自承「不要把目录树当圣经」（HER-001, HER-003）。
  - OpenClaw：同一 WS + HTTP 端口（默认 `127.0.0.1:18789`），首帧必须 `connect`，用 role + scope + caps 区分客户端（OC-02, OC-03）。
  - Claude Code：`cli.tsx` 在加载 REPL 前做 argv 快速分发（version/daemon/bridge/tasks/env-runner/self-hosted/Chrome MCP/Computer-Use MCP）（CC-01）。
- **Inferred**
  - 三家都把「多入口 → 内核」的耦合点做得尽可能薄：Hermes 用一个 `AIAgent` 类；OpenClaw 用一套 WS 协议 + role 声明；Claude Code 用一个 bootstrap 分支表。**差异在于抽象层级**：Hermes 在 Python 对象上，OpenClaw 在线协议上，Claude Code 在入参分支上。
  - 「快速分发 + 完整加载」的分层启动是减少冷启动延迟的通用手段；OpenClaw 显式把它上升到原则「manifest-first / lazy runtime」（OC-17）。

### 2.2 Agent 内核边界

- **Observed**
  - Hermes：同步 tool-calling 循环，`max_iterations + iteration_budget + grace call + _interrupt_requested`（HER-004）。
  - OpenClaw：`agent` RPC 返回 `{runId, acceptedAt}` 后异步进入 `agentCommand → runEmbeddedPiAgent`，事件桥接到 `agent` 流（OC-06）。
  - Claude Code：async generator `queryLoop`，状态含 `autoCompactTracking / recoveryCount / hasAttemptedReactiveCompact / pendingToolUseSummary / stopHookActive / transition`（CC-06）。
- **Inferred**
  - OpenClaw 把 agent 内核**外包**给 pi-agent-core，自己做「协议 + 会话 + 通道」；Hermes 和 Claude Code 都**自研**内核。这是本质差异：OpenClaw 的其他章节（channels、queue、subagent、session 工具）更像**稳态控制面**，不像「推理引擎」。
  - 三家都有「iterations budget + 中断」机制，但实现风格不同：Hermes 显式预算 + 抢占；OpenClaw 通过 lane 串行化（queue mode 决定 steering）；Claude Code 用 transition state 表达 reactive compact 与 microcompact 的触发点。

### 2.3 工具抽象

- **Observed**
  - Hermes：`ToolEntry(name, toolset, schema, handler, check_fn, requires_env, is_async, description, emoji, max_result_size_chars)`；handler 必须返回 JSON 字符串；`coerce_tool_args` 对 LLM 字符串化输出做类型还原（HER-005, HER-010）。
  - OpenClaw：插件按 capability 合约注册；`message` 工具由 core 持有，channel 插件以 `describeMessageTool` 返回 action/schema 片段（OC-16, OC-21）。
  - Claude Code：`Tool` 接口同时带 schema、permission、concurrency 标志、UI render（React Node）、自动分类器输入、MCP 指纹；`buildTool` 统一 fail-closed 默认值（CC-02, CC-03）。
- **Inferred**
  - **UI 与 Tool 的耦合程度** 差异极大：Claude Code 直接把 `React.ReactNode` 放进 Tool 接口，非 Ink 消费者必须自己重实现渲染（CC-37）。Hermes / OpenClaw 把「工具语义」与「UI 呈现」解耦。对多端产品而言，CC 的耦合是债务。
  - **语言强类型 vs 运行时约定**：Claude Code (Zod) 与 OpenClaw (TypeBox → JSON Schema → Swift) 都走 schema-first；Hermes 在运行期 `coerce_tool_args` 修补 LLM 输出。schema-first 更容易出 SDK，运行期 coerce 更容易活下去。

### 2.4 权限 / 审批 / 沙箱

- **Observed**
  - Hermes：`check_all_command_guards` 唯一入口；模式库 `DANGEROUS_PATTERNS` 含 `rm/chmod/mkfs/dd/SQL DESTRUCTIVE/fork bomb/curl|sh/heredoc-exec/git destructive/ssh/env exfil/gateway 自杀` 等；输入先 `strip_ansi + NFKC`；容器型 env 早退；smart 模式调辅助 LLM `APPROVE/DENY/ESCALATE`；Gateway 用队列 + Event；`always` 持久化到 `config.yaml` 的 `command_allowlist`，tirith 命中禁 `always`（HER-039, HER-040, HER-041）。
  - OpenClaw：三旋钮 `tool policy × sandbox × exec approvals`；审批绑定 argv/cwd/file operand，不能精确绑定则拒绝；`tools.elevated` 是显式后门（OC-20, OC-23, OC-24）。
  - Claude Code：6 permission mode；rule 源 `SETTING_SOURCES + cliArg + command + session`；deny-rule 在模型看到工具**之前**就剥离，含 `mcp__server` 前缀通配；`PermissionContext` 里提供 `awaitAutomatedChecksBeforeDialog` 供 coordinator workers 用；sandbox 委托 `@anthropic-ai/sandbox-runtime`，`excludedCommands` 明确不是安全边界（CC-13, CC-14, CC-15, CC-17, CC-18）。
- **Inferred**
  - 三家都**不认为「沙箱」单独够用**：Hermes 用模式库补漏；OpenClaw 用 exec approvals 绑定；Claude Code 用 permission rule + hook 链。
  - **审批持久化粒度** 不同：Hermes 有 `once / session / always` 三级；OpenClaw exec approvals 按 argv/cwd 指纹；Claude Code 把规则挂在 settings 分层里。**Octopus 若要统一事件重放（见 ADR 0005），审批决策需要是可序列化对象，不能只藏在进程内字典里**。
  - **沙箱跳过审批** 的策略有争议：Hermes 明确在 docker/singularity/modal/daytona 下跳审批（HER-041），OpenClaw 保留 exec approvals 作为独立层（OC-24）。前者更简单，后者更防御。

### 2.5 子 Agent / 多 Agent

- **Observed**
  - Hermes：`DELEGATE_BLOCKED_TOOLS = {delegate_task, clarify, memory, send_message, execute_code}`，`MAX_DEPTH=1`，并发上限 3；子 agent 的 toolset 自动剔除被 block 的集合（HER-014）。
  - OpenClaw：`sessions_spawn` 独立 lane，context `isolated/fork`，sandbox `inherit/require`；完成 announce 回父通道；thread-bound session 支持把子 agent 钉到帖子（OC-27）。
  - Claude Code：coordinator prompt 只允许 AgentTool/TaskStop/SendMessage；worker 结果以 user-role `<task-notification>` XML 注入；worker tool 集 = `ASYNC_AGENT_ALLOWED_TOOLS` − 内部工具；`createSubagentContext` 里 `setAppState` no-op、`localDenialTracking` 保留、`renderedSystemPrompt` 冻结以共享 prompt cache（CC-08, CC-12）。
- **Inferred**
  - 三家对「子 Agent 能不能对用户说话」答案都是**不能直接**：Hermes `clarify/send_message` 被禁，OpenClaw 通过 announce 回父，Claude Code 通过 `<task-notification>` XML。都在显式防御「子 agent 伪装用户」。
  - 「prompt cache 共享」只在 Claude Code 与 Hermes 被显式建模（CC-08 冻结 `renderedSystemPrompt`、HER-027 session 中段禁改 system）。OpenClaw evidence 未覆盖（非 observed）。

### 2.6 记忆模型

- **Observed**
  - Hermes：Builtin `MEMORY.md/USER.md` + 至多一个外部 provider；召回文本包 `<memory-context>`，`sanitize_context` 剥上轮注入的 fence；写入前 `_MEMORY_THREAT_PATTERNS` 检测注入/exfil/ssh 后门；session 内写盘但**不动系统提示**（HER-016, HER-017, HER-018, HER-019）。
  - OpenClaw：Markdown 文件即记忆；「dreaming」后台巩固，必须过分数、频次、多样性门槛才能升入 `MEMORY.md`；`memory/contextEngine` 是独占 slot（OC-14, OC-33, OC-38）。
  - Claude Code：枚举化 memory 四类（user/feedback/project/reference），每类声明 `<scope>` = private/team/条件；agent-level memory 支持 `user|project|local`（CC-31）。
- **Inferred**
  - 三家都选择**文件而非数据库**作为主记忆载体（OpenClaw 最纯粹，Claude Code 有 memdir，Hermes 走 Markdown + SQLite 消息检索）。
  - **外部 provider 的合规面** 只有 Hermes 显式建模（`sanitize_context` + 威胁扫描），OpenClaw 靠「插件同信任」简化，Claude Code 靠 scope 分类。对 Octopus 的 Memory Trust（ADR 0006）方向，Hermes 的做法更接近。

### 2.7 上下文压缩 & prompt caching

- **Observed**
  - Hermes：摘要前先廉价预裁旧 tool 输出；模板含 `handoff` / `Do NOT answer questions from this summary` / `resume from ## Active Task`；Anthropic `system_and_3` 缓存策略；**session 中绝不改 system/toolset/memory**；压缩链 `parent.end_reason='compression'` + 子 session 构成继续（HER-023, HER-026, HER-027）。
  - OpenClaw：Context Engine 四 hook `ingest/assemble/compact/afterTurn` + 可选 subagent hooks；`ownsCompaction` 可独占或委托；拆分点保持 assistant tool_call ↔ toolResult 配对；transcript 原样保留，压缩只影响下次喂给模型的视图（OC-13, OC-15）。
  - Claude Code：`query.ts` 固定序 `tool-result-budget → snip → microcompact → collapse → autocompact`；tool 级 `maxResultSizeChars`（`Infinity` = Read 这类自限工具）；`ContentReplacementState` 记录溢出文件；`task_budget = {total, remaining}` 跨 compaction 必须重发（CC-06, CC-32, CC-33, CC-34）。
- **Inferred**
  - 三家都把压缩做成 pipeline，而不是单 prompt 摘要；但**抽象单位不同**：Hermes 是 compressor（函数）、OpenClaw 是 Engine（插槽）、Claude Code 是 service 组合（多文件）。
  - 「prompt cache 稳定性」是贯穿约束：Hermes 显式写进 AGENTS.md（HER-027），Claude Code 在 `assembleToolPool` 和 `renderedSystemPrompt` 两处强调（CC-04, CC-08）；OpenClaw evidence 未直接覆盖。**这意味着任何「session 中期修改系统面」的特性（热插 skills / 热切 toolset / 热改 memory）都需要谨慎设计回退点**。

### 2.8 扩展性：插件 / MCP / Skills / Hooks

- **Observed**
  - 扩展来源：Hermes 4 源、Claude Code 独立 loader（built-in / custom / plugin / skills-6-kinds / hooks-4-sources / MCP-7-scopes）、OpenClaw 四步加载管线 + `plugins.slots`（HER-033, HER-034, HER-036, HER-037, HER-042 / OC-16, OC-17, OC-18, OC-22, OC-29 / CC-19, CC-20, CC-22, CC-23, CC-25, CC-26, CC-27, CC-28, CC-38）。
  - 信任面：Hermes「插件不得修改核心」（HER-035）；OpenClaw「原生插件 = 任意代码执行」（OC-18）；Claude Code admin-trusted vs user-controlled 划分（CC-20）。
- **Inferred**
  - 「Hook 事件空间」三家重叠度高：`pre/post tool`、`pre/post llm/api`、`session_*`、`subagent_*` 都在。差异在**语义丰富度**：Claude Code 的 `PreToolUse` 可返回 `updatedInput / additionalContext / permissionDecision`，即 hook 能**改写工具输入与权限决策**（CC-23）。Hermes 的 `pre_tool_call` 返回 block message 即拒绝（HER-034）。OpenClaw 有 `block: false = no-op` 的显式优先级规则（OC-29）。
  - **插件与核心进程的信任域** 是 OpenClaw 与 Claude Code 分歧点：OpenClaw 同信任（OC-18），Claude Code 在 admin/user 二分并限制 frontmatter MCP（CC-20）。Hermes 的硬规矩「不改核心文件」接近 admin-trusted 单一信任面。

### 2.9 会话持久化与检索

- **Observed**
  - Hermes：单 WAL SQLite，`_execute_write` BEGIN IMMEDIATE + 20–150ms 抖动重试，每 50 次写后 PASSIVE checkpoint；FTS5 + `_sanitize_fts5_query` 处理保留字/非法括号/CJK fallback；压缩链 + `list_sessions_rich(project_compression_tips=True)` 投射「连续对话 = 一行」（HER-020, HER-021, HER-022, HER-023, HER-024）。
  - OpenClaw：append-only JSONL transcript；无中央 DB；进程感知文件级 session write lock（OC-09, OC-30）。
  - Claude Code：task stdout 落 `getTaskOutputPath(id)` + `outputOffset` 增量追尾；`ContentReplacementState` 记录溢出；sidechain transcript + append-only hook events 作为各自独立存储（CC-30, CC-33）。
- **Inferred**
  - 「全文检索 vs 顺序重放」是核心取舍：Hermes 拿 FTS5 换了检索体验（HER-022）；OpenClaw / Claude Code 让「文件/JSONL」自己承担重放性。**对 Octopus 的 Event Store + Projection + Replay（ADR 0005）方向，Hermes 的 SQLite 模型是一个强映射参考**。
  - Hermes `parent.end_reason='compression' + child.started_at >= parent.ended_at` 的血缘建模（HER-023）是一个可复用的**压缩 = 新版本**语义，比「覆盖 transcript」更适合 replay。

### 2.10 配置、密钥与隔离

- **Observed**
  - Hermes：`~/.hermes/config.yaml`（设置） + `~/.hermes/.env`（仅密钥）；三 loader 并存（`load_cli_config / load_config / gateway YAML 直读`）；`_config_version` 仅结构迁移升（HER-050）。
  - OpenClaw：`~/.openclaw/openclaw.json`（JSON5），严格模式：未知 key / 非法值拒绝启动；保 last-known-good 自恢复；workspace `.env` 不能覆盖 `OPENCLAW_*` 或 channel endpoint key（OC-30, OC-31）。
  - Claude Code：settings 分层 `userSettings < projectSettings < localSettings < flagSettings < policySettings`；permission-rule 源在 setting 源之上叠加 `cliArg + command + session`（CC-14）。
- **Inferred**
  - 「密钥与设置物理分离」是 Hermes 的原则；OpenClaw 用「严格校验 + .env 屏蔽」，Claude Code 用「显式分层」。**Octopus 的 Secret Boundary（ADR 0004）应该对齐 Hermes 的分离原则，并采用 OpenClaw 的严格校验（未知 key 拒启动）以减少漂移**。

### 2.11 定时/后台任务与多 Agent 路由

- **Observed**
  - Hermes：cron 文件锁 `.tick.lock`、`ThreadPoolExecutor + copy_context`、wake-gate JSON、`[SILENT]` 静音、基于 inactivity 超时；cron 工具集决议顺序 `job > 平台 hermes tools > full default`（HER-046, HER-047）。
  - OpenClaw：路由 8 级（peer / parentPeer / guildId+roles / guildId / teamId / accountId / 通道通配 / 默认 agent）；`agent/accountId/binding` 三元；`dmPolicy` + `contextVisibility` 正交（OC-08, OC-10, OC-26）。
  - Claude Code：`Task` 七类（local_bash/local_agent/remote_agent/in_process_teammate/local_workflow/monitor_mcp/dream）；Task ID = 类型前缀 + 8 随机字符；只有 `kill` 是多态分发（CC-29, CC-30）。
- **Inferred**
  - Hermes 的 cron / Claude Code 的 Task / OpenClaw 的 session lane 同构：**都在解「主循环之外的、可恢复、可观测的后台执行单元」**。对 Octopus 若需要支持长运行 / 批量，Task 的「类型枚举 + 统一 stdout 落盘 + offset 尾追」最结构化（CC-30）。

---

## 3. Recommended for Octopus（每条引 evidence ID）

> 以下推荐均面向 Octopus 目前已声明的方向（Rust core + schema-first contracts + M0/M1/M2 发布边界 + event-store/projection/replay + memory trust + secret boundary）。每条推荐都明确标出来源证据与推荐原因；**推荐本身不是 observed 事实**。

### R-01 [Agent Runtime] 单内核 + 薄多表面

- **建议**：Octopus 应只实现一个 agent 运行内核（Rust），所有表面（CLI / TUI / Gateway / MCP-serve / IDE/ACP）通过「协议适配 + Toolset 差异」接入同一内核，不维护并行内核。
- **支撑证据**：HER-001（Hermes 一核多面）、OC-06（OpenClaw 显式把 pi-agent-core 抽出）、CC-01（Claude Code 在 bootstrap 做快速分发但核心同源）。
- **反面证据**：OpenClaw 把内核外包给 pi-agent-core 说明「只做控制面」也可行（OC-06），但这与「schema-first Rust core」方向冲突，故选择对齐 Hermes/Claude Code。

### R-02 [Protocol] Schema-first 契约用于跨进程/跨端

- **建议**：所有跨进程表面（Gateway ↔ 客户端、MCP、ACP、Plugin host）用 schema 派生 SDK（Rust 为源，生成 TS/Swift/Kotlin），不手写序列化。
- **支撑证据**：OC-04（TypeBox → JSON Schema → Swift）、CC-02/CC-03（Zod schema + buildTool fail-closed 默认）、HER-010（运行期 coerce 是修补而非契约）。
- **注意**：Octopus 的 `contracts/openapi/` 目录暗示此方向已选定；本推荐是把它**扩大**到所有协议层，而不仅是 REST。

### R-03 [Tool Contract] Tool 契约与 UI 呈现解耦

- **建议**：Tool 定义仅含 schema + handler + 分类元数据（`is_concurrency_safe / is_readonly / is_destructive / requires_env`）+ 大小预算（`max_result_size_chars`）；**不得**把渲染函数（React / Ink）塞进 Tool 类型。客户端自渲染。
- **支撑证据**：CC-02/CC-37（Claude Code 直接返回 React.ReactNode 导致非 Ink 消费者必须重实现）、HER-005（Hermes 只返回 JSON 字符串 + emoji/description 元数据）、OC-21（OpenClaw `MessagePresentation` 抽象合约）。
- **理由**：Octopus 将有 CLI + IDE + 可能的 Web 表面；CC 方式是技术债。

### R-04 [Tool Pool] Prompt-cache-stable 的 tool 汇编

- **建议**：Tool 池组装顺序必须确定且稳定（built-ins 先、MCP 后、按名字排序），deny-rule 在模型看到之前就剥离，避免 session 中段 tool schema 漂移。
- **支撑证据**：CC-04（`assembleToolPool` + 稳定排序 + dedup）、CC-17（deny-rule 前置）、HER-027（session 中不改 toolset）、HER-009（动态 schema 后处理是例外，需每轮重算）。
- **与 Octopus 事件存储的关系**：tool 漂移会使 replay 结果发散，稳定组装是 replay 可重放性的前提（ADR 0005）。

### R-05 [Permission] 审批决策必须是可序列化事件

- **建议**：所有权限/审批决策以「事件」形式进入 Event Store，而非仅活在进程内字典；决策键应绑定 argv 指纹（或 tool + 规范化 input hash）以支持 replay。
- **支撑证据**：OC-24（exec approvals 绑定 argv/cwd/file operand，无法精确绑定则拒绝）、HER-040（`once/session/always` 三级 + `command_allowlist` 持久化）、CC-14/CC-15（settings 分层 + `shouldAvoidPermissionPrompts / awaitAutomatedChecksBeforeDialog`）。
- **反例**：Hermes 的 `_session_approved` 是进程内字典，Gateway 重启会丢（HER-040），对事件重放不友好。

### R-06 [Sandbox] 沙箱与审批正交；沙箱不得成为审批的代替

- **建议**：Octopus M0 沙箱（ADR 0009）应保留审批链独立：即使在沙箱内，破坏性操作（`git push -f`、`rm -rf`、密钥文件访问）仍需显式审批。
- **支撑证据**：OC-23/OC-24（三旋钮 + exec approvals 独立）、CC-18（`excludedCommands` 显式声明不是安全边界）、HER-041（Hermes 在容器 env 下早退跳审批 — **反面，不建议照搬**）。
- **理由**：Hermes 的「容器 ⇒ 免审批」在多用户或供应链污染场景是风险面。

### R-07 [Sub-agent] 子 Agent 硬边界 + 显式 announce

- **建议**：子 Agent 的 toolset 必须从父集显式裁剪（blocklist + 白名单），子 Agent **不能直接向用户说话**，结果以结构化消息回父 session；限制并发数 + 深度。
- **支撑证据**：HER-014（blocklist `{delegate_task, clarify, memory, send_message, execute_code}`、depth ≤3、并发 ≤3）、OC-27（announce + `isolated/fork` 上下文、`inherit/require` 沙箱）、CC-12（coordinator XML `<task-notification>` + `ASYNC_AGENT_ALLOWED_TOOLS`）。

### R-08 [Session] 事件源持久化 + 压缩作为新版本

- **建议**：session 用「append-only 事件日志 + 快照 + compaction = 新 session 指针」建模；避免在原 transcript 上原地重写。FTS/向量索引作为 **projection** 派生，而非主数据。
- **支撑证据**：HER-020/HER-023（SQLite + `parent.end_reason='compression'` + 子 session tip 投射）、OC-09/OC-30（JSONL append-only、文件级 session lock、无中央 DB）、CC-30/CC-32（task stdout 分段落盘 + `ContentReplacementState` 溢出记账 + 压缩 pipeline 有序）。
- **与 ADR 0005 关系**：这条是 ADR 0005 的直接来源。推荐把 Hermes 的压缩血缘语义 + Claude Code 的 budget/offset 记账合并。

### R-09 [Memory Trust] 外部 memory provider 必须隔栏 + 内容扫描

- **建议**：任何外部 memory（向量库 / 第三方 provider / 爬虫抽取）的召回文本必须被「上下文栅栏」包裹（如 `<external-untrusted>…</external-untrusted>`），并在写入前做 prompt injection / exfiltration / 凭据残留扫描。独占 slot：最多一个外部 provider。
- **支撑证据**：HER-016/HER-017/HER-019（Hermes 外部 provider 独占 + `<memory-context>` fence + `sanitize_context` + `_MEMORY_THREAT_PATTERNS`）、OC-33/OC-34（`plugins.slots` 独占 + external content 用 `<<<EXTERNAL_UNTRUSTED_CONTENT…>>>` 包裹）、CC-31（memory 分 scope）。
- **与 ADR 0006 关系**：这是 Memory Trust 的三项最低门槛。

### R-10 [Prompt Cache] Session 中段禁改系统面

- **建议**：把「session 运行期间不修改系统提示、toolset、memory」作为**硬约束**写进架构文档；需要修改必须等到下一 session 或走显式重启/fork。
- **支撑证据**：HER-027（Hermes 在 AGENTS.md 里明确 prompt cache 不能破）、CC-08（Claude Code 冻结 `renderedSystemPrompt` 供 subagent 共享缓存）。OpenClaw evidence 未覆盖，故本条依据主要为 Hermes + CC。
- **适配 Octopus**：Rust core 需要把这一约束编译到接口类型（如 `SessionHandle` 里 `set_system_prompt` 仅在创建期可调用）。

### R-11 [Plugin Trust] 明确 admin-trusted vs user-controlled 边界

- **建议**：插件信任域按来源分两级：**admin-trusted**（仓内 / policySettings / `managed`）可携带 MCP / hooks / agents 全能力；**user-controlled**（项目本地 / 用户目录）默认禁用 frontmatter-MCP 与 native hooks。
- **支撑证据**：CC-20/CC-27（strictPluginOnlyCustomization + admin-trusted 二分）、OC-18（OpenClaw 同信任 — **反面**）、HER-035（Hermes「不改核心」规则接近 admin-trusted 单层）。
- **理由**：OpenClaw 的同信任模型适合单运营者（OC-01），但 Octopus 如目标多租户或团队使用，需要 CC 的二分。

### R-12 [Hook System] Hook 能改写 input + 返回 permission decision

- **建议**：`pre_tool_use` hook 必须能返回 `updated_input / additional_context / permission_decision / stop_reason / system_message`，而不仅返回 "block/continue" 布尔。
- **支撑证据**：CC-23（Claude Code `PreToolUse` 可改写 `updatedInput / additionalContext / permissionDecision`）、HER-034（Hermes 支持 `transform_tool_result`）、OC-29（OpenClaw `before_tool_call` / `block: false = no-op` 优先级语义）。
- **理由**：富语义 hook 是扩展性的上限；布尔 hook 上限太低。

### R-13 [Config] 严格校验 + 密钥与设置物理分离 + 分层叠加

- **建议**：
  1. 配置文件有 JSON Schema，未知键拒绝启动并保 last-known-good；
  2. 密钥与业务设置分文件存放；
  3. 允许分层叠加 `user < project < local < flag < policy`，权限规则额外支持 `cliArg/command/session` 源。
- **支撑证据**：OC-31（严格校验 + last-known-good 自恢复）、HER-050（`config.yaml` ↔ `.env` 分离）、CC-14（settings 分层 + permission 源叠加）。
- **与 ADR 0004 关系**：密钥分离是 Secret Boundary 的最小面。

### R-14 [Skills] Skills 作为 user-message 注入，不碰 system prompt

- **建议**：Skill 激活生成的文本以 **user message**（或等价 non-system 消息）注入，不要改系统提示；Skill 从多源加载时用确定性优先级（workspace > user > bundled）。
- **支撑证据**：HER-037/HER-038（Hermes skill 作为 user message 注入、6 源优先级、模板变量 + 内联 shell 上限）、OC-22（OpenClaw 六级加载优先级 + per-agent allowlist）、CC-26（Claude Code 6 源 + SkillTool 一等 + query 内预取）。
- **理由**：支持 R-10 Prompt Cache 硬约束。

### R-15 [Operational] 跨进程锁与事件不回放的诚实声明

- **建议**：在架构文档里显式记载「事件是否可重发」这类语义决策；使用文件锁（fcntl/msvcrt/advisory lock）串行化单入口工作单元；多写用 `BEGIN IMMEDIATE + 抖动重试`。
- **支撑证据**：HER-021/HER-046（Hermes WAL + cron .tick.lock）、OC-05（OpenClaw 明确「事件不回放，断连走 snapshot」）、OC-30（文件级 session write lock 跨进程感知）。
- **理由**：Octopus event-store 的 replay 语义（ADR 0005）必须同时定义「客户端断连时回放与否」，照搬 OpenClaw 的显式声明可以避免隐含假设。

### R-16 [Naming] Session = 路由键与上下文容器，不是鉴权 token

- **建议**：在 API 设计阶段显式声明 `session_id` 不是 auth token；鉴权由独立机制承担。
- **支撑证据**：OC-09（OpenClaw 文档显式「Trust boundary matrix」）、HER-020（Hermes session 是容器）、CC-14（Claude Code 把 `session` 放在权限规则源之一而非鉴权）。
- **理由**：在 schema-first SDK 中若把 session id 当 token，客户端泄漏风险会放大。

---

## 4. 不可直接比较 / 本次未覆盖

以下项在 evidence-index 里是 **Unverified** 或不构成横向对比点，本矩阵不下结论：

- OpenClaw pi-agent-core 的内部实现与 Hermes/Claude Code 内核的运行期性能对比（OC-35）。
- OpenClaw Canvas Host / A2UI 与 Claude Code Ink UI 的渲染细节（OC-36、CC-37 只指出接口耦合而未展开实现）。
- Claude Code 外部 shipped build 中各 feature flag 的实际开关状态（CC-12, CC-36 标记 Unverified）。
- Hermes TUI `tui_gateway/server.py` 完整方法集（HER-045 Medium，正文未逐一核对）。
- OpenClaw Memory Wiki ↔ Memory Core 的数据一致性（OC-38 Unverified）。
- Hermes 多 provider 适配器完整流程（HER-049 Medium）。

若后续 Octopus 设计需要落到这些未验证点，应在对应 reference-analysis 文件中先做二次取证（按用户指令，本次不主动触发）。

---

## 5. 更新钩点

- 本文与 `docs/architecture/reference-analysis/evidence-index.md` 中的 ID 命名空间保持同步；新增 evidence 应先更新 evidence-index 再回填本矩阵。
- 推荐条 (R-01 … R-16) 若被某条 ADR 采纳或驳回，应在推荐条末尾追加 `→ ADR-xxxx` 引用或 `Rejected: <原因>` 标记。
- `directly observed` 与 `inferred` 的判定以 evidence-index Confidence 为准：Confidence=High → observed；Confidence=Medium 且本矩阵引用其正文章节 → observed（范围内）；其他合成判断归 inferred。
