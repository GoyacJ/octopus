# 13 · SDK 正式收口与 deferred capability 边界

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 本文件承接 `12-post-w8-capability-hardening.md`。目标不是重开 W1–W8 或立即新增新能力面，而是把 SDK 重构从“实现已落地、控制面仍有残口”收口到可以正式宣告完成的状态，并冻结仍保持 non-live / decl-only 的后续边界。
>
> 阅读顺序：**本文件 →** `docs/plans/sdk/00-overview.md` → `docs/plans/sdk/02-crate-topology.md` → `docs/plans/sdk/12-post-w8-capability-hardening.md` → `docs/sdk/03-tool-system.md` → `docs/sdk/11-model-system.md` → `docs/sdk/12-plugin-system.md` → `crates/octopus-platform/src/runtime_sdk/{builder.rs,plugin_boot.rs,subagent_runtime.rs}` → `crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}` → `crates/octopus-server/src/workspace_runtime/tests/{mod.rs,support.rs,support_runtime.rs,support_workspace.rs}`。

## Status

状态：`done`

## Active Work

当前 Task：`完成 · SDK formal closeout 与 deferred capability freeze`

当前 Step：`Exit Gate satisfied · 状态 / checkpoint / 变更日志已收口`

### Pre-Task Checklist（起稿阶段）

- [x] 已复核 `12-post-w8-capability-hardening.md` 的完成 checkpoint 与 live surface freeze。
- [x] 已复核 `00-overview.md §5` 当前 DoD 剩余缺口。
- [x] 已复核 `02-crate-topology.md` 与 `ls crates/` 的 crate 集合是否一致。
- [x] 已复核 `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` 的现状输出。
- [x] 已识别本 tranche 是否触发 `docs/sdk/README.md` `## Fact-Fix 勘误`。
- [x] 已识别 `octopus-core / telemetry` 的 owner / target state 是否明确。
- [x] 当前 git 工作树状态已知；本批次 diff 计划 ≤ 800 行。
- [x] 已识别所有 `Stop if:` 条款。

Open Questions：

- 无。

## Goal

把 SDK 重构从“代码实现已落地但控制文档和硬门禁仍未完全闭合”的状态收口到正式完成，同时把 deferred capability 的下一轮边界写成唯一控制口径。

## Non-goal

- 不重开 W1–W8 或 `12-post-w8-capability-hardening.md` 已完成的交付。
- 不在本 tranche 内无 owner 地新增新的 live tool / model / plugin family。
- 不把 control-plane 缺口下放到 `octopus-server` / `apps/desktop` 做 page-local 或 transport-local 绕行。

## Architecture

- `docs/plans/sdk/00 / 02 / 12 / README` 必须和 live code、workspace gate、crate 拓扑是同一份答案；formal completion 先修控制面，不靠口头完成。
- `octopus-platform` 继续持有 live runtime builder 的组装权；仍保持 non-live / decl-only 的 capability 必须在控制面明确 owner、触发条件和对外合同边界。
- `octopus-server` 的 `workspace_runtime` 测试支撑文件拆分只做结构整理，不改变 runtime contract、OpenAPI、schema 或业务语义。

## Scope

- In scope：
  - 对齐 `docs/plans/sdk/README.md`、`00-overview.md`、`02-crate-topology.md`、`12-post-w8-capability-hardening.md` 的状态、crate 集合和 formal completion 条件。
  - 对齐 `octopus-core / telemetry` 这两个额外 crate 的控制面归属，并消掉 `20 vs 22` / `15+5 vs 实际目录集` 的控制面歧义。
  - 把 `crates/octopus-server/src/workspace_runtime/tests/support.rs` 拆到 ≤ 800 行并保持测试语义不变。
  - 冻结 deferred capability 的下一轮边界：哪些继续 non-live、哪些是 decl-only、哪些具备重新进入 live scope 的前置。
  - 跑 formal closeout gate，并在全部满足后补 `00-overview.md` 的 `Completed YYYY-MM-DD`。
