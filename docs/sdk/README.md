# Agent Harness SDK — 架构与设计规范

> 本目录（`docs/sdk/`）是 Octopus Agent Harness SDK 的**架构源真理**，面向实施者（人类工程师 + AI Coding Agent）。
>
> 本文不是教学文章，而是**规范性文档（normative spec）**：凡涉及实现细节，必附权威来源。读者可沿着链接定位真实实现，避免"幻觉式模仿"。

## 文档索引

| 文档 | 范围 |
|---|---|
| `README.md`（本文） | 总览：哲学、顶层架构、阅读顺序 |
| `01-core-loop.md` | Agent 核心循环（query loop / tool loop） |
| `02-context-engineering.md` | 上下文工程（系统提示词、压缩、Just-in-Time 检索、记忆） |
| `03-tool-system.md` | 工具系统（Tool 契约、并发、权限、MCP、Skills） |
| `04-session-brain-hands.md` | 三分架构：Brain / Hands / Session（Managed Agents 模型） |
| `05-sub-agents.md` | 子代理与多代理编排（Orchestrator-Workers、GAN 式生成器-评估器） |
| `06-permissions-sandbox.md` | 权限模型、审批、沙箱与网络隔离 |
| `07-hooks-lifecycle.md` | Hook 生命周期与确定性守护 |
| `08-long-horizon.md` | 长时任务模式（初始化器代理、进度文件、上下文重置） |
| `09-observability-eval.md` | 可观测性、评测与回放 |
| `10-failure-modes.md` | 生产级失败模式与缓解策略 |
| `11-model-system.md` | 多厂商 / 多模型系统（Provider · Surface · Model 三层 + 角色路由 + 协议适配） |
| `12-plugin-system.md` | 插件体系（Manifest · Registry · Runtime 三层；扩展点、所有权、分发、安全） |
| `13-contracts-map.md` | 契约地图：SDK 章节 ⇄ OpenAPI ⇄ `@octopus/schema` ⇄ `@octopus/ui` ⇄ UI 意图 IR 的双向索引 |
| `14-ui-intent-ir.md` | UI 意图 IR：SDK 层宿主中立的渲染契约（`RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactRef`），融合 Claude.ai 成果物展示与 Claude Code 内联 diff 两种范式 |
| `references.md` | 全部外部参考资料索引（一级来源） |

> 阅读顺序建议：`README → 04 → 11 → 12 → 01 → 02 → 03 → 14 → 06 → 05 → 07 → 08 → 09 → 10 → 13`。`04` 定义**边界**，`11` 定义**模型可替换接口**，`12` 定义**所有扩展点的统一注册/分发契约**；`03 → 14` 连读以理解"工具契约 → UI 意图契约"的跨层衔接；其余章节都在这些边界内展开；`13` 给出 SDK 规范层与磁盘真相源（OpenAPI / Schema / UI / UI 意图 IR）的双向入口。

## 0. 本文档遵循的"第一性原理"

Anthropic 官方在 *Building Effective Agents*（Dec 2024）总结了三条不可妥协原则，本 SDK 直接采纳为宪法：

1. **Maintain simplicity**：保持代理设计的简洁；不要把能用一段代码解决的问题包成框架。
2. **Prioritize transparency**：明确地暴露代理的计划、工具调用、中间状态。
3. **Carefully craft the Agent–Computer Interface (ACI)**：ACI 的工程量应与 HCI 相当，对工具描述/参数/错误信息像对人一样认真打磨。

> 来源：[Anthropic · Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)（2024-12-19）末尾 "Summary" 段。

另外两条来自 *Effective context engineering for AI agents*（Sep 2025）与 *Managed Agents*（2026）的判断，构成"架构级"约束：

4. **Context 是稀缺资源**：Transformer 注意力预算有限、存在 "context rot"。每一个 token 都在消耗这个预算。→ 见 `02-context-engineering.md`。
5. **Harness 编码的假设会随模型进化而失效（staleness）**：因此 harness 必须围绕**稳定接口**设计，允许局部替换而不扰动整体。→ 见 `04-session-brain-hands.md`。

## 1. 什么是 Agent Harness

**定义**（Anthropic 官方）：

