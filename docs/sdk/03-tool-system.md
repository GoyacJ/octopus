# 03 · 工具系统（Tool System）

> "Tools form a novel kind of contract between deterministic systems and non-deterministic agents."
> — [Anthropic · Writing effective tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)

工具是 Harness 与世界交互的**唯一通道**。本章定义 Octopus SDK 的工具**契约**、**注册机制**、**调度模型**、**权限关卡**、**MCP 集成**与**Skills**。

## 3.1 Tool 契约（核心数据结构）

### 3.1.1 最小字段

```ts
interface Tool<Input = unknown, Output = unknown> {
  name: string                              // 全局唯一；命名空间化（e.g. 'fs_read'）
  version?: string                          // 对 Prompt cache 与 schema 演进有用

  description: string                       // 给模型的自然语言说明；关键！
  inputSchema: JSONSchemaV7                 // 入参 schema
  outputFormat?: 'concise' | 'detailed' | OutputSchema // 响应格式枚举（见 §3.3.3）

  isConcurrencySafe(parsed: Input): boolean // 默认返回 false（保守）
  permissionPolicy?: PermissionPolicy       // 见 06-permissions

  validate(raw: unknown): Result<Input, ValidationError>
  execute(
    input: Input,
    ctx: ToolUseContext
  ): AsyncIterable<ToolProgress> | Promise<ToolResult<Output>>

  // 可选：UI 显示描述符（**非** React/Vue 节点；Brain 必须对宿主框架无感）
  displayDescriptor?: ToolDisplayDescriptor
}

// 序列化的 display 描述，由宿主 adapter 解释为具体 UI。
type ToolDisplayDescriptor = {
  progress?: ToolProgressDescriptor         // 进度态的展示提示
  result?: ToolResultDescriptor             // 最终结果的展示提示
}
// 具体字段（标题 / icon / 关键 metric / 截断策略等）留给实现，
// 但必须是 JSON 可序列化的，不得出现 React.ReactNode / VNode / 函数指针。
```

> **UI 框架边界**：Brain 只输出**序列化描述符**；宿主层（桌面 `apps/desktop` 的 Vue / 浏览器宿主的 Vue）把描述符渲染成组件。这与 `AGENTS.md` §Frontend Governance（desktop baseline = Vue 3 + Tauri 2）一致，并避免把核心 SDK 绑定到任何单一 UI 框架上。Claude Code 源码中的 `renderProgress/renderResultForAssistant` 返回 `React.ReactNode`，那是其 Ink/TUI 宿主的实现选择，**不属于跨宿主核心契约**。

> **正式契约**：`displayDescriptor` 的完整形状（5 个生命周期钩子 `onToolUse` / `onToolProgress` / `onToolResult` / `onToolRejected` / `onToolError`，以及 10 种 `RenderBlock.kind`、`AskPrompt` / `ArtifactRef` 顶层类型）见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.3 / §14.5。本节的 `ToolDisplayDescriptor` 是 14 章 `RenderLifecycle` 的**历史占位**，语义等价于其子集；新工具**必须**直接按 14 章契约实现。

### 3.1.2 `ToolUseContext` 与 `ToolPermissionContext`（是**两个不同类型**）

Claude Code 把运行时上下文拆成两层：**执行上下文** `ToolUseContext`（工具需要用到的 I/O、状态、回调）和**权限上下文** `ToolPermissionContext`（只给权限判定子系统读）。文档早期版本曾把两者合并，容易误导实现者；此处严格分开。

#### `ToolPermissionContext`

来自 `restored-src/src/Tool.ts:123-138`（源定义在 `types/permissions.ts:427-441`，`Tool.ts` 用 `DeepImmutable<>` 包装再导出）：

```123:138:docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts
export type ToolPermissionContext = DeepImmutable<{
  mode: PermissionMode
  additionalWorkingDirectories: Map<string, AdditionalWorkingDirectory>
  alwaysAllowRules: ToolPermissionRulesBySource
  alwaysDenyRules: ToolPermissionRulesBySource
  alwaysAskRules: ToolPermissionRulesBySource
  isBypassPermissionsModeAvailable: boolean
  isAutoModeAvailable?: boolean
  strippedDangerousRules?: ToolPermissionRulesBySource
  shouldAvoidPermissionPrompts?: boolean
  awaitAutomatedChecksBeforeDialog?: boolean
  prePlanMode?: PermissionMode
}>
```

