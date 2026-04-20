# 05 · 子代理与多代理编排（Sub-agents & Multi-agent Orchestration）

> "The lead agent analyzes the query, develops a strategy, and spawns subagents to explore different aspects simultaneously. Subagents act as intelligent filters…"
> — [Anthropic · How we built our multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)（2025）

本章定义多代理架构的**两种规范模式**、**何时选择哪种**、**上下文隔离与信息流**。

## 5.1 Why：为什么要多代理

### 5.1.1 三大好处

1. **突破单窗口容量**：N 个子代理 = N 个上下文窗口 → 信息处理量 ≈ N 倍
2. **并行加速**：Anthropic 报告研究类任务端到端耗时 ↓90%
3. **关注点隔离**：每个子代理有自己的 system prompt / tool 白名单 / 评估标准；主代理看不到中间噪声

### 5.1.2 三大代价

1. **Token 成本暴涨**：多代理比单代理**多 ~15×** token 用量（Anthropic 内测数据）
2. **协调复杂度**：主代理可能误派、子代理可能越界
3. **调试难度**：并发路径比线性对话难理解

### 5.1.3 适用 / 不适用

**适用**：

- 广度优先研究（search 多个来源）
- 大代码库扫描（并行读 100+ 文件）
- 多候选方案生成（GAN 式 Generator × N）
- 不同评估维度独立打分（Evaluator 的分工）

**不适用**：

- 强顺序依赖（必须串行的任务）
- 高写冲突（多个 agent 要写同一文件）
- 预算敏感（15× token 代价无法承担）

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"When to use multi-agent"。

## 5.2 模式 A：Orchestrator-Workers

### 5.2.1 形状

```
   ┌─────────────┐
   │ Lead Agent  │  plan → dispatch → synthesize
   └──────┬──────┘
          │
   ┌──────┼────────┬──────────┐
   ▼      ▼        ▼          ▼
 sub_1  sub_2    sub_3  ... sub_N        (parallel, isolated contexts)
   │      │        │          │
   ▼      ▼        ▼          ▼
  res_1  res_2   res_3  ...  res_N
   └──────┴──────┬─┴──────────┘
                 ▼
           ┌──────────┐
           │ Lead synthesis │ → final answer
           └──────────┘
```

### 5.2.2 Lead Agent 的任务

1. **Plan**：把用户目标拆成可并行的子任务（推荐 `TodoWrite` 记录）
2. **Dispatch**：对每个子任务调 `task` / `delegate` 工具，传入：
   - 子任务清晰描述
   - 输出格式要求（对齐后续 synthesize）
   - 工具白名单（默认收窄）
   - effort budget hint（`quick` / `medium` / `very thorough`）
3. **Synthesize**：聚合所有子代理的 summary，形成最终回答

### 5.2.3 子代理的任务

- 独立上下文窗口，不感知兄弟姐妹
- 只看到自己的 system prompt + 父代理传入的任务
- **返回 condensed summary**，不 dump 全部过程
- 如果需要继续深入，**由自己决定**是否再 spawn 孙子代理（深度默认 ≤ 2）

### 5.2.4 实现提示

来自 Anthropic 的实际经验（[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)）：

| 教训 | 对策 |
|---|---|
| **教 orchestrator 如何委派** | 在 lead prompt 里写清楚：任务描述、输出格式、工具建议、边界条件 |
| **effort scaling** | 简单任务 1 个子代理；复杂任务最多 10 个 |
| **"start wide, then narrow"** | 子代理先广泛探索，再聚焦高相关结果 |
| **并行工具调用** | 子代理内部也应 parallelize 自己的工具调用 |
| **guide thinking**（extended thinking） | 让子代理在开始前写一段 `<thinking>` 规划 |

### 5.2.5 Claude Code 对应实现

- `restored-src/src/tools/AgentTool/AgentTool.ts`：暴露给模型的 `task` 工具
- `restored-src/src/coordinator/coordinatorMode.ts`：协调器模式
- 子代理的 `AgentDefinition` 可 YAML 声明（`agents/<name>.md` frontmatter）

> 来源：Claude Code `restored-src`；Cursor 文档中 `Task` 工具的行为（见本文顶端 tool signature 注释）。