- Out of scope：
  - 直接实现 `web_search`、`skill`、`task_list`、`task_get`、plugin model provider、plugin MCP、stub-backed model adapters 的新 live 能力。
  - 引入新的 OpenAPI / schema 功能面，除非只是为对齐已稳定事实所必须的 Fact-Fix。
  - 新开 W9 / Phase 2 之外的并行计划文件。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | `telemetry` 退役后若仍存在隐式引用、workspace 残留或控制文档漏改，formal completion 的 crate gate 仍会失真。 | 以目录实盘、Cargo/default-members 与 `00 / 02 / 03` 四处一致为准；任一处不一致即视为 Task 2 未完成。 | #1 / #2 |
| R2 | 直接在 `support.rs` 拆文件时若顺手改了 runtime 行为，会把门禁修复和语义变更混在一起。 | 只做 test support 拆分；任何 contract 改动都回退。 | #3 |
| R3 | deferred capability 若只写“以后再说”，下一轮实现仍会重复争论 live / non-live / decl-only 边界。 | Task 4 必须写 owner、entry criteria 和 contract touchpoints。 | #4 / #5 |
| R4 | 若 `docs/sdk/*` 与当前冻结边界冲突，但不走 Fact-Fix，就会留下两份真相源。 | 一旦命中规范冲突，只能追加 `docs/sdk/README.md` `## Fact-Fix 勘误`。 | #5 |
| R5 | 若 formal closeout 时 README 仍保留 `draft` / `in_progress`，`00` 的 DoD #9 不能成立。 | 最终 gate 必须一次性收口状态、Goal、checkpoint 与 change log。 | #6 |

## 承 Post-W8 的现状

- `12-post-w8-capability-hardening.md` 已完成：live runtime builder 已接上真实 `TaskFn`、plugin live bootstrap，stub-only builtin tools 与 stub-backed model families 已从 live surface 收口。
- 当前 workspace gate 已复跑通过：`cargo test --workspace`、`cargo clippy --workspace -- -D warnings`、`pnpm -C apps/desktop test` 与 repo 级 `> 800` 行扫描全部通过。
- 当前 formal completion 已收口：
  - `docs/plans/sdk/README.md` 已把 `00 / 01 / 02 / 03 / 13` 统一收口为 `done`；`00-overview.md` 已补 `Completed 2026-04-23`，DoD #9 已满足。
  - `docs/plans/sdk/00-overview.md §2 / §5` 与 `docs/plans/sdk/02-crate-topology.md §3 / §8` 已对齐到 `21` 个 crate 目录：`15` 个 SDK crate + `5` 个业务 crate + `1` 个共享业务 core crate `octopus-core`；孤立且无活调用方的 `crates/telemetry` 已登记退役并从 workspace 实盘删除。
  - `crates/octopus-server/src/workspace_runtime/tests/support.rs` 已收口为薄 re-export 门面；repo 级 ≤ 800 行硬门禁已不再命中。
- 当前 deferred capability 的已知冻结面：
  - `web_search`、`skill`、`task_list`、`task_get` 已从 live runtime registry 和 live builtin catalog 隐藏，仍保持 non-live。
  - plugin `SkillDecl / ModelProviderDecl / McpServerDecl` 仍是 declaration-only registry data，不伪装成 live runtime capability。
  - `openai_responses`、`gemini_native`、`vendor_native` 相关 stub-backed model families 已从 live defaults / live platform snapshot 退场，但当已有 `configuredModels` 指向这些 builtin 时，平台仍会把它们作为 `unsupported` 的隐藏元数据保留下来，保持 config-visible / runtime-unsupported。

## 公共面变更登记

