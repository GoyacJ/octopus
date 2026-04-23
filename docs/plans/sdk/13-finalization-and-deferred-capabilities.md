# 13 · SDK 正式收口与 deferred capability 边界

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 本文件承接 `12-post-w8-capability-hardening.md`。目标不是重开 W1–W8 或立即新增新能力面，而是把 SDK 重构从“实现已落地、控制面仍有残口”收口到可以正式宣告完成的状态，并冻结仍保持 non-live / decl-only 的后续边界。
>
> 阅读顺序：**本文件 →** `docs/plans/sdk/00-overview.md` → `docs/plans/sdk/02-crate-topology.md` → `docs/plans/sdk/12-post-w8-capability-hardening.md` → `docs/sdk/03-tool-system.md` → `docs/sdk/11-model-system.md` → `docs/sdk/12-plugin-system.md` → `crates/octopus-platform/src/runtime_sdk/{builder.rs,plugin_boot.rs,subagent_runtime.rs}` → `crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}` → `crates/octopus-server/src/workspace_runtime/tests/{mod.rs,support.rs,support_runtime.rs,support_workspace.rs}`。

## Status

状态：`draft`

## Active Work

当前 Task：`Task 1 · 控制面与退出条件复核`

当前 Step：`Step 1 · 对齐 README / 00 / 02 / 12 的现状口径`

### Pre-Task Checklist（起稿阶段）

- [ ] 已复核 `12-post-w8-capability-hardening.md` 的完成 checkpoint 与 live surface freeze。
- [ ] 已复核 `00-overview.md §5` 当前 DoD 剩余缺口。
- [ ] 已复核 `02-crate-topology.md` 与 `ls crates/` 的 crate 集合是否一致。
- [ ] 已复核 `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'` 的现状输出。
- [ ] 已识别本 tranche 是否触发 `docs/sdk/README.md` `## Fact-Fix 勘误`。
- [ ] 已识别 `octopus-core / telemetry` 的 owner / target state 是否明确。
- [ ] 当前 git 工作树状态已知；本批次 diff 计划 ≤ 800 行。
- [ ] 已识别所有 `Stop if:` 条款。

Open Questions：

- `crates/telemetry` 是 refactor 目标态中的保留 crate，还是应并入 / 退役后从目标 crate 集合中删除。
- `docs/sdk/*` 规范文本是否需要对 non-live / decl-only capability 边界做 Fact-Fix，还是控制面文档补充即可。

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
| R1 | `00-overview.md` 的 target matrix 只写了 `15 + 5 = 20`，但 live crates 还多出 `octopus-core` 与 `telemetry`；如果这组额外 crate 的归属不收口，formal completion 的 crate DoD 会一直悬空。 | `octopus-core` 按已知业务共享 crate 收口到控制面；`telemetry` 再决定保留、并入还是退役。 | #1 / #2 |
| R2 | 直接在 `support.rs` 拆文件时若顺手改了 runtime 行为，会把门禁修复和语义变更混在一起。 | 只做 test support 拆分；任何 contract 改动都回退。 | #3 |
| R3 | deferred capability 若只写“以后再说”，下一轮实现仍会重复争论 live / non-live / decl-only 边界。 | Task 4 必须写 owner、entry criteria 和 contract touchpoints。 | #4 / #5 |
| R4 | 若 `docs/sdk/*` 与当前冻结边界冲突，但不走 Fact-Fix，就会留下两份真相源。 | 一旦命中规范冲突，只能追加 `docs/sdk/README.md` `## Fact-Fix 勘误`。 | #5 |
| R5 | 若 formal closeout 时 README 仍保留 `draft` / `in_progress`，`00` 的 DoD #9 不能成立。 | 最终 gate 必须一次性收口状态、Goal、checkpoint 与 change log。 | #6 |

## 承 Post-W8 的现状

- `12-post-w8-capability-hardening.md` 已完成：live runtime builder 已接上真实 `TaskFn`、plugin live bootstrap，stub-only builtin tools 与 stub-backed model families 已从 live surface 收口。
- 当前 workspace gate 状态稳定：`cargo test --workspace`、`cargo clippy --workspace -- -D warnings`、`pnpm -C apps/desktop test` 已通过；formal closeout 的主要缺口不在测试失败，而在控制文档和 ≤ 800 行硬门禁。
- 当前 formal completion 仍有明确缺口：
  - `docs/plans/sdk/README.md` 已新增本计划为 `draft`，说明 SDK 重构的 formal closeout 还未完成；`00` 的 DoD #9 仍未满足。
  - `docs/plans/sdk/00-overview.md §5` 仍把目标 crate 集合写成 `15 个 SDK crate + 5 个业务 crate（共 20 个目录）`，但当前 `crates/` 目录实际是 `22` 个：额外还有 `crates/octopus-core` 与 `crates/telemetry`。
  - `docs/plans/sdk/02-crate-topology.md §8` 已把 `octopus-core` 和 `telemetry` 写进 workspace members，但 `00-overview.md` 的 DoD 和矩阵口径没有同步；其中 `telemetry` 的“W6 被 observability 引用后评估是否并入”注记已过时，当前仓内没有它的外部引用。
  - `crates/octopus-server/src/workspace_runtime/tests/support.rs` 仍为 `812` 行；同目录已经有 `mod.rs`、`support_runtime.rs`（`487` 行）和 `support_workspace.rs`（`357` 行），但 repo 级 ≤ 800 行硬门禁仍命中 `support.rs`。
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

> 本 tranche 默认不新增 legacy 退役；`octopus-core` 已在 `03-legacy-retirement.md §7.7` 明确为保留 crate，仅 `telemetry` 可能触发退役或并入决策。

| 潜在退役项 | `03` 回填位置 | 当前结论 | 触发条件 |
|---|---|---|---|
| `crates/telemetry` | `docs/plans/sdk/03-legacy-retirement.md §7`（新增对应小节） | 待 `Task 2` 冻结：保留并入目标矩阵，或登记退役。 | Task 2 选择 retire |
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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

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
