# Octopus Built-in Capability Catalog / ToolSearch Exposure Runtime Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

让 Octopus 的 built-in tools 从平铺注册表升级为 role-aware 的 `BuiltinCapabilityCatalog`，并让 `ToolSearch`、deferred tool 暴露、运行时持久化与后续 turn request 装配形成一条完整闭环：deferred capability 只有在 discovery 被记录后才会成为模型可调用工具，并且该状态可恢复、可审计、可回放。

## Architecture

本次实现采用 hybrid source-of-truth。`SessionCapabilityState` 继续承担热路径决策与 replan 输入，但同时新增 durable 的 discovery / exposure projection，作为 session resume、runtime projection、artifact serialization 与后续 provider request assembly 的恢复依据。最终目标不是重写 `ToolSearch`，而是把它从“select 后激活工具”的局部逻辑，升级为“发现 -> 激活 -> 暴露 -> 调用”的正式 runtime state machine。

在这条链路之前，先对 built-in capability 做 clean-cutover：把当前 `mvp_tool_specs()` 的平铺定义和 `builtin_exec.rs` 的名称分发，替换为带分类、角色可见性与 handler 归属的 `BuiltinCapabilityCatalog`。worker primitives、control-plane built-ins、orchestration built-ins 必须先在 catalog 层被区分，planner 和 ToolSearch 才有稳定的所有权边界。

项目当前仍处于开发期，因此本计划明确采用 clean-cutover 思路：如果现有状态结构、projection 字段、transport shape 或恢复链路与目标架构冲突，应直接替换为正确模型，而不是继续叠加长期兼容层。允许一次性迁移、数据重建或测试基线更新，但不接受为了保留不合理旧抽象而引入双轨逻辑。

Claude Code 仅作为机制参考，而不是实现模板。Octopus 可以保留 `ToolSearch select:<tool>` 这一用户可见语法，但不能把 Anthropic `tool_reference` message 形状或 TypeScript 模块切分直接当作内部 canonical model；内部唯一真相必须是 Octopus 自己的 built-in catalog 与 exposure projection。

## Tech Stack

- Rust workspace: `crates/tools`, `crates/octopus-runtime-adapter`
- OpenAPI contract: `contracts/openapi/src/**`
- Generated transport types: `packages/schema/src/generated.ts`
- Verification: `cargo test`, `pnpm openapi:bundle`, `pnpm schema:generate`, `pnpm schema:check`

## Reference Inputs

实现时必须先读以下 Claude Code 源码锚点，再动对应任务。借鉴的是机制、边界和状态迁移，不是照抄 JS 结构或 API wire shape。

- built-in catalog 与单点装配
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts`
  - `getAllBaseTools()`：`193-250`
  - `getTools()`：`271-327`
  - `assembleToolPool()`：`345-367`
- `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`
  - `Tool`：`362-699`
  - `ToolUseContext`：`158-339`
  - `buildTool()`：`721-792`

- role / mode-aware 可见性与 agent 过滤
- `docs/references/claude-code-sourcemap-main/restored-src/src/constants/tools.ts`
  - `ALL_AGENT_DISALLOWED_TOOLS`：`36-46`
  - `ASYNC_AGENT_ALLOWED_TOOLS`：`55-71`
  - `COORDINATOR_MODE_ALLOWED_TOOLS`：`107-112`
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/agentToolUtils.ts`
  - `filterToolsForAgent()`：`70-115`
  - `resolveAgentTools()`：`122-224`

- deferred tool 定义、ToolSearch 选择与 discovered state 提取
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/prompt.ts`
  - `isDeferredTool()`：`53-107`
  - `getPrompt()`：`119-120`
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/ToolSearchTool.ts`
  - `ToolSearchTool`：`304-470`
  - `select:` 直选路径：`358-405`
  - `tool_reference` 结果映射：`439-469`
- `docs/references/claude-code-sourcemap-main/restored-src/src/utils/toolSearch.ts`
  - `isToolSearchEnabled()`：`385-473`
  - `extractDiscoveredToolNames()`：`545-580`