`PermissionMode` 的真实枚举见 §6.2；Octopus SDK 首版**采用** `ToolPermissionContext` 的字段集合，不增删。

#### `ToolUseContext`（精简摘要）

真实结构（`Tool.ts:158-300`）较大，仅摘出对 SDK 而言**必要**的字段：

```ts
type ToolUseContext = {
  options: {
    tools: Tools
    mcpClients: MCPServerConnection[]
    agentDefinitions: AgentDefinitionsResult    // 供 task / subagent 工具
    mainLoopModel: string
    thinkingConfig: ThinkingConfig
    isNonInteractiveSession: boolean
    maxBudgetUsd?: number
    customSystemPrompt?: string
    appendSystemPrompt?: string
    refreshTools?: () => Tools                  // MCP 中途连上后刷新
  }
  abortController: AbortController              // 等价于外部 AbortSignal
  readFileState: FileStateCache                 // 文件读状态 LRU
  messages: Message[]                           // 本轮历史
  getAppState(): AppState
  setAppState(f: (prev: AppState) => AppState): void
  setInProgressToolUseIDs: (f: (prev: Set<string>) => Set<string>) => void
  toolUseId?: string
  agentId?: AgentId                             // 仅 subagent 会设置
  agentType?: string
  // —— 以下为 SDK 扩展（非 Claude Code 原生，由 Brain 注入）
  sessionId: string
  projectRoot: string
  emitEvent: (evt: Event) => void
}
```

> 与 `ToolPermissionContext` 的关系：`ToolUseContext` 通过 `getAppState()` 读到应用级状态（应用状态持有一个 `ToolPermissionContext` 快照），再交给权限网关；两者生命周期不同——`ToolUseContext` 随单个工具调用创建，`ToolPermissionContext` 随 session / mode 切换更新。
> 来源：`restored-src/src/Tool.ts`；`restored-src/src/types/permissions.ts`。

### 3.1.3 `ToolResult` / `ToolProgress`

SDK 在 `Tool` 与模型之间保留**两套形状**：

- **边界形状**（tool → model）：与 Anthropic API 的 `tool_result` 对齐，只使用 `{ output: string, is_error: boolean }`。这是 Brain 送进下一轮 `messages[]` 的字段，也是 04 章 `SessionEvent.toolResult` 的落库字段。
- **内部形状**（tool implementation）：`Result<O, ToolError>`，方便实现语义化错误处理、type-narrowing。

**两套形状之间的唯一转换点**是 `services/tools/toolExecution.ts` 的封装层（Octopus 建议在 `@octopus/agent-core` 的 `toolExecution.ts` 里做同样的事）：序列化 `output`、填 `is_error`、把 `ToolError.remediation` 拼进 `output` 文本以供模型读取。

```ts
// —— 内部（工具实现可选使用）
type ToolResult<O> =
  | { success: true,  output: O, displayText?: string }
  | { success: false, error: ToolError }

type ToolError = {
  type: 'validation' | 'permission' | 'execution' | 'timeout' | 'cancelled'
  message: string               // 对模型可读；简洁可操作
  remediation?: string          // 修复建议，模型可直接据此重试
  stderr?: string               // 可选，仅当必要
}

// —— 边界（Brain 写入 history，Session 落盘；与 Anthropic API 对齐）
type ToolResultBoundary = {
  tool_use_id: string
  output: string                // 结构化数据应 serialize 后放入
  is_error: boolean             // validation/permission/execution/timeout/cancelled 都 true
}

type ToolProgress =
  | { phase: 'start' }
  | { phase: 'stream', chunk: string }       // 对流式工具（e.g. long WebFetch）有用
  | { phase: 'snapshot', data: unknown }     // JSON 可序列化
```

> 04 章 §4.2.2 / §4.4.3 使用的是**边界形状** `{ output, is_error }`；本节第一段描述的 `success`/`error` 仅出现在工具内部。

## 3.2 Tool Registry

### 3.2.1 责任

- 工具发现、版本选择
- 确定性排序（Prompt cache 稳定性 C1）
- 为每个 session 过滤"可见工具集"（按 permission、plan mode、用户禁用项）

