# 14 · 未实现 / 最小化 / 兼容实现收口

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 本文件承接 `13-finalization-and-deferred-capabilities.md`。`13` 已完成 formal closeout，但代码里仍留有一批未实现 capability、最小化 scaffold、兼容式 shim、以及文档已承诺但当前仍缩水的公共契约。这个 tranche 不重开 W1–W8，也不推翻 `13` 的完成态，只把这些 residual gap 收口成一条可执行的下一轮计划。
>
> 阅读顺序：**本文件 →** `docs/plans/sdk/02-crate-topology.md` → `docs/plans/sdk/03-legacy-retirement.md` → `docs/plans/sdk/12-post-w8-capability-hardening.md` → `docs/plans/sdk/13-finalization-and-deferred-capabilities.md` → `docs/sdk/03-tool-system.md` → `docs/sdk/06-permissions-sandbox.md` → `docs/sdk/07-hooks-lifecycle.md` → `docs/sdk/08-long-horizon.md` → `docs/sdk/09-observability-eval.md` → `docs/sdk/11-model-system.md` → `docs/sdk/12-plugin-system.md` → `docs/sdk/14-ui-intent-ir.md` → `crates/octopus-sdk-tools/src/{spec.rs,tool.rs,context.rs,builtin/{mod.rs,w5_stubs.rs,web_search.rs}}` → `crates/octopus-sdk-contracts/src/{event.rs,permissions.rs,hooks.rs,ui_intent.rs,subagent.rs}` → `crates/octopus-sdk-permissions/src/{gate.rs,policy.rs}` → `crates/octopus-sdk-sandbox/src/spec.rs` → `crates/octopus-sdk-observability/src/{tracer.rs,usage.rs,replay.rs}` → `crates/octopus-sdk-model/src/{adapter/stubs.rs,catalog/builtin/{openai.rs,google.rs,minimax.rs},role_router.rs}` → `crates/octopus-sdk-plugin/src/{bundled.rs,lifecycle.rs,registry.rs,api.rs}` → `crates/octopus-platform/src/runtime_sdk/{builder.rs,plugin_boot.rs,registry_bridge/{builtins.rs,snapshot.rs}}` → `crates/octopus-sdk-core/src/{brain_loop.rs,tool_dispatch.rs}` → `crates/octopus-sdk-context/src/compact.rs` → `crates/octopus-sdk-session/src/{jsonl.rs,sqlite/schema.rs}` → `crates/octopus-cli/src/run_once.rs`。

## Status

状态：`in_progress`

## Active Work

当前 Task：`Task 2 · 收口 tool contract、tool exposure、UI intent 与 residual builtin tools`

当前 Step：`Step 1 · 审计结论已回填到 tool surface / prompt surface / model catalog / plugin bootstrap / file-write / long-horizon / compat 清单；待按 Task 2 Step 1 起批`

### Pre-Task Checklist（起稿阶段）

- [x] 已复核 `13-finalization-and-deferred-capabilities.md` 的完成态与 deferred freeze。
- [x] 已复核当前 residual 证据点：builtin tools、tool contract / UI intent、permissions / sandbox、hooks、observability、model adapters / builtin catalog、plugin boot、prompt cache / compaction、session compat、minimal scaffold。
- [x] 已复核 `docs/plans/sdk/AGENTS.md` 的编号、README 预登记、状态流转与模板要求。
- [x] 已识别本 tranche 涉及的公共面登记位置：`02-crate-topology.md §2.1 / §2.2 / §2.3 / §2.4 / §2.6 / §2.7 / §2.8 / §2.9 / §2.11 / §2.12 / §2.13 / §2.14 / §3.1 / §3.5 / §5 / §6`。
- [x] 已识别潜在退役登记位置：`03-legacy-retirement.md §7` 的新增小节。
- [ ] 当前 git 工作树干净；当前已存在 `docs/plans/sdk/README.md` 与本文件的起稿变更。
- [x] 已识别 `01-ai-execution-protocol.md §5` Stop Conditions，尤其是 #1 / #4 / #6 / #8 / #9 / #10 / #11。

## Task 1 Freeze Result

> 下表是后续 Task 2–Task 9 的唯一输入。后续任务只能执行，不再重新争论 scope、owner 或 live/compat 状态。

| 残余面 | 冻结状态 | 唯一 owner / 批次 | 约束 |
|---|---|---|---|
| `ToolSpec` / `ToolContext` / `RenderLifecycle` / `SessionEvent::Render` | `implement now` | `octopus-sdk-tools` + `octopus-sdk-contracts` + `octopus-sdk-core` / Task 2 | 不能再保留“类型已声明、runtime 只写最小子集”的双轨状态。 |
| request-time tool surface + `ToolSearch` / deferred exposure + prompt-side tool guidance | `implement now` | `octopus-sdk-core::brain_loop` + `octopus-sdk-context::prompt` + `octopus-platform::runtime_sdk::registry_bridge` / Task 2 | `request.tools` 与 `SystemPromptBuilder` 必须共用同一份 request-time assembled surface。 |
| builtin model catalog / defaults / role router / platform snapshot | `implement now` | `octopus-sdk-model` + `octopus-platform::runtime_sdk::registry_bridge` / Task 3 | `docs/references/vendor-matrix.md` 是唯一事实源；catalog/defaults/snapshot 不得继续各写一份 live truth。 |
| plugin bootstrap / bundled-local discovery / runtime-vs-decl-only 边界 | `implement now` | `octopus-sdk-plugin` + `octopus-platform::runtime_sdk::plugin_boot` / Task 4 | 必须是 manifest-first；不得继续从预载入插件代码反推 manifest。 |
| permissions / sandbox / hooks / tool execution governance | `implement now` | `octopus-sdk-permissions` + `octopus-sdk-sandbox` + `octopus-sdk-hooks` + `octopus-sdk-core` / Task 5 | `PreFileWrite / PostFileWrite` 必须挂到真实写路径 owner，不接受只扩枚举。 |
| session-owned query loop / continuation / retry / recovery + observability / subagent summary | `implement now` | `octopus-sdk-core` + `octopus-sdk-observability` + `octopus-sdk-subagent` / Task 6 | 主循环 contract 必须通过明确测试证明，不接受只靠整包测试碰运气。 |
| prompt cache / compaction / context reset-handoff | `implement now` | `octopus-sdk-core` + `octopus-sdk-context` / Task 7 | long-horizon 不是只有 compaction；`runtime/notes`、`runtime/todos` 与 `context_restored` 语义必须同批闭环，或同批 Fact-Fix 收窄。 |
| `RuntimeSdkDeps::minimal(...)`、`octopus-cli:minimal` 与 crate-level minimal scaffold 注记 | `test-only` | `octopus-platform` + `octopus-cli` + `octopus-sdk-{contracts,session,model}` / Task 8 | 生产主路径不得再以 minimal 入口或 minimal scaffold 注记作为默认答案。 |
| legacy session reader / wrapper / SQLite migration | `supported compat` | `octopus-sdk-session` / Task 8 | 仅服务旧 JSONL / SQLite 数据恢复；保留边界与测试必须显式登记。 |
| `shim_tool_context()` | `hide from live` | `octopus-sdk-tools::registry` -> `octopus-sdk-core::tool_dispatch` / Task 8 | 可以保留临时 compat 支架，但不能再充当 live execution owner。 |
| `hidden_builtin_model(...)` 与 hidden unsupported builtin metadata | `supported compat` | `octopus-platform::runtime_sdk::registry_bridge::{builtins,snapshot,overrides}` / Task 3 + Task 8 | 仅允许作为 `configuredModels` 的显式 compat projection，不得回流成 live catalog truth。 |

## Goal

把 SDK 当前仍存在的未实现 capability、最小化 scaffold、兼容式 shim、以及文档已承诺但实现仍缩水的 contract surface 收口成一套唯一执行答案：该补齐的补齐到真实 owner/runtime/contract 路径，该退出的退出 live path，该保留的 compat 明确冻结为窄合同并加测试。

## Non-goal

- 不回退 `13` 的 `done` 状态。
- 不在 `apps/desktop`、`octopus-server` 或 `octopus-cli` 做 page-local / transport-local workaround 去掩盖 shared-layer 缺口。
- 不新增与 residual closure 无关的新 provider family、plugin marketplace 能力或业务域功能。
- 不在本 tranche 之外并行起新的 SDK 未来计划文件。

## Architecture

- 这一轮的真实 owner 仍在 `octopus-sdk-*` 与 `octopus-platform`。live runtime、catalog、snapshot、event stream、session persistence、control docs 必须给出同一份答案，不能继续靠 scaffold、shim、hidden metadata 或缩水 contract 维持“看起来兼容”。
- 执行顺序固定为：先冻结 residual matrix，再按闭环收口 `tool contract -> tool exposure / request assembly -> UI intent / builtin live policy -> models -> plugins -> permissions / sandbox / hooks -> observability / replay / subagent coordination -> prompt cache / compaction / long-horizon -> minimal / compat 退役`。不按单个字段散修。
- compatibility path 不是默认合理存在。每一条 compat 代码都要回答三件事：服务哪个旧数据/旧入口、当前是否仍受支持、何时退役；答不出来就不能继续留在主路径。