- tool schema 暴露与 request assembly
- `docs/references/claude-code-sourcemap-main/restored-src/src/utils/api.ts`
  - `toolToAPISchema()`：`119-225`
  - `normalizeToolInput()`：`566-681`
  - `normalizeToolInputForAPI()`：`685-717`
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts`
  - ToolSearch 开关、discovered 过滤、`defer_loading` schema 构造：`1118-1267`
  - model-aware tool-search 字段清理：`1269-1284`

- dispatch-time guardrail 与 compaction / resume 保持 discovered state
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts`
  - `buildSchemaNotSentHint()`：`578-597`
  - validation error 上附加 retry hint：`614-630`
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/compact/compact.ts`
  - `preCompactDiscoveredTools` 挂到 compact boundary：`596-611`

## Current Repo Anchors

- `crates/tools/src/tool_registry.rs`
  - `mvp_tool_specs()` 仍是 built-in 定义主入口。
  - `execute_tool_search()` 与 searchable projection 仍然直接依赖平铺 spec。
- `crates/tools/src/builtin_exec.rs`
  - `execute_tool()` 仍以裸字符串 `match` 选择 handler。
- `crates/tools/src/capability_runtime/state.rs`
  - 当前只有 `activated_tools`，无法表达 discovered / exposed 分层。
- `crates/tools/src/capability_runtime/provider.rs`
  - 仍然同时承担 built-in 装配、surface 计算与 ToolSearch 结果拼装。
- `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
  - 当前 summary / projection 仍以 `visible_capabilities` + `activated_tools` 为主。
- `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
  - pending tool execution 直接基于 `visible_capabilities` 解析执行目标。
- `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
  - request assembly 仍然依赖一次性 `visible_capabilities`，不是 durable exposure projection。

## Scope

- In scope:
- `crates/tools` 内部的 discovery / exposure 状态模型、planner surface、ToolSearch 选择副作用、执行期 guardrail。
- `crates/tools` 内部的 built-in capability catalog、single assembly point、role-aware availability 与 built-in handler ownership。
- `crates/octopus-runtime-adapter` 的 capability projection、capability state snapshot、runtime persistence、run/session summary、resume/replay 路径。
- `contracts/openapi` 与 `@octopus/schema` 中 runtime capability summary / snapshot 的 transport contract。
- `ToolSearch select:<tool>` 后下一轮暴露、未暴露 deferred tool 的 retry hint、相关回归测试。
- 用新的 exposure state model 替换现有过粗的 `activated_tools` 近似语义，必要时同步删除不合理的旧字段或旧派生路径。
- 用新的 built-in catalog 替换现有 `mvp_tool_specs() + builtin_exec.rs match` 的平铺结构，必要时同步删除不合理的旧 helper 与旧字段假设。
- Out of scope:
- `prompt_skill` / `resource` 的 discovery 机制重构。
- Claude Code 式 transcript compaction / `tool_reference` 完整复刻。
- UI catalog 搜索、桌面页面展示、tool result pairing 修复层的全面重建。
- `AgentTool` / subrun / team orchestration 的单独协议重构。
- 为已判定不合理的内部状态结构维持长期双写、双读或别名兼容层。
- 为平铺 built-in 注册表、裸字符串分发或历史命名继续保留长期兼容入口。

## Risks Or Open Questions

- `discoveredTools`、`exposedTools`、`exposureSnapshot`、`schemaHash` 的公开字段命名需要在实现前冻结，避免先改内部状态后反复改 transport contract。
- 需要确认 transport 是否公开逐工具 `schemaHash`；如果当前前端没有消费场景，可以先公开名称级 snapshot，把 hash 保持为内部持久化字段。
- 需要确认 ToolSearch 发现是否立即视为 `activated`，还是区分 “discovered but not activated” 与 “activated and exposed” 两级状态；本计划默认保留两级。
- 需要冻结 built-in 分类与角色暴露元数据，至少区分 worker primitives、control-plane built-ins、orchestration built-ins，以及 `main_thread_only`、`async_worker_allowed`、`plan_mode_only`、`non_interactive_blocked` 等暴露约束。
- 若实现过程中发现现有 runtime session resume 并不会重建 tools block，而是完全重算 surface，需要优先修正恢复链路，不得只在单次 turn 内修补。
- 如果旧 snapshot / projection 结构与目标状态机冲突，默认策略是一轮 cutover 或一次性迁移，而不是引入长期兼容 fallback。

## Execution Rules