| 变更点 | 登记位置 | 当前冻结结论 | 触发任务 |
|---|---|---|---|
| formal completion 的 crate 拓扑 / DoD 文案 | `docs/plans/sdk/00-overview.md §2 / §5`、`docs/plans/sdk/02-crate-topology.md §1 / §3 / §8` | `00 / 02 / README / live crates/` 必须说同一件事；不允许继续保留 `15+5` 与 `octopus-core / telemetry` 这组额外 crate 的歧义。 | Task 1 / Task 2 |
| deferred capability 的 live / non-live / decl-only 边界 | `docs/plans/sdk/02-crate-topology.md §2 / §3 / §5`、必要时 `docs/sdk/README.md` `## Fact-Fix 勘误` | 只有真实可执行能力才算 live；其余必须带 owner 和 re-entry criteria。 | Task 4 |
| `workspace_runtime` test support 模块边界 | `crates/octopus-server/src/workspace_runtime/tests/**` | 仅重构测试支撑层；不引入新的 runtime public contract。 | Task 3 |

## 退役登记

> 本 tranche 默认不新增 legacy 退役；`octopus-core` 已在 `03-legacy-retirement.md §7.7` 明确为保留 crate。Task 2 已确认 `telemetry` 为孤立 legacy helper，并登记退役。

| 潜在退役项 | `03` 回填位置 | 当前结论 | 触发条件 |
|---|---|---|---|
| `crates/telemetry` | `docs/plans/sdk/03-legacy-retirement.md §7.9` + `§9` | 已退役；目录已删除，不再计入 workspace 目标矩阵。 | 已完成 |
| `crates/octopus-core` | `docs/plans/sdk/03-legacy-retirement.md §7.7` | 已知保留，不在本 tranche 内讨论退役。 | 不适用 |
| `workspace_runtime` 测试支撑旧文件边界 | 不涉及 `03` | 只拆分文件，不视为 legacy 退役。 | 不适用 |

## Exit Gate 对齐表

| Exit Gate | 本 tranche 落点 | 验证 |
|---|---|---|
| `README / 00 / 02 / 12` 的状态与现状一致 | Task 1 / Task 5 | `rg -n "12-post-w8|Completed|15 个 SDK crate|5 个业务 crate|20 个目录|octopus-core|telemetry" docs/plans/sdk/{README.md,00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md}` |
| target crate 集合与 live repo 一致，`octopus-core / telemetry` 的 ownership 不再悬空 | Task 2 | `find crates -maxdepth 1 -mindepth 1 -type d | sort` + `rg -n "octopus-core|telemetry" Cargo.toml crates docs/plans/sdk` |
| repo 级 ≤ 800 行门禁通过 | Task 3 / Task 5 | `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` |
| workspace gate 全绿 | Task 5 | `cargo test --workspace`；`cargo clippy --workspace -- -D warnings`；`pnpm -C apps/desktop test` |
| deferred capability 边界成为唯一口径 | Task 4 | `rg -n "web_search|skill|task_list|task_get|ModelProviderDecl|McpServerDecl|hidden_builtin_model|unsupported|non-live|decl-only|gpt-5\\.4|gemini-2\\.5|MiniMax-M2\\.7" docs/plans/sdk/{00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md} crates/octopus-platform crates/octopus-sdk-tools crates/octopus-sdk-plugin` |
| formal completion 可被控制文档直接证明 | Task 5 | `awk -F'|' '/^\\| `/{status=$4; gsub(/^ +| +$/,"",status); if(status!="`done`") print $0}' docs/plans/sdk/README.md` + `rg -n "Completed 2026-" docs/plans/sdk/00-overview.md` |

## Execution Rules

- 先消掉控制面和 crate 集合的歧义，再跑 formal closeout；不要在 `20 / 22` 语义未定时直接写 `Completed`。
- `Task 3` 只允许拆模块，不允许顺手修 runtime 行为；任何行为差异都另起 batch。
- deferred capability 的边界登记要落在已有控制文档，不再新增平行 truth source。
- 若 `docs/sdk/*` 正文与当前冻结边界冲突，必须走 `docs/sdk/README.md` 的 Fact-Fix；不要在 `00 / 02 / 12` 里私自改规范语义。
- 每个 batch 结束必须追加 checkpoint，并同步当前 Task / Step / 状态。