> "Agents are LLMs autonomously using tools in a loop."
>
> — [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

**Harness（骨架/外壳）** 是包裹 LLM、让它能"在一个循环里自主使用工具"的**所有非 LLM 代码**：

- 提示词装配与缓存
- 工具注册、调度、并发、权限
- 上下文压缩与 Just-in-Time 加载
- 会话持久化与事件日志
- Hook / 拦截 / 审批
- 子代理编排
- 沙箱与网络策略
- 可观测性与评测

> Anthropic 原话："harnesses encode assumptions about what Claude can't do on its own." —— [Managed Agents](https://www.anthropic.com/engineering/managed-agents)

换言之，**harness 是对模型当前不足的一种补偿**。模型变强之后，补偿部件（例如 Sonnet 4.5 需要的 context-reset）会变成 dead weight，必须可拆。这直接决定本 SDK 的**"可替换接口"**设计原则。

## 2. 顶层架构（Brain / Hands / Session）

受 Anthropic *Managed Agents*（2026）直接启发，本 SDK 采用**三元解耦架构**：

```
┌───────────────────────────────────────────────────────────────────┐
│                          Session (事件日志)                         │
│         append-only log: messages, tool_use, tool_result,          │
│         plan_updates, checkpoints, state_snapshots …               │
│     API: emitEvent(id,event) · getEvents(range) · wake(sid)        │
└──────────────▲──────────────────────────────────────▲──────────────┘
               │ durable/replay                       │ durable/replay
               │                                      │
┌──────────────┴──────────────┐       ┌───────────────┴───────────────┐
│  Brain  (harness + model)   │◀─────▶│   Hands  (sandboxes & tools)  │
│  • agent loop               │ tool  │   • BashTool / FS / Edit      │
│  • prompt builder           │ call  │   • WebSearch / WebFetch      │
│  • context engineer         │       │   • MCP servers               │
│  • permission gate          │       │   • Code execution sandbox    │
│  • sub-agent orchestrator   │       │   • provision/execute API     │
└─────────────────────────────┘       └───────────────────────────────┘
```

三者之间**只暴露窄接口**：

| 接口 | 位置 | 语义 | 来源 |
|---|---|---|---|
| `execute(name, input) → string \| error` | Hands 对 Brain | 一次工具调用；容器/沙箱故障作为工具错误返回给模型 | [Managed Agents](https://www.anthropic.com/engineering/managed-agents) |
| `provision({resources})` | Hands 内部 | 按需（lazy）启动沙箱；不再预置 | 同上 |
| `emitEvent(id, event)` / `getEvents(id, range)` | Brain ↔ Session | 事件追加与区间检索；支持**倒回/重读**（rewind/reread） | 同上 |
| `wake(sessionId)` | Brain 自身 | 从 Session 重建 harness 状态；harness 本身是 cattle（无状态） | 同上 |

**直接产物与原则**：

1. **Brain = cattle**：harness 崩了可以原地重启，状态全在 Session。
2. **Hands = cattle**：沙箱崩了当作工具错误反馈给模型，由模型或 harness 决定是否重试；不做 "nursing"。
3. **Session ≠ Claude 上下文窗口**：Session 是**context 对象**，活在窗口之外，harness 可按需 slice/transform 后再喂给模型。这是长时任务可行的根基。
4. **凭据永不进入沙箱**：Git 令牌在 clone 时注入 remote，OAuth 令牌存 vault，由 MCP 代理注入；模型永远拿不到明文凭据。（见 `06-permissions-sandbox.md`）

> 关键引用（[Managed Agents](https://www.anthropic.com/engineering/managed-agents)）：
> "We virtualized the components of an agent: a session (the append-only log of everything that happened), a harness (the loop that calls Claude and routes Claude's tool calls to the relevant infrastructure), and a sandbox (an execution environment where Claude can run code and edit files). This allows the implementation of each to be swapped without disturbing the others."

## 3. 功能全景（Feature Map）

本 SDK 需要提供的能力，按架构层次排列：

### 3.1 Brain 层

| 功能 | 描述 | 主要参考 |
|---|---|---|
| **Agent Loop** | 核心循环：调用模型 → 解析 tool_use → 路由到 Hands → 注入 tool_result → 回到模型。支持 max_iterations / max_turns / token budget 双重限流。 | Claude Code `restored-src/src/query.ts`；Hermes `run_agent.py::run_conversation`；[Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents) |
| **Prompt Builder** | 系统提示词分段装配：identity + tool guidance + output format + examples；支持 XML/Markdown 分段。 | [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §Anatomy；Hermes `agent/prompt_builder.py` |
| **Prompt Caching 稳定性** | 禁止中途重排工具、重载 memory、改写历史前缀；否则破坏 Anthropic prompt cache 命中率，成本暴涨。 | Hermes AGENTS.md §"Prompt Caching Must Not Break"；OpenClaw CLAUDE.md §"Prompt Cache Stability" |
| **Context Compression** | 两条路径：**compaction**（摘要整个前半段开新窗口）与 **tool-result clearing**（只清工具输出）。长会话的默认抑制手段。 | [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §Compaction；Claude Code `restored-src/src/services/compact/*` |
| **Token Budget / Auto-continue** | 按每轮 token 消耗决策是"继续"还是"停止"；检测 diminishing returns。 | Claude Code `restored-src/src/query/tokenBudget.ts`（`checkTokenBudget`） |
| **Context Anxiety 缓解** | 模型接近上下文上限时会过早"收工"。两个对策：①上下文重置 + 结构化 handoff；②直接升级到更强模型（Opus 4.5+）移除此行为。 | [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)；[Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps) |
| **Sub-agent Orchestration** | Orchestrator-Workers 模式；生成器-评估器（GAN 式）模式；子代理拥有独立上下文窗口，返回 condensed 摘要。 | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)；[Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps)；Claude Code `restored-src/src/tools/AgentTool/*`、`restored-src/src/coordinator/coordinatorMode.ts` |

### 3.2 Hands 层

| 功能 | 描述 | 主要参考 |
|---|---|---|
| **Tool Registry** | 基于 JSON Schema 的工具注册表；分派、可用性、错误包装统一化。所有 handler 返回确定性结构（JSON/字符串）。 | Hermes `tools/registry.py`；Claude Code `restored-src/src/Tool.ts` |
| **Tool 并发批处理** | 将一次模型响应中的多个 tool_use 分批：只读工具并发（默认 10），写工具串行。 | Claude Code `restored-src/src/services/tools/toolOrchestration.ts`（`partitionToolCalls`, `runToolsConcurrently`） |
| **内置工具集（核心）** | FileRead / FileWrite / FileEdit（含多段编辑） / Glob / Grep / Bash / WebSearch / WebFetch / AskUserQuestion / TodoWrite / Agent(subagent) / Skill / Sleep / TaskList + TaskGet + TaskOutput（监控后台任务） | [Claude Agent SDK 概览](https://docs.claude.com/en/api/agent-sdk/overview)；Claude Code `restored-src/src/tools/*`（`FileReadTool`/`FileWriteTool`/`FileEditTool`/`GlobTool`/`GrepTool`/`BashTool`/`WebSearchTool`/`WebFetchTool`/`AskUserQuestionTool`/`TodoWriteTool`/`AgentTool`/`SkillTool`/`SleepTool`/`TaskListTool`/`TaskGetTool`/`TaskOutputTool`） |
| **MCP 客户端** | 标准 MCP 协议；本地 stdio、HTTP、SDK in-process；OAuth 凭据通过 vault + 代理；工具命名空间化。 | [MCP 官网](https://modelcontextprotocol.io)；[Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)；Claude Code `restored-src/src/services/mcp/*` |
| **Sandbox 运行时** | OS 级隔离：Linux bubblewrap / macOS seatbelt；文件系统白名单 + 网络代理白名单。沙箱内无凭据。 | [Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)；`github.com/anthropic-experimental/sandbox-runtime` |
| **Skills（文件系统 + Markdown）** | `.claude/skills/<name>/SKILL.md` 按需加载；不塞进每轮提示词，按相关度激活。 | [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §Create skills；Claude Code `restored-src/src/skills/*`；Hermes `skills/` |
| **Code Execution Mode** | 把 MCP 工具暴露为文件系统下的 TS/Py 模块，让模型写代码调用；降低 tool-definition 与中间结果的 token 消耗。 | [Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) |

### 3.3 Session 层

| 功能 | 描述 | 主要参考 |
|---|---|---|
| **Append-only Event Log** | 所有消息、工具调用、结果、plan 更新、checkpoints 都落到事件流；支持 `getEvents(range)` 的随机区间检索。 | [Managed Agents](https://www.anthropic.com/engineering/managed-agents) |
| **Checkpoint / Rewind** | 每次工具调用建立 checkpoint；支持 `/rewind` 与 `Esc+Esc` 式的会话 + 代码回退。 | [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §Rewind with checkpoints |
| **Resume / Wake** | `claude --continue` / `claude --resume <id>`：基于 Session 重建 Brain，不需要再解释上下文。 | 同上 §Resume conversations；[Claude Agent SDK Sessions](https://docs.claude.com/en/agent-sdk/sessions) |
| **Context Snapshot / Config Snapshot** | 每次运行记录 `config_snapshot_id` + `effective_config_hash`；保证 session 绑定启动时的有效配置。 | Octopus 本仓库 `AGENTS.md` §"Config snapshot rules"（本仓库治理规则） |

### 3.4 模型层（Model System）

| 功能 | 描述 | 主要参考 |
|---|---|---|
| **Provider / Surface / Model 三层** | Provider（厂商+认证+计费）· Surface（接口面 + 协议族）· Model（具体 id + family + track）。代码绑定前两层稳定接口，不硬编码模型 ID。 | 本仓 `docs/references/vendor-matrix.md`；`11-model-system.md` §11.2 |
| **Model Catalog** | 内置目录（从 vendor-matrix 派生）+ 用户/工作区覆写 + 可选远端同步（models.dev）；启动不联网，`octopus models refresh` 才走网络。 | Hermes `cli-config.yaml` §"Model Aliases"；Claude Code `modelCapabilities.ts` 本地缓存 |
| **Model Roles** | 业务按角色编程：`main`/`fast`/`best`/`plan`/`compact`/`vision`/`web_extract`/`embedding`/`eval`/`subagent_default` → 由 SDK 按策略映射到具体模型。 | Hermes §"Auxiliary Models"；Claude Code `opusplan` |
| **Reference Resolution** | 优先级链：turn override > session override > flag/env > Agent Definition > project/workspace/user runtime config > 内置默认。Family alias 不降级继承（subagent opus 继承父 Opus 4.6）。 | Claude Code `utils/model/model.ts`、`utils/model/agent.ts::aliasMatchesParentTier` |
| **Canonical Naming** | 跨 Bedrock/Vertex/Foundry 多种写法归一化为同一 short name（定价、日志、能力查询统一）。 | Claude Code `firstPartyNameToCanonical` |
| **Protocol Adapters** | 五种 `protocol_family` 各一 adapter：`openai_chat` / `openai_responses` / `anthropic_messages` / `gemini_native` / `vendor_native`；Canonical Message IR 统一中立表达。 | vendor-matrix.md §Surface Matrix；[Anthropic Messages](https://docs.claude.com/en/api/messages) / [OpenAI Responses](https://platform.openai.com/docs/api-reference/responses) |
| **Routing / Fallback** | 静态角色路由 + Smart Routing（按复杂度）+ OpenRouter 风格 provider routing + 多级 fallback（overloaded/5xx/prompt_too_long）。fallback 必发 `model_fallback` 事件，禁止 silent degradation。 | Hermes `cli-config.yaml` §"Smart Model Routing" / §"OpenRouter Provider Routing"；Claude Code `FallbackTriggeredError` |
| **Auxiliary Models** | vision / web_extract / compact / classifier 各自独立 `(provider, model)`，默认 auto 检测最便宜候选。 | Hermes §"Auxiliary Models (Experimental)" |
| **Prompt/Context Cache** | 两种模式：prompt_caching（Anthropic 断点）/ context_cache_object（Google/Ark/BigModel）。受 C1 Cache 稳定性约束。 | [Anthropic Prompt Caching](https://docs.claude.com/en/docs/build-with-claude/prompt-caching)；vendor-matrix.md cache surface |
| **Auth 多方案** | api_key / x_api_key / oauth / aws_sigv4 / gcp_adc / azure_ad；凭据存 OS keychain，不进沙箱。 | Claude Code `services/api/client.ts`；Hermes 十余种凭据；Managed Agents §凭据注入 |
| **本地模型** | Ollama / LM Studio / vLLM / llama.cpp 均按 `openai_chat` 兼容 surface 接入；`custom` provider 自行配置 base_url。 | Hermes `cli-config.yaml` §"Local servers" |
| **生命周期治理** | track = preview/stable/latest_alias/deprecated/sunset；allowlist/denylist；deprecated 模型启动时提示迁移。 | vendor-matrix.md `track` 字段；Claude Code `modelAllowlist.ts` |

### 3.5 插件层（Plugin System）

| 功能 | 描述 | 主要参考 |
|---|---|---|
| **统一 Manifest** | `plugin.json` 描述扩展点（tools/skills/commands/agents/hooks/mcpServers/lspServers/channels/providers/...）、依赖、compat 范围、setup 元数据；不加载代码即可校验。 | Claude Code `PluginManifestSchema`（Zod）；OpenClaw `openclaw.plugin.json` + `package.json:openclaw.*` |
| **中央 Registry** | plugin → registry ← core 单向流；每类扩展点维护独立索引；id 唯一性、冲突诊断统一在 registry。 | OpenClaw `docs/plugins/architecture.md` §"Registry model" |
| **扩展点全集** | tool / skill / command / agent / output-style / hook / mcp-server / lsp-server / model-provider（含 11 章所有子能力）/ channel / context-engine / memory-backend / http-route / rpc-handler / service。 | OpenClaw 12 类 capability + Claude Code `PluginComponent` |
| **所有权模型** | 一公司 / 一功能 = 一插件；避免 `if vendor===...` 散布在核心；跨 provider 共享能力通过 core capability 契约，不通过插件互 import。 | OpenClaw §"Capability ownership model" |
| **分发多源** | built-in / bundled / marketplace(github/git/npm/url/file/directory/settings/hostPattern) / 直接源(npm/pip/url/github/git-subdir) / MCPB 离线 bundle (`.mcpb`/`.dxt`)。 | Claude Code `MarketplaceSourceSchema` 9 型、`PluginSourceSchema`、`mcpbHandler.ts` |
| **版本锁** | semver + `gitSha`（40 字符全 SHA）+ `compat.pluginApi` range；session 记录 `pluginsSnapshot` 保证回放。 | Claude Code `gitSha` schema；OpenClaw `pluginApiRange` |
| **依赖解析** | 拓扑排序 + 循环拒绝；bare name 同 marketplace 解析；`dependency-unsatisfied` 错误。 | Claude Code `DependencyRefSchema` |
| **安全门** | 路径逃逸 + 世界可写拒绝 + ownership 校验；`npm install --omit=dev --ignore-scripts`；保留名白名单 + 非 ASCII 拒绝（homograph 防护）；企业策略 `strictKnownMarketplaces`/`blockedMarketplaces`。 | Claude Code `isBlockedOfficialName`、`validateOfficialNameSource`；OpenClaw §"Load pipeline" |
| **执行模型** | native 插件 in-process 核心等价信任；MCP server 子进程隔离；未来 v2 计划 OS 级 sandbox 型；MCPB 走子进程。 | OpenClaw §"Execution model" |
| **Slot 机制** | 单选扩展点（`contextEngine` / `memoryBackend` / `primaryProvider`）按 `plugins.slots.<name>` 配置；其他同 slot 候选自动禁用。 | OpenClaw `plugins.slots`；Hermes `plugins/context_engine/` + `plugins/memory/<name>/` |
| **诊断** | `octopus plugins inspect <id>` 显示 shape + capability；`plugins doctor` 四级信号（valid / advisory / legacy / hard error）；22 型分类错误 + fix 提示。 | OpenClaw `plugins inspect`/`doctor`；Claude Code `PluginError` 22 型 + `getPluginErrorMessage` |
| **SDK 边界** | `@octopus/plugin-sdk/*` 窄子路径公共入口；禁止 root barrel；跨插件不得深导入 `src/*`。 | OpenClaw §"Plugin SDK import paths" |
| **热重载** | `/reload-plugins` 重跑 discover→enable→load→register；活跃 session 不受影响（符合本仓 runtime config 治理）；新 session 享有新插件。 | Claude Code `performBackgroundPluginInstallations` |

### 3.6 横切面（cross-cutting）

| 功能 | 描述 | 主要参考 |
|---|---|---|
| **Hooks** | 确定性生命周期钩子：`PreToolUse` / `PostToolUse` / `Stop` / `SessionStart` / `SessionEnd` / `UserPromptSubmit` / `PostSampling` / `PreCompact` / `PostCompact` … 用于硬约束（如：写入必须落到审计日志） | [Claude Agent SDK Hooks](https://docs.claude.com/en/agent-sdk/hooks)；[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §Set up hooks；Claude Code `restored-src/src/hooks/*` |
| **Permissions** | 三种模式：默认提示、Allowlist、Auto Mode（分类器审查）、Bypass、Sandbox。工具可"按参数"授权。 | [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §Configure permissions |
| **Observability** | 全量 tracing：每个工具调用的 input/output/duration/err；交互模式（决策树而非隐私内容）；用于调试 emergent behavior。 | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §Debugging benefits |
| **Evaluation** | 小样本（~20 真实任务）起步；LLM-as-judge 单调用评分（0.0–1.0 + pass/fail）；端到端状态评估而非逐步评估。 | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §Effective evaluation；[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) |
| **Deployment (Rainbow)** | 有状态 agent 不能同步升级；采用 rainbow deployment，新旧版本并存直到旧会话结束。 | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §Deployment |

## 4. 设计约束（Design Constraints）

本 SDK 的任何实现都必须同时满足下列约束：

### C1. Prompt Cache 稳定性
- 工具顺序确定性（map/set/registry → deterministic sort）
- 不在对话中途重排工具、重写过去的 turn
- 只在尾部做 append / compaction，不触动 cached prefix
> 来源：[Anthropic prompt caching](https://docs.claude.com/en/docs/build-with-claude/prompt-caching)；Hermes AGENTS.md；OpenClaw CLAUDE.md §"Prompt Cache Stability"

### C2. 工具契约（ACI）质量
- 每个工具**单一、非重叠的用途**；避免冗余工具
- 参数名语义化（`user_id` 不是 `user`）
- 错误消息携带**可操作的修复提示**（不是 stack trace）
- 支持 `response_format: concise|detailed`（类 GraphQL 字段选择）
- 默认输出截断（Claude Code 的 Shell 输出默认 `BASH_MAX_OUTPUT_DEFAULT=30_000` **字符**；其它工具按 SDK 建议的 token 或 match 上限）
- 命名空间化（`asana_search` vs `jira_search`）
> 来源：[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)

### C3. 凭据零暴露
- 沙箱内绝不持有长期凭据
- Git 令牌在 clone 阶段注入 remote；push/pull 直接生效
- MCP OAuth 令牌存 vault，经"MCP 代理"在调用链注入，模型/harness 从不触碰明文
> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents)；[Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)

### C4. 并发安全
- 只读工具可并发（默认 max_concurrency = 10）
- 写入/破坏性工具必须串行；工具自报 `isConcurrencySafe(parsedInput)`
> 来源：Claude Code `restored-src/src/services/tools/toolOrchestration.ts`（`partitionToolCalls`, 默认 `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY=10`）

### C5. 可观测性全覆盖
- 每个 tool_use 记录：id / name / input / output / duration / error / permission_decision
- 交互决策树可脱敏统计
- 支持会话回放（replay）用于调试
> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §Debugging

### C6. "按需"而非"预置"（Lazy Provisioning）
- 沙箱容器由 Brain 通过 `execute()` 首次调用时按需分配，不在会话启动时预置
- TTFT（首 token 延迟）直接受益于此（Managed Agents 报告 p50 ↓60%, p95 ↓90%）
> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents)

### C7. 配置稳定性（Octopus 本地约束）
- Runtime config 以**文件为主**（`config/runtime/*.json`），DB 不是 runtime 配置的真理源
- 每次 session 记录 `config_snapshot_id` + `effective_config_hash` + `started_from_scope_set`
- 活跃 session 不受运行中配置文件变更影响（除非显式 hot-reload）
> 来源：本仓库 `AGENTS.md` §"Runtime Config And Runtime Persistence Rules"

## 5. 非目标（Non-goals）

为保持 KISS 原则与避免过度工程：

- **不构建大一统 "Agent Framework"**（如 LangChain 风格）；采用组合式小模块。
- **不把业务逻辑放进 harness**：harness 只提供能力，业务在 agent 定义 / skill / subagent 中表达。
- **不替代操作系统/容器编排**：沙箱能力复用 OS 原语（bubblewrap/seatbelt）与成熟运行时（Docker/Firecracker）。
- **不在 harness 里写模型专属 workaround**（例如某个模型版本的 quirk），这些必须是可开关的 feature flag，且标注失效模型版本，随模型迭代被删除（参考 Anthropic 对 "context anxiety" 的处理）。
- **不重新发明 MCP**：直接采用 MCP 作为外部工具协议。
- **不做"实时 hot-reload config 到活跃 session"**：违反 Octopus 持久化治理规则。

## 6. 与业内成熟实现的映射（参考矩阵）

> "Octopus SDK 建议"列中的包名均为**建议产物**，不保证当前已存在；落地时按 Octopus monorepo 现状定夺。

| 架构元素 | Claude Code（还原源） | Hermes Agent | OpenClaw | Octopus SDK 建议（待落地） |
|---|---|---|---|---|
| Agent loop | `src/query.ts` + `src/services/tools/toolOrchestration.ts` | `run_agent.py::run_conversation` | `src/` 内部 TS + plugin sdk | 独立包 `@octopus/agent-core` |
| Tool registry | `src/Tool.ts` + `src/tools/*` | `tools/registry.py` | `src/plugin-sdk` | `@octopus/agent-tools` |
| MCP client | `src/services/mcp/*` | `tools/mcp_tool.py`（~1050 LOC） | 插件化 | `@octopus/agent-mcp` |
| Sub-agent | `src/tools/AgentTool/` + `coordinator/coordinatorMode.ts` | `tools/delegate_tool.py` | N/A (channel-centric) | `@octopus/agent-subagent` |
| Skills | `src/skills/` + `bundled/` | `skills/` + Skills Hub | `.agents/skills/*/SKILL.md` | `packages/skills`（复用 `.claude/` 格式） |
| Hooks | `src/hooks/*` + `query/stopHooks.ts` | （部分内建） | `hooks.internal.entries` | `@octopus/agent-hooks` |
| Sandbox | 基于 bubblewrap/seatbelt | Docker/SSH/Modal/Daytona 多后端 | 沙箱镜像 `Dockerfile.sandbox*` | 可配置后端，抽象 `SandboxBackend` |
| Session log | `src/history.ts` + `~/.claude/*` | SQLite (FTS5) `hermes_state.py` | `~/.openclaw/agents/<id>/sessions/*.jsonl` | SQLite + JSONL（符合本仓 `AGENTS.md` 持久化治理） |
| Permissions | `src/hooks/toolPermission/*` + `useCanUseTool.tsx` | `tools/approval.py` | Command allowlist / pairing | 子集：`default` / `acceptEdits` / `bypassPermissions` / `plan` + allowlist（详见 06 §6.2） |
| Compaction | `src/services/compact/*` | `agent/context_compressor.py` | N/A | `@octopus/agent-context` |

## 7. 术语表（Glossary）

| 术语 | 定义 |
|---|---|
| **Harness** | 包裹 LLM 的所有非 LLM 代码。Anthropic 官方术语。 |
| **Brain / Hands / Session** | Managed Agents 提出的三元解耦；见 §2。 |
| **ACI** | Agent-Computer Interface；对应 HCI。工具的可用性契约。 |
| **Context rot** | 上下文越长，模型对其内容的精度越低的现象。 |
| **Context anxiety** | 模型感知到接近上下文上限时过早"收尾"的行为；Sonnet 4.5 尤明显，Opus 4.5+ 消失。 |
| **Progressive disclosure** | 按需、分层地披露工具与信息（例如 `tool_search` + 按需加载定义），节省 context。 |
| **Just-in-Time 检索** | 只保存轻量引用（路径/URL/查询），运行时用工具拉取数据；与 pre-inference embedding 检索相对。 |
| **Compaction** | 将对话前半段摘要后重开一个上下文窗口。 |
| **Tool-result clearing** | 清除已消费的旧工具输出；比 compaction 更轻量。 |
| **Code Mode / Code Execution with MCP** | 把 MCP 工具暴露为代码 API，让模型写代码调用而非逐一 tool_use。 |
| **Rainbow deployment** | 多版本并存的渐进式发布；有状态 agent 的标准姿势。 |
| **Sprint contract** | 生成器与评估器在编码前协商的"完成定义"。 |
| **Goldilocks Zone** | 系统提示词在"信息过载"与"信息不足"之间的平衡区；出自 Anthropic *Effective context engineering*。见 `02-context-engineering.md` §2.3。 |
| **Durable Scratchpad** | 本 SDK 命名：将 `NOTES.md` / `runtime/notes/<session>.md` 类结构化笔记提升为一等工具地位。见 `02-context-engineering.md` §2.6.2。 |
| **Agentic Memory** | 代理自主维护的跨 session 记忆（MCP `memory` / 键值 / 语义记忆等）。见 `02-context-engineering.md` §2.7。 |
| **Initializer + Coding Agent** | 长时任务模式：由 initializer 代理产出 `CLAUDE.md` / `NOTES.md` / `init.sh`，coding agent 依据这些 artifact 续作。见 `08-long-horizon.md` §8.3。 |
| **Autonomy Dial** | 权限模式从严到宽的连续光谱（`plan` → `default` → `acceptEdits` → `bypassPermissions`）；出自 Anthropic *Claude Code best practices*。见 `06-permissions-sandbox.md` §6.2。 |
| **Protocol Adapter** | 将各 surface 的原生协议（OpenAI Chat / Responses / Anthropic Messages / Gemini native / vendor native）翻译到 SDK 内部中立表达的适配层。见 `11-model-system.md` §11.6。 |
| **Canonical Message IR** | SDK 内部对消息 / 工具调用 / 工具结果的中立中间表达，Protocol Adapter 的公共数据类型。见 `11-model-system.md` §11.6。 |
| **Canonical Naming** | 把同一模型在 Bedrock / Vertex / Foundry 等多 provider 下的不同 id 归一化为同一 short name，用于日志 / 定价 / 能力查询。见 `11-model-system.md` §11.3。 |
| **Slot** | 插件体系中的"单活扩展点"（如 `contextEngine` / `memoryBackend` / `primaryProvider`）：同一 slot 只能启用一个候选，其它候选自动禁用。见 `12-plugin-system.md` §12.9。 |
| **MCPB** | Claude Desktop Extensions 的离线 bundle 格式（`.mcpb` / `.dxt`），在 Plugin 分发中作为**子进程隔离**的离线交付形态。见 `12-plugin-system.md` §12.6。 |
| **Effective Config Hash** | 每次 session 绑定的"启动时有效配置"的内容哈希；配合 `config_snapshot_id` 保证回放与活跃 session 的配置稳定性。见 §C7 与本仓 `AGENTS.md` §Runtime Config。 |
| **UI Intent IR** | SDK 层的 UI 意图中间表达：JSON 可序列化、discriminated union、宿主中立的渲染契约。SDK 产出 IR，业务层产出组件；两者解耦。见 `14-ui-intent-ir.md` 全章。 |
| **Render Block** | UI 意图 IR 的最小渲染单元，10 种 `kind`（`text` / `markdown` / `code` / `diff` / `list-summary` / `progress` / `artifact-ref` / `record` / `error` / `raw`）；插件**不得**自行扩 kind。见 `14-ui-intent-ir.md` §14.3 / §14.4。 |

## 8. 顶层路线图（后续文档）

- [x] `README.md`（本文）
- [x] `01-core-loop.md`
- [x] `02-context-engineering.md`
- [x] `03-tool-system.md`
- [x] `04-session-brain-hands.md`
- [x] `05-sub-agents.md`
- [x] `06-permissions-sandbox.md`
- [x] `07-hooks-lifecycle.md`
- [x] `08-long-horizon.md`
- [x] `09-observability-eval.md`
- [x] `10-failure-modes.md`
- [x] `11-model-system.md`
- [x] `12-plugin-system.md`
- [x] `13-contracts-map.md`
- [x] `14-ui-intent-ir.md`
- [x] `references.md`
- [x] `fact-fix 修订（2026-04-20）` — 对齐 `restored-src v2.1.88` 的常量、类型名、工具路径与枚举；见 `docs/plans/2026-04-20-sdk-fact-fix.md`。
- [x] `P0 跨章一致性修订（2026-04-20）` — 工具命名 snake_case、`runtime/notes/` 路径、插件扩展点互链；见 `docs/plans/2026-04-20-sdk-p0-cross-link-fix.md`。
- [x] `P1 文档收尾（2026-04-20）` — README §7 术语表扩充、`restored-src/` 简称与 v2.1.88 行号锚定；见 `docs/plans/2026-04-20-sdk-p1-doc-polish.md`。
- [x] `13 契约地图落地（2026-04-20）` — 见 `docs/plans/2026-04-20-sdk-contracts-map.md`。
- [x] `14 UI 意图 IR（2026-04-20）` — 见 `docs/plans/2026-04-20-sdk-ui-intent-ir.md`。
- [ ] `SDK 重构总控计划（2026-04-20 起）` — 规范层→实现层的落地计划与周度门禁见 [`docs/plans/sdk/`](../plans/sdk/README.md)：`00-overview` 定义 8 周路线与退出条件，`01-ai-execution-protocol` 定义 AI 推进规约（三层 Checklist + Stop Conditions + 每周门禁）。

本轮（2026-04-20）已完成全部 15 份文档（01–14 + README + references）。

---

## Fact-Fix 勘误

> 本节从 **2026-04-20 SDK 重构启动**起作为 `docs/sdk/*` 与实际实现之间偏离的**累计登记簿**。
>
> 规则：
> - `docs/plans/sdk/01-ai-execution-protocol.md §4 Weekly Gate` 与 `00-overview.md §5 #8` 强制要求：一旦发现规范描述与实现矛盾，必须在本小节追加一条勘误。
> - 每条勘误须含：发现日期、受影响章节、矛盾描述、最终以规范或实现哪一方为准、PR / commit 引用。
> - 列表为空不代表没有矛盾；是"截至本行日期尚未登记"的意思。
> - 存量勘误（2026-04-20 之前）沿用各自专门的 plan 文档：`2026-04-20-sdk-fact-fix.md` / `p0-cross-link-fix.md` / `p1-doc-polish.md`。

| # | 发现日期 | 受影响章节 | 现象 | 最终基准 | 登记 PR / commit |
|---|---|---|---|---|---|
| 1 | 2026-04-21 | `14-ui-intent-ir.md` §14.3.1 / §14.3.2, `docs/plans/sdk/02-crate-topology.md` §2.1 | `14` 章把 `RenderBlock` 写成内联 discriminated union、把 `RenderLifecycle` 写成工具侧 5 钩子对象；W1 Rust contracts 需要事件流载体以嵌入 `SessionEvent::Render` 与 `RenderEmitter::emit(block, lifecycle)`。 | 以 W1 Rust contracts 为准：`RenderBlock` 采用 `kind + payload + meta` 载体，`RenderLifecycle` 采用 5 个 hook 同名 phase 枚举（`on_tool_use`…`on_tool_error`）；`14` 章保留为工具侧逻辑 IR 参考，`02` 已同步登记 Rust 载体形状。 | `goya/sdk-w1-contracts-session` working tree |
| 2 | 2026-04-21 | `11-model-system.md` §11.6.1, `docs/plans/sdk/02-crate-topology.md` §2.3, `docs/plans/sdk/05-week-2-model.md` Task 2 / Task 8 | 规范层标准角色集含 `rerank`，但 W2 执行层公共面当前只冻结 10 个 `ModelRole` 值；若直接把 `rerank` 放入 W2，会把未实现的角色解析与路由契约一并拉进本周范围。 | 以 W2 执行基线为准：`02 §2.3` 与 `05-week-2-model.md` 当前公开 10 值 `ModelRole`，`rerank` 延后到后续周次与真实实现同批纳入；届时需同步回写 `11`/`02`/`05`。 | `main` working tree |
| 3 | 2026-04-21 | `03-tool-system.md` §3.4（monitor tools），`docs/plans/sdk/02-crate-topology.md` §2.4，`docs/plans/sdk/06-week-3-tools-mcp.md` Task 7 / Task 10 | 规范层示意表把 `task_output` 与 `task_list / task_get` 并列成 monitor 家族，但 W3 首版冻结的 15 个 builtins 只包含 `task_list / task_get`，没有独立 `TaskOutputTool`，contracts 也没有等价 retrieval 契约。 | 以 W3 执行基线为准：本周只交付 15 个工具，不扩成第 16 个；`task_output` 先 defer。后续若折叠进 `task_get` 或改成 session/event retrieval，必须先登记到 `02 §2.1 / §2.4`，再由 W5/W6 决定落点。 | `goya/w3-sdk-tools-mcp` working tree |
| 4 | 2026-04-21 | `07-hooks-lifecycle.md` §7.2, `docs/plans/sdk/00-overview.md` §3 W4, `docs/plans/sdk/07-week-4-permissions-hooks-sandbox-context.md` | 规范层 hooks 清单覆盖 `PreSampling / PostSampling / SubagentSpawn / SubagentReturn / OnToolError / PreFileWrite / PostFileWrite` 等完整生命周期，但 W4 周计划与出口状态只冻结 8 个最小事件面。若按规范全文直接执行 W4，会把 W5/W6 才承接的子代理/采样/文件写回语义提前拉进本周。 | 以 W4 执行基线为准：W4 只实现 `PreToolUse / PostToolUse / Stop / SessionStart / SessionEnd / UserPromptSubmit / PreCompact / PostCompact` 8 个事件；其余 hook 点仍是规范层目标，但延后到后续周次与真实 Brain Loop / subagent / file-write 流水线同批纳入。届时需同步回写 `07`/`00`/对应周计划。 | `main` working tree |
| 5 | 2026-04-21 | `05-sub-agents.md`, `12-plugin-system.md`, `docs/plans/sdk/02-crate-topology.md`, `docs/plans/sdk/08-week-5-subagent-plugin.md` | W5 首稿把 declaration 层和 runtime 层混成一层：`ToolDecl / HookDecl` 被写成可直接注册到执行路径，`HookDecl` 误用 runtime `HookEvent`，`SubagentContext.tools` 误写成 registration API，且 `worker_boot.rs` / `subagent_runtime.rs` 被当成现有实现来源，`plugins_snapshot` 也被放成尾部 session 小补丁。 | 以 W5 执行基线为准：manifest/declaration 只负责静态元数据；tools/hooks 真正接线必须走 executable runtime registration；插件声明层使用 `HookPoint`，不是 `HookEvent`；`octopus-sdk-subagent` 按当前 SDK crate 边界绿色实现，不从 `worker_boot.rs` / `subagent_runtime.rs` 迁抽象；`plugins_snapshot` 优先扩进 `SessionStarted / SessionSnapshot / SessionStore`，若 W1 首事件无法扩面则退回紧随其后的 `session.plugins_snapshot` 次事件，但 replay/快照合同仍必须可恢复同一份插件集合。规范层目标不变，但 W5 周计划按这条基线执行。 | `main` working tree |

---

**最后更新**：2026-04-21 · 所有设计决策均对应 `references.md` 中的一级来源条目；源码引用以 `restored-src v2.1.88` 快照为准。
