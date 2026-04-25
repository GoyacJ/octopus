# ADR-0018 · 不引入 Loop-Intercepted Tools（反向决议）

> 状态：Accepted（**反向决议** · 显式选择不做）
> 日期：2026-04-25
> 决策者：架构组
> 关联：ADR-0011（Tool Capability Handle）、ADR-0004（Agent/Team 拓扑）、ADR-0016（Programmatic Tool Calling）、ADR-0017（Steering Queue）、`crates/harness-engine.md`、`crates/harness-tool.md`、`crates/harness-subagent.md`、`extensibility.md`

## 1. 背景与问题

### 1.1 候选模式：Loop-Intercepted Tools

业内多家 Agent 实现 (Hermes / Claude Code) 把若干"系统副作用类"工具直接捏在主循环里、绕过普通的 ToolOrchestrator 流水线，例如：

| 系统 | 候选工具 | 主循环耦合形态 |
|---|---|---|
| Hermes | `todo`, `memory`, `session_search`, `delegate_task`, `clarify`, `send_message` | `AIAgent.run_conversation` 命中这几个 sentinel name 后**直接走主循环里写好的内联分支**，不走通用 tool dispatch（HER-008） |
| Claude Code | `AgentTool`, `TaskStop`, `SendMessage` | coordinator prompt 里只允许这几类 tool；`queryLoop` 内联 hooks 处理结果（CC-12） |
| OpenClaw | `sessions_*`, `subagents`, `agents_list` | core 持有，channel 插件无法替换；走专门的 RPC 帧而非 tool RPC（OC-27 / OC-28） |

这种模式**收益**是：

- 主循环可以在不经过 hook / permission / budget 全套流水线的前提下"立即生效"
- 实现简单，无需额外抽象

但**代价**是：

- "Tool 一等公民"破裂为"普通 Tool + 主循环超能力 Tool"两层抽象
- 主循环代码里散落 N 个 `if name == "todo" { ... }` 分支，KISS 严重违反
- Hook 链 / Permission 链 / Budget 链都要在每个内联分支处复制一遍，否则就**直接绕过**安全模型
- 测试矩阵爆炸：每个内联工具都要单独构造 mock 主循环

### 1.2 候选清单（如果引入 Octopus 会动什么？）

经审视 Hermes / CC 的清单，对应到 Octopus 现有内置工具：

| 候选 | Octopus 现状 | 备注 |
|---|---|---|
| `todo_write` | 已实现为普通 `TodoTool` + `ToolCapability::TodoStore`（ADR-0011） | 走 ToolOrchestrator |
| `clarify` | 已实现为 `ClarifyTool` + `ToolCapability::ClarifyChannel`（v1.3） | 走 ToolOrchestrator |
| `send_message` | 已实现为 `SendMessageTool` + `ToolCapability::UserMessenger`（v1.3）| 走 ToolOrchestrator |
| `delegate / agent` | 已实现为 `AgentTool` + `ToolCapability::SubagentRunner`（ADR-0011） | 走 ToolOrchestrator |
| `task_stop` | 已实现为 `TaskStopTool` + `ToolCapability::RunCanceller` | 走 ToolOrchestrator |
| `memory_write / recall` | `harness-memory` 拆分为 `MemoryStore + MemoryLifecycle`，Tool 层借 `ToolCapability::MemdirWriter`（v1.4） | 走 ToolOrchestrator |
| `session_search` | 业务可基于 `harness-journal` projection 自定义（非内置） | 走 ToolOrchestrator（如有） |

**Octopus 已经把 Hermes / CC 的"loop-intercepted"用例全部用 `ToolCapability` 借用模式覆盖**：Tool 是普通 Tool，但通过 capability 反向借用高权限子系统（todo store / messenger / subagent runner / memdir writer 等），实现完全等价。

## 2. 决策

**Octopus SDK 不引入任何 loop-intercepted tools。所有 Tool 一律走 `ToolOrchestrator` 统一流水线。**

延续既有原则：

1. **Tool 是一等公民**：注册表 / 流水线 / Hook 链 / Permission 链 / Budget 链对所有 Tool 一视同仁
2. **高权限副作用通过 `ToolCapability` 借用**：而非把 Tool 实现塞进主循环
3. **主循环只做编排，不做业务**：见 P3（Single Loop, Single Brain）

### 2.1 等价能力对照

| Hermes / CC 的"主循环超能力" | Octopus 的对等模式 |
|---|---|
| 主循环看到 `todo` 直接 mutate todo list | `TodoTool` + `ToolCapability::TodoStore` 句柄；`TodoStoreCapAdapter` 把内核句柄收敛为窄 trait（ADR-0011 §2.6） |
| 主循环看到 `clarify` 直接对 UI 推 elicitation | `ClarifyTool` + `ToolCapability::ClarifyChannel`；`Event::ClarifyRequested` 走 EventStream |
| coordinator 主循环只允许特定 tool 集 | 通过 `BuiltinToolset::Coordinator` + `Subagent.required_capabilities` 表达 |
| 主循环 monkey-patch tool 行为 | `harness-hook` 5 介入点（`PreToolUse / TransformToolResult / PostToolUse / ...`，v1.6）|