## Task Ledger

### Task 1: 复核 formal completion 差口并对齐控制面基线

Status: `done`

Files:
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/12-post-w8-capability-hardening.md`
- Modify: `docs/sdk/README.md`（仅命中 Fact-Fix 时）

Preconditions:
- `12-post-w8-capability-hardening.md` 的实现 checkpoint 与代码现状一致。
- 当前 live runtime builder / plugin / task surface 已按 `12` 收口。

Step 1:
- Action: 复核 formal completion 还未闭合的控制面差口：`README` 状态、`00` 的 crate DoD、`02` 的 target topology、`12` 的完成态说明。
- Done when: 当前仍未完成的原因在控制文档里是显式、单一、可复核的，不再存在“代码已 done，但索引 / 总控仍写旧状态”的隐性差口。
- Verify: `rg -n "13-finalization|Completed|15 个 SDK crate|5 个业务 crate|20 个目录|octopus-core|telemetry" docs/plans/sdk/{README.md,00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md}`
- Stop if: 要描述差口就必须先改动实现代码，而不是先把控制面现状写准。

Step 2:
- Action: 做只依赖当前事实的最小对齐：状态、说明文字、open question、变更日志先与现状一致；需要等 `Task 2` 决策的内容明确挂起，不抢写结果。
- Done when: control-plane baseline 已可作为后续 Task 2–Task 5 的唯一入口。
- Verify: `find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' | sort` + `rg '^\\| `[0-9]{2}-' docs/plans/sdk/README.md`
- Stop if: baseline 对齐会隐含修改 `docs/sdk/*` 规范正文，而不是补 Fact-Fix。

### Task 2: 对齐 `octopus-core / telemetry` 的控制面归属并消掉 crate 拓扑歧义

Status: `done`

Files:
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（若选择 retire）
- Modify: `Cargo.toml`（仅当 workspace / default-members 需同步）
- Modify: `crates/telemetry/**`（仅当选择迁移或退役）

Preconditions:
- Task 1 已把 baseline 差口显式写出。
- `octopus-core` 的保留事实已由 `03-legacy-retirement.md §7.7` 与业务侧 Cargo 依赖复核。
- `crates/telemetry` 的当前引用面已经复核。

Step 1:
- Action: 审计额外 crate 的实际依赖方向、使用方和 owner：确认 `octopus-core` 继续作为业务共享 crate 保留；判断 `telemetry` 是保留、并入，还是登记退役。
- Done when: `15+5` 目标矩阵之外的目录不再是“未知偏差”，而是有明确归属的控制面事实。
- Verify: `rg -n "octopus-core|telemetry" Cargo.toml crates docs/plans/sdk docs/sdk`
- Stop if: `telemetry` 已经泄漏进 SDK 对外公共面，或 `octopus-core` 的保留需要改动 SDK 分层规则才能成立。

Step 2:
- Action: 按 Step 1 的结论更新 `00 / 02`；若 `telemetry` 选择退役，同批把 `03` 补到可执行状态；若保留，则把 DoD、crate 数和描述改成与 live repo 一致，并清理 `02 §8` 中过时的 `telemetry` 注记。
- Done when: `00-overview.md §2 / §5`、`02-crate-topology.md` 与 `find crates/` 的结果完全一致。
- Verify: `find crates -maxdepth 1 -mindepth 1 -type d | sort` + `rg -n "20 个目录|15 个 SDK crate|5 个业务 crate|octopus-core|telemetry" docs/plans/sdk/{00-overview.md,02-crate-topology.md}`
- Stop if: 需要为了保留 `telemetry` 临时篡改分层规则，或为了退役它而引入跨层反向依赖。

### Task 3: 拆分 `workspace_runtime` 测试支撑文件并过掉 ≤ 800 行硬门禁

Status: `done`