- 不要把 discovery / exposure 逻辑散落在 `executor.rs`、`agent_runtime_core.rs`、`persistence.rs` 各自维护副本；必须先确定单一状态模型。
- 不要只补一个 `ToolSearch -> activate_search_selection()` 的局部补丁后就宣布完成；必须连同 projection、resume、transport 一起闭环。
- 不要继续把 built-ins 维持成“注册表定义在一处，字符串分发在另一处，角色裁剪散落多处”的结构；必须先建立 single assembly point。
- 参考 Claude Code 时只迁移机制，不照搬 TypeScript class 形态、feature-flag 矩阵或 `tool_reference` wire 结构；Octopus 内部 canonical state 必须是自身 projection。
- 任何新增 runtime transport 字段都必须先改 `contracts/openapi/src/**`，再运行 bundle / schema 生成，不得手改生成文件。
- 若 provider request assembly 无法明确区分 “activated but not yet exposed” 与 “already exposed to model”，停止实现并先确认状态机。
- 如果现有字段、快照文件、summary contract 与目标架构冲突，优先删除或替换旧结构，不为开发期内部实现保留长期兼容分支。
- 允许一次性重建测试基线、更新序列化快照或清理派生字段；不允许为了“先跑起来”继续增加历史包袱。
- 不要把 control-plane built-ins 当成普通 worker tool 处理；`ToolSearch`、`SendUserMessage`、`StructuredOutput`、`EnterPlanMode`、`ExitPlanMode`、`Config` 等必须在 catalog 层拥有独立分类与暴露策略。

## Global Exit Condition

本计划完成的判断标准不是“ToolSearch 能跑”，而是：built-ins 已经切换到 catalog 单点装配；planner / provider / executor / request assembly 共用同一套 exposure-aware projection；`ToolSearch select:<tool>` 会留下可持久化、可恢复、可回放的 discovered / activated / exposed 状态；未暴露 deferred tool 的直接调用会稳定返回 retry hint；resume、compact、approval 恢复后仍能重建同一轮工具可见面。

## Task Ledger

### Task 1: 重建 `BuiltinCapabilityCatalog` 与 built-in 单点装配

Status: `done`

Files:
- Create: `crates/tools/src/builtin_catalog.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/builtin_exec.rs`
- Modify: `crates/tools/src/capability_runtime/planner.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/tools/src/lib.rs`
- Test: `crates/tools/src/split_module_tests.rs`

Reference anchors:
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts`
  - `getAllBaseTools()`：built-in 单点定义
  - `getTools()`：按 mode / deny rule 裁剪
  - `assembleToolPool()`：built-in + MCP 单点合并
- `docs/references/claude-code-sourcemap-main/restored-src/src/constants/tools.ts`
  - role / mode 可见性集合，不与工具定义混写
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/agentToolUtils.ts`
  - `filterToolsForAgent()`：worker / async / main-thread 约束
- `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`
  - `Tool` / `buildTool()`：工具元数据和默认行为集中定义

Preconditions:
- 采用 clean-cutover：不保留平铺 built-in 注册表与名称分发的长期双轨逻辑。
- 先冻结 built-in 分类和角色暴露元数据，再进入 ToolSearch / exposure 状态机改造。

Step 1:
- Action: 在 `crates/tools/src/builtin_catalog.rs` 定义 built-in capability catalog，显式声明 built-in 分类、角色可见性、权限、默认 visibility 与 handler key。
- Done when: `mvp_tool_specs()` 不再承担 built-in 的完整语义来源；worker primitives、control-plane built-ins、orchestration built-ins 在 catalog 层被明确区分。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: 需要同时把 plugin / MCP / skill 也迁入同一 catalog 才能完成 built-in cutover，导致任务边界失控。

Step 2:
- Action: 让 `provider.rs` 和相关 surface assembly 从 `BuiltinCapabilityCatalog` 读取 built-ins，形成 built-in 的 single assembly point，而不是继续从多个 helper 拼接。
- Done when: built-in capability 的 planner 输入、ToolSearch 搜索面与最终 visible/deferred surface 都共享同一套 catalog 元数据。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: `provider.rs` 仍然需要依赖旧的裸 `ToolSpec` 列表才能决定角色可见性，说明 catalog 边界定义不完整。

