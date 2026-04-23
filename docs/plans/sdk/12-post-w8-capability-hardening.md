# Post-W8 · live capability hardening

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 本文件是 W8 收尾后的 follow-up tranche。`04`–`11` 的完成状态保持不变；本 tranche 只处理 live runtime 仍暴露的 stub / 未接线能力收口。
> formal completion 的控制面收口、crate 拓扑对齐与最后一个 `> 800` 行命中改由 `13-finalization-and-deferred-capabilities.md` 接手；这些残口不回写为本 tranche 未完成。
>
> 阅读顺序：**本文件 →** `docs/sdk/03-tool-system.md §3.2 / §3.4 / §3.6 / §3.7` → `docs/sdk/05-sub-agents.md §5.2 / §5.4 / §5.6 / §5.9` → `docs/sdk/11-model-system.md §11.4 / §11.6 / §11.7 / §11.17` → `docs/sdk/12-plugin-system.md §12.3 / §12.8.4 / §12.15 / §12.16` → `docs/sdk/13-contracts-map.md` → `docs/plans/sdk/02-crate-topology.md §2.3 / §2.4 / §2.10 / §2.11 / §2.14 / §3.1 / §5` → `crates/octopus-platform/src/runtime_sdk/builder.rs` → `crates/octopus-sdk-tools/src/builtin/{mod.rs,w5_stubs.rs,web_search.rs}` → `crates/octopus-sdk-model/src/{adapter/stubs.rs,catalog/builtin/{openai.rs,google.rs,minimax.rs}}` → `crates/octopus-sdk-plugin/src/lifecycle.rs` → `crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs}`。

## Status

状态：`done`

## Active Work

当前 Task：`完成 · Post-W8 capability hardening tranche`

当前 Step：`Exit Gate satisfied · 状态 / checkpoint / 变更日志已对齐`

### Pre-Task Checklist（起稿阶段）

- [x] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [x] 已阅读 `00-overview.md §1` 与 W8 收尾状态，且确认 `04`–`11` 保持 `done`。
- [x] 已阅读 `docs/sdk/03 / 05 / 11 / 12 / 13` 与本 tranche 直接相关章节。
- [x] 已核对 live runtime 的 4 组现状证据：builtin tool stubs、stub-backed model adapters、`task_fn` 缺口、plugin lifecycle 未上 live path。
- [x] 已识别本 tranche 涉及的 SDK 对外公共面变更（是）。
- [x] 已识别是否可能触及 `contracts/openapi/src/**` 或 `packages/schema/src/**`（是，但必须在 Task 2–5 稳定后才进入）。
- [x] 当前 git 工作树状态已知；本批次只新增计划文档与索引，不处理现有脏改动。
- [x] 已识别所有 `Stop if:` 条款；遇到任一条件立即停止并汇报。

Open Questions：

- `task` / `web_search` / `skill` / `task_list` / `task_get` 在规范层都有位置，但 live builder 仍直接注册 stub。Task 1 必须先冻结每个能力是“本 tranche 实现”还是“从 live 面隐藏”。
- `gpt-5.4` / `gpt-5.4-mini` / `gemini-2.5-flash` / `MiniMax-M2.7` 当前仍出现在 platform catalog snapshot；若 Task 4 选择先隐藏而非实现，需要同步处理默认选择、已保存配置与 UI/transport 暴露。
- `PluginLifecycle::run()` 除了 discovery roots 之外还需要一组已加载进进程的 `Box<dyn Plugin>`；Task 3 必须先冻结 live builder 持有哪些 bundled / local plugin implementation，再谈 provider / MCP 之类的后续能力面。

### 已确认的审核决策（2026-04-23）

| # | 决策点 | 确认结论 | 关联章节 |
|---|---|---|---|
| D1 | tranche 定位 | **这是 W8 之后的 hardening tranche，不回改 W1–W8 的完成态。** | Status / Scope / Exit Gate |
| D2 | 执行顺序 | **先收口 live capability surface，再做 contract reconciliation。** 不先改 OpenAPI / schema。 | Architecture / Task 2–Task 6 |
| D3 | 组装权归属 | **`octopus-platform` 继续持有 live runtime builder 的组装权。** `task_fn`、plugin discovery、model provider 不下放到 `server` / `desktop` / `cli`。 | Architecture / Task 3 / Task 4 / Task 5 |
| D4 | live 暴露规则 | **任何出现在 live tool registry / model catalog / snapshot 的能力，必须可执行；否则就从 live 面隐藏。** | Goal / Task 2 / Task 4 / Task 5 |
| D5 | 插件路径 | **Plugin Registry 不能继续是 live builder 的空占位。** live builder 至少要消费真实 discovery + 已加载 plugin set。 | Task 3 / Task 6 |
| D6 | 契约回填时机 | **`contracts/openapi/**`、`packages/schema/**`、desktop fixtures 只在 live surface 稳定后一起收口。** | Task 6 |
| D7 | 共享 consumer path | **`octopus-infra` 生成的 capability projection 属于 live surface 本身。** 不允许只改 `builtin_tool_catalog()` 而忽略 `WorkspaceToolCatalogEntry` 投影路径。 | Task 2 / Task 6 |
| D8 | plugin runtime reach | **本 tranche 先把 plugin runtime tools / hooks 接上 live builder。** `skill` / `model_provider` / `MCP` 若当前只有声明级登记，就先按 decl-only policy 收口，不把它们误写成已 live 可执行。 | Task 3 / Task 4 |
| D9 | task registry 范围 | **`task` 与 `task_list` / `task_get` 不绑定推进。** 本 tranche 只给 `task` 定义 `TaskFn` owner；`task_list` / `task_get` 若无共享 host owner，继续留在 non-live。 | Task 2 / Task 5 |

## Live Surface Policy Matrix（Task 1 freeze）

> 适用范围：本 tranche 只冻结当前已审计到的 residual gap。未列出的既有 live capability 继续按现状保持 live。

### Builtin tools