Files:
- Modify: `crates/octopus-server/src/workspace_runtime/tests/support.rs`
- Modify: `crates/octopus-server/src/workspace_runtime/tests/mod.rs`
- Modify: `crates/octopus-server/src/workspace_runtime/tests/support_runtime.rs`
- Create: `crates/octopus-server/src/workspace_runtime/tests/support_*.rs`（按拆分结果命名）
- Modify: `crates/octopus-server/src/workspace_runtime/tests/support_workspace.rs`（若边界需要重排）

Preconditions:
- Task 2 不再阻塞 formal closeout 的 crate / DoD 语义。
- 已确认本 Task 只处理 test support 层。

Step 1:
- Action: 先按职责拆出 `support.rs` 的模块边界，明确哪些是 fixture builder、哪些是 catalog / model helper、哪些是 workspace / runtime setup。
- Done when: 拆分方案能把 `support.rs` 降到 ≤ 800 行，且没有任何 runtime contract 改动需求。
- Verify: `find crates/octopus-server/src/workspace_runtime/tests -maxdepth 1 -type f -name 'support*.rs' -exec wc -l {} +`
- Stop if: 仅靠拆分无法通过硬门禁，必须连带改测试语义或 server runtime contract。

Step 2:
- Action: 按已冻结边界拆文件、更新 `mod` / `use` / helper 导出，并保持测试行为不变。
- Done when: `cargo test -p octopus-server` 通过，且全仓 `> 800` 扫描不再命中该文件。
- Verify: `cargo test -p octopus-server` + `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: 拆分引入 OpenAPI / schema / transport DTO 变更，或需要在业务代码里绕过现有 test support。

### Task 4: 冻结 deferred capability 的下一轮边界

Status: `done`

Files:
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/12-post-w8-capability-hardening.md`
- Modify: `docs/sdk/README.md`（仅命中 Fact-Fix 时）
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（仅当真有 retire）

Preconditions:
- `12-post-w8-capability-hardening.md` 的 live-only freeze 仍成立。
- Task 1–Task 3 已消掉 formal closeout 的显性差口，或已确认与本 Task 无交叉阻塞。

Step 1:
- Action: 列出仍保持 non-live / decl-only 的 capability：完全隐藏的 builtin tools、declaration-only plugin entries、以及 config-visible / runtime-unsupported 的 hidden builtin model families，并写清它们各自的 owner 和 re-entry 前提。
- Done when: 可以明确区分三种状态：`non-live and hidden from runtime/catalog`、`decl-only registry data`、`config-visible but runtime-unsupported hidden metadata`。
- Verify: `rg -n "web_search|skill|task_list|task_get|SkillDecl|ModelProviderDecl|McpServerDecl|hidden_builtin_model|unsupported|gpt-5\\.4|gemini-2\\.5|MiniMax-M2\\.7" docs/plans/sdk/{00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md} crates/octopus-platform crates/octopus-sdk-tools crates/octopus-sdk-model crates/octopus-sdk-plugin`
- Stop if: 某项能力已经在 live path 可执行，但控制面仍把它写成 non-live。

Step 2:
- Action: 把下一轮 capability 进入 live scope 必须触达的 contract surface 写清楚：builder、catalog / snapshot、OpenAPI / schema、desktop fixture / test、control docs 各自改哪一层。
- Done when: 下一轮实现可以直接按 owner 和 contract touchpoints 开工，不需要重新做一轮边界争论。
- Verify: `rg -n "non-live|decl-only|re-entry|builder|snapshot|OpenAPI|schema|desktop fixture|contract" docs/plans/sdk/{00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md}`
- Stop if: 结论与 `docs/sdk/*` 正文冲突，但又无法通过 `docs/sdk/README.md` 的 Fact-Fix 合法表达。

### Task 5: 跑 formal closeout gate 并把 SDK 重构状态收口到 `done`

Status: `done`

Files:
- Modify: `docs/plans/sdk/13-finalization-and-deferred-capabilities.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/00-overview.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`（仅若 gate 收尾还需补日志）
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（仅若 Task 2 走 retire）

Preconditions:
- Task 1–Task 4 全部完成。
- formal completion 的 crate 集合、line gate 和 deferred boundary 已全部有据可查。