Step 3:
- Action: 用 catalog-backed handler resolution 替换 `builtin_exec.rs` 中不合理的裸字符串 `match` 分发，至少让 built-in metadata 与执行 handler 拥有稳定映射关系。
- Done when: built-in dispatch 不再依赖散落的字符串常量和重复命名假设，control-plane built-ins 的执行入口可由 catalog 元数据追溯。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: 需要为了兼容旧接口继续长期保留 catalog dispatch 和字符串 match 两套入口。

Notes:
- 本任务完成后，后续 ToolSearch / exposure 任务都以新的 built-in catalog 为基础继续，不回头兼容旧平铺结构。
- 借鉴 Claude Code 的是“分类 + 可见性 + 单点装配”机制，不是把 feature flag 分支数量原样搬进 Octopus。
- Current batch:
  - 执行位置：Task 1 Step 1 -> Step 3
  - 当前状态：已完成。built-ins 已切到 `BuiltinCapabilityCatalog` 单点装配；`planner.rs` 的默认 visibility 改由 catalog 提供；`builtin_exec.rs` 已按 catalog handler key 分发，不再依赖裸字符串 `match`。

### Task 2: 建立 `CapabilityExposureSnapshot` 与 discovery 状态模型

Status: `done`

Files:
- Create: `crates/tools/src/capability_runtime/exposure.rs`
- Modify: `crates/tools/src/capability_runtime/mod.rs`
- Modify: `crates/tools/src/capability_runtime/state.rs`
- Modify: `crates/tools/src/capability_runtime/planner.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/tools/src/tool_registry.rs`
- Modify: `crates/tools/src/lib.rs`
- Test: `crates/tools/src/split_module_tests.rs`

Reference anchors:
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/prompt.ts`
  - `isDeferredTool()`：deferred 判定不等于“搜索结果”，而是独立 metadata
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/ToolSearchTool.ts`
  - `select:` 与 keyword search 只返回最小发现结果
- `docs/references/claude-code-sourcemap-main/restored-src/src/utils/toolSearch.ts`
  - `isToolSearchEnabled()`：ToolSearch 开关和模型能力 gating
  - `extractDiscoveredToolNames()`：message history -> discovered state 提取
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts`
  - `discoveredToolNames` 参与 visible/deferred surface 计算

Preconditions:
- Task 1 已经冻结 built-in catalog、分类和单点装配边界。
- 采用 hybrid source-of-truth：`SessionCapabilityState` 是热路径状态，durable exposure snapshot 是恢复与审计状态。
- 字段命名在实现开始前确认，至少区分 `discovered`、`activated`、`exposed`。

Step 1:
- Action: 在 `crates/tools/src/capability_runtime/exposure.rs` 定义统一的 discovery / exposure 数据结构，并在 `state.rs` 中为 session state 增加相应字段、访问器、序列化与默认值。
- Done when: session state 能表达 deferred tool 的 `discovered`、`activated`、`exposed` 状态，不再只剩 `activated_tools` 这一类近似语义。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: 需要把 `prompt_skill` 或 `resource` 同时纳入同一状态机，导致本批实现边界失控。

Step 2:
- Action: 在 `planner.rs` 与 `provider.rs` 中把 exposure snapshot 纳入 effective surface 计算，明确 `visible_tools`、`deferred_tools` 与 `activated_tools` 的衍生规则。
- Done when: planner 可以基于 session state + exposure snapshot 解释为什么工具处于 deferred、visible 或 hidden，而不是只依赖 allowlist 与 activated 集合。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: planner 需要依赖 adapter 层或 transport 层数据才能决定 surface，破坏 `crates/tools` 的所有权边界。

Step 3:
- Action: 更新 `tool_registry.rs` 的 searchable projection，使 ToolSearch 结果能返回 discovery / exposure 所需最小元数据，并补齐 `split_module_tests.rs` 的状态转换断言。
- Done when: ToolSearch 输出与内部 state model 语义一致，测试能区分 “可搜索但未暴露” 与 “已暴露到下一轮 surface”，且搜索结果不重复携带完整执行 schema 或 UI-only 冗余字段。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: ToolSearch 结果被要求继续兼容旧的激活语义或旧 transport 命名，导致新状态模型被迫退化。

Notes:
- 本任务完成前，不进入 OpenAPI 和 adapter projection 改动。
- Claude Code 用 `tool_reference` block 传递 discovered names；Octopus 只需要保留等价语义，不需要把该 wire 结构本身变成内部 truth。
- Current batch:
  - 执行位置：Task 2 Step 1 -> Step 3
  - 当前状态：已完成。`SessionCapabilityState` 已切到 `CapabilityExposureSnapshot`；planner 仅在 `exposed` 或 `granted` 时把 deferred tool 提升到 visible surface；`ToolSearch` 结果现在显式返回 `discovered` / `activated` / `exposed` 元数据，并保留已暴露 deferred tool 的可搜索性。

### Task 3: 把 ToolSearch 选择与 deferred tool 调用变成正式执行链路

Status: `done`

Files:
- Modify: `crates/tools/src/capability_runtime/events.rs`
- Modify: `crates/tools/src/capability_runtime/executor.rs`
- Modify: `crates/tools/src/capability_runtime/provider.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Test: `crates/tools/src/split_module_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`