## Scope

In scope：

- 收口 `octopus-sdk-tools` 中仍为 stub、hidden-only 或 shim-backed 的 builtin tool 路径。
- 收口 `octopus-sdk-tools` / `octopus-sdk-contracts` 中仍缩水的工具契约、`ToolUseContext` / `ToolPermissionContext` 边界、UI intent / `RenderLifecycle` / `SessionEvent` 渲染载荷。
- 收口 `octopus-sdk-core` / `octopus-sdk-tools` / `octopus-platform` 中仍缺失的 request-time tool surface、`ToolSearch` / deferred tool exposure、discover/expose state 与 runtime-visible tool assembly。
- 收口 `octopus-sdk-core` 中仍停留在最小实现的 session/query loop、stop hook continuation、token budget continuation、overflow / retry policy 与 request ownership。
- 收口 `octopus-sdk-model` 中仍为空 catalog、stub adapter、路由未齐的 model family / role routing。
- 收口 `octopus-sdk-plugin` 与 `octopus-platform` 中仍依赖 example/bundled placeholder 的 plugin runtime bootstrap。
- 收口 `octopus-sdk-permissions` / `octopus-sdk-sandbox` / `octopus-sdk-hooks` 中仍停留在最小子集的 mode / rule bucket / lifecycle / sandbox policy contract。
- 收口 `octopus-sdk-core` / `octopus-sdk-permissions` / `octopus-sdk-sandbox` 中仍缺失的 tool execution governance，包括 hook timing、permission denied / retry hint、sandbox provenance、progress / rejection / error writeback。
- 收口 `octopus-sdk-observability` / `octopus-sdk-contracts` / `octopus-sdk-subagent` 中仍缺失的 tracing / replay / evaluation event fields，以及 `coordinator / worker` parent-child role / summary / replay contract。
- 收口 `octopus-sdk-core` / `octopus-sdk-context` 中仍未落地的 prompt cache / compaction / long-horizon 路径。
- 审计并收口 `RuntimeSdkDeps::minimal`、`octopus-cli:minimal`、crate-level minimal scaffold、session legacy migration、tool registry shim 等最小化 / 兼容路径。
- 在 live surface 稳定后回填 `02-crate-topology.md`、`03-legacy-retirement.md`、必要时 `docs/sdk/README.md` Fact-Fix、以及触及 capability surface 的 transport/schema/tests。

Out of scope：

- 重开 `04`–`13` 已完成 tranche 的完成态。
- 为了保留 placeholder 行为而新增业务层绕行。
- 在未先冻结 owner 与 contract 的情况下，直接把更多 capability 暴露进 live runtime。

## Risks

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | 某些 residual tool / model / plugin path 没有共享 owner，只能靠 host 本地兜底。 | Task 1 先冻结 owner；没有共享 owner 的能力不得继续保留在 live surface。 | #2 / #8 |
| R2 | model family 与 role router 同步收口时，容易把默认选择、已保存配置与 live catalog 搞成三套答案。 | Task 3 必须同批处理 builtin catalog、role defaults、platform snapshot、configuredModels compat。 | #8 |
| R3 | `ToolSpec` / `RenderLifecycle` / `SessionEvent` 这组契约横跨 contracts / tools / core / platform，容易只改类型不改真实事件语义。 | Task 2 必须同批处理 contracts、runtime emission 与 tests；不允许只补占位字段。 | #8 / #10 |
| R4 | 权限 / 沙箱 / hooks 如果只补枚举、不补决策链和 event emission，会继续留下两份 contract。 | Task 5 必须同批处理 mode、rule bucket、hook 点、sandbox surface 与 permission events。 | #8 / #10 |
| R5 | 这一轮横跨 tools / model / plugin / session / docs，diff 很容易失控。 | 按 Task 分批推进；任何单批预计 > 800 行就拆。 | #6 |
| R6 | 如果 `docs/sdk/*` 与实际收口方案冲突，但不走 Fact-Fix，会留下两份真相源。 | 一旦命中规范冲突，只能回写 `docs/sdk/README.md` `## Fact-Fix 勘误`。 | #1 |
| R7 | observability 如果只改 tracer，不补 `SessionEvent` / replay / subagent 元数据，跨父子代理追踪仍然断裂。 | Task 6 必须同批处理 tracer、event、replay、subagent summary。 | #4 / #10 |
| R8 | prompt cache / compaction 实现一旦改变工具顺序或系统 prompt 前缀，会直接伤到命中率。 | Task 7 必须以现有稳定性测试为硬门禁，不允许先上行为再补测试。 | #4 |
| R9 | session compat 迁移与 legacy JSONL / SQLite 表兼容一旦处理失误，会伤到恢复与审计链。 | Task 8 先区分“受支持 compat”与“待退役 shim”，再决定删除或保留。 | #9 / #10 |
| R10 | compatibility shim 如果只做“先留着”，下一轮还会继续把临时路径当正式实现。 | Task 8 必须给每条 compat path 写清 status：`retire now`、`keep as supported compat`、`narrow to test-only`。 | #8 / #9 |
| R11 | 如果 `brain_loop.rs` 继续直接全量 `tools.schemas_sorted()`，后续 render / permission / replay 都会建立在错误 tool surface 上。 | Task 2 必须先收口 request-time tool surface、deferred tool policy 与 discovery / exposure state，再推进 builtin / render。 | #8 / #10 |
| R12 | 如果主循环继续停在 `MAX_BRAIN_LOOP_ITERATIONS + 单次 stop hook 注入`，`docs/sdk/01 / 08` 承诺的 continuation、budget、retry 语义仍会只有文档没有运行时闭环。 | Task 6 必须同批处理 session-owned query loop、stop hook continuation、token budget continuation 与失败恢复策略。 | #8 / #10 |
| R13 | 如果 tool execution 继续只有 `permission -> sandbox -> execute` 骨架，不补 denied/retry、hook timing、progress/writeback、execution span，权限与 hook 仍会停在“有接口、无治理闭环”。 | Task 5 必须同批处理 tool execution governance 与 permission / hook / sandbox contract。 | #8 / #10 |
| R14 | 如果 render 仍只在 assistant text 或 `result.render` 时回写，`OnToolUse / OnToolProgress / OnToolRejected / OnToolError` 会继续停在类型层。 | Task 2 必须把 render lifecycle、tool writeback 与 transcript/replay 同批落地。 | #8 / #10 |

## Residual Gap Matrix（已确认存在）

