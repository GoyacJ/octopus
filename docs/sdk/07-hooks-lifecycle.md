# 07 · Hooks 与生命周期

> "Hooks enable custom code to execute at specific points in an agent's execution, providing deterministic control over its behavior."
> — [Claude Agent SDK · Hooks](https://docs.claude.com/en/api/agent-sdk/hooks)

Hooks 是 harness 的**确定性注入点**。模型是非确定的；hooks 让我们能在它旁边编织"硬约束"。

## 7.1 核心定位

- **Hooks ≠ tools**：tools 由模型决策调用；hooks 由 harness 在特定生命周期点**强制**调用
- **Hooks ≠ permissions**：permissions 是"能不能做"；hooks 是"做之前/之后还要干啥"
- **Hooks = 确定性护栏**：同样的输入永远触发同样的 hook 行为

## 7.2 生命周期点清单

本 SDK 提供的 hook 点（融合 Claude Code / Agent SDK 已公开能力 + Octopus 需求）：

### 7.2.1 会话层

| Hook | 触发时机 | 可做 |
|---|---|---|
| `SessionStart` | session 创建后、首轮前 | 注入系统消息、初始化日志、记录 config snapshot |
| `SessionEnd` | session 正常 / 异常结束 | 刷新日志、清理 sandbox、触发通知 |
| `UserPromptSubmit` | 用户新 prompt 进来、模型还未响应 | 自动附加上下文（例如相关文件）、拒绝危险 prompt |
| `PreCompact` | 即将压缩 | 把关键决策冻结到 `NOTES.md` |
| `PostCompact` | 压缩完成 | 校验 summary 完整性、审计 |

### 7.2.2 模型调用层

| Hook | 触发时机 | 可做 |
|---|---|---|
| `PreSampling` | 每轮模型调用前 | 动态调整 temperature、system 段、工具可见性 |
| `PostSampling` | 模型返回、进入 parse 前 | 统计 token、触发 telemetry |
| `Stop` | 模型选择 end_turn 时 | 注入补充任务（最重要！见下） |
| `SubagentSpawn` | 要派子代理前 | 修改传给子代理的参数、审计 |
| `SubagentReturn` | 子代理返回 | 摘要再压缩、存档 |

### 7.2.3 工具层

| Hook | 触发时机 | 可做 |
|---|---|---|
| `PreToolUse(tool, input)` | 权限通过、执行前 | 修改 input（sanitize）、拒绝、记录审计 |
| `PostToolUse(tool, input, output)` | 执行后、注入回 history 前 | 修改 output（mask 敏感）、触发副作用 |
| `OnToolError(tool, input, error)` | 工具失败 | 提供替代 tool_result、重试策略 |

### 7.2.4 文件/代码层（可选）

| Hook | 触发时机 | 可做 |
|---|---|---|
| `PreFileWrite(path, content)` | `fs_write` / `fs_edit` 前 | 跑 lint/formatter |
| `PostFileWrite(path)` | 文件写完后 | `git add`、diff 展示 |

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Set up hooks"；[Claude Agent SDK - Hooks](https://docs.claude.com/en/api/agent-sdk/hooks)；Claude Code `restored-src/src/hooks/*`。

## 7.3 Hook 形状（规范）

### 7.3.1 签名

```ts
type HookContext = {
  sessionId: string
  event: Event                  // 触发此 hook 的事件
  meta: { agentRole, trace_id, ... }
}

type HookResult =
  | { kind: 'pass' }                        // 不改变流程
  | { kind: 'rewrite', payload: unknown }   // 修改即将发生的数据
  | { kind: 'block', reason: string }       // 阻止后续步骤
  | { kind: 'inject', messages: Message[] } // 注入新消息（仅 Stop/UserPromptSubmit 可用）

type Hook = (ctx: HookContext) => Promise<HookResult>
```

### 7.3.2 注册与优先级

- 注册：`harness.registerHook(point, hook, {priority: number, source: 'session'|'project'|'workspace'})`
- 同一 point 有多个 hook → 按优先级顺序串行执行；一旦 `block` 立刻短路
- `rewrite` 的输出作为下一个 hook 的输入（链式）

## 7.4 最常见四类 hook 用法

### 7.4.1 `Stop`：让代理"别太早收工"

Claude Code `restored-src/src/query/stopHooks.ts` 实现了 stop hooks：模型说 `end_turn`，harness 仍可以：

- 查看是否所有 `TodoWrite` 的项都 `done`
- 检查是否有 `skill` 说"任务后必须跑 lint"
- 注入一段 system：_"还有以下未完成: [...]，请继续。"_

避免模型"话说了一半"。

### 7.4.2 `UserPromptSubmit`：自动注入上下文

例：

- 用户粘贴 issue URL → hook 用 `github.fetch_issue` 预先查到正文，作为附加 user content 注入
- 用户问代码 → hook 根据 path 预先 `Read` 相关文件

### 7.4.3 `PreToolUse(shell_exec)`：审计与禁用

例：

- 记录 audit log (`runtime/events/audit.jsonl`)
- 正则匹配禁止字段（例：`.env.production`）→ `block`
- 把命令发给 classifier，若"高风险"则 `block` 并返回"请分解成更小步骤"

### 7.4.4 `PostFileWrite`：格式化与提交

例：

- 写完 TS 文件自动 `prettier`
- 写完 `.sql` 自动 `sqlfluff lint`
- 每次写完 `git add`（不 commit；留给用户）

## 7.5 Hook 与 Permissions 的协同

```
event: tool_use 到来
   ↓
PreToolUse hooks         （可 rewrite input / block）
   ↓
Permission gate          （can-use-tool 决策）
   ↓
tool.execute()
   ↓
PostToolUse hooks        （可 rewrite output / 追加副作用）
```

**规则**：PreToolUse 不能绕过 deny；但能把 "ask" 变成 "allow" 吗？**不能**。Hooks 只能更严，不能更松。

> 这是 Octopus 的硬约束（本仓安全原则）；Claude Code 允许 hook "允许"某些情景但伴随白皮书级审计 —— Octopus 默认关闭此扩展。

## 7.6 Hook 配置源（Source）

同权限：`session > project > plugin > workspace > defaults`。

配置示例（`.agent/hooks.yaml`）：

```yaml
hooks:
  PreToolUse:
    - match: { tool: shell_exec }
      script: scripts/hooks/audit-shell.js
      priority: 10
    - match: { tool: fs_write, pathGlob: "apps/**/*.ts" }
      script: scripts/hooks/prettier-check.ts
      priority: 20
  PostFileWrite:
    - match: { pathGlob: "packages/schema/**" }
      script: scripts/hooks/regen-openapi.ts

  Stop:
    - script: scripts/hooks/todos-not-complete.ts
      priority: 100
```

### Hook 运行环境

- 本地 Node/Python 脚本（最常用）
- MCP tool（复用已有能力）
- 内嵌 JS（针对轻量场景）

## 7.7 错误与超时

- Hook 执行默认超时 10s
- Hook 异常不阻塞核心流程（除非返回 `block`）；写 error 到事件流
- Hook 不可自 spawn 子代理（防嵌套爆炸）

## 7.8 Hooks 对 VibeCoding 的价值

VibeCoding 强调"流式协作"；Hooks 在此特别有用：

- 自动把命令输出显示为卡片（PostToolUse 渲染）
- 自动把编辑的文件显示为 diff（PostFileWrite）
- 自动在 UI 打断点（PreToolUse 触发一次用户确认）

## 7.9 Octopus 落地约束

- Hooks 由 `packages/hooks` 统一管理；项目/工作区级配置遵循 `config/runtime` 结构
- Hook 写事件必须符合 `runtime/events/*.jsonl` append-only
- 敏感 hook（例：执行 git 提交）必须经过 workspace allow 方可启用
- 禁止 hook 直接访问底层 SQLite/Blob，必须通过 Session 接口

## 7.11 Hook 来源与插件

Hook 的**注册来源**与 §7.6 的**优先级**是两个正交维度。按来源分，hook 可来自以下四类：

| 来源 | 载体 | 生命周期 | 说明 |
|---|---|---|---|
| **session** | 运行时临时注册 | session 结束即销毁 | 调试、一次性拦截 |
| **project** | `.agent/hooks.yaml`（项目根） | 与项目同寿命 | 团队共享的守护规则 |
| **workspace** | `config/runtime/workspace.json` 或 `workspace.hooks.*` | 与工作区同寿命 | 管理员层面的合规/审计 |
| **plugin** | 插件 manifest 的 `hooks` 字段 | 随插件启用/禁用一并激活或回收 | 由第三方扩展功能自带的守护行为 |

- **Plugin 来源的 hook** 与 §7.6 的优先级叠加：插件声明的 hook 在合并后参与同一条优先级链，插入在 `project` 与 `workspace` 之间，不享受"插件默认胜出"。
- 当插件被禁用时，其注册的所有 hook **必须**一并回收；不允许遗留"孤立 hook"。
- 插件不得直接写 `packages/hooks` 的内部实现；它只能通过 Plugin API 声明 hook（见 [12 §12.3 扩展点全景](./12-plugin-system.md)、[12 §12.5 Manifest 规范](./12-plugin-system.md) 与 [12 §12.8.4 Register](./12-plugin-system.md)）。

> 为什么单列：Hooks 是 Octopus 少数**既是核心契约、又是 plugin 扩展点**的能力；不在本章点出插件来源，会导致 07 章看起来"只认 project/workspace 两来源"，与 12 §12.3 表格产生口径冲突。

## 7.10 反模式

| 反模式 | 症状 | 纠正 |
|---|---|---|
| **Hook 做重业务逻辑** | 维护地狱，逻辑分散在 hook + 工具 + prompt | 业务逻辑放 tools/subagents，hooks 只做守护 |
| **Hook 调用 LLM** | 延迟放大；成本不可控 | 少量 hook 可接分类器 LLM，其它用规则 |
| **Hook 无超时** | 卡死 session | 强制 10s 默认超时 |
| **Hook 改历史 turn** | 破坏 prompt cache | 只允许改"即将发生"的数据，不改既成历史 |

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Claude Agent SDK · Hooks](https://docs.claude.com/en/api/agent-sdk/hooks) | Hook 点清单、签名 |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Set up hooks" | 典型用法 |
| Claude Code restored src `hooks/*`, `query/stopHooks.ts` | 实现参考 |
| 本仓 `AGENTS.md` §Persistence Governance | 审计流、runtime/events 约束 |