### 2.2 何时该破例？

**默认不破例**。如未来确实出现"`ToolCapability` 抽象无法表达"的场景，必须满足全部条件方可立 ADR 推翻本决议：

1. 给出明确反例：哪个语义无法用"普通 Tool + ToolCapability"表达
2. 证明在不破坏 P3（Single Loop / Single Brain）与 ADR-0011 的前提下没有等价方案
3. 显式给出主循环新分支的 Hook / Permission / Budget 等价路径
4. 提供量化收益（典型至少 30% 端到端延迟下降）+ 受益场景占比（≥ 主流场景 20%）

## 3. 不采纳的替代方案

### 3.1 引入"主循环超能力 Tool 注册表"

- ❌ 与 ADR-0011 `CapabilityRegistry` 重叠：相同的"内核内部资源借用"语义
- ❌ 维护两套 Tool 抽象（普通 / loop-intercepted）违反 KISS
- ❌ 测试矩阵翻倍

### 3.2 仅对"内置工具"开放 loop-intercepted

- ❌ 内置/插件二分本应由 `TrustLevel` 表达；引入 loop-intercepted 等于在 trust 维度之外再搞一套权限分层
- ❌ 与 `extensibility.md §3` "Tool 一等公民"原则冲突

### 3.3 把 `ToolOrchestrator` 拆为"快路径 + 慢路径"

- ❌ 假设性能问题，无证据；现有 ToolOrchestrator 在大并发下已经验证 ≤ 5ms overhead
- ❌ 双路径意味着双测试矩阵、双反模式列表

### 3.4 把 PTC（ADR-0016）当作 loop-intercepted 入口

- ❌ ADR-0016 已显式定位 PTC 是普通 Tool（只是输入是脚本），仍走 ToolOrchestrator 全套
- ❌ 如果允许 PTC 内部跳过 hook / permission，PTC 会变成绕过安全模型的后门

## 4. 影响

### 4.1 正向

- Tool 抽象保持单层，KISS、可教、可审计
- 新增工具只需要决定 `ToolCapability` 借用什么、`DenyPattern` 命中什么、`ResultBudget` 如何配置；不需要回头改主循环
- 主循环代码量保持小（13~14 个阶段，14 阶段含 ADR-0017 Steering）
- 与 ADR-0011 形成正向反馈：`ToolCapability` 越完善，loop-intercepted 的吸引力越低

### 4.2 代价

- 部分极小的"性能差"（一次 Hook chain 走完约 0.5-2ms）将一直存在；本 ADR 接受这个代价
- 业务侧若试图"复刻 Hermes 主循环"，会发现走不通，需要重新理解 `ToolCapability`；本 ADR 把"为什么"沉淀下来减少重复讨论

## 5. 落地清单

| 项 | 责任文档 | 说明 |
|---|---|---|
| 在 `extensibility.md §3` 末尾补一段"反向决议：不引入 loop-intercepted tools" | `extensibility.md` | 引用本 ADR；与 ADR-0011 / ADR-0016 形成三角引用 |
| 在 `harness-tool.md §11 反模式` 中补一项 "loop-intercepted tools" | `crates/harness-tool.md` | 与既有反模式同级 |
| `comparison-matrix.md` R-22 回填 | `reference-analysis/comparison-matrix.md` | → ADR-0018 |
| 不修改任何 Tool / Engine / Subagent SPEC（本 ADR 是反向决议）| —— | —— |

## 6. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| HER-008 | `reference-analysis/evidence-index.md` | Hermes 主循环 sentinel-name 内联分支 |
| HER-014 | 同上 | `DELEGATE_BLOCKED_TOOLS` 与子 agent 黑名单（同时是"loop-intercepted = 难审计"的反例） |
| CC-12 | 同上 | Claude Code coordinator 内联控制；不构成本 ADR 推荐路径 |
| OC-27 / OC-28 | 同上 | OpenClaw `sessions_spawn / sessions_*` 在 core 内的内联实现 |
| ADR-0011 | `adr/0011-tool-capability-handle.md` | 用 capability 借用替代 loop-intercepted 的核心机制 |
| ADR-0004 | `adr/0004-agent-team-topology.md` | Subagent/Team 边界 + blocklist 已通过普通 Tool + capability 表达 |
| ADR-0016 | `adr/0016-programmatic-tool-calling.md` | PTC 仍是普通 Tool；不是 loop-intercepted 的入口 |