| 类别 | 当前证据 | 问题性质 | 本 tranche 要求 | 对应任务 |
|---|---|---|---|---|
| tool contract / UI intent lifecycle | `docs/sdk/03-tool-system.md`、`docs/sdk/14-ui-intent-ir.md` 对比 `crates/octopus-sdk-tools/src/{spec.rs,tool.rs,context.rs}`、`crates/octopus-sdk-contracts/src/{ui_intent.rs,event.rs}`、`crates/octopus-sdk-core/src/{brain_loop.rs,tool_dispatch.rs}` | `ToolSpec` 仍缺 `version / outputFormat / validate / displayDescriptor`；`ToolUseContext` / `ToolPermissionContext` 未分离；`RenderLifecycle` 仍是 phase enum，`SessionEvent::Render` 只承单个 `RenderBlock`，且 live path 主要只在 assistant text 或 `result.render` 时回写 | tools / contracts / core / platform 需要给出唯一 render/writeback contract，并补齐 `OnToolUse / OnToolProgress / OnToolRejected / OnToolError`；做不到就一次性 Fact-Fix 收窄，不再维持“历史占位 + 最小实现”双轨 | Task 2 / Task 9 |
| tool exposure / ToolSearch / deferred tools | `docs/plans/agent/2026-04-18-claude-code-tool-use-architecture.md`、`docs/references/claude-code-sourcemap-main/restored-src/src/{tools.ts,services/api/claude.ts,tools/ToolSearchTool/ToolSearchTool.ts,utils/toolSearch.ts}` 对比 `crates/octopus-sdk-core/src/brain_loop.rs`、`crates/octopus-sdk-tools/src/registry.rs`、`crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}` | 当前请求路径仍直接全量 `tools.schemas_sorted()`；`octopus-sdk-*` 没有 `ToolSearch + deferred tools + discovered/exposed state` 闭环 | live tool surface 必须改成 request-time assembled；deferred capability 只有在 discovery / exposure 后才能进入可见 tools block；不得把 provider wire 形状直接当内部 truth | Task 2 |
| session / query loop / continuation policy | `docs/sdk/01-core-loop.md`、`docs/sdk/08-long-horizon.md`、`docs/references/claude-code-sourcemap-main/restored-src/src/{QueryEngine.ts,query.ts}` 对比 `crates/octopus-sdk-core/src/brain_loop.rs` | 当前主循环仍是固定 4 次迭代的最小实现；缺少 QueryEngine 风格的 session-owned mutable state、stop hook continuation、token budget continuation、overflow / retry / prompt-too-long recovery policy | session/query 主链路必须给出唯一 request ownership 与 continuation contract；不能继续停在“能跑一轮工具调用”的最小脑循环 | Task 6 |
| residual builtin tools | `crates/octopus-sdk-tools/src/builtin/w5_stubs.rs`、`crates/octopus-sdk-tools/src/builtin/web_search.rs`、`crates/octopus-sdk-tools/src/registry.rs` | 仍存在 W5 stub、`NotYetImplemented` 与 shim context | 每个 builtin 只能是 live、non-live、或 supported compat 三者之一；不能继续停在“最小化可编译”状态 | Task 2 / Task 8 |
| model adapters / builtin catalog / role router | `crates/octopus-sdk-model/src/adapter/stubs.rs`、`catalog/builtin/{openai.rs,google.rs,minimax.rs}`、`role_router.rs` | stub adapter、空 catalog、角色路由覆盖不全 | live model family 不能继续依赖 `AdapterNotImplemented` 或空 `Vec::new()` catalog | Task 3 |
| plugin runtime bootstrap | `crates/octopus-platform/src/runtime_sdk/plugin_boot.rs`、`crates/octopus-sdk-plugin/src/bundled.rs`、`lifecycle.rs` | runtime boot 仍带 example/bundled placeholder 色彩，runtime / decl-only 边界不够硬 | live plugin runtime、snapshot 与 declaration-only component 必须分清 | Task 4 |
| permissions / sandbox contract | `docs/sdk/06-permissions-sandbox.md` 对比 `crates/octopus-sdk-contracts/src/permissions.rs`、`crates/octopus-sdk-permissions/src/{gate.rs,policy.rs}`、`crates/octopus-sdk-sandbox/src/spec.rs` | 当前只有四态 `PermissionMode` + 单一 `PermissionContext`；缺 `dontAsk` / `auto` / `bubble`、`ToolPermissionContext` rule bucket、egress/policy integration contract | 权限 mode、rule bucket、non-interactive 行为、sandbox egress surface 需要有唯一公开合同；所有权限决策必须可审计 | Task 5 / Task 9 |
| hook lifecycle | `docs/sdk/07-hooks-lifecycle.md` 对比 `crates/octopus-sdk-contracts/src/hooks.rs` | 当前只有 8 个 hook 点；缺 `PreSampling` / `PostSampling` / `SubagentSpawn` / `SubagentReturn` / `OnToolError` / `PreFileWrite` / `PostFileWrite` | hook 生命周期要么补齐到 owner/runtime，要么一次性 Fact-Fix 收窄，不再长期挂着“文档完整、实现最小” | Task 5 / Task 9 |
| tool execution governance | `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts` 对比 `crates/octopus-sdk-core/src/tool_dispatch.rs`、`crates/octopus-sdk-contracts/src/{event.rs,hooks.rs,ui_intent.rs}` | 当前虽有 `pre-hook -> permission -> sandbox -> post-hook` 骨架，但 denied / retry hint、hook timing summary、progress / rejection writeback、execution span 与 richer permission metadata 仍明显偏薄 | tool execution 必须形成完整治理闭环；不能继续让 permission / hook / sandbox 只停留在最小调用序列 | Task 5 / Task 6 |
| observability / evaluation / replay | `docs/sdk/01-core-loop.md`、`docs/sdk/09-observability-eval.md` 对比 `crates/octopus-sdk-contracts/src/{event.rs,subagent.rs}`、`crates/octopus-sdk-observability/src/{tracer.rs,usage.rs,replay.rs}`、`crates/octopus-sdk-core/src/{brain_loop.rs,tool_dispatch.rs}` | `SessionEvent` 缺 `permission_decision` 等关键事件；`TraceSpan` 仅 `name + fields`；缺 `trace_id / span_id / parent_span_id / agent_role / input_hash / model_version` 等约束字段；`SubagentSummary` 只有 `trace_id` 无层级 | tracing / replay / session summary / eval ledger 需要补到可串联父子代理与权限决策；不能继续停留在“有 tracer 类型但无规范字段”状态 | Task 6 / Task 9 |
| coordinator / worker role surface | `docs/sdk/05-sub-agents.md`、`docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts` 对比 `crates/octopus-sdk-subagent/src/orchestrator.rs`、`crates/octopus-sdk-contracts/src/subagent.rs` | 当前已有 worker 编排与 `SubagentSpec`，但缺显式 `coordinator mode`、worker tool surface、resume mode 对齐与 parent-child summary contract | subagent 公共合同需要明确 parent / worker 角色面、可见 tool surface 与回放 / summary 语义；不能继续只剩最小 fan-out / fan-in | Task 6 / Task 9 |
| prompt cache / long-horizon | `crates/octopus-sdk-core/src/brain_loop.rs`、`crates/octopus-sdk-context/src/compact.rs` | cache metadata 仍为 `Vec::new()` / `CacheControlStrategy::None`，`Hybrid` 直接 abort | 不能继续以未实现状态占住规范要求的长程能力位 | Task 7 |
| minimal scaffold / entrypoint | `crates/octopus-platform/src/runtime_sdk/builder.rs`、`crates/octopus-cli/src/run_once.rs`、`crates/octopus-sdk-{contracts,session,model}/src/lib.rs` | 最小化 scaffold / minimal entrypoint 仍明确存在 | 生产主路径不能继续依赖 minimal 入口；保留就必须收窄用途 | Task 8 |
| compatibility shims | `crates/octopus-sdk-session/src/{jsonl.rs,sqlite/schema.rs}`、`crates/octopus-sdk-tools/src/registry.rs`、`crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}` | legacy 迁移、shim context、hidden unsupported metadata 仍在 | compat 不是默认长期存在；需要 owner、边界、测试与退役策略 | Task 8 |
| control docs / contract reconciliation | `docs/plans/sdk/{02-crate-topology.md,03-legacy-retirement.md,12-post-w8-capability-hardening.md,13-finalization-and-deferred-capabilities.md}`、`docs/sdk/README.md` | 前两轮已冻结边界，但下一轮实现尚未回写为唯一控制面 | live/runtime/contract/docs 需要重新合一 | Task 1 / Task 9 |

## Claude Code 对照实现索引（实现前强制阅读）

> 下列位置是本 tranche 的强制参考。实现时不能只读 `docs/sdk/*` 摘要后直接改代码；至少要同时打开对应的 Claude Code 还原源码与本仓 owner 文件。

### Shared Baseline

- `docs/sdk/references.md §C1`
- `docs/references/claude-code-sourcemap-main/README.md`
- `docs/plans/agent/2026-04-18-claude-code-tool-use-architecture.md`

### Task 2 · tool contract / tool exposure / UI intent / builtin tools

- `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`
  - `ToolPermissionContext`
  - `ToolUseContext`
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts`
  - `getAllBaseTools()`
  - `ToolSearchTool` optimistic inclusion
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts`
  - `isToolSearchEnabled(...)` 调用：`~1120`
  - `extractDiscoveredToolNames(...)` 过滤：`~1158`
  - `willDefer(...)` / deferred schema 组装：`~1207+`
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/ToolSearchTool.ts`
  - `query`
  - `select:<tool_name>`
  - deferred tool description cache invalidation
- `docs/references/claude-code-sourcemap-main/restored-src/src/utils/toolSearch.ts`
  - `getToolSearchMode()`
  - `modelSupportsToolReference()`
  - auto threshold / optimistic gate
- `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`
  - `runTools(...)`：`~1382`

### Task 3 · model request assembly / provider routing

- `docs/sdk/references.md §C1`
  - `restored-src/src/utils/model/providers.ts`
  - `restored-src/src/utils/model/model.ts`
  - `restored-src/src/services/api/claude.ts`

### Task 4 · plugin / skills runtime

- `docs/references/claude-code-sourcemap-main/restored-src/src/plugins/builtinPlugins.ts`
- `docs/references/claude-code-sourcemap-main/restored-src/src/plugins/bundled/index.ts`
- `docs/references/claude-code-sourcemap-main/restored-src/src/skills/bundledSkills.ts`
- `docs/references/claude-code-sourcemap-main/restored-src/src/skills/loadSkillsDir.ts`

### Task 5 · permissions / sandbox / hooks

- `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`
  - `ToolPermissionContext`
- `docs/sdk/references.md §C1`
  - `restored-src/src/types/permissions.ts`
  - `restored-src/src/hooks/toolPermission/useCanUseTool.tsx`
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts`
  - `runPreToolUseHooks(...)`：`~800`
  - `startToolSpan(...)` / `startToolExecutionSpan()`：`~909` / `~1176`
  - `executePermissionDeniedHooks(...)`：`~1081`
  - `runPostToolUseHooks(...)`：`~1483`
- `docs/references/claude-code-sourcemap-main/restored-src/src/query/stopHooks.ts`