Reference anchors:
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/ToolSearchTool.ts`
  - `select:` 直选语义与 no-op 选择已加载工具的处理
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts`
  - `buildSchemaNotSentHint()`：dispatch-time retry hint
- `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`
  - `queryLoop()`：tool call 并不直接修改工具池，而是通过下一轮 request assembly 生效

Preconditions:
- Task 1 已冻结 built-in catalog 与角色可见性边界。
- Task 2 已经冻结 discovery / exposure 状态模型与命名。
- 现有 `ToolSearch select:<tool>` 仍然是合法入口，不更换用户可见语法。

Current batch:
- 执行位置：Task 3 Step 1 -> Step 3
- 当前状态：已完成。`ToolSearch select:<tool>` 现在会留下 selection detail，并显式写入 discovered / activated / exposed 状态；未暴露 deferred tool 的直接调用会稳定返回 `ToolSearch select:<tool>` retry hint，同时发出 failed capability event；runtime adapter 端到端验证了该 guard 不会污染下一轮 surface。

Step 1:
- Action: 在 `executor.rs` 中把 `ToolSearch` 的副作用从“只激活工具”升级为“记录 discovered / activated transition，并发出可持久化的 execution event / state update”。
- Done when: `ToolSearch select:<tool>` 后，runtime 内部可以区分该工具是被发现、被激活，还是已经暴露给模型。
- Verify: `cargo test -p tools split_module_tests`
- Stop if: 发现现有 execution hook / mediation hook 无法承载 discovery transition 事件，需要先重构事件总线，而不是把 transition 状态偷偷塞回 adapter 临时逻辑。

Step 2:
- Action: 为 deferred tool 直接调用但 schema 未暴露的场景增加 deterministic guardrail，返回明确 retry hint，而不是仅给出通用参数或 unavailable 错误。
- Done when: 任何未暴露 deferred tool 的直接调用都会返回指向 `ToolSearch select:<tool>` 的错误信息，且不会污染 capability state。
- Verify: `cargo test -p tools split_module_tests` 
- Verify: `cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Stop if: 现有 runtime model driver 会吞掉 tool failure detail，导致 hint 无法稳定进入 transcript / artifact。

Step 3:
- Action: 在 adapter 侧 pending tool execution 路径中复用同一套 exposure guard，而不是绕过它直接执行 `visible_capabilities`。
- Done when: 首轮只有 `ToolSearch`、二轮暴露所选工具、未暴露 deferred tool 无法被 pending execution 绕过，相关 runtime 测试覆盖通过。
- Verify: `cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Stop if: pending tool execution 当前依赖旧的 `visible_capabilities` 快照，必须先调整 resume / rebuild 流程才能正确实现。

### Task 4: 持久化 exposure projection，并把它接入 runtime transport contract

Status: `done`