| 能力 | 当前证据 | 冻结结论 | 执行位置 | 备注 |
|---|---|---|---|---|
| `task` | `AgentTool::new()` 默认挂 `ErrorTaskFn("TaskFn not injected")`；`RuntimeSdkFactory::build_live()` 传 `task_fn: None` | `implement now` | Task 5 | 只在 `octopus-platform` 注入真实 `TaskFn` 后保留 live；在此之前不允许继续裸暴露。 |
| `web_search` | `WebSearchTool::execute()` 固定回 `ToolError::NotYetImplemented` | `hide from live surface` | Task 2 | 当前没有共享 web search provider owner；不在 `server` / `desktop` 本地兜底。 |
| `skill` | `SkillTool` 仍是 W5 stub | `hide from live surface` | Task 2 | Skill 资产继续通过 `octopus-infra` 的 skill catalog 暴露；这和 live runtime tool 不是一回事。 |
| `task_list` | `TaskListTool` 仍是 W5 stub | `defer but non-live` | Task 2 | 没有共享 host task registry owner 前，不进入 live runtime。 |
| `task_get` | `TaskGetTool` 仍是 W5 stub | `defer but non-live` | Task 2 | 同上。 |

### Plugin paths

| 能力 | 当前证据 | 冻结结论 | 执行位置 | 备注 |
|---|---|---|---|---|
| `PluginLifecycle::run()` + real discovery config | `RuntimeSdkFactory::build_live()` 现在只放空 `PluginRegistry::new()`，不跑 lifecycle | `implement now` | Task 3 | live builder 必须持有唯一的 discovery config 与 loaded plugin set。 |
| plugin runtime `Tool` / `Hook` registration | `PluginRegistry` 已有 runtime handle，但 live path 没接入 | `implement now` | Task 3 | 这是本 tranche 唯一进入 live runtime 的 plugin runtime component。 |
| plugin `SkillDecl` | 当前只有 manifest/metadata 路径 | `defer but non-live` | Task 3 | 不伪装成 live executable capability。 |
| plugin `ModelProviderDecl` | 当前只有 declaration，没有 live provider bootstrap | `defer but non-live` | Task 3 / Task 4 | 本 tranche 不把 provider 注册权迁到 plugin runtime。 |
| plugin `McpServerDecl` | 当前只在 catalog / 管理面消费 | `defer but non-live` | Task 3 | 不把 declaration 级 MCP 注册写成 live runtime MCP tool。 |

### Stub-backed model families

| 能力 | 当前证据 | 冻结结论 | 执行位置 | 备注 |
|---|---|---|---|---|
| `openai_responses` (`gpt-5.4`, `gpt-5.4-mini`) | `OpenAiResponsesAdapter` 仍是 stub；platform snapshot/defaults 仍暴露 `gpt-5.4*` | `hide from live surface` | Task 4 | 直到 adapter 落地前，从 live catalog、role defaults、platform snapshot、default selections 一起移除。 |
| `gemini_native` (`gemini-2.5-*`) | `GeminiNativeAdapter` 仍是 stub；`RoleRouter::Compact/Fast` 仍引用 `gemini-2.5-flash` | `hide from live surface` | Task 4 | 直到 adapter 落地前，从 live catalog 和默认路由里一起移除。 |
| `vendor_native` (`MiniMax-M2.7`) | `VendorNativeAdapter` 仍是 stub；platform snapshot 仍暴露 `MiniMax-M2.7` | `hide from live surface` | Task 4 | 当前 tranche 不补 vendor-native adapter。 |

### Shared consumer paths

| 路径 | 当前证据 | 冻结结论 | 执行位置 | 备注 |
|---|---|---|---|---|
| `builtin_tool_catalog()` -> `WorkspaceToolCatalogEntry` | `octopus-infra` 直接把 builtin metadata 投影成 `availability: healthy` | `implement now` | Task 2 | 共享投影必须和 live builtin policy 同步，不能保留私有 stub denylist。 |
| platform model snapshot / default selections | `registry_bridge::{builtins,snapshot,overrides}` 仍把 stub-backed models 记成默认可用 | `implement now` | Task 4 | snapshot/defaults 与 `ModelCatalog` 同步收口。 |
| plugin snapshot | live builder 现在拿到的是空 snapshot | `implement now` | Task 3 | snapshot 必须来自真实 discovery 结果，而不是空占位。 |

## Deferred Capability Freeze（转交 `13` 继续维持）

| 类别 | 当前冻结面 | 当前 owner / 代码证据 | 下一轮 live re-entry 触点 |
|---|---|---|---|
| `non-live and hidden from runtime/catalog` | `web_search`、`skill`、`task_list`、`task_get` | `octopus-sdk-tools::builtin::{mod.rs,catalog.rs}` 已把这 4 项移出 `register_builtins()` 与 `builtin_tool_catalog()`；`octopus-platform::runtime_sdk::builder` 只消费 live builtin 集 | 先在共享层定义 runtime owner / transport source，再同批更新 `octopus-sdk-tools`、`octopus-platform::runtime_sdk::builder`、shared capability projection、`contracts/openapi/**` / `packages/schema/src/**` / `apps/desktop/**` 与控制文档 |
| `decl-only registry data` | plugin `SkillDecl`、`ModelProviderDecl`、`McpServerDecl` | `octopus-sdk-plugin::{manifest.rs,registry.rs}` 仍只把这些组件登记进 declaration store；live path 仅接 `PluginLifecycle::run()` + runtime `Tool/Hook` registration | 先在 `octopus-platform::runtime_sdk::{plugin_boot,builder}` 定义 bootstrap owner，再扩 shared registry bridge、host contract、desktop fixtures 与控制文档，不能只改 manifest/registry |
| `config-visible but runtime-unsupported hidden metadata` | `openai_responses`、`gemini_native`、`vendor_native` builtin family | `octopus-platform::runtime_sdk::registry_bridge::{builtins.rs,snapshot.rs}` 用 `hidden_builtin_model()` 保留已有 `configuredModels`，统一降级为 `status = unsupported` | 同批补 `octopus-sdk-model` adapter 实现与 builtin catalog / role defaults，再更新 platform snapshot/default selections、capability-facing contract/desktop fixtures 与控制文档 |

