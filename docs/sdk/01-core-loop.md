# 01 · Agent 核心循环（Core Loop）

> "Agents are LLMs autonomously using tools in a loop." — Anthropic
> [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

本章定义 Octopus Agent Harness 的**单进程单会话**执行语义。多代理编排见 [`05-sub-agents.md`](./05-sub-agents.md)。

## 1.1 循环形状（Canonical Shape）

```
while not done and budget_remaining:
    response = model.sample(
        system=system_prompt,
        messages=history,
        tools=tool_schemas,
    )
    history.append(response)
    emit_event(assistant_message=response)

    if response.stop_reason == "end_turn":
        check_stop_hooks()          # 可注入补充任务
        if no_stop_hook_intercepts():
            return response         # 终止

    if response.stop_reason == "tool_use":
        tool_calls = partition(response.tool_uses)  # 并发 vs 串行
        for batch in tool_calls:
            results = await Promise.all(
                execute(tc, permission_gate) for tc in batch
            )
            history.extend(results)
            emit_event(tool_results=results)

    if budget_should_compact():
        history = compact(history)  # 摘要 + 保留关键 anchor
```

### 形状层面的六个关键点

1. **停止原因驱动**：循环以 `stop_reason` 判定流转，不是 "是否还有 tool_use"。这样能自然集成 Anthropic API 的 `end_turn` / `tool_use` / `max_tokens` / `stop_sequence` 语义。
2. **工具批处理**：一次模型响应中的多 tool_use **必须**先做 "可并发分区" 再执行。只读工具并发，写工具串行。
3. **Stop hooks**：`end_turn` 到达时给 hook 一次"继续任务"的机会；若 hook 返回新 user/system 消息，不结束循环。
4. **Budget 双线**：`max_turns`（轮数）与 token budget（消耗预算）同时监控。
5. **Compaction 在循环内部**：不是外部定时任务；根据 token 水位由循环自己触发。
6. **事件流**：每一步都 `emit_event` 到 Session，Brain 崩溃可从 Session 重放恢复。

### 参考实现

| 项目 | 文件 | 说明 |
|---|---|---|
| Claude Code（还原源） | `restored-src/src/query.ts` | 核心 `query()` 生成器；处理 compaction、fallback、token budget 的入口 |
| Claude Code | `restored-src/src/services/tools/toolOrchestration.ts` | `partitionToolCalls` / `runToolsConcurrently` / `runToolsSerially` |
| Claude Code | `restored-src/src/services/tools/toolExecution.ts` | 单次工具调用的权限 + 执行 + 计时 |
| Claude Code | `restored-src/src/query/tokenBudget.ts` | `BudgetTracker` + `checkTokenBudget` |
| Hermes | `hermes/run_agent.py` | `AIAgent.run_conversation()` 同步循环；兼容 Anthropic / OpenAI 两种 schema |
| Hermes | `hermes/agent/prompt_builder.py` | 系统提示词装配 |
| Hermes | `hermes/agent/context_compressor.py` | 长会话压缩 |

## 1.2 停止条件（Termination）

一个循环可以因为以下任一原因终止：

| 原因 | 触发 | 回放语义 |
|---|---|---|
| `end_turn` + 无 stop-hook 拦截 | 模型自认为已完成 | 正常终止，Session 保留 `final_message` |
| `max_turns` 到达 | 硬上限（默认 120，可配置） | 返回 `TurnLimitError` 给调用者；Session 可 resume |
| Token budget 耗尽 | `checkTokenBudget` 返回 `stop` 或 `diminishing_returns` | 发射 `budget_exhausted` 事件；下一轮必须 compaction 才能继续 |
| `FallbackTriggeredError` | `overloaded_error` / `prompt too long` | 模型切换（fallback model）或触发 compaction |
| `AbortSignal` | 外部取消 | 优雅中止进行中的工具调用；标记 `cancelled` |
| Permission decision=`deny` 且不可恢复 | 用户拒绝关键工具 | 返回 `permission_denied` 作为最终消息 |

### Token Budget 公式（来自 Claude Code）

真实源码只有两个常量：

```3:4:docs/references/claude-code-sourcemap-main/restored-src/src/query/tokenBudget.ts
const COMPLETION_THRESHOLD = 0.9
const DIMINISHING_THRESHOLD = 500
```

语义（去掉注释后的决策伪代码，对应 `checkTokenBudget` line 45-93）：

```
if agentId || budget == null || budget <= 0:
    return stop (no completion event)         # subagent 不走该机制

turnTokens    = globalTurnTokens
deltaSinceLastCheck = turnTokens - tracker.lastGlobalTurnTokens

isDiminishing = tracker.continuationCount >= 3
             && deltaSinceLastCheck       <  DIMINISHING_THRESHOLD   # 500 tokens
             && tracker.lastDeltaTokens   <  DIMINISHING_THRESHOLD

if !isDiminishing && turnTokens < budget * COMPLETION_THRESHOLD:      # <90%
    tracker.continuationCount += 1
    return continue(nudgeMessage)

if isDiminishing || tracker.continuationCount > 0:
    return stop (with completion event, diminishingReturns = isDiminishing)

return stop (no completion event)
```

几个关键事实常被误读：

- 只有两个常量：`COMPLETION_THRESHOLD = 0.9`（百分比）与 `DIMINISHING_THRESHOLD = 500`（**token delta** 阈值，不是百分比）。**不存在** `MIN_CONTINUATION_TOKENS`。
- "diminishing returns" 的判定必须**至少已经连续推动 3 次**（`continuationCount >= 3`），且**最近两次增量**都 < 500 tokens，才判定为"继续下去也榨不出东西"。
- 预算未触顶（`turnTokens < budget * 0.9`）且非 diminishing 时才继续；否则收尾。
- 子代理（有 `agentId`）或 `budget<=0` 时直接 stop（不触发 completion event）。

> 本 SDK 建议：直接采用这两个常量；允许通过 feature flag 覆盖。不引入未出现在源码中的第三个阈值。

## 1.3 消息装配（Message Assembly）

### 1.3.1 "Three-Layer Prompt"

Anthropic 官方建议的提示分层结构：

1. **Identity / System**：代理身份 + 总体目标 + 长期约束（例：安全红线、回复风格）
2. **Tool Guidance**：工具集说明（实际由 tools 字段承载；额外的 tool 使用指引放 system 段）
3. **Turn context**：本轮目标 / 当前任务 / 用户输入

> 来源：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §"The anatomy of effective context"。

### 1.3.2 Goldilocks Zone（长度原则）

- **不要过度规约**（hard-code 逻辑到提示词会脆裂）
- **不要过度模糊**（指望模型无师自通）
- 提示词应该"足够具体以引导行为，足够灵活以给模型发挥空间"。

> 同上来源。

### 1.3.3 提示词分段约定

采用 **XML 标签 + Markdown 双轨**（Anthropic 官方 best practice）：

```xml
<role>
  你是 Octopus 的 Workspace Sidekick...
</role>

<capabilities>
  你可以调用 File / Shell / WebSearch 等工具...
</capabilities>

<guidelines>
  - 修改代码前必须先 Read
  - 凭据相关变更必须触发 approve-credentials hook
  ...
</guidelines>

<output_format>
  使用 Markdown；代码块需带文件路径...
</output_format>
```

Hermes `agent/prompt_builder.py` 采用类似分段。Claude Code 使用混合 Markdown+XML。

## 1.4 工具调用批处理（Tool Use Batching）

### 1.4.1 分区算法

真实算法**不是**"一次切成两大堆"（所有并发一堆、所有串行一堆）。Claude Code 按**相邻合并**策略把调用序列切成若干批次：连续可并发的合并为一批（调度时 `Promise.all`），遇到不可并发的工具立刻自成一批（串行执行），之后再继续聚合。这样保留了原调用顺序（对写依赖链至关重要），又在相邻只读区段拿到了并发收益。

```
# partitionToolCalls（连续相邻合并为批）
batches = []
current = { kind: 'concurrent', items: [] }

for tc in tool_uses:
    tool = registry.get(tc.name)
    safe = tool.isConcurrencySafe(tc.parsed_input)

    if safe:
        if current.kind == 'serial':          # 结束一个 serial 批
            batches.append(current)
            current = { kind: 'concurrent', items: [] }
        current.items.append(tc)              # 追加到当前 concurrent 批
    else:
        if current.items:                     # 先把前面的并发批收尾
            batches.append(current)
        batches.append({ kind: 'serial', items: [tc] })   # 串行批每个 1 个
        current = { kind: 'concurrent', items: [] }

if current.items:
    batches.append(current)

# 执行
for b in batches:
    if b.kind == 'concurrent':
        run_concurrently(b.items, max_concurrency=CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY)
    else:
        run_serially(b.items)
```

> 来源：Claude Code `restored-src/src/services/tools/toolOrchestration.ts` 的 `partitionToolCalls` + `runToolsConcurrently` + `runToolsSerially`。
> 默认 `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY=10`。
> 关键不变量：**批之间严格保持原顺序**；任何不可并发工具独占一批。

### 1.4.2 `isConcurrencySafe` 的契约

每个工具必须实现：

```
isConcurrencySafe(parsedInput): boolean
// 返回 true  → 幂等、无副作用、可并发
// 返回 false → 有写入/顺序依赖
```

**安全的**：`fs_read`、`fs_grep`、`fs_glob`、`web_search`、`web_fetch`、`get_*`
**不安全的**：`fs_write`、`fs_edit`、`shell_exec`（除非显式声明 read-only）

### 1.4.3 并发的两类益处

- **性能**：研究任务中并发 5 个子代理 + 并发 3 个工具，Anthropic 报告整体用时缩短 90%。
- **正确性**：把"能并发的尽量并发"强制 harness 先考虑正确性再考虑并发。

> 来源：[How we built our multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Parallel tool calling"

## 1.5 错误与重试

### 1.5.1 错误分类

| 类别 | 来源 | 默认行为 |
|---|---|---|
| Model API error（`overloaded_error` / 5xx） | 上游 | 指数退避 + 可选 fallback model；Claude Code 使用 `FallbackTriggeredError` |
| `prompt_too_long` | 上游 | 立即触发 compaction，重放最后一条 user message |
| Tool validation error | Hands | 作为 `tool_result.is_error=true` 反馈给模型，**不中断循环** |
| Tool 执行异常 | Hands 内部 | 同上；异常对象被 sanitize 成 string（去栈、去凭据） |
| Sandbox 崩溃 | Hands | 同上；"brain 不 nursing hands" |
| Permission denied | Permission Gate | `tool_result` 返回 "permission denied, reason=..." |
| AbortSignal | 外部 | 对所有进行中的工具发 cancel；循环干净退出 |

> 关键设计：**工具错误是模型的输入，不是异常**。模型会自行 try-catch 式地换策略。
> 来源：[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Hands (provisioning and execute)"

### 1.5.2 上游 API 错误处理

Claude Code 的 fallback 思路（`services/api/withRetry.ts` + `query.ts`）：

1. 遇到 5xx/overloaded → 指数退避 + 内部 `withRetry` 包装
2. 某个重试通道被路由到了 fallback 模型时，`withRetry` 抛出 `FallbackTriggeredError`（携带 `originalModel` / `fallbackModel` 元数据）
3. `query.ts` 捕获 `FallbackTriggeredError` 后切换到 `fallbackModel` 继续对话；仍失败则向外层策略上抛（提醒用户 / 放弃 / resume later）

> 来源：`restored-src/src/services/api/withRetry.ts:160`（`FallbackTriggeredError` 定义）、`services/api/withRetry.ts:347`（抛出点）、`restored-src/src/query.ts:894`（捕获与切换）。

### 1.5.3 `prompt_too_long` 的特殊语义

```
try model.sample(...)
catch PromptTooLongError:
    emit_event('pre_compact_forced')
    history = compact(history, reason='prompt_too_long')
    emit_event('post_compact_forced')
    retry once
```

> 来源：Claude Code `restored-src/src/query.ts`：`PROMPT_TOO_LONG_ERROR_MESSAGE`（line 42 定义 / line 643 注入）、`stop_reason: 'prompt_too_long'` 分支（line 1175、1182）。
> 官方设计依据：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §Compaction。

## 1.6 中断与恢复（Interruption & Resume）

### 1.6.1 中断

用户随时可以按 `Esc`：

- 正在流式接收：中断 stream，保留已生成的部分文本作为 `partial_message`
- 正在执行工具：通过 `AbortSignal` 取消；工具需响应 signal 释放资源
- `Esc Esc`：把会话 + 代码状态回滚到上一个 checkpoint

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Course-correct early and often" / §"Rewind with checkpoints"。

### 1.6.2 恢复

两层恢复语义：

1. **Continue**：`claude --continue`：不做任何重放、从最后状态续跑。
2. **Resume by ID**：`claude --resume <session-id>`：选择任意历史 session 拉起一份新的 Brain。
   - 重建：system prompt、tool registry、working dir、permissions、最近 N turns。
   - `getEvents(range)` 按需回填更远的历史。

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Resume conversations"；
> [Claude Agent SDK – Sessions](https://docs.claude.com/en/api/agent-sdk/sessions)；
> [Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Sessions"。

## 1.7 Plan Mode

### 1.7.1 语义

Plan Mode = "只读探索态"：模型可以读代码、grep、WebSearch，但**拒绝**任何写入/执行 side-effect 的工具。

用于"先规划再动手"的人机协作模式。

### 1.7.2 实现方式

- 权限系统提供 `prePlanMode` 记录上一个 mode；切到 plan 时临时把所有写工具 ban 掉。
- 退出 plan：把记录的 `prePlanMode` 还原。

> 来源：Claude Code `restored-src/src/Tool.ts` 的 `ToolUseContext.prePlanMode`；
> [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Separate exploration and planning from coding"。

### 1.7.3 工作流约定

Anthropic 官方推荐的"四步法"：

1. **Explore**：读相关代码 + WebSearch + 提问
2. **Plan**（Plan Mode）：输出可执行方案（推荐使用 `TodoWrite` 工具生成清单）
3. **Implement**：退出 Plan Mode，逐步执行
4. **Commit**：git commit + summary

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Best practices"。

## 1.8 非交互模式（Headless / Non-interactive）

### 1.8.1 使用场景

- CI/CD 中的"AI code review"
- Cron 定时任务
- 批量脚本（fan-out 多个问题给多个 Claude 实例）

### 1.8.2 I/O 约定

- **Input**：命令行参数或 stdin
- **Output**：结构化（JSON lines，便于下游脚本解析）
- 中断信号：SIGTERM 转 AbortSignal
- 默认不打开审批对话，使用 allowlist / auto mode

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Automate with non-interactive mode"。

## 1.9 Token Budget vs. Turn Budget vs. Task Budget

| 预算 | 作用域 | 作用对象 | 来源 |
|---|---|---|---|
| `maxOutputTokens` | 单次模型调用 | 限制 completion 长度 | Claude Code `QueryParams.maxOutputTokensOverride` |
| `maxTurns` | 一次 agent 会话 | 工具调用 + 模型回合总数 | `QueryParams.maxTurns`；默认 120 |
| `taskBudget.total` | 一次外部任务 | token 消耗上限；跨 turn 累加 | `QueryParams.taskBudget.total`；用于 `checkTokenBudget` |
| `iteration_budget` | Hermes 对应概念 | 类似 `maxTurns` | Hermes `run_agent.py` |
| Session-level soft cap | Managed Agents 层 | 超出时强制 compaction 或 spawn 子代理 | [Managed Agents](https://www.anthropic.com/engineering/managed-agents) |

**规范建议**：Octopus SDK 同时暴露这四层，默认只用 `maxTurns` + `taskBudget`，其他为可选。

## 1.10 最小事件模型（Event Shape）

Session 追加的事件类型（**最小集**，可扩展但不改语义）：

```ts
type Event =
  | { type: 'user_message',        content: UserContent }
  | { type: 'assistant_message',   content: AssistantContent, stop_reason, usage }
  | { type: 'tool_use',            id, name, input }
  | { type: 'tool_result',         tool_use_id, output, is_error, duration_ms }
  | { type: 'permission_decision', tool_use_id, decision, reason }
  | { type: 'plan_update',         todos: TodoItem[] }
  | { type: 'compact',             before_tokens, after_tokens, reason }
  | { type: 'subagent_spawn',      subagent_id, parent_id, task }
  | { type: 'subagent_return',     subagent_id, summary }
  | { type: 'checkpoint',          checkpoint_id, label }
  | { type: 'sandbox_lifecycle',   sandbox_id, phase: 'provision'|'ready'|'gone' }
  | { type: 'hook_invocation',     hook: string, result }
  | { type: 'error',               phase, message }
```

> 来源：本 SDK 综合 Claude Code Session layout（`~/.claude/agents/<id>/sessions/*.jsonl`）+ OpenClaw `~/.openclaw/agents/<id>/sessions/*.jsonl` + [Managed Agents](https://www.anthropic.com/engineering/managed-agents) Session 描述。

## 1.11 对 VibeCoding 的契合

VibeCoding（围绕 AI-assisted iterative dev）的关键诉求：

- **快速反馈**：`emit_event` 实时推到 UI，用户可在 UI 看到 plan/tool_use/tool_result 流动。
- **可回滚**：checkpoint + rewind 保证"试错成本低"。
- **可见计划**：`TodoWrite` 工具 + `plan_update` 事件让用户同步看到任务分解。
- **随时打断**：`Esc` 中断 + 重写 prompt 继续。

本章定义的循环完整支持这些模式。

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents) | 循环定义、简洁原则 |
| [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) | Goldilocks Zone、Compaction |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) | Plan Mode、Course-correct、Resume、Headless |
| [Managed Agents](https://www.anthropic.com/engineering/managed-agents) | Session 事件模型、错误即输入 |
| [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | 并发效益、Evaluation 基础 |
| Claude Code restored src | `query.ts`, `toolOrchestration.ts`, `toolExecution.ts`, `tokenBudget.ts`, `Tool.ts`（`ToolUseContext`, `QueryParams`） |
| Hermes Agent | `run_agent.py`, `agent/prompt_builder.py`, `agent/context_compressor.py` |