Files:
- Modify: `contracts/openapi/src/components/schemas/runtime.yaml`
- Regenerate: `contracts/openapi/octopus.openapi.yaml`
- Regenerate: `packages/schema/src/generated.ts`
- Modify: `packages/schema/src/capability-runtime.ts`
- Modify: `crates/octopus-runtime-adapter/src/capability_state.rs`
- Modify: `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- Modify: `crates/octopus-runtime-adapter/src/run_context.rs`
- Modify: `crates/octopus-runtime-adapter/src/persistence.rs`
- Modify: `crates/octopus-runtime-adapter/src/lib.rs`
- Test: `crates/octopus-runtime-adapter/src/runtime_compatibility_tests.rs`

Reference anchors:
- `docs/references/claude-code-sourcemap-main/restored-src/src/utils/toolSearch.ts`
  - `extractDiscoveredToolNames()`：discovered set 必须可从历史恢复
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/compact/compact.ts`
  - `preCompactDiscoveredTools`：compaction 后仍能恢复 discovered state
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts`
  - `discoveredToolNames` 过滤后再生成本轮 tools block

Preconditions:
- Task 2 和 Task 3 已确定公开最小字段集合。
- 公开 contract 先以 runtime summary / snapshot 为中心，不引入新的独立 API endpoint。

Current batch:
- 执行位置：Task 4 Step 1 -> Step 3
- 当前状态：已完成。runtime transport 已公开 `discoveredTools` / `exposedTools`；OpenAPI bundle、generated schema、adapter projection 与 legacy payload 兼容测试全部对齐。

Step 1:
- Action: 在 `runtime.yaml` 中扩展 `RuntimeCapabilityPlanSummary` 与 `RuntimeCapabilityStateSnapshot`，加入 exposure 相关字段；随后运行 OpenAPI bundle 与 schema 生成。
- Done when: OpenAPI human source、bundled artifact、generated schema 对 exposure 语义达成一致，且没有手改生成文件；公开 contract 只暴露运行时需要消费的稳定字段，不泄露 handler key、内部 enum 或调试态冗余数据。
- Verify: `pnpm openapi:bundle`
- Verify: `pnpm schema:generate`
- Verify: `pnpm schema:check`
- Stop if: 需要在多个 runtime response schema 之间复制同一组字段而无法复用现有 schema 组件。

Step 2:
- Action: 在 `capability_state.rs`、`capability_planner_bridge.rs`、`run_context.rs`、`persistence.rs` 中持久化 exposure snapshot，并让 run/session summary 使用同一份 projection。
- Done when: capability state snapshot、DB projection、run context、session summary 读取到的 exposure 数据一致，不再只有 `activated_tools` 级别的粗粒度信息。
- Verify: `cargo test -p octopus-runtime-adapter runtime_compatibility_tests`
- Verify: `cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Stop if: 当前 SQLite projection 或 snapshot 文件格式与目标结构冲突且需要长期双格式读取；这时应先设计一次性 cutover 或迁移。

Step 3:
- Action: 更新 `packages/schema/src/capability-runtime.ts` 导出与调用侧引用，删除或替换已经不合理的旧字段假设，并把 runtime compatibility 测试更新到新的 canonical shape。
- Done when: transport alias、runtime serialization、调用侧消费都以新的 exposure-aware contract 为准，不再为了开发期旧 shape 保留长期兼容分支。
- Verify: `cargo test -p octopus-runtime-adapter runtime_compatibility_tests`
- Verify: `pnpm schema:check`
- Stop if: 需要同时维护新旧两套 capability summary/snapshot shape 超过一次性迁移窗口。

### Task 5: 让 request assembly、resume 与回放统一依赖 exposure projection

Status: `done`

Files:
- Modify: `crates/octopus-runtime-adapter/src/agent_runtime_core.rs`
- Modify: `crates/octopus-runtime-adapter/src/session_service.rs`
- Modify: `crates/octopus-runtime-adapter/src/execution_events.rs`
- Modify: `crates/octopus-runtime-adapter/src/subrun_orchestrator.rs`
- Modify: `crates/octopus-runtime-adapter/src/memory_selector.rs`
- Test: `crates/octopus-runtime-adapter/src/capability_runtime_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/approval_runtime_tests.rs`
- Test: `crates/octopus-runtime-adapter/src/actor_runtime_tests.rs`

Reference anchors:
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts`
  - `filteredTools` + `toolSchemas`：每轮 request assembly 由当前 discovered set 决定
  - `normalizeMessagesForAPI(messages, filteredTools)`：request payload 与 visible tool set 一起收敛
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/compact/compact.ts`
  - compact boundary 挂 discovered snapshot，避免 resume / compact 丢失已加载工具
- `docs/references/claude-code-sourcemap-main/restored-src/src/utils/toolSearch.ts`
  - history scan 只是一种恢复手段，真正的长期依赖应收敛到 durable projection