## Goal

让当前 live SDK runtime 只暴露真实可执行的 tools / models / plugins，把 `task_fn` 与 plugin lifecycle 接到 `octopus-platform` 的 live builder，再按稳定后的能力面回填 transport contracts、schema 与控制文档。

## Architecture

- **继续由 `octopus-platform` 组装 live runtime**：W7 已把业务入口切到 SDK。Post-W8 不再把缺口分散修在 `server` / `desktop` / `cli`，而是在 `runtime_sdk::builder` 这一处补齐 `TaskFn`、plugin discovery、tool exposure 与 model exposure。

- **tool / model / plugin 三条 live 面必须共用同一口径**：`register_builtins()`、`ModelCatalog::new_builtin()`、platform registry snapshot、default selections、plugin snapshot，以及 `octopus-infra` 生成的 capability projection 现在并不完全一致。Post-W8 要么把缺失能力接上线，要么把它们从 live surface 一次性隐藏，不能继续“catalog 看得到、执行时失败”。

- **contract reconciliation 放在最后**：只有当 live capability surface 稳定后，才允许改 `/api/v1/runtime/*`、schema、desktop store/fixture 与 `docs/sdk/README.md` 的 Fact-Fix。否则会把临时状态写成新真相源。

## Scope

- In scope：
  - 审计并收口仍在 live runtime 暴露的 stub builtin tools。
  - 把 shared capability consumer path（尤其 `octopus-infra` 里的 `WorkspaceToolCatalogEntry` 投影）纳入 builtin live surface 收口，而不是只改 SDK 内部 catalog。
  - 把 plugin discovery / lifecycle 接到 `octopus-platform` live builder，并先冻结 runtime tools/hooks 与 decl-only component 的边界。
  - 审计并收口仍在 live model catalog / default selection / registry snapshot 暴露的 stub-backed models。
  - 给 live builder 注入真实 `TaskFn`，或明确把 `task` 从 live 面移除。
  - 在 capability surface 稳定后，回填 `contracts/openapi/**`、`packages/schema/**`、`octopus-server` transport、desktop fixtures/tests 与控制文档。
  - 更新 `docs/plans/sdk/02-crate-topology.md`、必要时更新 `00-overview.md §6` 风险登记或 `docs/sdk/README.md` Fact-Fix。
- Out of scope：
  - 重开 W1–W8 的交付范围或把 `04`–`11` 改回 `in_progress`。
  - 新增 provider、plugin marketplace、smart routing、MCPB bundle、UI 新功能。
  - 改变 runtime config / persistence 的权责分层。
  - 在业务页或 transport 层做 page-local / adapter-local 绕行补丁去掩盖 SDK 缺口。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | live tool registry 暴露 stub tool，会让模型与 UI 同时误判“能力存在”。 | Task 2 必须把 executable registry 和 metadata/catalog 口径统一。 | #1 / #8 |
| R2 | 隐藏 stub-backed models 会影响默认选择、已保存 `configuredModels` 和前端下拉。 | Task 4 与 Task 6 必须同批处理 default selections、snapshot 与 transport。 | #3 / #8 |
| R3 | `task_fn` live injection 可能需要 host 持有后台任务 owner，而不仅是 builder 参数。 | 先在 platform 定义 `task` 的唯一 ownership；`task_list` / `task_get` 没有共享 owner 时不一起推进。 | #2 |
| R4 | plugin lifecycle 接入 live path 后，manifest compat / allowlist / denylist 规则若不清楚，容易把错误策略写进 builder。 | 先以 `docs/sdk/12` 为准；若规范层不够支撑，再走 Fact-Fix。 | #8 |
| R5 | `docs/sdk/11` 已要求 provider 通过 plugin registry 暴露，但当前 live model catalog 仍以 builtin hardcode 为主。 | Task 3 先建立 live plugin path 与 decl/runtime 边界；Task 4 再决定 builtin provider 的阶段性处理。 | #8 |
| R6 | 若先改 OpenAPI / schema，再去改 live surface，会把临时能力集冻结成对外合同。 | 严格保持 Task 6 在 Task 2–5 之后。 | #3 |
| R7 | tool catalog / permission metadata 与 executable registry 分离不当，会再引入 UI 展示与实际权限不一致。 | 若需要双视图，必须放在 SDK shared layer，不准在 desktop 本地过滤。 | #1 / #8 |

## 承 W8 的冻结面

- W8 的 `done` 只代表“重构主线与既定 8 周出口状态已完成”，不代表当前 live surface 已无 residual stub / empty-path。
- 本 tranche 只收口 W8 后审计暴露的 capability gap：`web_search` 与 W5 stub tools 仍上 live registry、`task_fn` 在 live builder 中为空、stub-backed model adapters 仍通过 catalog 暴露、plugin lifecycle 只在测试路径跑通。
- `octopus-infra` 仍会从 `builtin_tool_catalog()` 直接生成 `WorkspaceToolCatalogEntry`，desktop fixtures 也仍保留旧 capability set；这些都算 live / shared consumer 残留，不是“UI 本地小问题”。
- 若执行中证明是 `docs/sdk/*` 规范层与现实现状冲突，而不是实现漏接，则回写 `docs/sdk/README.md` `## Fact-Fix 勘误`，不强行把错误 surface 做成“实现完成”。
- formal completion 仍需继续推进时，只能转交新的收口计划，不得把本 tranche 的 `done` 状态改回 `in_progress`。

## 公共面变更登记