### 3.2.2 可见性

在每次循环的 turn，Registry 根据以下因素决定哪些工具暴露给模型：

| 过滤器 | 行为 |
|---|---|
| `plan_mode` | 隐藏所有非只读工具 |
| `allowlist` | 只保留白名单 |
| `denylist` | 排除黑名单 |
| 用户动态禁用（`/disable-tool <name>`） | 隐藏 |
| `required_permissions` 缺失 | 隐藏 + 文档注释 |
| Subagent 允许工具白名单 | 只暴露给相应 subagent |

### 3.2.3 命名空间

强制约定：`<source>_<action>` 或 `mcp__<server>__<tool>`。

- 核心工具：`fs_read`, `fs_write`, `fs_edit`, `shell_exec`, `web_search`, `web_fetch`
- MCP：`mcp__github__list_prs`
- Skill-exposed 子工具：`skill__<skill-name>__<tool>`（可选）

> 来源：[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Namespace your tools"；Claude Code `restored-src/src/services/mcp/*` 的 `mcp__` 前缀约定。

## 3.3 ACI 设计规范（Agent-Computer Interface）

这是工具最重要的部分。**工具不是给人看的函数，是给模型看的契约**。

### 3.3.1 好工具的 5 条原则（来自 Anthropic 官方）

1. **Choose the right tools**：不要把 API 的每个端点都包成一个工具。**合并**相关操作；**去掉**冗余工具。
2. **Namespace your tools**：见 §3.2.3。
3. **Return meaningful context**：输出**语义化**而不是机器内部 ID；**token-efficient**；必要时 **分页/截断**。
4. **Use `response_format` enum**：默认 `concise`；调试时 `detailed`。
5. **Prompt-engineer descriptions**：description 本身是一段被读的提示词；认真写、举例子。

> 来源：[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)。

### 3.3.2 默认截断

任何可能产出大输出的工具必须内建截断。以下为 Octopus SDK 的默认值；**`Bash` 的数值来自 Claude Code 源码，其余为 SDK 自拟建议**，实现时应显式可配置。

- **Bash / PowerShell / Shell**：默认输出上限 `BASH_MAX_OUTPUT_DEFAULT = 30_000` **字符**（**不是 token**），环境变量 `BASH_MAX_OUTPUT_LENGTH` 可覆盖，硬上限 `BASH_MAX_OUTPUT_UPPER_LIMIT = 150_000` 字符；超过则截断 + 提示"已截断，使用 `grep`/`head` 二次过滤"。
- **Grep**：默认 100 matches（SDK 建议）。
- **WebFetch**：默认 30_000 字符（SDK 建议，与 Bash 对齐）。
- **List-style 工具**：默认 50 items + pagination token（SDK 建议）。

```3:14:docs/references/claude-code-sourcemap-main/restored-src/src/utils/shell/outputLimits.ts
export const BASH_MAX_OUTPUT_UPPER_LIMIT = 150_000
export const BASH_MAX_OUTPUT_DEFAULT = 30_000

export function getMaxOutputLength(): number {
  const result = validateBoundedIntEnvVar(
    'BASH_MAX_OUTPUT_LENGTH',
    process.env.BASH_MAX_OUTPUT_LENGTH,
    BASH_MAX_OUTPUT_DEFAULT,
    BASH_MAX_OUTPUT_UPPER_LIMIT,
  )
  return result.effective
}
```

> 来源：Claude Code `restored-src/src/utils/shell/outputLimits.ts`；被 `tools/BashTool/utils.ts`、`tools/PowerShellTool/prompt.ts`、`utils/task/TaskOutput.ts` 共用；[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Truncate and paginate"。

### 3.3.3 `response_format` 规范

```json
{
  "name": "github_list_prs",
  "parameters": {
    "repo": "octopus",
    "state": "open",
    "response_format": "concise"    // 默认；只返回 (id, title, author, status)
                                    // "detailed" 则附 body, reviews, commits
  }
}
```

- 默认 concise；模型按需切换。
- 某些工具可有超过两档（e.g. `concise` / `detailed` / `full`）。

> 来源：[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Give agents a say in verbosity"。

### 3.3.4 错误消息的"可操作性"

反例：

```
Error: EACCES: permission denied, open '/etc/passwd'
```

正例：

```
error.type: permission
error.message: 访问 '/etc/passwd' 被 allowlist 拒绝
error.remediation: 该路径不在 additionalWorkingDirectories；如需访问请通过 AskUserQuestion 请求用户批准，或切换到 additionalWorkingDirectories 内的路径。
```

模型读 `remediation` 后可直接决策下一步。

> 来源：[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Prompt-engineer error messages"。

### 3.3.5 参数命名

- 避免 `id`；用 `user_id`、`issue_id`。
- 避免 `type` / `kind` 作为 enum 字段名（名字太泛）。
- 日期时间用 ISO-8601 字符串。
- 凡与时间相关的字段，在 description 里**明确时区**。

### 3.3.6 描述（description）的最小模板

```
<action> <object>. Returns <shape>.
Use when: <具体场景>。
Do not use when: <容易误用的场景>。
Example: ... (1–2 个 input/output 对)
```

## 3.4 内置核心工具（Core Tools）

| 工具 | 语义 | 并发 | 来源 |
|---|---|---|---|
| `fs_read` | 读文件（区间、line-prefixed） | ✅ | Claude Code `tools/FileReadTool` |
| `fs_write` | 覆写文件 | ❌ | Claude Code `tools/FileWriteTool` |
| `fs_edit` | String replacement / multi-edit（含多段编辑能力） | ❌ | Claude Code `tools/FileEditTool` |
| `fs_glob` | 按 glob 找文件 | ✅ | Claude Code `tools/GlobTool` |
| `fs_grep` | 基于 ripgrep | ✅ | Claude Code `tools/GrepTool` |
| `shell_exec` | 子进程执行 | ❌（除非只读） | Claude Code `tools/BashTool`（Windows 侧 `PowerShellTool`） |
| `web_search` | 搜索引擎调用 | ✅ | Claude Code `tools/WebSearchTool` |
| `web_fetch` | 拉取 URL 内容转 markdown | ✅ | Claude Code `tools/WebFetchTool` |
| `ask_user_question` | 结构化多选 | ❌ | Claude Code `tools/AskUserQuestionTool`；[Claude Agent SDK 概览](https://docs.claude.com/en/api/agent-sdk/overview) |
| `todo_write` | 结构化 todo 清单 | ❌ | Claude Code `tools/TodoWriteTool` |
| `task` | Spawn subagent | ❌ | Claude Code `tools/AgentTool` |
| `skill` | 激活 skill | ❌ | Claude Code `tools/SkillTool` |
| `sleep` | 等待（用于轮询 / rate-limit） | ✅ | Claude Code `tools/SleepTool` |
| `task_list` / `task_get` / `task_output` | 查看后台任务的状态、单条状态、输出 | ✅ | Claude Code `tools/TaskListTool`、`TaskGetTool`、`TaskOutputTool`（共同承担"monitor"语义） |

> 注：Claude Code 还内建 `NotebookEditTool`、`LSPTool`、`MCPTool`、`EnterPlanModeTool` 等特定场景工具；Octopus SDK 首版不强制实现，但扩展表会保留这些工具名以便日后对齐。

### 3.4.1 为什么这些是"核心"

它们构成**最小通用集**：读、写、搜、执行、问、记、托付、技能。每个实际 harness（Claude Code / Hermes / OpenClaw）都提供这些或近似变体。

## 3.5 Tool 调度（Dispatcher / Orchestrator）

### 3.5.1 流程

```
model returns response with N tool_use blocks:
  ↓
validate each input against Tool.inputSchema
  ↓
run hooks: PreToolUse(each)   → may mutate input / veto
  ↓
permission gate: canUseTool(input, mode) → allow | ask | deny
  ↓
partition: concurrent vs serial   (via tool.isConcurrencySafe(input))
  ↓
execute:
  - concurrent batch: Promise.all(limit=10)
  - serial batch:     one by one
each execution emits ToolProgress; wrap in ToolResult
  ↓
run hooks: PostToolUse(each) → may rewrite output / block
  ↓
emit tool_result events to Session; append to model history
```

### 3.5.2 超时与取消

- 工具默认超时：Bash 120s、WebFetch 30s、其它 30s；可 per-call override
- 取消语义：`ToolUseContext.abortSignal.onabort` → 工具必须尽快归还资源
- 取消后仍需 emit `tool_result { ok:false, error.type:'cancelled' }` 供模型感知

### 3.5.3 并发上限

- 单 session 默认 `max_concurrency = 10`（=Claude Code 值）
- Subagent 独立计数；不共享父代理的并发额度

> 来源：Claude Code `restored-src/src/services/tools/toolOrchestration.ts`；
> [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Parallel tool calling"。

## 3.6 MCP（Model Context Protocol）

### 3.6.1 为什么采用 MCP

- 行业标准（Anthropic 主推；OpenAI、Google、Microsoft 等已适配）
- 避免每个三方集成都重新设计工具 schema
- 可插拔：HTTP、stdio、in-process SDK

### 3.6.2 客户端形态

| 形态 | 传输 | 典型用途 |
|---|---|---|
| `stdio` | 子进程 stdin/stdout | 本地工具 server（最常见） |
| `http` / `sse` | 远程 | 多租户、企业级集成 |
| `sdk`（in-process） | 函数直呼 | 自研 server、低延迟、测试 |

> 来源：[Claude Agent SDK - MCP](https://docs.claude.com/en/api/agent-sdk/mcp)；[MCP 官网](https://modelcontextprotocol.io)。

### 3.6.3 Code Execution Mode with MCP

把 MCP 工具暴露成**代码**而非工具 schema，模型写 TS/Python 调用。

示例目录结构（来自 Anthropic）：

```
servers/
  google-drive/
    getDocument.ts
  salesforce/
    updateRecord.ts
  ...
```

模型：

```ts
import { getDocument } from '@servers/google-drive/getDocument'
import { updateRecord } from '@servers/salesforce/updateRecord'

const doc = await getDocument({ docId: '<id>' })
// 只摘取姓名 + 标题
const rows = doc.rows.slice(0, 20).map(r => ({ name: r.name, title: r.title }))
await updateRecord({ objectType: 'Opportunity', ...rows[0] })
```

**Anthropic 报告 token 消耗从 150,000 ↓ 到 2,000**（-98.7%）。

> 来源：[Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) §"From 150,000 to 2,000 tokens"。

### 3.6.4 MCP 的安全注意

- MCP Server 是独立进程，**信任边界**必须明确
- 令牌/凭据应存 vault，经代理注入，不走模型
- 错误消息要 sanitize（不泄露内部路径/堆栈）

> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Credentials never enter the sandbox"；[Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) §"Tradeoffs and considerations"。

### 3.6.5 MCP Server 的注册入口

在 Octopus 中，**所有** MCP Server 的注册都通过 Plugin Registry 暴露的扩展点完成；核心**不**对 MCP server id 做 switch，也**不**在业务层硬编码 server 列表。具体的 manifest 字段见 [12 §12.3 扩展点全景](./12-plugin-system.md)，注册阶段见 [12 §12.8.4 Register](./12-plugin-system.md)，安全边界与凭据隔离见 [12 §12.10 安全与沙箱](./12-plugin-system.md)。这也意味着：`mcpServers.*` 配置本质上是一种"内建的 plugin manifest"；外部 MCP 厂商接入走 `api.registerMcpServer(...)` 走同一条路径。

## 3.7 Skills（按需激活的"经验包"）

### 3.7.1 定义

一个 Skill 是一个目录：

```
.claude/skills/<skill-name>/
  SKILL.md          ← 必有。含 frontmatter 描述与激活条件
  scripts/...       ← 可选，skill 可携带的脚本/工具
  examples/...      ← 可选示例
  references/...    ← 可选长文；主 SKILL.md 用 progressive disclosure 指向
```

### 3.7.2 SKILL.md frontmatter

```yaml
---
name: pdf-tools
description: Use when the user wants to read, merge, split, watermark PDFs.
allowed_tools:
  - fs_read
  - shell_exec
dependencies:
  - poppler-utils
---
```

### 3.7.3 激活机制

- 启动：registry 扫描 `.claude/skills/*/SKILL.md`，把 `(name, description)` 作为"元工具"候选
- 模型通过 `skill` 工具或隐式匹配 description 后激活
- 激活后：把 SKILL.md 正文注入下一轮；`references/*` 仍按需读

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Create skills"；[Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md) §"Skills"；Claude Code `restored-src/src/skills/*`。

### 3.7.4 VS Subagent

| 对比 | Skills | Subagent |
|---|---|---|
| 上下文 | 注入到**主**代理 | 独立窗口 |
| 粒度 | 经验 / 模板 / 小脚本 | 完整任务 |
| 调用 | `skill` 工具激活；无返回"摘要" | `task` 工具；返回 summary |
| 用途 | 复用的做法（怎么写 PDF）       | 一次性大任务（读 200 文件后报告 bug） |

> 来源：[Anthropic · Equipping agents for the real world with Agent Skills](https://www.anthropic.com/news/agent-skills)。

## 3.8 Hooks 与工具的关系

Hooks 是工具调度流程里的**可插拔拦截器**。详见 [`07-hooks-lifecycle.md`](./07-hooks-lifecycle.md)。

与工具相关的钩子：

- `PreToolUse(toolName, input)` → 可返回 `block`/`rewrite(new_input)`
- `PostToolUse(toolName, input, output)` → 可返回 `rewrite(new_output)`
- `onToolError(toolName, error)` → 可返回替代 `tool_result`

常用场景：

- 强制 `fs_write` 前调 `prettier --check`
- 把 `shell_exec` 的命令记入审计日志
- 屏蔽含凭据的 stdout

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Set up hooks"；[Claude Agent SDK - Hooks](https://docs.claude.com/en/api/agent-sdk/hooks)。

## 3.9 分发与版本

### 3.9.1 工具生命周期

- `register(tool)` → `unregister(name)` 是 hot-op，但必须保持 Prompt cache 稳定性：**只在 session 启动时生效**，运行中的 session 工具列表不变
- 版本：`name@version`；description 中明示 breaking change

### 3.9.2 Plugins / Extensions 生态

- Claude Code Plugins：Anthropic 官方的扩展打包格式
- OpenClaw Plugin SDK：`@openclaw/plugin-sdk/*` 公开接口；插件注册工具 + channel + memory store
- Hermes Toolsets：`toolsets.py` 集合预置 toolsets

> 来源：[Claude Agent SDK - Plugins](https://docs.claude.com/en/api/agent-sdk/plugins)；OpenClaw `CLAUDE.md` §"Plugin & extension boundaries"。

## 3.10 Octopus 落地约束

- **所有工具的返回必须落 Session 事件流**（可反查）
- **工具的 `displayText` 不能包含完整路径 / 密钥**（UI 层展示用）
- **`shell_exec` 默认跑在 Tauri 沙箱子进程里**，宿主访问走 `apps/desktop/src/tauri/shell.ts`（契合本仓 `AGENTS.md` §"Request Contract Governance"）
- **新工具必须在 `contracts/openapi/` 或 `packages/schema` 里定义输入输出的共享 schema**（保持前后端契约一致）
- **工具不得 import 任何 UI 库**：`Tool.displayDescriptor` / `RenderLifecycle` 只能返回 JSON 可序列化的 IR 描述符；具体组件实现归业务层（`apps/desktop` 走 `@octopus/ui`，CLI 走 Ink/TUI）。违反此约束的工具一律拒绝注册。原则与 IR 枚举见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.2 / §14.10

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) | ACI 原则、response_format、错误消息、命名空间、截断 |
| [Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) | Code Mode、token 优化 |
| [Claude Agent SDK – MCP](https://docs.claude.com/en/api/agent-sdk/mcp) | MCP 客户端形态 |
| [Claude Agent SDK – Tools overview](https://docs.claude.com/en/api/agent-sdk/overview) | 内置工具清单 |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) | Skills、Hooks、Plan Mode、Subagent |
| [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | 并发效益 |
| [Managed Agents](https://www.anthropic.com/engineering/managed-agents) | 凭据隔离、错误即输入 |
| [Agent Skills](https://www.anthropic.com/news/agent-skills) | Skill 概念 |
| Claude Code restored src | `Tool.ts`, `toolOrchestration.ts`, `toolExecution.ts`, `services/mcp/*`, `skills/*` |
| Hermes `tools/`, `toolsets.py` | Tool registry、toolset 打包 |
| OpenClaw `plugin-sdk/` | Plugin 边界 |
| [Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md) | `tool_search` meta-tool、Skills 拓扑 |