Preconditions:
- Task 4 已把 exposure snapshot 纳入 run/session summary 与 capability state snapshot。
- 请求装配仍由 adapter 单点负责，不能把 tools block 生成逻辑分叉到多个 runtime 入口。

Step 1:
- Action: 在 `agent_runtime_core.rs` 与 `session_service.rs` 中改造 request assembly，使当前轮发给模型的工具集合来自 exposure-aware projection，而不是只看一次性 `visible_capabilities`。
- Done when: 新会话、继续运行、resume、approval/auth 恢复后，都能稳定复现相同的 visible/deferred 结果，不依赖内存中偶然保留的 planner 结果。
- Verify: `cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Stop if: 当前模型请求装配与 capability projection 之间没有单点边界，必须先抽取装配函数再继续。

Current batch:
- 执行位置：Task 5 Step 1 -> Step 3
- 当前状态：已完成。request assembly、approval/auth replay、subrun 默认 summary 与回归测试现在统一依赖 exposure-aware capability projection；checkpoint replay 优先使用持久化 `capability_state_ref`，subrun 默认 ref 命名已与主 runtime 对齐。

Step 2:
- Action: 在 `execution_events.rs`、`subrun_orchestrator.rs`、`memory_selector.rs` 等衍生路径里补齐新字段传播，确保 event timeline、subrun 默认 summary、memory selection 不会丢失 exposure 状态或因新增字段崩溃。
- Done when: 事件流、默认 run summary、memory selector 对新增 capability summary 字段全部兼容，且不会把 exposure snapshot 误当作 UI-only metadata 丢弃。
- Verify: `cargo test -p octopus-runtime-adapter approval_runtime_tests`
- Verify: `cargo test -p octopus-runtime-adapter actor_runtime_tests`
- Stop if: 某些路径仍依赖旧 summary 语义且只能通过保留旧字段兜底；这时应先收敛 canonical projection，再更新派生视图。

Step 3:
- Action: 用 end-to-end 回归测试覆盖完整链路：首轮只暴露 `ToolSearch`，`select:` 后下一轮暴露目标工具，未暴露 deferred tool 直接调用得到 retry hint，resume 后仍保持同一 exposure 结果。
- Done when: tools crate、runtime adapter、approval/resume 相关测试都覆盖 discovery -> activation -> exposure -> execution 的完整闭环。
- Verify: `cargo test -p tools split_module_tests`
- Verify: `cargo test -p octopus-runtime-adapter capability_runtime_tests`
- Verify: `cargo test -p octopus-runtime-adapter approval_runtime_tests`
- Verify: `cargo test -p octopus-runtime-adapter runtime_compatibility_tests`
- Stop if: 测试只能在单个 driver stub 下通过，而无法在真实 runtime persistence 场景复现。

## Batch Checkpoint Format

After each batch, append a short checkpoint using this shape:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed: 定义 exposure snapshot 结构，并接入 planner surface
- Verification:
  - `cargo test -p tools split_module_tests` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 1
```

## Checkpoint 2026-04-19 10:31

- Batch: Task 1 Step 1 -> Task 1 Step 3
- Completed: 新增 `crates/tools/src/builtin_catalog.rs` 作为 built-in 唯一真相；`mvp_tool_specs()` 改为从 catalog 派生；`CapabilityProvider` 改为持有 catalog 并以它做 built-in 冲突校验、surface 编译输入和本地执行判断；`builtin_exec.rs` 改为通过 catalog handler key 解析 canonical tool/alias。
- Verification:
  - `cargo test -p tools builtin_capability_catalog_ -- --nocapture` -> pass
  - `cargo fmt --all` -> pass
  - `cargo test -p tools split_module_tests` -> pass (103 passed)
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-19 10:52

- Batch: Task 2 Step 1 -> Task 2 Step 3
- Completed: 新增 `crates/tools/src/capability_runtime/exposure.rs` 并把 `SessionCapabilityState` 改为持有 `CapabilityExposureSnapshot`；补齐 `discover_tool`、`activate_discovered_tool`、`expose_tool` 访问器和 store 转发；planner/provider 改为以 `exposed` 或 `granted` 作为 deferred tool 提升条件；`ToolSearch` searchable projection 现在返回 `discovered`、`activated`、`exposed` 三层状态，并允许已暴露 deferred tool 继续被检索到。
- Verification:
  - `cargo test -p tools session_capability_state_tracks_discovery -- --nocapture` -> pass
  - `cargo test -p tools session_exposure_snapshot_controls -- --nocapture` -> pass
  - `cargo test -p tools session_capability_store_persists_and_restores_shared_runtime_state -- --nocapture` -> pass
  - `cargo fmt --all` -> pass
  - `cargo test -p tools split_module_tests` -> pass (104 passed)
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-19 11:28