| 变更点 | 登记位置 | 当前冻结结论 | 触发条件 |
|---|---|---|---|
| live builtin tool 暴露规则（含 `task` 的 `TaskFn` 依赖） | `docs/plans/sdk/02-crate-topology.md §2.4 / §2.14 / §3.1` | live registry 只能注册真实可执行 tool；若需要 metadata-only builtin，必须明确区分 catalog 与 runtime registry。 | Task 2 / Task 5 |
| plugin discovery / lifecycle / snapshot 的 live path | `docs/plans/sdk/02-crate-topology.md §2.11 / §2.14 / §3.1` | `PluginRegistry` 不能再以空实例进入 live builder；snapshot 必须来自真实 discovery + 已加载 plugin set。 | Task 3 |
| stub-backed model family 的 live catalog 暴露规则 | `docs/plans/sdk/02-crate-topology.md §2.3 / §3.1 / §5` | `ModelCatalog`、role defaults、platform snapshot、runtime config 默认选择与 transport DTO 必须对同一模型集给出同一答案。 | Task 4 |
| `/api/v1/runtime/*` 的 capability-facing DTO | `docs/plans/sdk/02-crate-topology.md §5` + `docs/sdk/README.md` `## Fact-Fix 勘误` | 只在 live surface 稳定后修改；禁止先改 transport 再补 builder。 | Task 6 |

## 退役登记

> 本 tranche 默认**不新增 legacy crate 退役**；若执行中发生“物理删除 residual compatibility shim / placeholder path”，必须同批回填 `03-legacy-retirement.md`。

| 潜在退役项 | `03` 回填位置 | 当前结论 | 触发条件 |
|---|---|---|---|
| live builder 中的 stub-only capability exposure | `docs/plans/sdk/03-legacy-retirement.md §6.1`（若删除的是 platform bridge 残余兼容层） | 当前先以“隐藏或替换”处理，不预设物理删除。 | Task 2 / Task 3 / Task 5 真删兼容代码时 |
| stub protocol adapter 的过渡兼容路径 | `docs/plans/sdk/03-legacy-retirement.md §5.1`（若删旧 adapter 对齐垫片） | 当前先以 capability gating 为主。 | Task 4 真删桥接或 fallback shim 时 |

## Exit Gate 对齐表（Post-W8）

> `00-overview.md §3` 的 W1–W8 出口状态保持不变。本 tranche 单独管理 live surface hardening 的出口门禁。

| Exit Gate | 本 tranche 落点 | 验证 |
|---|---|---|
| live runtime 与 shared capability projection 不再把 stub-only builtin tool 暴露给模型或 host catalog | Task 2 / Task 5 | `cargo test -p octopus-sdk-tools -p octopus-platform -p octopus-infra` |
| plugin discovery / lifecycle 进入 live builder，snapshot 反映真实发现结果，且 runtime/decl 边界冻结 | Task 3 | `cargo test -p octopus-sdk-plugin -p octopus-sdk-core -p octopus-platform` |
| platform snapshot / default selections 不再指向 `AdapterNotImplemented` 的模型家族 | Task 4 | `cargo test -p octopus-sdk-model -p octopus-platform` |
| `/api/v1/runtime/*`、schema 与 desktop fixtures 只暴露稳定后的 capability set | Task 6 | `pnpm openapi:bundle && pnpm schema:generate && cargo test -p octopus-server && pnpm -C apps/desktop test` |
| 控制文档、README、checkpoint、变更日志与最终实现一致 | Task 7 | `cargo test --workspace && cargo clippy --workspace -- -D warnings && pnpm -C apps/desktop test` |

## Execution Rules

- 先做 capability policy freeze，再做代码；不要边实现边决定“这个能力到底该不该 live 暴露”。
- 不准在 `apps/desktop` / `octopus-server` 本地过滤 stub surfaces 来掩盖 SDK/platform 缺口；shared-layer ownership 必须回到 `octopus-sdk-*` 或 `octopus-platform`。
- `octopus-infra` 产出的 tool/capability projection 也算 shared layer；只改 `builtin_tool_catalog()` 但不改 projection 路径，视为未完成 Task 2。
- 若需要同时保留 metadata 与 executable 两个视图，必须在共享层明确建模，不准让 UI 自己猜。
- `Task 3` 必须先冻结 plugin runtime / decl 边界，再进入 `Task 4` 的 model surface 收口；不允许一边改 model catalog 一边临时决定 plugin provider 要不要 live。
- `task_list` / `task_get` 不跟随 `task` 自动进入实现范围；若找不到共享 host owner，按 Task 2 继续留在 non-live。
- `contracts/openapi/**`、`packages/schema/**`、desktop fixtures 只在 Task 6 进入；之前一律禁止。
- 任何实现选择若会推翻 `docs/sdk/*` 的规范性表述，先写 `docs/sdk/README.md` Fact-Fix，再继续。
- 每批次 diff 默认 ≤ 800 行；超出就继续拆 batch。

## Task Ledger

### Task 1: 冻结 post-W8 scope 与 live surface policy

Status: `done`

Files:
- Modify: `docs/plans/sdk/12-post-w8-capability-hardening.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/00-overview.md`（仅当需要补风险登记或 tranche 注记）
- Modify: `docs/sdk/README.md`（仅当命中 Fact-Fix）

Preconditions:
- W8 `11-week-8-cleanup-and-split.md` 已完成。
- live builder、builtin tools、stub adapters、plugin lifecycle 的现状证据已复核。

Step 1:
- Action: 把当前 live capability gap 整理成统一矩阵：每个 tool / model family / plugin path 只能有一个结论：`implement now`、`hide from live surface`、或 `defer but non-live`。
- Done when: 后续 Task 2–Task 6 不再需要临时决定“某个能力是否应该对外可见”。
- Verify: `rg -n "register_builtins|WebSearchTool|SkillTool|TaskListTool|TaskGetTool|task_fn: None|PluginRegistry::new\\(|ModelCatalog::new_builtin|OpenAiResponsesAdapter|GeminiNativeAdapter|VendorNativeAdapter" crates/octopus-platform crates/octopus-sdk-tools crates/octopus-sdk-model crates/octopus-sdk-plugin`
- Stop if: 结论会把 W1–W8 的完成态直接推翻，而不是作为 follow-up hardening 收口。