## 5.3 模式 B：Generator ↔ Evaluator（GAN 式）

来自 [Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps)（2026）。

### 5.3.1 形状

```
┌──────────┐  writes full spec   ┌──────────────┐
│ Planner  │ ─────────────────▶  │  Generator   │
└──────────┘                     │   (coder)    │
     ▲                           └──────┬───────┘
     │                                  │
     │ feedback loop                    ▼
     │                           ┌──────────────┐
     └───────────────────────────│  Evaluator   │
                                 │ (Playwright) │
                                 └──────────────┘
```

- **Planner**：把用户 prompt 展开为完整 spec（从 1 句话到 10 页）
- **Generator**：按 spec 实现；不自评
- **Evaluator**：启动 app 或渲染 UI，用 Playwright 抓截图/日志，按 4 维打分并给出**具体修改建议**

### 5.3.2 为什么要独立 Evaluator

**关键发现**（Anthropic）：

> "Don't let the generator critique its own work — even when prompted to."

LLM 对自己刚写的代码过度宽容；必须**独立的 Evaluator agent**，且：

- 跑在**独立上下文**（不看 Generator 的推理过程）
- 用**具体可验证**的评估标准（渲染出的截图、测试输出）
- 输出 `pass/fail + reasons + action items`

### 5.3.3 评估维度（前端应用示例）

```
{
  "design_quality": 0..1,
  "originality":    0..1,
  "craft":          0..1,   // 响应式、动效、细节
  "functionality":  0..1,   // 按钮能点、路由通、数据正常
  "overall_pass":   true/false,
  "next_actions": [ "fix ...", "improve ...", ...]
}
```

### 5.3.4 Sprint Contract

对于大功能，Generator 与 Evaluator 在开工前**共同签署 sprint contract**：

- 本 sprint 要做什么（具体范围）
- 完成定义（how we'll know it's done）
- 禁区（不要改 X）

**注意**：Anthropic 发现**随着模型变强（Opus 4.6），sprint construct 可以被移除**。这印证了 §4.1.1 的"补偿代码会 staleness"。

## 5.4 上下文流动规则

### 5.4.1 父 → 子

- **只给必要信息**（任务本身 + 必要背景 + 输出格式 + 工具白名单）
- 不转发整个父对话（那就相当于父窗口+子窗口，污染了 isolation 价值）
- 可给 `scratchpad_hint`：子代理如果发现某些重要信息，写到公共记忆（例如 `NOTES.md`）

### 5.4.2 子 → 父

**三种回传形式**：

| 形式 | 适用 | 例子 |
|---|---|---|
| **Summary（文本）** | 默认 | "找到了 3 个相关文件，最可能的是 X，理由是..." |
| **Structured JSON** | 上下游需要机读 | `{issues:[...], severity:...}` |
| **File ref** | 输出很大 | `{notes_path: 'runtime/notes/subagent-abc.md'}` |

大输出**必须走 file ref**，避免污染父窗口。

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Subagent output to filesystem"。

### 5.4.3 兄弟之间不直接通信

默认：兄弟子代理相互不感知。需要协作时：

- 通过父代理中转
- 或通过共享 `NOTES.md` / Session 里的 durable state

避免子代理之间的双向 tight coupling；父代理永远是 orchestrator。

## 5.5 并发与预算

### 5.5.1 并发

- 子代理默认 **max 5 并发**（Anthropic 研究系统默认）
- 超出则排队
- 每个子代理自己的工具并发不共享父计数

### 5.5.2 Token Budget

- 父代理需要给每个子代理设 `taskBudget.total`（字段命名与 01 §1.2、`QueryParams.taskBudget` 保持一致；YAML frontmatter 中可使用 snake_case 别名 `task_budget`，解析层需完成 snake_case ↔ camelCase 映射）。
- `quick` ~ 10k tokens, `medium` ~ 40k, `very thorough` ~ 150k（示例基准）。
- 子代理的 `checkTokenBudget` 在命中 `COMPLETION_THRESHOLD` 后收尾。

> 来源：Claude Code `restored-src/src/query/tokenBudget.ts`（单代理已有的机制）；Anthropic 内部实践。