Step 1:
- Action: 跑 formal closeout 所需全量 gate，并记录输出结论。
- Done when: workspace 测试、clippy、desktop tests 和 line gate 全部通过，且没有新的 control-plane mismatch。
- Verify: `cargo test --workspace` + `cargo clippy --workspace -- -D warnings` + `pnpm -C apps/desktop test` + `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'`
- Stop if: 任一 gate 失败，或失败项要求新增并行计划才能收口。

Step 2:
- Action: 在全部 gate 满足后，把 `00-overview.md` 的 `Goal` 补成 `Completed YYYY-MM-DD`，把 `README` / 本文件 / 相关控制文档状态收口为 `done`，并追加 checkpoint / 变更日志。
- Done when: `00` 的 DoD 能直接从控制文档和验证结果证明，`README` 不再保留 `draft / in_progress / blocked / pending`。
- Verify: `awk -F'|' '/^\\| `/{status=$4; gsub(/^ +| +$/,"",status); if(status!="`done`") print $0}' docs/plans/sdk/README.md` + `rg -n "Completed 2026-" docs/plans/sdk/00-overview.md`
- Stop if: formal completion 仍依赖一个尚未起稿或尚未完成的后续 plan。

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-23 | 首稿：新增 SDK 正式收口与 deferred capability 边界计划，聚焦 control-plane 对齐、`telemetry` 归属、`support.rs` ≤ 800 行门禁和 deferred capability 下一轮边界冻结。 | Codex |
| 2026-04-23 | 审计修订：按当前实现状态把 crate 差口从单一 `telemetry` 修正为 `octopus-core + telemetry`，补记 workspace gate 已通过、`support.rs` 仍是唯一 >800 行命中，并把 deferred capability 细化为 `hidden runtime tools / decl-only plugin entries / config-visible but runtime-unsupported hidden models` 三类。 | Codex |
| 2026-04-23 | Task 1 完成：同步 worktree 中的 `README / 00 / 02 / 12` 控制面基线，显式记录 formal closeout 仍被 `octopus-core / telemetry` 归属与 `support.rs` 行数门禁阻塞，并把本计划切到 `in_progress`。 | Codex |
| 2026-04-23 | Task 2 完成：确认 `octopus-core` 为共享业务 core crate；确认 `telemetry` 为无活调用方、未进入 `default-members` 的孤立 legacy helper，补 `03-legacy-retirement.md` 后直接删除目录，并把 crate 控制面收口为 `15 + 5 + 1 = 21`。 | Codex |
| 2026-04-23 | Task 3 完成：将 `support.rs` 改为薄 re-export 门面，复用 `support_runtime.rs` / `support_workspace.rs` 承载具体 helper；同步清理 `mod.rs`、`tests.rs`、`transport.rs` 的冗余导入与重复 helper，确保 `cargo test -p octopus-server` 通过且全仓不再命中 `> 800` 行。 | Codex |
| 2026-04-23 | Task 4 完成：把 deferred capability 的三类冻结边界与下一轮 live re-entry 触点同步写入 `00-overview.md`、`02-crate-topology.md`、`12-post-w8-capability-hardening.md`，明确 shared-layer owner、contract/desktop/docs 的跟进顺序。 | Codex |
| 2026-04-23 | Task 5 完成：复跑 `cargo test --workspace`、`cargo clippy --workspace -- -D warnings`、`pnpm -C apps/desktop test` 与 repo 级 `> 800` 行扫描；补齐 worktree 的前端依赖后确认全量 gate 通过，并将 `README` / `00-overview.md` / 本文件统一收口为 `done`。 | Codex |

## Checkpoints

## Checkpoint 2026-04-23 14:22