### Task 6 · query loop / observability / subagent coordination

- `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts`
  - `QueryEngineConfig`
  - `mutableMessages`
  - `submitMessage(...)`
- `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`
  - `queryLoop(...)`：`~241`
  - `buildQueryConfig()`：`~295`
  - `handleStopHooks(...)`：`~1267`
  - `checkTokenBudget(...)`：`~1309`
  - `runTools(...)`：`~1382`
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts`
  - tracing / permission decision / tool span
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/analytics/index.ts`
- `docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`

### Task 7 · compaction / long-horizon

- `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`
- `docs/references/claude-code-sourcemap-main/restored-src/src/query/stopHooks.ts`
- `docs/sdk/references.md §C1`
  - `restored-src/src/services/compact/*`
  - `restored-src/src/query/tokenBudget.ts`

## 公共面变更登记

| 变更点 | 登记位置 | 当前要求 | 触发任务 |
|---|---|---|---|
| tool contract、`ToolUseContext` / `ToolPermissionContext`、UI intent / render event 形状 | `docs/plans/sdk/02-crate-topology.md §2.1 / §2.4 / §2.12 / §2.14 / §3.1 / §5 / §6` | `ToolSpec`、`Tool`、`RenderLifecycle`、`SessionEvent::Render` 必须是同一套合同；业务层不得继续吃 tool-name 特判。 | Task 2 |
| tool exposure、`ToolSearch`、deferred tool policy 与 runtime-visible surface | `docs/plans/sdk/02-crate-topology.md §2.1 / §2.4 / §2.14 / §3.1 / §5 / §6` | 首轮 surface、discovery 后 surface、已暴露 deferred tool surface 必须是同一套 runtime contract；不允许继续静态全量 `schemas_sorted()`。 | Task 2 |
| builtin tool live policy、registry 与 catalog 边界 | `docs/plans/sdk/02-crate-topology.md §2.4 / §2.14 / §3.1 / §5` | 任何仍出现在 live runtime / shared projection 的 tool 都必须可执行；metadata-only 与 runtime-executable 必须显式区分。 | Task 2 |
| session / query loop、stop hook、token budget continuation 与 request ownership | `docs/plans/sdk/02-crate-topology.md §2.1 / §2.6 / §2.14 / §5` | session-owned query state、continuation policy、stop hook / token budget / retry 语义必须在 runtime 与控制面里是同一套答案；不允许继续只靠固定轮数脑循环。 | Task 6 |
| model family、builtin catalog、role routing 与 platform snapshot | `docs/plans/sdk/02-crate-topology.md §2.3 / §3.1 / §5` | live model surface、role default、configured model compat 与 transport-facing snapshot 必须是同一组答案。 | Task 3 |
| plugin runtime bootstrap、snapshot 与 decl-only component 边界 | `docs/plans/sdk/02-crate-topology.md §2.11 / §2.14 / §3.1 / §5` | runtime `Tool/Hook`、plugin discovery、bundled/local plugin boot 与 decl-only `SkillDecl/ModelProviderDecl/McpServerDecl` 必须分层登记。 | Task 4 |
| permissions、sandbox、hooks lifecycle 与 permission event 形状 | `docs/plans/sdk/02-crate-topology.md §2.1 / §2.7 / §2.8 / §2.9 / §2.14 / §5` | `PermissionMode` / rule bucket / `HookEvent` / sandbox spec / permission events 必须是同一套答案，不再停留在最小子集。 | Task 5 |
| tool execution governance、permission denied / retry、progress / rejection writeback 与 execution spans | `docs/plans/sdk/02-crate-topology.md §2.1 / §2.7 / §2.8 / §2.9 / §2.10 / §2.14 / §5 / §6` | `pre-hook -> permission -> sandbox -> execution -> post-hook -> render/writeback -> tracing` 必须是同一条 shared-layer 管线；不能再分散成最小骨架 + host 猜测。 | Task 5 / Task 6 |
| observability、replay、subagent tracing 与 session summary | `docs/plans/sdk/02-crate-topology.md §2.1 / §2.10 / §2.13 / §2.14 / §5` | trace/event/replay/usage ledger 必须能串起 `session -> tool -> subagent`；关键字段不能只在文档里存在。 | Task 6 |
| `coordinator / worker` role surface 与 subagent summary / replay contract | `docs/plans/sdk/02-crate-topology.md §2.10 / §2.13 / §2.14 / §5` | parent / worker 角色面、tool surface、resume metadata、summary contract 必须一起登记，不再只剩最小 fan-out / fan-in。 | Task 6 |
| prompt cache / compaction / long-horizon contract | `docs/plans/sdk/02-crate-topology.md §2.6 / §2.14 / §5` | `cache_breakpoints`、`cache_control`、compaction strategy 的实际行为要与规范和测试对齐。 | Task 7 |
| session compatibility 与 minimal runtime entrypoint | `docs/plans/sdk/02-crate-topology.md §2.2 / §3.5 / §5` | session recovery / compat contract、CLI/runtime entrypoint 责任边界要写清，不再靠 minimal 名义长期悬空。 | Task 8 |

## 退役登记

> 本 tranche 默认不预设“全部删除”。只有在 residual path 被证明只是临时 shim、且已有稳定替代时，才进入 `03-legacy-retirement.md`。

| 潜在退役项 | `03` 回填位置 | 当前结论 | 触发条件 |
|---|---|---|---|
| `RuntimeSdkDeps::minimal` / `octopus-cli:minimal` / crate-level minimal scaffold | `docs/plans/sdk/03-legacy-retirement.md §7`（新增 `minimal entrypoints` 小节） | 待 Task 8 审计后决定：退役或收窄为 test-only。 | 主路径已存在真实 runtime/CLI boot 且不再需要 minimal 默认入口 |
| session legacy JSONL envelope / SQLite legacy table migration | `docs/plans/sdk/03-legacy-retirement.md §7`（新增 `session compatibility` 小节） | 先视为 compat contract 候选，不预设删除。 | 已确认旧数据迁移窗口关闭，或 compat 逻辑已被更窄的迁移工具替代 |
| tool registry shim context / hidden unsupported metadata | `docs/plans/sdk/03-legacy-retirement.md §7`（新增 `runtime shims` 小节） | 先审计，不预设全部删除。 | live/runtime/contract 已有更清晰的建模，shim 不再承担受支持兼容职责 |

## Exit Gate 对齐表

| Exit Gate | 本 tranche 落点 | 验证 |
|---|---|---|
| `ToolSpec` / `ToolContext` / `RenderLifecycle` / `SessionEvent` 不再比 `docs/sdk/03` / `14` 更窄，assistant/tool render-writeback lifecycle 已覆盖 `OnToolUse / OnToolProgress / OnToolRejected / OnToolError`，或已一次性 Fact-Fix 收窄 | Task 2 | `cargo test -p octopus-sdk-tools -p octopus-sdk-contracts -p octopus-sdk-core -p octopus-platform` |
| request-time tool surface 不再直接全量 `schemas_sorted()`；`ToolSearch` / deferred tool exposure 已有唯一 shared-layer contract | Task 2 | `cargo test -p octopus-sdk-tools -p octopus-sdk-core -p octopus-platform` |
| live tool surface 不再暴露 stub-only / shim-backed builtin tool | Task 2 / Task 8 | `cargo test -p octopus-sdk-tools -p octopus-platform` |
| live model surface 不再依赖 stub adapter、空 builtin catalog 或缺失 role route | Task 3 | `cargo test -p octopus-sdk-model -p octopus-platform` |
| live plugin bootstrap 与 snapshot 不再依赖 example-only placeholder，runtime / decl-only 边界明确 | Task 4 | `cargo test -p octopus-sdk-plugin -p octopus-sdk-core -p octopus-platform` |
| permissions / sandbox / hooks contract、tool execution governance 与 `permission_decision` / denied-retry / progress-writeback event 不再停留在最小子集 | Task 5 | `cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox -p octopus-sdk-core` |
| session/query main loop 已补齐 request ownership、stop hook continuation、token budget continuation 与失败恢复 contract | Task 6 | `cargo test -p octopus-sdk-core -p octopus-sdk-context -p octopus-sdk-observability` |
| tracing / replay / eval / subagent / coordinator-worker 元数据可以串联父子代理与权限决策 | Task 6 | `cargo test -p octopus-sdk-observability -p octopus-sdk-subagent -p octopus-sdk-core` |
| prompt cache / long-horizon 路径不再停留在 `None` / `Vec::new()` / `Hybrid aborted` | Task 7 | `cargo test -p octopus-sdk-core -p octopus-sdk-context -p octopus-sdk-model` |
| minimal scaffold / compat shim 要么退役，要么收窄为受支持 compat 合同并带测试 | Task 8 | `cargo test -p octopus-sdk-session -p octopus-cli -p octopus-platform -p octopus-sdk-tools` |
| `02 / 03 / 14 / docs/sdk/README.md` 与实际 live/runtime/contract 一致；如触及 transport/schema，同步完成生成链与 consumer tests | Task 9 | `pnpm openapi:bundle`；`pnpm schema:generate`；`pnpm schema:check`；`cargo test --workspace`；`cargo clippy --workspace -- -D warnings`；`pnpm -C apps/desktop test` |
| repo 级行数与索引守护通过 | Task 9 | `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`；`find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' | sort`；`rg '^\\| `[0-9]{2}-' docs/plans/sdk/README.md` |