### 5.5.3 "Effort Scaling"

Lead agent 要**根据用户问题的复杂度**选 effort 级别：

- 简单事实问题 → 不派子代理，直接答
- 中等 → 1–3 子代理
- 大型研究 → 5–10 子代理，可能嵌套

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Scale effort to query complexity"。

## 5.6 Agent Definition（静态声明）

本 SDK 支持用 Markdown + frontmatter 声明子代理：

```md
---
name: code-reviewer
description: 审阅 PR diff，给出 blocking/non-blocking issues。
model: claude-sonnet-4-5
allowed_tools:
  - fs_read
  - fs_grep
  - fs_glob
  - ask_user_question
max_turns: 20
task_budget: 40000    # YAML snake_case；解析为 taskBudget.total（见 §5.5.2）
---

# 任务指引

你是 ...
- 先看 diff
- 然后看相关测试
- 最后给出结构化报告 JSON: {blocking:[], suggestions:[]}
```

registry 扫描 `.agents/**/*.md`；`task` 工具按 name 找到并激活。

> 来源：Claude Code `restored-src/src/Tool.ts` 的 `ToolUseContext.agentDefinitions`；[Claude Agent SDK – Subagents](https://docs.claude.com/en/api/agent-sdk/subagents)。

## 5.7 多代理系统的运维难题

### 5.7.1 Emergent behavior

多代理系统会出现**任何单代理行为无法解释的现象**：

- 大量子代理同时调用某个 MCP → 打爆 rate limit
- 子代理间互相引用同一个错误结论
- 长尾失败（1/50 次特殊顺序触发 bug）

### 5.7.2 Tracing

**最小可观测性套件**：

- 每个父-子调用链用 `trace_id` + `span_id` 串起
- 所有 tool_use 埋 `agent_role`（lead / research_sub / eval_sub / ...）
- 聚合统计：token 消耗、并发深度、失败率、尾延迟

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Production reliability and engineering challenges"。

### 5.7.3 Rainbow deployment

有状态 agent 不能"一次全切到新版本"：

- 运行中的会话留在旧版本直至完成
- 新会话走新版本
- 长时任务可能跨越多个部署周期

> 来源：同上，§"Rainbow deployments"。

## 5.8 常见反模式

| 反模式 | 症状 | 纠正 |
|---|---|---|
| **过度委派** | 任何小任务都 spawn 子代理 | 只对真能受益于并行/隔离的任务用 |
| **透明转发** | 父代理把自己的整个历史塞给子代理 | 只给任务本身 + 必要上下文 |
| **子代理无约束** | 无工具白名单、无预算、无输出格式 | 硬性约束全部三项 |
| **父代理单线程等** | 一次只派一个子代理 | 并行 spawn，利用 Promise.all |
| **自评估** | Generator 自己判定 "pass" | 独立 Evaluator，且用外部验证（Playwright） |
| **永远单层** | 不支持嵌套子代理 | 允许子代理也 spawn（深度 ≤ 2） |
| **无 tracing** | 出 bug 无法复现 | 强制 trace_id 贯穿 |

## 5.9 Octopus 落地约束

- 子代理 session 也必须符合本仓 `AGENTS.md` 的 Session 存储规则（JSONL + SQLite）
- 子代理**禁止**自主修改 workspace config（仅父代理通过 `runtime_config` API）
- 子代理的工具白名单必须比父代理**严格**（最小权限原则）
- `trace_id` 存入 `runtime/events/*.jsonl`，用于审计

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | 本章主干：orchestrator-workers、effort scaling、production 挑战 |
| [Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps) | Generator-Evaluator（GAN 式）、sprint contract |
| [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents) | Workflow vs agent 分类、评估-优化模式 |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) | Subagent 的日常用法 |
| [Claude Agent SDK – Subagents](https://docs.claude.com/en/api/agent-sdk/subagents) | `task` 工具、Agent Definition 规范 |
| Claude Code restored src `tools/AgentTool/*`、`coordinator/coordinatorMode.ts` | 实现参考 |
| Hermes `tools/delegate_tool.py` | 另一种 delegate 实现参考 |