- Batch: Task 3 Step 1 -> Task 3 Step 3
- Completed: `crates/tools/src/capability_runtime/executor.rs` 中的 `ToolSearch select:<tool>` 副作用从旧 `activate()` 近似语义改为显式记录 discovered / activated / exposed，并把 selection detail 写入完成事件；未暴露 deferred tool 现在会返回稳定的 `ToolSearch select:<tool>` retry hint，且不会污染 capability state；runtime loop 侧验证了 pending tool execution 会复用同一 guard，而不会把 deferred capability 偷跑进 visible surface。
- Verification:
  - `cargo test -p tools tool_search_select_records_selection_detail_and_exposure_state -- --nocapture` -> pass
  - `cargo test -p tools deferred_tool_direct_call_returns_retry_hint_without_state_mutation -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter submit_turn_direct_deferred_tool_call_returns_retry_hint_without_exposing_tool -- --nocapture` -> pass
  - `cargo fmt --all` -> pass
  - `cargo test -p tools split_module_tests` -> pass (106 passed)
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests` -> pass (12 passed)
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-19 12:13

- Batch: Task 4 Step 1 -> Task 4 Step 3
- Completed: `RuntimeCapabilityPlanSummary`、`RuntimeCapabilitySummary` 与 `RuntimeCapabilityStateSnapshot` 已公开 `discoveredTools` / `exposedTools`；adapter 侧 summary/snapshot projection 会持久化并回读这两组字段；legacy payload 缺失新字段时通过默认值保持兼容。
- Verification:
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `pnpm schema:check` -> pass
  - `cargo test -p octopus-runtime-adapter runtime_compatibility_tests` -> pass (10 passed)
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests` -> pass (12 passed)
- Blockers:
  - none
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-19 12:41

- Batch: Task 5 Step 1
- Completed: 为 approval replay 增加 exposure-aware 回归测试，覆盖“聚合态 capability_state_ref 被污染后，resume 仍应继续使用 checkpoint capability snapshot”场景；`agent_runtime_core.rs` 的 approval/auth checkpoint loader 现已优先采用 checkpoint 自带的 `capability_state_ref`，避免 request assembly 在恢复时退回错误 projection。
- Verification:
  - `cargo test -p octopus-runtime-adapter resolve_approval_replays_selected_deferred_tool_from_checkpoint_capability_state -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests` -> pass (13 passed)
- Blockers:
  - none
- Next:
  - Task 5 Step 2

## Checkpoint 2026-04-19 13:08

- Batch: Task 5 Step 2 -> Task 5 Step 3
- Completed: `subrun_orchestrator.rs` 的默认 `capability_state_ref` 命名已切到与主 runtime 一致的 `{run_id}-capability-state` 形状，并补充断言默认 `discovered_tools` / `exposed_tools` 为空；approval replay 回归测试进一步校验 `approval.resolved` 事件会携带 exposure-aware summary；端到端验证覆盖了首轮仅暴露 `ToolSearch`、`select:` 后下一轮暴露 deferred tool、未暴露直接调用返回 retry hint，以及 replay / compatibility 保持同一 exposure 结果的完整闭环。
- Verification:
  - `cargo test -p octopus-runtime-adapter test_subrun_uses_runtime_capability_state_ref_shape_and_empty_exposure_fields -- --nocapture` -> pass
  - `cargo test -p octopus-runtime-adapter capability_runtime_tests` -> pass (13 passed)
  - `cargo test -p octopus-runtime-adapter approval_runtime_tests` -> pass (14 passed)
  - `cargo test -p octopus-runtime-adapter actor_runtime_tests` -> pass (3 passed)
  - `cargo test -p tools split_module_tests` -> pass (106 passed)
  - `cargo test -p octopus-runtime-adapter runtime_compatibility_tests` -> pass (10 passed)
- Blockers:
  - none
- Next:
  - 进入 `finishing-a-development-branch` 收尾流程