Step 2:
- Action: 同批回填 `02-crate-topology.md` 的相关小节；若发现规范层冲突，再补 `docs/sdk/README.md` Fact-Fix 或 `00-overview.md §6` 风险登记。
- Done when: 控制文档不再默认假设这些 residual stub surfaces 已经 live。
- Verify: `rg -n "post-W8|capability hardening|TaskFn|PluginRegistry|OpenAiResponses|GeminiNative|VendorNative" docs/plans/sdk/{12-post-w8-capability-hardening.md,02-crate-topology.md,00-overview.md}`
- Stop if: 需要直接改 `docs/sdk/*` 正文才能解释当前阶段目标。

### Task 2: 审计并收口 stub builtin tools 的 live 暴露

Status: `done`

Files:
- Modify: `crates/octopus-sdk-tools/src/builtin/mod.rs`
- Modify: `crates/octopus-sdk-tools/src/builtin/catalog.rs`
- Modify: `crates/octopus-sdk-tools/src/builtin/w5_stubs.rs`
- Modify: `crates/octopus-sdk-tools/src/builtin/web_search.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/builder.rs`
- Modify: `crates/octopus-infra/src/resources_skills/service.rs`
- Modify: `crates/octopus-sdk-tools/tests/builtin_stubs.rs`
- Modify: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`
- Modify: `crates/octopus-infra/src/resources_skills/tests.rs`

Preconditions:
- Task 1 已冻结每个 builtin 的 live policy。

Step 1:
- Action: 把 `web_search`、`skill`、`task_list`、`task_get` 以及“无 `TaskFn` 的 `task`”从 stub 直注册状态改成统一的 live gating 规则。
- Done when: live runtime 不再把执行时只会返回 `NotYetImplemented` / `TaskFn not injected` 的 tool 注册给模型。
- Verify: `cargo test -p octopus-sdk-tools -p octopus-platform`
- Stop if: 现有 SDK 公共面无法同时表达 executable registry 与 metadata/catalog 的差异。

Step 2:
- Action: 对齐 shared builtin catalog consumer paths、permission metadata 与测试，至少覆盖 `octopus-infra` 生成的 `WorkspaceToolCatalogEntry`；desktop fixtures 保持冻结，统一留到 Task 6。
- Done when: shared layer 不再把 stub tool 投影成健康可执行 capability，且不再需要在下游业务侧维持私有 stub denylist。
- Verify: `cargo test -p octopus-sdk-tools -p octopus-platform -p octopus-infra`
- Stop if: 需要在业务侧继续保留一套独立 builtin 名单才能维持现有页面行为。

### Task 3: 把 plugin discovery / lifecycle 接到 live builder

Status: `done`

Files:
- Modify: `crates/octopus-platform/src/runtime_sdk/{builder.rs,mod.rs}`
- Create: `crates/octopus-platform/src/runtime_sdk/plugin_boot.rs`（如需要）
- Modify: `crates/octopus-sdk-plugin/src/{lifecycle.rs,bundled.rs,manifest.rs}`（如需要）
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge/{snapshot.rs,builtins.rs}`
- Modify: `crates/octopus-platform/tests/{runtime_sdk_bridge.rs,runtime_config_bridge.rs}`

Preconditions:
- Task 1 已冻结 plugin live path 的 ownership 与目标态。

Step 1:
- Action: 基于 workspace/user/plugin roots 与明确的 loaded plugin set 构造 discovery config，在 live builder 中真正执行 `PluginLifecycle::run()`，而不是传空 `PluginRegistry`。
- Done when: `plugins_snapshot` 来自真实 discovery 结果，且 live builder 明确持有一组唯一的 bundled / local plugin implementation。
- Verify: `cargo test -p octopus-sdk-plugin -p octopus-sdk-core -p octopus-platform`
- Stop if: 当前代码无法给出 live builder 持有哪组 `Box<dyn Plugin>` 的唯一 owner。

Step 2:
- Action: 冻结 plugin component 的 runtime / decl-only 边界：`tools/hooks` 进入 live runtime；`skill` / `model_provider` / `MCP` 若当前只有声明级登记，就明确保持 decl-only 或 defer，不把它们伪装成已 live 可执行。
- Done when: plugin extensibility 的 live 口径不再把 runtime tool/hook 与 decl-only provider/skill/MCP 混成一件事。
- Verify: `cargo test -p octopus-sdk-plugin -p octopus-sdk-core -p octopus-platform`
- Stop if: 接线只能靠在 `server` / `desktop` 重建一套并行注册逻辑。

### Task 4: 审计并收口 stub-backed models 的 live catalog

Status: `done`

Files:
- Modify: `crates/octopus-sdk-model/src/adapter/stubs.rs`
- Modify: `crates/octopus-sdk-model/src/catalog/builtin/{openai.rs,google.rs,minimax.rs,ark.rs}`
- Modify: `crates/octopus-sdk-model/src/role_router.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/builder.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge/{builtins.rs,snapshot.rs,overrides.rs}`
- Modify: `crates/octopus-sdk-model/tests/{catalog_builtin.rs,role_router.rs,fallback.rs}`
- Modify: `crates/octopus-platform/tests/runtime_config_bridge.rs`

Preconditions:
- Task 1 已冻结每个 stub-backed model family 的处理策略。
- Task 3 已冻结 plugin runtime / decl-only 边界，尤其是 plugin-provided model/provider 在本 tranche 的 live policy。

Step 1:
- Action: 对 `OpenAiResponsesAdapter`、`GeminiNativeAdapter`、`VendorNativeAdapter` 逐个做二选一：补 live adapter，或把其 surface/model/default alias 从 live catalog 与 platform snapshot 隐去。
- Done when: live catalog 中出现的模型都能走到真实 adapter，而不是 `AdapterNotImplemented`。
- Verify: `cargo test -p octopus-sdk-model -p octopus-platform`
- Stop if: 隐藏某个 family 会导致已有 `configuredModels` 或 role defaults 无法给出稳定迁移策略。