- Week: Finalization
- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed:
  - 将 `13-finalization-and-deferred-capabilities.md` 与 `docs/plans/sdk/README.md` 同步进 worktree，并把 `12-post-w8-capability-hardening.md` 的索引状态收口为 `done`
  - 在 `00-overview.md` 显式补记 W1–W8 主线已完成但 formal closeout 仍挂在 `13`；DoD、风险登记与目标矩阵现状同步暴露 `22` 个 crate 目录这一差口
  - 在 `02-crate-topology.md` 显式区分“目标矩阵”与“live workspace 现状”，并把 `telemetry` 的过时注记改成“当前无外部引用，待 Task 2 冻结归属”
  - 在 `12-post-w8-capability-hardening.md` 补记 tranche 边界：capability hardening 已完成，formal closeout / crate 拓扑 / `support.rs` 门禁转交 `13`
- Files changed:
  - `docs/plans/sdk/README.md` (modified)
  - `docs/plans/sdk/00-overview.md` (modified)
  - `docs/plans/sdk/02-crate-topology.md` (modified)
  - `docs/plans/sdk/12-post-w8-capability-hardening.md` (modified)
  - `docs/plans/sdk/13-finalization-and-deferred-capabilities.md` (added/modified)
- Verification:
  - `rg -n "13-finalization|Completed|15 个 SDK crate|5 个业务 crate|20 个目录|octopus-core|telemetry" docs/plans/sdk/{README.md,00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md}` -> pass
  - `find docs/plans/sdk -maxdepth 1 -type f -name '[0-9][0-9]-*.md' | sort` -> pass
  - `rg '^\| \`[0-9]{2}-' docs/plans/sdk/README.md` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - `octopus-core / telemetry` 的最终控制面归属仍未冻结
  - `crates/octopus-server/src/workspace_runtime/tests/support.rs` 仍为 `812` 行
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-23 15:07

- Week: Finalization
- Batch: Task 2 Step 1 -> Task 2 Step 2
- Completed:
  - 审计 `octopus-core` 的业务侧依赖面，确认它是共享业务 core crate，不属于 SDK 目标矩阵外的异常目录
  - 审计 `crates/telemetry` 后确认其仅剩孤立 helper 实现、未进入 `default-members`、仓内无活 Rust 调用方，因此按 formal closeout 直接退役
  - 同步更新 `00-overview.md`、`02-crate-topology.md`、`03-legacy-retirement.md`，把 crate 口径收口为 `15` 个 SDK crate + `5` 个业务 crate + `1` 个共享业务 core crate `octopus-core`，共 `21` 个目录
  - 删除 `crates/telemetry` 目录
- Files changed:
  - `docs/plans/sdk/00-overview.md` (modified)
  - `docs/plans/sdk/02-crate-topology.md` (modified)
  - `docs/plans/sdk/03-legacy-retirement.md` (modified)
  - `docs/plans/sdk/13-finalization-and-deferred-capabilities.md` (modified)
  - `crates/telemetry/Cargo.toml` (deleted)
  - `crates/telemetry/src/lib.rs` (deleted)
- Verification:
  - `find crates -maxdepth 1 -mindepth 1 -type d | sort` -> pass (`21` dirs; no `crates/telemetry`)
  - `test -d crates/telemetry && echo exists || echo removed` -> `removed`
  - `rg -n "octopus-core|telemetry" Cargo.toml crates docs/plans/sdk docs/sdk` -> pass (`octopus-core` 仅保留业务共享依赖；`telemetry` 只剩文档/历史记录命中)
- Exit state vs plan:
  - matches
- Blockers:
  - `crates/octopus-server/src/workspace_runtime/tests/support.rs` 仍为 `812` 行
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-23 15:38

- Week: Finalization
- Batch: Task 3 Step 1 -> Task 3 Step 2
- Completed:
  - 将 `crates/octopus-server/src/workspace_runtime/tests/support.rs` 收口为薄 re-export 门面，把 runtime helper 与 workspace helper 分别落在 `support_runtime.rs`、`support_workspace.rs`
  - 同步更新 `tests/mod.rs`，正式纳入 `support_runtime.rs` / `support_workspace.rs` 模块，保留 `support.rs` 作为新测试树的统一入口
  - 清理 `tests/mod.rs`、`tests.rs`、`support.rs`、`support_workspace.rs` 的冗余导入，并把 `transport.rs` 改为复用共享 `sample_runtime_event()`，消掉 split 后新增的 warning