## Execution Rules

- 先冻结 matrix，再写代码。任何 residual item 没有定类之前，不进入实现。
- 不能把 shared-layer 缺口下放到 `desktop`、`server`、`cli` 本地兜底。
- 任何 retained compat path 都必须明确：服务哪个旧数据/旧入口、当前是否受支持、哪一层负责验证。
- `minimal` 不能继续作为默认生产入口；保留也只能是 test-only 或显式降级路径。
- 按闭环批次推进，不按字段散修：先 `tool contract`，再 `request-time tool exposure / ToolSearch`，再 `permissions / hooks / sandbox + tool execution governance`，再 `query loop + render + event + replay`，最后 `compat retirement`。
- `ToolSpec` / `RenderLifecycle` / `PermissionMode` / `HookEvent` / `TraceSpan` 这类 contract widening，必须和 runtime emission / replay 同批落地；不接受“类型先到位，行为以后再补”。
- `request.tools`、system prompt 的 tools guidance、builtin catalog / defaults / snapshot、manifest discovery、file-write hook、`context_restored` 与 compat projection 都必须绑定到同一条 owner path；不允许再留 shadow truth。
- 所有权限决策、render lifecycle、subagent trace 边都进入事件流 / replay 后，才算残余收口。
- prompt cache / compaction 变更必须先补或复用稳定性测试，再进入默认路径。
- 任何 OpenAPI / schema 变更都必须走 `contracts/openapi/src/** → pnpm openapi:bundle → pnpm schema:generate` 链。
- 命中 `docs/sdk/*` 规范冲突时，先写 `docs/sdk/README.md` `## Fact-Fix 勘误`，不允许静默分叉。
- 每个 batch 默认 ≤ 800 行；超出就拆分，不挤进一个 PR。
- 每个 batch 结束后更新本文件状态、checkpoint、变更日志，并同步 `README.md` 状态。

## Task Ledger

### Task 1: 冻结 residual gap matrix 与执行顺序

Status: `done`

Files:
- Modify: `docs/plans/sdk/14-residual-capability-closure.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/sdk/README.md`（仅命中 Fact-Fix 时）

Preconditions:
- `13-finalization-and-deferred-capabilities.md` 已完成并保持 `done`。
- 当前 residual 证据点已复核。

Step 1:
- Action: 把所有 residual item 逐项定类为 `implement now`、`hide from live`、`keep as supported compat`、或 `narrow to test-only`。
- Done when: 每个 item 都有唯一 owner、唯一状态、唯一进入批次。
- Verify: `rg -n "minimal scaffold|NotYetImplemented|AdapterNotImplemented|hidden_builtin_model|Hybrid => Err|task_fn: None|displayDescriptor|ToolPermissionContext|dontAsk|permission_decision|PreSampling|trace_id" crates/octopus-sdk-tools crates/octopus-sdk-model crates/octopus-sdk-plugin crates/octopus-platform crates/octopus-sdk-core crates/octopus-sdk-context crates/octopus-sdk-session crates/octopus-cli crates/octopus-sdk-contracts crates/octopus-sdk-permissions crates/octopus-sdk-sandbox crates/octopus-sdk-observability docs/sdk`
- Stop if: 需要引入业务域 owner，或定类结果与 `docs/sdk/*` 第一性约束直接冲突。

Step 2:
- Action: 把冻结结果回填到本计划、`02` 公共面登记与 `03` 退役候选表；若命中规范冲突，同批写 `docs/sdk/README.md` Fact-Fix。
- Done when: 后续 Task 2–Task 9 不再重新争论 scope 与 owner，且 `14 / 02 / 03` 的 compat 清单只保留冻结后的显式状态。
- Verify: `rg -n "implement now|hide from live|supported compat|test-only" docs/plans/sdk/14-residual-capability-closure.md docs/plans/sdk/{02-crate-topology.md,03-legacy-retirement.md}`；`rg -n "hide from live|supported compat|test-only" docs/plans/sdk/03-legacy-retirement.md`
- Stop if: 需要直接改 `docs/sdk/*` 正文而不是通过 Fact-Fix 表达。

Step 3:
- Action: 为 Task 2–Task 7 补齐 Claude Code 对照文件、关键符号与入口位置，并把 `ToolSearch / deferred tools`、`coordinator / worker` 两条遗漏链路补入 matrix 与任务台账。
- Done when: 后续执行者打开本计划即可定位 Claude Code 对照实现，不需要重新做一轮源码摸排。
- Verify: `rg -n "ToolSearchTool|toolExecution.ts|coordinatorMode.ts|QueryEngine.ts|builtinPlugins.ts|bundledSkills.ts" docs/plans/sdk/14-residual-capability-closure.md`
- Stop if: 某条计划任务找不到足够稳定的参考实现，只能靠推断补设计。

### Task 2: 收口 tool contract、tool exposure、UI intent 与 residual builtin tools

Status: `in_progress`

Files:
- Modify: `crates/octopus-sdk-tools/src/{spec.rs,tool.rs,context.rs,registry.rs}`
- Modify: `crates/octopus-sdk-tools/src/builtin/{mod.rs,catalog.rs,w5_stubs.rs,web_search.rs}`
- Modify: `crates/octopus-sdk-contracts/src/{ui_intent.rs,event.rs}`
- Modify: `crates/octopus-sdk-core/src/{brain_loop.rs,tool_dispatch.rs}`
- Modify: `crates/octopus-sdk-context/src/prompt.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/{builder.rs,registry_bridge/{builtins.rs,snapshot.rs}}`
- Test: `crates/octopus-sdk-tools/tests/{builtin_stubs.rs,builtin_web.rs,registry_stability.rs,tool_contract.rs}`
- Test: `crates/octopus-sdk-contracts/tests/{session_event_render.rs,ui_intent_ir.rs}`
- Test: `crates/octopus-sdk-context/tests/{prompt_cache_fingerprint.rs,prompt_stability.rs}`
- Test: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`

Preconditions:
- Task 1 已冻结 tool surface、tool exposure、render contract 与 residual builtin 的 live policy / owner。

Reference:
- 先读本文件 `Claude Code 对照实现索引` 的 `Task 2` 条目。

Step 1:
- Action: 对齐 `ToolSpec` / `Tool` / `ToolContext` 与 `docs/sdk/03-tool-system.md` 的最小合同，明确 `version`、`outputFormat`、`validate`、`display/render`、`ToolUseContext` / `ToolPermissionContext` 的归宿。
- Done when: 工具公开面不再只剩 `{ name, description, input_schema, category } + execute` 这一套缩水合同；若某字段明确不支持，同批写 Fact-Fix。
- Verify: `cargo test -p octopus-sdk-tools -p octopus-sdk-contracts`
- Stop if: 需要让业务层继续 `switch(toolName)`，或引入 host-only context 才能满足工具/UI 合同。

Step 2:
- Action: 对齐 request-time tool surface 装配，补齐 `ToolSearch / deferred tools / discovered-exposed state` 或 Octopus 等价内部状态闭环；`brain_loop` 的 `request.tools` 与 `SystemPromptBuilder::build(PromptCtx { tools })` 必须共用同一份 assembled surface，禁止继续在 `brain_loop.rs` 中无差别全量发 `tools.schemas_sorted()` 或让 prompt 继续读全量 registry。
- Done when: 首轮 surface、发现后 surface、未暴露 deferred tool 的 guard / retry hint、prompt-side tools guidance、以及 resume / replay 后 surface 重建，都有稳定 shared-layer contract；prompt fingerprint / stability 对这条 surface 变更有显式测试归属。
- Verify: `cargo test -p octopus-sdk-tools -p octopus-sdk-core -p octopus-platform`；`cargo test -p octopus-sdk-context --test prompt_cache_fingerprint`；`cargo test -p octopus-sdk-context --test prompt_stability`
- Stop if: 需要把 provider wire 结构直接当内部 truth，或只能靠 host / page-local workaround 才能实现工具暴露。

Step 3:
- Action: 对齐 `RenderLifecycle` / `SessionEvent::Render` / builtin runtime registry，补齐 `onToolUse / onToolProgress / onToolResult / onToolRejected / onToolError` 的数据承载，并清掉 `NotYetImplemented` / shim-backed builtin 主路径。
- Done when: `RenderLifecycle` 不再只是 phase enum；每个 builtin 要么有真实可执行 owner，要么不再出现在 live runtime / shared capability projection。
- Verify: `cargo test -p octopus-sdk-tools -p octopus-sdk-contracts -p octopus-sdk-core -p octopus-platform`
- Stop if: 需要把 tool-specific render 逻辑下放到 `desktop` / `cli`，或 live policy 仍未稳定。

### Task 3: 收口 model adapters、builtin catalog 与 role routing

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-model/src/{adapter/stubs.rs,adapter/mod.rs,adapter/openai_chat.rs,adapter/anthropic_messages.rs,provider_impl.rs,role_router.rs,lib.rs}`
- Modify: `crates/octopus-sdk-model/src/catalog/builtin/{openai.rs,google.rs,minimax.rs,mod.rs}`
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}`
- Modify: `docs/sdk/README.md`（仅命中 Fact-Fix 时）
- Test: `crates/octopus-sdk-model/tests/{adapter_openai.rs,catalog_builtin.rs,prompt_cache_stability.rs,role_router.rs}`
- Test: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`