Step 2:
- Action: 收口 role defaults、canonical defaults、alias 解析、configuredModels fallback/migration、provider metadata 与 registry snapshot 的同一口径。
- Done when: `ModelCatalog`、platform snapshot、runtime config 默认选择与 UI DTO 不再各说各话。
- Verify: `cargo test -p octopus-sdk-model -p octopus-platform`
- Stop if: 需要让 `server` / `desktop` 自己修正模型列表才能通过测试。

### Task 5: 把 `task_fn` / `task` tool 接到 live runtime

Status: `done`

Files:
- Modify: `crates/octopus-platform/src/runtime_sdk/{builder.rs,mod.rs}`
- Create: `crates/octopus-platform/src/runtime_sdk/subagent_runtime.rs`（如需要）
- Modify: `crates/octopus-sdk-core/src/{plugin_boot.rs,subagent_boot.rs}`
- Modify: `crates/octopus-sdk-tools/src/builtin/w5_stubs.rs`
- Modify: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`

Preconditions:
- Task 2 已确认 `task` 的 live exposure 规则。
- `docs/sdk/05` 的 subagent contract 仍可作为唯一规范源。

Step 1:
- Action: 在 `octopus-platform` 定义 live `TaskFn` ownership，并把它注入 `RuntimeSdkFactory::build_live()`；`task_list` / `task_get` 若仍无共享 host owner，则继续按 Task 2 保持 non-live。
- Done when: live builder 不再默认产出“带 `task` 名字但没有 `TaskFn`”的 runtime。
- Verify: `cargo test -p octopus-platform -p octopus-sdk-core -p octopus-sdk-tools`
- Stop if: 子代理执行需要额外的新 host contract，而 `docs/sdk/05` 与当前 platform surface 都没有 owner。

Step 2:
- Action: 新增或更新 integration tests，覆盖 `task` 在 live runtime 中“可执行”或“不可见”两种唯一正确形态。
- Done when: 再也不会出现用户态看到 `task`，执行后只回 `TaskFn not injected`。
- Verify: `cargo test -p octopus-platform -p octopus-sdk-core -p octopus-sdk-tools`
- Stop if: 需要引入第二套 runtime loop 或 page-local worker queue 才能让测试通过。

### Task 6: 在 capability surface 稳定后回填 contracts / schema / transport

Status: `done`

Files:
- Modify: `contracts/openapi/src/**`
- Modify: `packages/schema/src/**`
- Modify: `crates/octopus-server/src/workspace_runtime/**`
- Modify: `apps/desktop/src/tauri/workspace-client.ts`
- Modify: `apps/desktop/src/stores/**`
- Modify: `apps/desktop/test/**`
- Modify: `docs/sdk/README.md`（若命中 Fact-Fix）

Preconditions:
- Task 2–Task 5 已完成或明确 `blocked` 且处理策略已冻结。

Step 1:
- Action: 逐条审计 `/api/v1/runtime/*`、desktop store/fixture、CLI capability-facing output，确认哪些字段是旧 capability set 残留。
- Done when: 所有 contract 变更都能回指到已经稳定的 live surface，而不是临时 builder 状态。
- Verify: `rg -n "tool|model|plugin|configuredModels|pluginsSnapshot" contracts/openapi/src packages/schema/src crates/octopus-server apps/desktop/src apps/desktop/test`
- Stop if: contract 需求仍依赖尚未稳定的 builder 行为。

Step 2:
- Action: 严格按 OpenAPI-first 更新源文件、bundle、schema、server transport 与 desktop fixtures/tests。
- Done when: server/desktop/CLI 对外只暴露稳定 capability set，且不再保留“实际上不可执行”的 UI/DTO 痕迹。
- Verify: `pnpm openapi:bundle && pnpm schema:generate && cargo test -p octopus-server && pnpm -C apps/desktop test`
- Stop if: 更新只能靠业务页私有 DTO 或本地 mock shape 才能完成。

### Task 7: Exit Gate、控制文档与变更日志收口

Status: `done`

Files:
- Modify: `docs/plans/sdk/12-post-w8-capability-hardening.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（若本 tranche 发生物理删除）
- Modify: `docs/plans/sdk/00-overview.md`（若需补风险登记）
- Modify: `docs/sdk/README.md`（仅 Fact-Fix）

Preconditions:
- Task 1–Task 6 全部完成或明确 `blocked`。

Step 1:
- Action: 跑 Post-W8 Exit Gate、补 checkpoint、状态、变更日志与 README 索引状态。
- Done when: 本文状态切到 `done` 或 `blocked`，并且控制文档与实现保持一致。
- Verify: `cargo test --workspace && cargo clippy --workspace -- -D warnings && pnpm -C apps/desktop test`
- Stop if: 仍有 live visible capability 依赖 stub / empty registry path。

Step 2:
- Action: 若本 tranche 发生对 `docs/sdk/*` 的规范修正、compat shim 删除或 contract 改写，同批完成 `Fact-Fix` 与 `03-legacy-retirement.md` 回填。
- Done when: 没有“代码已变、控制面未记”的残留。
- Verify: `rg -n "12-post-w8-capability-hardening|Fact-Fix|AdapterNotImplemented|TaskFn not injected" docs/plans/sdk docs/sdk/README.md`
- Stop if: 文档回填需要重新解释 W1–W8 的完成态。

## Checkpoint 2026-04-23 09:39

- Batch: 起稿 `12-post-w8-capability-hardening.md` -> 更新 `README.md` 索引
- Completed:
  - 新建 Post-W8 follow-up tranche 计划
  - 在索引登记 `12-post-w8-capability-hardening.md`
- Verification:
  - `rg -n "12-post-w8-capability-hardening.md" docs/plans/sdk/README.md` -> pass
- Blockers:
  - none
- Next:
  - Task 1 Step 1

## Checkpoint 2026-04-23 10:43

- Week: Post-W8
- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed:
  - 冻结 builtin tools / plugin paths / stub-backed model families 的唯一 live policy matrix
  - 把 plan 状态切到 `in_progress`，并把当前执行位置推进到 Task 2 Step 1
  - 明确共享 consumer path 也属于 hardening 范围，不允许只改 builder 不改 projection
- Files changed:
  - `docs/plans/sdk/12-post-w8-capability-hardening.md` (modified)
  - `docs/plans/sdk/02-crate-topology.md` (modified)
  - `docs/plans/sdk/README.md` (modified)
- Verification:
  - `rg -n "register_builtins|WebSearchTool|SkillTool|TaskListTool|TaskGetTool|task_fn: None|PluginRegistry::new\\(|ModelCatalog::new_builtin|OpenAiResponsesAdapter|GeminiNativeAdapter|VendorNativeAdapter" crates/octopus-platform crates/octopus-sdk-tools crates/octopus-sdk-model crates/octopus-sdk-plugin` -> pass
  - `rg -n "post-W8|capability hardening|TaskFn|PluginRegistry|OpenAiResponses|GeminiNative|VendorNative" docs/plans/sdk/{12-post-w8-capability-hardening.md,02-crate-topology.md,00-overview.md}` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - 基线 `cargo test -p octopus-sdk-tools -p octopus-platform -p octopus-sdk-model -p octopus-sdk-plugin -p octopus-infra` 首次命中 `octopus-sdk-model::provider_impl::tests::fallback_triggers_on_overloaded_then_succeeds` 的波动失败；单测重跑通过，先登记为现有噪音，不把它并入 Task 1 结论
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-23 11:24

- Week: Post-W8
- Batch: Task 2 Step 1 -> Task 2 Step 2
- Completed:
  - 把 `register_builtins()` 收窄到当前真实 live builtin，移除 `web_search`、`task`、`skill`、`task_list`、`task_get` 的默认注册
  - 把 `builtin_tool_catalog()` 同步收窄到同一能力面，避免 shared layer 继续把 stub builtin 当成健康 capability
  - 补 `octopus-sdk-tools` / `octopus-platform` / `octopus-infra` 测试，锁定 live registry 与 capability projection 的新口径
- Files changed:
  - `crates/octopus-sdk-tools/src/builtin/mod.rs` (modified)
  - `crates/octopus-sdk-tools/src/builtin/catalog.rs` (modified)
  - `crates/octopus-sdk-tools/tests/builtin_stubs.rs` (modified)
  - `crates/octopus-platform/tests/runtime_sdk_bridge.rs` (modified)
  - `crates/octopus-infra/src/resources_skills/service.rs` (modified)
  - `crates/octopus-infra/src/resources_skills/tests.rs` (modified)
- Verification:
  - `cargo test -p octopus-sdk-tools -p octopus-platform` -> pass
  - `cargo test -p octopus-sdk-tools -p octopus-platform -p octopus-infra` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-23 11:25

- Week: Post-W8
- Batch: Task 3 Step 1 -> Task 3 Step 2
- Completed:
  - 新增 `runtime_sdk::plugin_boot`，由 `octopus-platform` 统一组装 live plugin discovery config，并以 `example_bundled_plugins()` 作为当前唯一 loaded plugin set
  - `RuntimeSdkFactory::build_live()` 不再注入空 `PluginRegistry`，而是实际执行 `PluginLifecycle::run()` 并把真实 `plugins_snapshot` 交给 runtime
  - 保持 plugin runtime / decl-only 边界不变：本批只接通 runtime `tools/hooks` 所在 registry/snapshot，未把 `skill` / `model_provider` / `MCP` 伪装成 live executable capability
  - 补平台测试，锁定 live builder 持久化的 session snapshot 含 `example-noop-tool` 且来源为 `bundled`
- Files changed:
  - `crates/octopus-platform/Cargo.toml` (modified)
  - `crates/octopus-platform/src/runtime_sdk/builder.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/plugin_boot.rs` (added)
  - `crates/octopus-platform/tests/runtime_sdk_bridge.rs` (modified)
- Verification:
  - `cargo test -p octopus-sdk-plugin -p octopus-sdk-core -p octopus-platform` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-23 11:44

- Week: Post-W8
- Batch: Task 4 Step 1 -> Task 4 Step 2
- Completed:
  - 从 `sdk-model` builtin catalog、alias 与 role defaults 中移除仍然走 stub adapter 的 `gpt-5.4*`、`gemini-2.5-*`、`MiniMax-M2.7`
  - 把 platform `registry_bridge` 的 canonical defaults、provider metadata、snapshot 和 override 逻辑一起收口到同一 live 模型集
  - 对命中 hidden builtin 的已保存 `configuredModels` 保留配置可见性，但统一降级为 `unsupported`，并补 warning/test 锁定迁移行为
- Files changed:
  - `crates/octopus-sdk-model/src/catalog/builtin/openai.rs` (modified)
  - `crates/octopus-sdk-model/src/catalog/builtin/google.rs` (modified)
  - `crates/octopus-sdk-model/src/catalog/builtin/minimax.rs` (modified)
  - `crates/octopus-sdk-model/src/role_router.rs` (modified)
  - `crates/octopus-sdk-model/tests/catalog_builtin.rs` (modified)
  - `crates/octopus-sdk-model/tests/role_router.rs` (modified)
  - `crates/octopus-sdk-model/tests/fallback.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge/builtins.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge/snapshot.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge/overrides.rs` (modified)
  - `crates/octopus-platform/tests/runtime_config_bridge.rs` (modified)
- Verification:
  - `cargo test -p octopus-sdk-model -p octopus-platform` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-23 12:13

- Week: Post-W8
- Batch: Task 5 Step 1 -> Task 5 Step 2
- Completed:
  - 在 `octopus-sdk-tools` 给 `task` 调用补上父会话上下文透传，让 live `TaskFn` 能按当前 `ToolContext.session_id` 构造真实 parent session
  - 新增 `runtime_sdk::subagent_runtime`，由 `octopus-platform` 持有 live `TaskFn` owner，组装含 plugin tools 的 subagent parent registry，并把 `task_fn` 注入 `RuntimeSdkFactory::build_live()`
  - 补平台级测试，锁定两种正确形态：默认 live builtin registry 仍不裸露 stub `task`；一旦 builder 提供 `task_fn`，`task` 会被注入并执行，不再回 `TaskFn not injected`
- Files changed:
  - `crates/octopus-sdk-tools/src/task_fn.rs` (modified)
  - `crates/octopus-sdk-tools/src/builtin/w5_stubs.rs` (modified)
  - `crates/octopus-platform/Cargo.toml` (modified)
  - `crates/octopus-platform/src/runtime_sdk/builder.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/subagent_runtime.rs` (added)
  - `crates/octopus-platform/tests/runtime_sdk_bridge.rs` (modified)
- Verification:
  - `cargo test -p octopus-platform -p octopus-sdk-core -p octopus-sdk-tools` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 6 Step 1

## Checkpoint 2026-04-23 12:24

- Week: Post-W8
- Batch: Task 6 Step 1 -> Task 6 Step 2
- Completed:
  - 审计 `/api/v1/runtime/*`、schema、desktop fixture 与测试引用后，确认这批 residual 只落在 server test helper 和 desktop fixture/test，不需要改 `contracts/openapi/src/**` 或 `packages/schema/src/**` 源文件
  - 把 server 里的 `generation-only-model` 从 hidden `gemini-2.5-flash` 切到显式声明的 workspace-scoped OpenAI single-shot generation model，避免测试继续依赖已冻结为 non-live 的 builtin family
  - 从 desktop workspace fixture 中移除 `builtin:web_search` 的假健康 builtin 投影，并把 tools 分页测试改成自带第 7 条 builtin fixture，不再依赖旧 capability set
- Files changed:
  - `crates/octopus-server/src/workspace_runtime/tests/support.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime/tests/support_workspace.rs` (modified)
  - `apps/desktop/test/support/workspace-fixture-state.ts` (modified)
  - `apps/desktop/test/tools-view.test.ts` (modified)
- Verification:
  - `cargo test -p octopus-server` -> pass
  - `pnpm openapi:bundle` -> pass
  - `pnpm schema:generate` -> pass
  - `pnpm -C apps/desktop test` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 7 Step 1

## Checkpoint 2026-04-23 12:29

- Week: Post-W8
- Batch: Task 7 Step 1 -> Task 7 Step 2
- Completed:
  - 跑完 Post-W8 exit gate，全仓测试、desktop 全量测试、OpenAPI bundle、schema generate 全部通过
  - 为通过 `cargo clippy --workspace -- -D warnings`，补了 `octopus-sdk-session` 中一处 `ignored_unit_patterns` 小修，不涉及 capability 语义
  - 复核生成物后确认 `contracts/openapi/octopus.openapi.yaml` 与 `packages/schema/src/generated.ts` 无额外 diff，本 tranche 不需要再补 `Fact-Fix` 或 `03-legacy-retirement.md`
- Files changed:
  - `crates/octopus-sdk-session/src/sqlite/schema.rs` (modified)
  - `docs/plans/sdk/12-post-w8-capability-hardening.md` (modified)
- Verification:
  - `cargo test --workspace` -> pass
  - `cargo clippy --workspace -- -D warnings` -> pass
  - `pnpm -C apps/desktop test` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - plan complete

## Checkpoint 2026-04-23 12:43

- Week: Post-W8
- Batch: 审计残留修复
- Completed:
  - 根据实施完成审计结论，补修 `packages/schema` 里遗漏的 `web_search` capability union 残留，避免 schema 继续把已冻结为 non-live 的能力当成正式模型能力值
  - 同步移除 desktop locale 中对应的 capability 文案，清掉 UI 层对旧 capability id 的显式枚举痕迹
  - 复核代码引用后确认当前 live runtime、OpenAPI source 与 desktop fixture 都不再保留 `web_search` 的 capability-facing contract 残留
- Files changed:
  - `packages/schema/src/catalog.ts` (modified)
  - `apps/desktop/src/locales/en-US.json` (modified)
  - `apps/desktop/src/locales/zh-CN.json` (modified)
  - `docs/plans/sdk/12-post-w8-capability-hardening.md` (modified)
- Verification:
  - `rg -n "web_search|task_list|task_get|gpt-5\\.4|gemini-2\\.5|MiniMax-M2\\.7" contracts/openapi/src packages/schema/src apps/desktop/src` -> only builtin stub implementation/tests remain; no capability-facing contract hit for `web_search`
  - `pnpm -C apps/desktop test` -> pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - plan complete

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-23 | 首稿：新增 Post-W8 follow-up tranche，冻结“先收口 live capability surface，再做 contract reconciliation”的执行顺序；补 Task Ledger、公共面登记、Exit Gate 与 checkpoint。 | Codex |
| 2026-04-23 | 按代码现状审计收紧 Task Ledger：Task 2 补 `octopus-infra` consumer path 并把 desktop fixtures 明确后移到 Task 6；Task 3 改为 plugin live bootstrap + runtime/decl 边界冻结；Task 4 承接 model surface 收口并补 `role_router.rs` / `registry_bridge/overrides.rs`；Task 5 聚焦 `task_fn` ownership。 | Codex |
| 2026-04-23 | 审计补修 residual contract 痕迹：移除 `packages/schema` 与 desktop locale 中遗漏的 `web_search` capability 枚举/文案，补一条 post-audit checkpoint。 | Codex |
| 2026-04-23 | formal closeout 边界补记：明确本 tranche `done` 仅覆盖 live capability hardening；控制文档对齐、crate 归属收口和 `support.rs` ≤ 800 行门禁转交 `13-finalization-and-deferred-capabilities.md`。 | Codex |
| 2026-04-23 | Task 4 冻结 deferred capability 边界：新增三类后续能力的 owner / 代码证据 / live re-entry 触点表，明确 `13` 只维持冻结口径，不重开本 tranche 实施。 | Codex |