- Files changed:
  - `crates/octopus-server/src/workspace_runtime/tests/mod.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests/support.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests/support_runtime.rs` (modified earlier in batch)
  - `crates/octopus-server/src/workspace_runtime/tests/support_workspace.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests/transport.rs` (modified)
- Verification:
  - `cargo test -p octopus-server` -> pass
  - `cargo check -p octopus-server --tests --message-format short 2>&1 | rg 'warning:'` -> pass (no output)
  - `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` -> pass (no output)
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-23 15:52

- Week: Finalization
- Batch: Task 4 Step 1 -> Task 4 Step 2
- Completed:
  - 在 `00-overview.md` 补记三类 deferred capability 的 formal closeout 冻结口径：non-live hidden builtin、decl-only plugin registry data、config-visible but runtime-unsupported hidden builtin model family
  - 在 `02-crate-topology.md` 为 `sdk-model`、`sdk-tools`、`sdk-plugin` 和 `octopus-platform` 补齐 owner / 代码证据 / re-entry checklist，明确下一轮 live 化必须先改 shared runtime ownership，再动 contract 和 desktop
  - 在 `12-post-w8-capability-hardening.md` 新增 `Deferred Capability Freeze` 汇总表，把这三类能力正式转交 `13` 继续维持，不回滚本 tranche 的 `done`
- Files changed:
  - `docs/plans/sdk/00-overview.md` (modified)
  - `docs/plans/sdk/02-crate-topology.md` (modified)
  - `docs/plans/sdk/12-post-w8-capability-hardening.md` (modified)
  - `docs/plans/sdk/13-finalization-and-deferred-capabilities.md` (modified)
- Verification:
  - `rg -n "web_search|skill|task_list|task_get|SkillDecl|ModelProviderDecl|McpServerDecl|hidden_builtin_model|unsupported|openai_responses|gemini_native|vendor_native|plugin_boot|subagent_runtime|contracts/openapi|packages/schema|apps/desktop" docs/plans/sdk/{00-overview.md,02-crate-topology.md,12-post-w8-capability-hardening.md} crates/octopus-platform crates/octopus-sdk-tools crates/octopus-sdk-model crates/octopus-sdk-plugin` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-23 16:08

- Week: Finalization
- Batch: Task 5 Step 1 -> Task 5 Step 2
- Completed:
  - 复跑 `cargo test --workspace`，确认 workspace tests 全绿
  - 复跑 `cargo clippy --workspace -- -D warnings`，确认 workspace clippy 全绿
  - 在 worktree 内执行 `pnpm install --frozen-lockfile` 补齐前端依赖后，复跑 `pnpm -C apps/desktop test`，确认桌面端 `66` 个 test file、`580` 个测试全部通过
  - 将 `docs/plans/sdk/README.md` 的剩余 `draft` 状态全部收口为 `done`，并在 `docs/plans/sdk/00-overview.md` 补记 `Completed 2026-04-23`
- Files changed:
  - `docs/plans/sdk/README.md` (modified)
  - `docs/plans/sdk/00-overview.md` (modified)
  - `docs/plans/sdk/13-finalization-and-deferred-capabilities.md` (modified)
- Verification:
  - `cargo test --workspace` -> pass
  - `cargo clippy --workspace -- -D warnings` -> pass
  - `pnpm install --frozen-lockfile` -> pass
  - `pnpm -C apps/desktop test` -> pass (`66` files, `580` tests)
  - `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` -> pass (no output)
  - `awk -F'|' '/^\| `/{status=$4; gsub(/^ +| +$/,"",status); if(status!="`done`") print $0}' docs/plans/sdk/README.md` -> pass (no output)
  - `rg -n "Completed 2026-" docs/plans/sdk/00-overview.md` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - none