Preconditions:
- Task 1 已冻结哪些 model family 在本 tranche 补齐，哪些继续作为 runtime-unsupported compat。

Reference:
- 先读本文件 `Claude Code 对照实现索引` 的 `Task 3` 条目。
- 先复核 `docs/sdk/11-model-system.md` 与 `docs/references/vendor-matrix.md`，确认 `vendor-matrix` 是 builtin catalog / snapshot / defaults 的唯一事实源。

Step 1:
- Action: 补齐或收窄 builtin model family，并把 builtin catalog 的 live 列表绑定到 `docs/references/vendor-matrix.md` 的派生结果；不允许继续在 `catalog/builtin/*` 或 snapshot bridge 里手写新的 live truth。
- Done when: 任何仍在 live catalog / default routing 中可见的 model family 都有真实 adapter 与模型清单，且 catalog 来源不是额外手写的一份 Rust 列表；做不到就同批 Fact-Fix 收窄 live surface。
- Verify: `cargo test -p octopus-sdk-model --test catalog_builtin`；`cargo test -p octopus-sdk-model --test adapter_openai`
- Stop if: 某个 live family 缺少共享 secret/config contract，只能靠 host 特判继续工作。

Step 2:
- Action: 同步 role router、platform snapshot、configured model compat 与默认选择，让 runtime 对每个 role 和 configured model 都只有一套答案；`catalog/builtin/*`、role defaults 与 `runtime_sdk/registry_bridge/snapshot.rs` 不得再各自维持独立硬编码清单。
- Done when: role routing 不再只覆盖最小集合；configured non-live builtin model 的处理方式有明确 compat contract；catalog / defaults / snapshot 三处不存在分叉的 live truth。
- Verify: `cargo test -p octopus-sdk-model --test role_router`；`cargo test -p octopus-platform --test runtime_sdk_bridge`
- Stop if: 需要先改 OpenAPI / schema 才能完成 runtime 内部一致性。

### Task 4: 收口 plugin runtime bootstrap 与 bundled/local plugin 边界

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-plugin/src/{bundled.rs,lifecycle.rs,manifest.rs,registry.rs,api.rs,lib.rs}`
- Modify: `crates/octopus-platform/src/runtime_sdk/{plugin_boot.rs,builder.rs,registry_bridge/snapshot.rs}`
- Modify: `crates/octopus-sdk-core/src/plugin_boot.rs`（若需要与 core boot 对齐）
- Test: `crates/octopus-sdk-plugin/tests/{lifecycle.rs,registry.rs,security_gates.rs}`
- Test: `crates/octopus-sdk-core/tests/plugin_subagent_integration.rs`
- Test: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`

Preconditions:
- Task 1 已冻结 plugin runtime component 与 decl-only component 的边界。

Reference:
- 先读本文件 `Claude Code 对照实现索引` 的 `Task 4` 条目。

Step 1:
- Action: 明确 live plugin bootstrap 的 owner、discovery roots、bundled/local plugin 来源，以及 runtime `Tool/Hook` 与 decl-only `SkillDecl/ModelProviderDecl/McpServerDecl` 的边界；discovery / validation 必须从 manifest 起步，再决定是否执行插件代码。
- Done when: `plugin_boot`、`PluginLifecycle`、`PluginRegistry` 与 snapshot 的职责边界明确，不再依赖 example-only placeholder 或预载入 plugin map 充当默认 live truth；manifest-first 成为显式验收条件。
- Verify: `cargo test -p octopus-sdk-plugin -p octopus-platform`
- Stop if: bootstrap 需要市场、分发或产品级决策，超出 `docs/sdk/12` 当前规范边界。

Step 2:
- Action: 把 live builder 接到真实 bootstrap 路径；对仍非 runtime 的 component 明确保持 decl-only，并补测试与控制面登记；禁止继续从 `example_bundled_plugins()` 或已加载代码对象出发，再反向匹配 manifest。
- Done when: live plugin snapshot 反映真实 manifest 发现结果，runtime / decl-only 边界在代码与文档中一致，插件代码只有在 manifest 通过验证后才进入执行路径。
- Verify: `cargo test -p octopus-sdk-core -p octopus-sdk-plugin -p octopus-platform`
- Stop if: 某个 component 要进入 live runtime，但没有统一 host contract。

### Task 5: 收口 permissions、sandbox、hooks 与 tool execution governance 契约

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-contracts/src/{permissions.rs,hooks.rs,event.rs}`
- Modify: `crates/octopus-sdk-permissions/src/{gate.rs,policy.rs}`
- Modify: `crates/octopus-sdk-sandbox/src/{spec.rs,lib.rs}`
- Modify: `crates/octopus-sdk-tools/src/context.rs`
- Modify: `crates/octopus-sdk-tools/src/builtin/{fs_write.rs,fs_edit.rs}`
- Modify: `crates/octopus-sdk-core/src/{brain_loop.rs,tool_dispatch.rs}`
- Test: `crates/octopus-sdk-permissions/tests/{gate_modes.rs,policy_rules.rs}`
- Test: `crates/octopus-sdk-hooks/tests/lifecycle.rs`
- Test: `crates/octopus-sdk-sandbox/tests/spec_contract.rs`
- Test: `crates/octopus-sdk-tools/tests/builtin_fs_write.rs`
- Modify: `docs/sdk/README.md`（仅命中 Fact-Fix 时）

Preconditions:
- Task 1 已冻结 permission / sandbox / hook target surface。

Reference:
- 先读本文件 `Claude Code 对照实现索引` 的 `Task 5` 条目。

Step 1:
- Action: 对齐 `PermissionMode`、`ToolPermissionContext`、rules-by-source bucket 与 `dontAsk / auto / bubble` 的公开边界，明确哪些实现、哪些通过 Fact-Fix 收窄。
- Done when: 权限合同不再只剩四态 mode + 单一 `PermissionContext`；规则桶与非交互模式有唯一答案。
- Verify: `cargo test -p octopus-sdk-permissions`
- Stop if: 需要引入未定义的分类器 / 产品交互能力，超出 `docs/sdk/06` 当前边界。

Step 2:
- Action: 对齐 hooks 生命周期与沙箱策略表面，补齐 `PreSampling` / `PostSampling` / `SubagentSpawn` / `SubagentReturn` / `OnToolError` / `PreFileWrite` / `PostFileWrite` 的 owner，并让所有权限决策写入事件流；`PreFileWrite / PostFileWrite` 必须挂到 `fs_write.rs` / `fs_edit.rs` 的真实落盘路径。
- Done when: hook / sandbox / permission event 不再停留在 8 点 hook + `fs_whitelist/network_proxy` 的最小子集；真实 file-write owner 会发出对应 hook / event，而不是只有 contracts 和 dispatch 层知道这些 hook。
- Verify: `cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox -p octopus-sdk-core`；`cargo test -p octopus-sdk-tools --test builtin_fs_write`
- Stop if: 需要引入新的宿主专属副作用通道，才能满足 hook / file-write 契约。

Step 3:
- Action: 对齐 tool execution governance，补齐 `pre-hook -> permission -> sandbox -> execution -> post-hook -> render/writeback -> tracing` 管线中的 denied / retry hint、hook timing、progress / rejection / error writeback、sandbox provenance 与 execution metadata。
- Done when: tool execution 不再只是“有权限门和沙箱就执行”的最小 skeleton；权限、hook、render、trace 语义在同一条 shared-layer 管线中可观测、可回放。
- Verify: `cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox -p octopus-sdk-core -p octopus-sdk-observability`
- Stop if: 需要把治理语义下放到 `desktop` / `cli` / host-specific 代码，shared layer 无法承接。

### Task 6: 收口 query loop、observability、subagent coordination 与 replay 契约

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-observability/src/{tracer.rs,usage.rs,replay.rs}`
- Modify: `crates/octopus-sdk-contracts/src/{event.rs,subagent.rs}`
- Modify: `crates/octopus-sdk-core/src/{brain_loop.rs,session_boot.rs,tool_dispatch.rs}`
- Modify: `crates/octopus-sdk-subagent/src/orchestrator.rs`
- Test: `crates/octopus-sdk-core/tests/{min_loop.rs,session_boot.rs,continuation_recovery.rs}`
- Test: `crates/octopus-sdk-observability/tests/{usage_replay.rs,trace_fields.rs}`
- Test: `crates/octopus-sdk-subagent/tests/{fan_in_fan_out.rs,condensed_summary.rs,trace_propagation.rs}`

Preconditions:
- Task 2–Task 5 已冻结 tool / model / permission / hook 的公开合同。

Reference:
- 先读本文件 `Claude Code 对照实现索引` 的 `Task 6` 条目。

Step 1:
- Action: 对齐 session / query 主循环，补齐 QueryEngine 风格的 session-owned mutable state、request ownership、stop hook continuation、token budget continuation、prompt-too-long / overflow / retry policy，并明确哪些进入 live path、哪些通过 Fact-Fix 收窄；测试至少覆盖 `stop-hook continuation`、`token-budget continuation`、`prompt-too-long recovery`、`retry-after-overflow` 四类用例。
- Done when: `brain_loop` 不再只是固定轮数的最小实现；session/query 主链路的 continuation 和失败恢复有可测试、可回放的唯一 contract，且 `min_loop.rs` / `session_boot.rs` 对这些场景有显式测试归属。
- Verify: `cargo test -p octopus-sdk-core --test min_loop`；`cargo test -p octopus-sdk-core --test session_boot`
- Stop if: 需要引入新的会话真相源或宿主侧隐藏状态，才能表达主循环语义。

Step 2:
- Action: 对齐事件级 tracing，补齐 `trace_id / span_id / parent_span_id / agent_role / input_hash / permission_decision / model_id / model_version / config_snapshot_id` 等字段在 tracer、event、runtime emission 中的归宿。
- Done when: tool / session / subagent 路径都能输出可串联的 trace 元数据，不再只有泛型 `TraceSpan { name, fields }`。
- Verify: `cargo test -p octopus-sdk-observability -p octopus-sdk-core`
- Stop if: 方案要求额外引入新的持久化真相源，违反当前 persistence 规则。

Step 3:
- Action: 对齐 replay、usage ledger、session summary 与 subagent tracing，使权限决策、失败、子代理树、回放字段可以复原会话结构。
- Done when: observability surface 不再只剩 aggregate usage；replay 可以看见关键结构事件。
- Verify: `cargo test -p octopus-sdk-observability --test usage_replay`；`cargo test -p octopus-sdk-subagent --test fan_in_fan_out`
- Stop if: replay / eval 需要引入第二套业务投影，超出 SDK 当前职责。

Step 4:
- Action: 对齐 `SubagentSpec` / `SubagentSummary` 与 `coordinator / worker` role contract，明确 parent-child 角色面、resume metadata、summary contract 与可见 surface 的归宿。
- Done when: subagent 公共面不再只剩最小 fan-out / fan-in；父子代理的 role / summary / trace 可以从事件与回放中互相还原。
- Verify: `cargo test -p octopus-sdk-subagent --test condensed_summary`；`cargo test -p octopus-sdk-subagent --test trace_propagation`
- Stop if: 需要引入新的宿主专属协调层，才能表达 parent-child role contract。

### Task 7: 收口 prompt cache、compaction 与 long-horizon 路径

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-core/src/{brain_loop.rs,session_boot.rs}`
- Modify: `crates/octopus-sdk-context/src/{compact.rs,scratchpad.rs}`
- Modify: `crates/octopus-sdk-model/src/provider_impl.rs`（若 cache metadata 透传需要补齐）
- Test: `crates/octopus-sdk-context/tests/{compactor_clear_tool_results.rs,compactor_summarize.rs,prompt_cache_fingerprint.rs,prompt_stability.rs}`
- Test: `crates/octopus-sdk-model/tests/prompt_cache_stability.rs`
- Test: `crates/octopus-sdk-core/tests/{min_loop.rs,session_boot.rs}`
- Test: `crates/octopus-sdk-context/tests/scratchpad_atomic.rs`
- Modify: `docs/sdk/README.md`（仅命中 Fact-Fix 时）

Preconditions:
- Task 2–Task 6 已稳定 tool / model / plugin 顺序与 live surface。

Reference:
- 先读本文件 `Claude Code 对照实现索引` 的 `Task 7` 条目。

Step 1:
- Action: 按 `docs/sdk/08-long-horizon.md` 与 `docs/sdk/11-model-system.md` 的要求，补齐 brain loop 的 `cache_breakpoints` / `cache_control` 策略，不再默认 `Vec::new()` / `CacheControlStrategy::None`。
- Done when: live request path 可以稳定地产生 cache metadata，且不会破坏 prompt fingerprint / stability invariants。
- Verify: `cargo test -p octopus-sdk-core -p octopus-sdk-model -p octopus-sdk-context`
- Stop if: prompt cache 稳定性测试回退到基线 80% 以下或无法解释的失败。

Step 2:
- Action: 实现 `Hybrid` compaction，或把规范与实现统一收窄到一个已实现策略并补 Fact-Fix / 测试。
- Done when: long-horizon 路径不再因为 `Hybrid` 未实现而直接 abort，且 compaction 结果与事件 / artifact 路径一致。
- Verify: `rg -n "CompactionStrategyTag::Hybrid => Err|CacheControlStrategy::None|cache_breakpoints: Vec::new\\(" crates/octopus-sdk-core crates/octopus-sdk-context`
- Stop if: 方案要求引入新的持久化真相源，违反当前 runtime persistence 规则。

Step 3:
- Action: 对齐 `docs/sdk/08-long-horizon.md` 定义的 context reset / handoff 路径，补齐 `runtime/notes/<session>.md`、`runtime/todos/<session>.json`、`context_restored` 的 owner 与启动恢复语义；若当前 tranche 不实现，必须同批 Fact-Fix 收窄 long-horizon 的 live 承诺。
- Done when: long-horizon 不再只剩 compaction 子集；context reset / handoff 要么进入 runtime code 与测试，要么从 live 承诺中被显式收窄，不再处于文档声称存在、实现未接线的状态。
- Verify: `cargo test -p octopus-sdk-context --test scratchpad_atomic`；`cargo test -p octopus-sdk-core --test session_boot`
- Stop if: 方案要求引入新的持久化真相源，违反当前 runtime persistence 规则。

### Task 8: 审计并收口 minimal scaffold 与 compatibility shims

Status: `pending`

Files:
- Modify: `crates/octopus-platform/src/runtime_sdk/builder.rs`
- Modify: `crates/octopus-cli/src/run_once.rs`
- Modify: `crates/octopus-sdk-{contracts,session,model}/src/lib.rs`
- Modify: `crates/octopus-sdk-session/src/{jsonl.rs,sqlite/schema.rs,lib.rs}`
- Modify: `crates/octopus-sdk-tools/src/registry.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Test: `crates/octopus-sdk-session/tests/{sqlite_jsonl.rs,plugins_snapshot_stability.rs}`
- Test: `crates/octopus-cli/tests/min_cli.rs`
- Test: `crates/octopus-sdk-tools/tests/registry_stability.rs`

Preconditions:
- Task 2–Task 7 已产出稳定的主路径与 live surface。

Step 1:
- Action: 逐项审计 `03-legacy-retirement.md §7.10–§7.12` 已登记的 minimal / legacy / shim / hidden compat metadata，按 Task 1 冻结结果定类为 `hide from live`、`supported compat` 或 `test-only`。
- Done when: `RuntimeSdkDeps::minimal`、`octopus-cli:minimal`、crate-level minimal scaffold、legacy session reader / wrapper / migration、`shim_tool_context()`、`hidden_builtin_model(...)` 与 hidden unsupported builtin metadata 都有逐条结论，不再只是“先留着”的默认残留。
- Verify: `rg -n "RuntimeSdkDeps::minimal|octopus-cli:minimal|intentionally keeps this crate as a minimal scaffold" crates/octopus-platform/src/runtime_sdk/builder.rs crates/octopus-cli/src/run_once.rs crates/octopus-sdk-contracts/src/lib.rs crates/octopus-sdk-session/src/lib.rs crates/octopus-sdk-model/src/lib.rs`；`rg -n "SkipLegacyRuntimeEnvelope|looks_like_legacy_runtime_envelope|parse_legacy_event_wrapper|legacy_event_id|migrate_legacy_tables" crates/octopus-sdk-session/src/jsonl.rs crates/octopus-sdk-session/src/sqlite/schema.rs`；`rg -n "shim_tool_context|hidden_builtin_model|configuredModels" crates/octopus-sdk-tools/src/registry.rs crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}`
- Stop if: 删除某条 compat path 会破坏恢复、审计或旧数据迁移，但又拿不出迁移替代方案。

Step 2:
- Action: 退役不再需要的 shim；对保留路径收窄作用域、补 contract tests，并同步 `03` 退役登记。
- Done when: 生产主路径不再依赖 minimal / shim 默认入口；保留 compat 的边界和测试已明确，且 `03 §7.10–§7.12` 的状态与本文件冻结结论一致。
- Verify: `cargo test -p octopus-sdk-session -p octopus-cli -p octopus-platform -p octopus-sdk-tools`
- Stop if: compat 收口需要一次性数据回填或人工修复现有 workspace 数据。

### Task 9: 回填 contracts / docs / gates 并完成 tranche 收口

Status: `pending`

Files:
- Modify: `docs/plans/sdk/{02-crate-topology.md,03-legacy-retirement.md,14-residual-capability-closure.md,README.md}`
- Modify: `docs/sdk/README.md`
- Modify: `contracts/openapi/src/**`（仅 capability-facing contract 真实变化时）
- Modify: `packages/schema/src/**`（仅 contract 真实变化时）
- Modify: `crates/octopus-server/**`（仅 transport / tests 需要对齐时）
- Modify: `apps/desktop/src/**`、`apps/desktop/test/**`（仅 consumer 面真实变化时）

Preconditions:
- Task 2–Task 8 已完成，主路径与 compat policy 已稳定。

Step 1:
- Action: 回填 `02` 公共面、`03` 退役表、`docs/sdk/README.md` Fact-Fix；如果 capability-facing contract 发生真实变化，再按链路更新 OpenAPI、schema、transport 与 desktop fixtures/tests。
- Done when: control docs、external contract 与 live code 说的是同一件事。
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: 某个 contract 变更只是为了给 placeholder 或 compat shim 续命。

Step 2:
- Action: 跑全量 gate、更新 checkpoint / change log / 状态，并把 `README.md` 与本文件收口到最终状态。
- Done when: workspace gate、line budget、索引守护全部通过；本 tranche 的完成态可直接从文档和验证命令证明。
- Verify: `cargo test --workspace`；`cargo clippy --workspace -- -D warnings`；`pnpm -C apps/desktop test`；`find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`；`find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' | sort`；`rg '^\\| `[0-9]{2}-' docs/plans/sdk/README.md`
- Stop if: 任一 gate 失败且无法归因到本 tranche 修改，或需要再新开并行计划才能完成。

## Batch Checkpoint Format

每个 batch 结束后追加：

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task <n> Step <i> -> Step <j>
- Completed:
  - short list
- Verification:
  - `command` -> pass / fail
- Blockers:
  - none / short reason
- Next:
  - Task <n> Step <k>
```

## Checkpoint 2026-04-23 15:43

- Batch: Task 1 Step 1 -> Step 2
- Completed:
  - 复核 `docs/sdk/03`、`06`、`07`、`09`、`14` 与当前 tools / contracts / permissions / sandbox / observability 实现差异
  - 把 tool contract / UI intent、permissions / sandbox、hooks、observability 残余补入 matrix、public surface、exit gate 与 task ledger
- Verification:
  - `rg -n "permission_decision|RenderLifecycle|ToolPermissionContext|dontAsk|PreSampling|trace_id" docs/sdk crates/octopus-sdk-contracts crates/octopus-sdk-tools crates/octopus-sdk-permissions crates/octopus-sdk-observability crates/octopus-sdk-core` -> pass
  - `git status --short` -> pass
- Blockers:
  - none
- Next:
  - Task 1 Step 2

## Checkpoint 2026-04-23 16:40

- Batch: Task 1 Step 3
- Completed:
  - 补入 `ToolSearch / deferred tools / request-time tool surface` 缺口，明确当前 `brain_loop.rs -> tools.schemas_sorted()` 不是可接受终态
  - 补入 `coordinator / worker` role surface 缺口，把 subagent 相关残余从“只有 fan-out / fan-in”提升为显式计划项
  - 新增 `Claude Code 对照实现索引`，按 Task 2–Task 7 列出强制参考文件、关键符号与入口位置
  - 调整 Task 2 为 `tool contract -> tool exposure -> UI intent / builtin` 闭环推进，避免按字段散修
- Verification:
  - `rg -n "ToolSearchTool|toolExecution.ts|coordinatorMode.ts|QueryEngine.ts|builtinPlugins.ts|bundledSkills.ts" docs/plans/sdk/14-residual-capability-closure.md` -> pass
  - `git status --short docs/plans/sdk/14-residual-capability-closure.md docs/plans/sdk/README.md` -> pass
- Blockers:
  - `02-crate-topology.md`、`03-legacy-retirement.md` 还未同步回填
- Next:
  - Task 1 Step 2

## Checkpoint 2026-04-23 17:18

- Batch: Task 1 Step 2
- Completed:
  - 把 Claude Code 核心链路审计中的 P1-P2 缺口回填到本计划
  - 新增 `session / query loop / continuation policy` 与 `tool execution governance` 两条 residual matrix
  - 扩写 `Task 2 / Task 5 / Task 6`，把 render-writeback lifecycle、tool execution governance、QueryEngine 风格主循环、stop hook / token budget continuation 收口为显式执行项
  - 补齐公共面登记、Exit Gate 与 Claude Code 对照索引中的对应参考锚点
- Verification:
  - `rg -n "query loop|tool execution governance|render-writeback|token budget continuation|progress / rejection / error writeback|session / query loop" docs/plans/sdk/14-residual-capability-closure.md` -> pass
  - `git diff -- docs/plans/sdk/14-residual-capability-closure.md` -> pass
- Blockers:
  - `02-crate-topology.md`、`03-legacy-retirement.md` 还未同步回填
- Next:
  - Task 1 Step 2

## Checkpoint 2026-04-23 17:25

- Batch: Task 1 Step 2 closeout
- Completed:
  - 把 residual closure 冻结结果回填到 `02-crate-topology.md`，补齐 contracts / tools / permissions / sandbox / subagent / ui-intent / observability / core / platform 的 owner 与 contract 冻结说明
  - 把 minimal entrypoints、session compatibility、runtime shims 三组 compat 候选回填到 `03-legacy-retirement.md`
  - 更新当前 Task / Step，确认 `Task 1` 完成并移除“02 / 03 未同步”阻塞
- Verification:
  - `rg -n "implement now|hide from live|supported compat|test-only" docs/plans/sdk/14-residual-capability-closure.md docs/plans/sdk/02-crate-topology.md docs/plans/sdk/03-legacy-retirement.md` -> pass
  - `git status --short docs/plans/sdk/02-crate-topology.md docs/plans/sdk/03-legacy-retirement.md docs/plans/sdk/14-residual-capability-closure.md` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-23 17:47

- Batch: plan audit consolidation
- Completed:
  - 把 review findings 统一回填为 `Task 1 Freeze Result`，移除会导致后续任务重新争论 scope 的未决 `Open Questions`
  - 同步修正 `Task 2 / 3 / 4 / 5 / 6 / 7 / 8` 的 owner、文件列表、验收条件与验证命令，补上 prompt surface、vendor-matrix、manifest-first、file-write、context reset 与 compat 清单逐项核对
  - 把本计划与 `README.md` 的状态切到 `in_progress`，并为 `03 §7.10–§7.12` 准备显式 compat 状态
- Verification:
  - `rg -n "Task 1 Freeze Result|prompt surface|vendor-matrix|manifest-first|PreFileWrite|context reset|shim_tool_context|hidden_builtin_model" docs/plans/sdk/14-residual-capability-closure.md` -> pass
  - `rg -n "状态：`in_progress`|Status: `in_progress`" docs/plans/sdk/14-residual-capability-closure.md docs/plans/sdk/README.md` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Change Log

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-23 | 首稿：基于 `13` 之后的 residual audit，起草下一 tranche；把未实现 capability、最小化 scaffold 与 compatibility shim 收口成统一执行计划 | Codex |
| 2026-04-23 | 二次审计 `docs/sdk/03 / 06 / 07 / 09 / 14` 与实现差异；补入 tool contract / UI intent、permissions / sandbox、hooks、observability 残余与对应任务 | Codex |
| 2026-04-23 | 三次审计 Claude Code 还原源码与当前 SDK 差异；补入 `ToolSearch / deferred tools`、`coordinator / worker` 残余，新增按任务分组的 Claude Code 对照实现索引，并把执行顺序改为闭环推进 | Codex |
| 2026-04-23 | 四次审计 Claude Code 核心链路；把 `query loop / stop hook / token budget continuation`、`tool execution governance`、`render-writeback lifecycle` 与 span-lite observability 缺口回填到 matrix、Task 2 / 5 / 6、公共面登记与 Exit Gate | Codex |
| 2026-04-23 | Task 1 收口：把冻结结果同步回 `02-crate-topology.md` 与 `03-legacy-retirement.md`，新增 minimal/compat/shim 审计登记，并将当前执行面切换到 `Task 2` | Codex |
| 2026-04-23 | 汇总本轮 review findings；把 `Open Questions` 收束为 `Task 1 Freeze Result`，同步补齐 prompt-side tool surface、`vendor-matrix` 单一真相源、manifest-first plugin bootstrap、真实 file-write hook owner、context reset / handoff 与 compat 逐项验证要求，并把计划状态切到 `in_progress` | Codex |
