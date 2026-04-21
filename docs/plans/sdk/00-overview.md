# 00 · SDK 重构总控计划

> **For Codex / Claude / Cursor Agent:** REQUIRED SUB-SKILL — Use `superpowers:executing-plans` 执行本目录的每一份子 Plan。
> 本文档是"方向/骨架/门禁"；任何实施必须在 `04–11-week-*.md` 子 Plan 中落到具体任务。

## Goal

在 **6–8 周**内完成一次**一次性彻底重构**：把现有 `crates/runtime` + `crates/tools` + `crates/plugins` + `crates/octopus-runtime-adapter` + `crates/api` 五条路径合并为一个遵循 `docs/sdk/01–14` 的 SDK 矩阵（14 个子 crate + 1 个顶层门面 crate），并让业务侧（`octopus-platform` / `octopus-server` / `octopus-desktop` / `octopus-cli`）只通过 `octopus-sdk` 这一个公共面使用所有 Agent 能力。

## Architecture

- **分层所有权**：SDK = 能力层；`octopus-platform` = 业务域；`octopus-server` = HTTP 路由；`octopus-desktop` = Tauri 宿主；`octopus-cli` = CLI 入口。四层单向依赖 SDK，不反向注入业务概念。
- **主流形**：业务层只持有 `AgentRuntime`、`SessionStore`、`ModelProvider`、`SecretVault` 四个 trait；所有 Brain / Hands / Session / Plugin / Model / UI-Intent 细节封闭在 SDK 内部。
- **正确层**：窄接口 + 注入式依赖让 `docs/sdk/04` 的三分架构第一次真正落地；legacy "adapter 胶水层"概念被删除。

## Scope

- In scope：
  - 创建 14 个 `octopus-sdk-*` 子 crate + 顶层 `octopus-sdk` re-export。
  - 迁移现有 Anthropic/OpenAI 提供商、内置 15 工具、MCP client、Skill 加载器、权限、钩子、沙箱、上下文压缩、子代理编排、插件 Manifest/Registry、UI Intent IR 产出。
  - 业务侧 `octopus-server` / `octopus-desktop` / `octopus-cli` 接线到新 SDK；`octopus-runtime-adapter` 全量下线。
  - 启用 `octopus-persistence` 作为全局唯一 SQLite 入口。
  - 拆分 `octopus-core/lib.rs`（3861 行）、`octopus-infra/*.rs`（单文件 2–5K 行）按功能切小文件。
- Out of scope：
  - Rust core 以外的语言 binding（TS/Py）——推迟到 SDK 稳定后的 Phase 2。
  - Smart Routing、远端 model catalog 同步、MCPB 离线 bundle（保留为 plugin 形态的 extension point，不入 W1–W8）。
  - 业务域新功能（Team / Workflow 的新行为）——重构期仅保留现有可跑通的最小面。

## Risks Or Open Questions

- 已采纳 10 项取舍：见 §1；其余以本文档为准。
- 若 W1–W3 发现 Anthropic prompt cache 命中率下降（对比基线 < 80%），**立即进入回滚评估**并更新总控路线。
- 若 `contracts/openapi` 侧的 DTO 与 SDK 侧 IR 出现不可调和的字段差（枚举收敛 / 字段重命名），**暂停实施**，先走一轮 `contracts/openapi/src/**` 更新。

## Execution Rules

以下规则在本目录所有子 Plan 中**无条件适用**：

1. 不在实施期移动 `docs/sdk/*` 规范文件；若规范与实现冲突，优先在实现中让步；必须修规范时，先开子 Plan。
2. 不把业务域对象（Project / Task / Workspace / Deliverable / User / Org）引入 SDK crate 的 `Cargo.toml` 或 `use` 语句。
3. 不允许跨 SDK 子 crate 反向依赖；依赖方向见 `02-crate-topology.md` §依赖图。
4. 不允许手改生成物；HTTP 契约变更必须走 `contracts/openapi/src/** → pnpm openapi:bundle → pnpm schema:generate`。
5. 不允许"跳过 checkpoint 合并推进"；每周结束必须完成 §4 全量 checkpoint。
6. 不允许单 PR 超过 **800 行 diff（不含 generated）**；超出必须拆分。
7. 任何新 SDK 公共符号（`pub` trait/struct/fn）在合入前必须在 `02-crate-topology.md` 的"对外公共面"表中登记。

---

## 1. 已锁定的 10 项取舍（确认基线）

> 与首次方案评审一致；后续子 Plan 不得再翻案，需要变更时必须新开专项决策 Plan 并在本节追加行。

| # | 决策 | 说明 / 依据 |
|---|---|---|
| 1 | SDK 语言：Rust-only（Phase 1） | 桌面 TS / CLI / Desktop 仍经 HTTP / IPC；binding 推到 Phase 2 |
| 2 | 删除 `Capability Planner / Surface / Exposure` 整套 | 回到 `Tool + MCP + Skill` 三段式（`docs/sdk/03`）；违反 KISS |
| 3 | 全量下线 `octopus-runtime-adapter` | legacy/new 胶水层取消；职责按 SDK 分解 |
| 4 | 内置工具走 MCP in-process shim | "内置 vs MCP"统一为一种协议；对齐 `docs/sdk/03 §Code Execution Mode` |
| 5 | Session 持久化双通道 | SQLite projection + `runtime/events/*.jsonl`；删 `runtime/sessions/*.json` 作恢复源 |
| 6 | UI Intent IR 作为 Session 事件 payload 输出 | SDK 产出 IR，业务消费；对齐 `docs/sdk/14` |
| 7 | 重构期最小业务面 | 桌面 smoke + `/api/v1/runtime/sessions` 基础 CRUD；Team/Workflow 降级到"可见但非核心路径" |
| 8 | `docs/plans/runtime/phase-*` 作废大部分 | 仅保留 Phase 1 资产契约；其余由本 SDK 计划替代 |
| 9 | Model 路由保守化 | 静态角色路由 + 多级 fallback；Smart Routing 作为 plugin，不入核心 |
| 10 | `octopus-core/lib.rs` 按领域切 12 个文件 | 与 `packages/schema/src/*` 粒度对齐 |

---

## 2. 目标 crate 矩阵（概要）

> 完整签名与依赖方向见 `02-crate-topology.md`。此处仅作导航。

### 2.1 SDK 侧（15 crate）

| 层 | Crate | 对业务可见 | 关键产物 |
|---|---|---|---|
| 门面 | `octopus-sdk` | ✅ | `pub use octopus_sdk_core::*` + 受控 re-export |
| 契约 | `octopus-sdk-contracts` | ✅ | `Message / ContentBlock / ToolUse / ToolResult / Event / Usage / RenderBlock / AskPrompt / ArtifactRef / LifecyclePhase` |
| 会话 | `octopus-sdk-session` | ✅（仅 `trait SessionStore`） | 默认 `SqliteJsonlSessionStore` |
| 模型 | `octopus-sdk-model` | ✅（仅 `trait ModelProvider`） | Provider/Surface/Model 三层 + ProtocolAdapter×5 + RoleRouter + Fallback |
| 工具 | `octopus-sdk-tools` | 内部 | `trait Tool` + 15 内置工具 + `partition_tool_calls` |
| MCP | `octopus-sdk-mcp` | 内部 | stdio / http / sdk transports |
| 上下文 | `octopus-sdk-context` | 内部 | `SystemPromptBuilder` / `Compactor` / `MemoryBackend(trait)` |
| 权限 | `octopus-sdk-permissions` | 内部 | `PermissionMode / PermissionGate / ApprovalPrompt` |
| 沙箱 | `octopus-sdk-sandbox` | 内部 | `trait SandboxBackend` + Bubblewrap / Seatbelt |
| 钩子 | `octopus-sdk-hooks` | 内部 | `HookEvent` + `HookRunner` |
| 子代理 | `octopus-sdk-subagent` | 内部 | Orchestrator-Workers + Generator-Evaluator |
| 插件 | `octopus-sdk-plugin` | 内部 | `PluginManifest / PluginRegistry / PluginLifecycle` |
| UI 意图 | `octopus-sdk-ui-intent` | 内部（IR 经事件流暴露） | 10 种 `RenderBlock.kind` + `AskPrompt` / `ArtifactRef` |
| 观测 | `octopus-sdk-observability` | 内部 | `TraceSpan / UsageLedger / ReplayTracer` |
| 核心 | `octopus-sdk-core` | ✅（仅 `AgentRuntime` / `AgentRuntimeBuilder`） | Brain Loop 串联 |

### 2.2 业务侧（5 crate）

| Crate | 职责 | 说明 |
|---|---|---|
| `octopus-platform` | 域对象与用例层 | 删除目前 `runtime.rs` 783 行的跨 SDK 胶水 |
| `octopus-persistence`（**新**） | 唯一 SQLite schema + repository | 所有 `rusqlite::Connection` 走这里 |
| `octopus-server` | Axum + OpenAPI 路由 | `handlers.rs` 4300 行按资源切 10+ 文件 |
| `octopus-desktop` | Tauri 宿主桥 | 替换 `octopus-desktop-backend` |
| `octopus-cli` | CLI 入口 | 合并 `rusty-claude-cli` + `commands` |

### 2.3 被删除（W7 收尾完成；完整退役矩阵见 `03-legacy-retirement.md`）

共 **11 个 crate**：

1. `crates/runtime`
2. `crates/tools`
3. `crates/plugins`
4. `crates/api`
5. `crates/octopus-runtime-adapter`
6. `crates/commands`（合并入 `octopus-cli`）
7. `crates/compat-harness`（无替代，完全废弃）
8. `crates/mock-anthropic-service`（fixture 迁入 `octopus-sdk-model/tests/fixtures/`）
9. `crates/rusty-claude-cli`（合并入 `octopus-cli`）
10. `crates/octopus-desktop-backend`（改名 / 重写为 `octopus-desktop`）
11. `crates/octopus-model-policy`（内嵌入 `octopus-sdk-model`）

---

## 3. 8 周路线

每周对应一个子 Plan 文件（见 §README 索引）。以下只列 **出口状态（exit state）** + **硬门禁（hard gate）**；具体任务见各子 Plan。

### W1 · `octopus-sdk-contracts` + `octopus-sdk-session`

- 出口状态：
  - `octopus-sdk-contracts` 导出全部 IR 类型，被 W2–W9 引用为唯一数据契约。
  - `octopus-sdk-session` 提供 `SessionStore` trait + `SqliteJsonlSessionStore` 默认实现；通过单元测试验证 append / stream / snapshot。
  - `config_snapshot_id` + `effective_config_hash` 作为会话首事件写入的契约测试通过。
- 硬门禁：
  - `cargo test -p octopus-sdk-contracts -p octopus-sdk-session` 全绿。
  - 新契约字段与 `contracts/openapi/src/**` 对齐（允许差异但必须登记到 `02-crate-topology.md §契约差异清单`）。

### W2 · `octopus-sdk-model`

- 出口状态：
  - Provider / Surface / Model 三层模型对象存在；至少两个 ProtocolAdapter（`anthropic_messages` + `openai_chat`）迁移完成。
  - `RoleRouter` + `FallbackPolicy` 覆盖 `main / fast / best / plan / compact` 五个 role。
  - Canonical Naming / model catalog 静态默认来自 `docs/references/vendor-matrix.md`。
- 硬门禁：
  - Prompt cache 基线测试：工具顺序 / system prompt 分段稳定，在 3 次连续调用中 `cache_read_input_tokens` 持续增长（mock 可接受）。
  - `cargo test -p octopus-sdk-model` 全绿。

### W3 · `octopus-sdk-tools` + `octopus-sdk-mcp`

- 出口状态：
  - 15 个内置工具（Read / Write / Edit / Glob / Grep / Bash / WebSearch / WebFetch / AskUserQuestion / TodoWrite / Agent / Skill / Sleep / TaskList / TaskGet）以 `trait Tool` 形式存在。
  - `partition_tool_calls` 严格实现只读并发 / 写串行；默认 `max_concurrency = 10`。
  - MCP stdio / http / sdk 三 transport 通过集成测试。
  - **`crates/tools/src/capability_runtime/*` 与 `adapter::capability_*_bridge.rs` 在本周末全部删除**（取舍 #2）。
- 硬门禁：
  - Bash 工具默认输出上限 `BASH_MAX_OUTPUT_DEFAULT = 30_000` 字符锁定。
  - `rg "capability_runtime|CapabilityPlanner|CapabilitySurface" crates/` 无结果。

### W4 · 权限 / 钩子 / 沙箱 / 上下文

- 出口状态：
  - `PermissionMode` 四态 + `PermissionGate` + `ApprovalPrompt`；对接 `AskPrompt` UI Intent。
  - `HookRunner` 支持 `PreToolUse / PostToolUse / Stop / SessionStart / SessionEnd / UserPromptSubmit / PreCompact / PostCompact`。
  - Bubblewrap / Seatbelt 两个 `SandboxBackend` 最小实现；容器内零凭据。
  - `Compactor`（compaction + tool-result clearing）与 `SystemPromptBuilder` 完成，工具顺序由确定性排序保证。
- 硬门禁：
  - 凭据零暴露合同测试：事件日志扫描不得含任何 `API_KEY / TOKEN / BEARER` 明文。命令：`cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox --test no_credentials_in_events --test no_credentials_leak`
  - `cargo test -p octopus-sdk-permissions -p octopus-sdk-hooks -p octopus-sdk-sandbox -p octopus-sdk-context` 全绿。

### W5 · 子代理 + 插件体系

- 出口状态：
  - `Orchestrator-Workers` + `Generator-Evaluator` 最小可运行示例；子代理独立上下文窗口，返回 condensed 摘要。
  - `PluginManifest / PluginRegistry / PluginLifecycle` 初版 + 最小 native plugin 示例。
  - `ToolRegistry / HookRunner` 通过 executable runtime registration 向插件开放；`SkillDecl / ModelProviderDecl` 在 W5 保持 metadata + builder slot。
- 硬门禁：
  - `crates/plugins/*` + `runtime::plugin_lifecycle` + `runtime::hooks` 四源合一，无重复生命周期。
  - plugin session 快照可从 `SessionStarted` 或紧随其后的 `session.plugins_snapshot` 恢复，并可回放。

### W6 · `octopus-sdk-core`（Brain Loop）

- 出口状态：
  - `AgentRuntimeBuilder::new().with_session_store(...).with_model_provider(...).with_secret_vault(...).build()` 链路可用。
  - 最小端到端链路跑通：CLI → `start_session` → 一轮模型调用 → 触发 `Bash` 工具 → `end_turn`，产生完整事件流与 IR。
- 硬门禁：
  - E2E 集成测试通过：`cargo test -p octopus-sdk --test e2e_min_loop`。
  - 事件流含首条 `session.started`（带 config snapshot）+ `tool.executed` + `assistant.message` + `render.block`。

### W7 · 业务侧切换 + 11 个遗留 crate 下线

- 出口状态：
  - `octopus-server` / `octopus-desktop` / `octopus-cli` 不再依赖任何遗留 crate。
  - §2.3 列出的 **11 个遗留 crate 目录整体删除**；由于 `Cargo.toml` 使用 `crates/*` 通配，无需手动移除 member；`default-members` 按 `02-crate-topology.md §8` 更新为 5 业务 crate + `apps/desktop/src-tauri`。
  - `/api/v1/runtime/*` 在新 SDK 下行为与 W6 出口状态一致。
- 硬门禁：
  - `cargo build --workspace` 全绿；`pnpm -C apps/desktop test` 关键 suite 绿。
  - `rg "(octopus_runtime_adapter|octopus-runtime-adapter|octopus_model_policy|rusty_claude_cli|octopus_desktop_backend|compat_harness|mock_anthropic_service)" crates/ apps/` 无生产代码命中。
  - `ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'` 无结果（11 个目录已全部删除）。
  - OpenAPI 契约快照与 W0 对比：无非预期的破坏性变更。

### W8 · 清理与拆分

- 出口状态：
  - `crates/octopus-persistence` 上线；`octopus-infra / octopus-server / octopus-sdk-session` 统一走它取 `Connection`。
  - `octopus-core/lib.rs` 3861 行按 12 个域切小文件；`octopus-infra/infra_state.rs`（5176）、`agent_assets.rs`（4577）、`projects_teams.rs`（4961）、`access_control.rs`（2983）全部按资源切分，单文件 ≤ 800 行。
  - `octopus-server/handlers.rs`（4300）、`workspace_runtime.rs`（9890）按资源切分。
  - §2.3 列出的 11 个遗留 crate 目录在 W7 已全部删除；本周的验证动作是"grep + `ls crates/` 复核 + 书面 Weekly Gate 勾选"。
- 硬门禁：
  - `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }' | wc -l` 为 0（单文件 ≤ 800 行硬约束）。
  - `rg "runtime/sessions/.*\.json" crates/` 仅命中测试或显式 debug 导出路径。
  - 全仓库 `cargo test --workspace` 全绿 + `pnpm test` 关键 suite 全绿。

---

## 4. Checkpoint 机制（强制）

每周结束、每个子 Plan 的 Task Ledger 完成后，都必须在对应 `04–11-week-*.md` 末尾追加一个 Checkpoint。不允许跳写。

```md
## Checkpoint YYYY-MM-DD HH:MM

- Week: W<n>
- Batch: Task <i> Step <j> → Task <i+1> Step <j>
- Completed:
  - <item>
- Files changed:
  - `path` (+added / -deleted / modified)
- Verification:
  - `cargo test -p <crate>` → pass
  - `pnpm openapi:bundle` → pass
  - `rg "<forbidden pattern>" crates/` → 0 hits
- Exit state vs plan:
  - matches / partial / blocked
- Blockers:
  - <none | 具体问题 + 待人判断点>
- Next:
  - <Task i+1 Step j+1 | Week <n+1> kick-off>
```

- Checkpoint 未写的周视为**未完成**，下周门禁不得开启。
- 若实际进度与出口状态不符，必须在 checkpoint 中标记 `partial` 或 `blocked`，并在总控 `Open Questions` 节追加。

---

## 5. 全局退出条件（Definition of Done）

全部满足方可宣告 SDK 重构完成，否则不得合入主干 release：

1. `octopus-sdk` 作为业务唯一入口：`rg "use (runtime|tools|plugins|api|octopus_runtime_adapter|octopus_model_policy|rusty_claude_cli|octopus_desktop_backend|compat_harness|mock_anthropic_service|commands)::" crates/octopus-{platform,persistence,server,desktop,cli} apps/desktop/src-tauri` 无匹配。
2. §2.3 列出的 **11 个遗留 crate 目录全部不存在**；`Cargo.toml` workspace 使用 `crates/*` 通配无需改动；`default-members` 已按 `02-crate-topology.md §8` 收敛到 5 业务 crate + Tauri app。
3. `ls crates/` 的目录集合与 `02-crate-topology.md §1` 的 15 个 SDK crate + 5 个业务 crate 完全吻合（共 20 个目录）。
4. 全仓库 `cargo test --workspace` 全绿 + `pnpm -C apps/desktop test` 关键 suite 全绿 + `cargo clippy --workspace -- -D warnings` 全绿。
5. Prompt cache 基线测试：工具顺序变更守护测试在 CI 中存在并绿。
6. 凭据零暴露合同测试：事件日志扫描 CI job 绿。
7. `find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }' | wc -l` 为 0（单文件行数硬上限）。
8. `docs/sdk/01–14` 与实现出现矛盾时已在 `docs/sdk/README.md` "## Fact-Fix 勘误" 小节追加条目。
9. `docs/plans/sdk/README.md` 索引状态全部 `done`；本文档 `Goal` 节追加 "Completed YYYY-MM-DD"。

---

## 6. 风险登记簿

> 子 Plan 发现新风险时在此节 append；已识别：

| # | 风险 | 触发条件 | 应对 |
|---|---|---|---|
| R1 | Prompt cache 命中率下降 | W2 / W3 tool 顺序变动 | 工具排序契约测试；命中率 < 80% 立即回滚 |
| R2 | 业务侧 OpenAPI 破坏性变更泄漏到前端 | W7 adapter 下线 | 合入前 `pnpm schema:check` + 桌面 smoke |
| R3 | MCP 子进程泄漏 / handle 不回收 | W3 MCP 迁移 | `octopus-sdk-mcp` 集成测试必须覆盖 process drop |
| R4 | Legacy `runtime/sessions/*.json` 仍被业务隐式依赖 | W1 / W7 | W1 即加 `rg` CI 守护；W7 完全清零 |
| R5 | 单 PR 行数失控 | 每周 | PR gate：diff > 800 行必须拆分（不含 generated） |
| R6 | 子 Plan 发散 | 每周 | 所有子 Plan 必须引用本文件 §10 取舍表；不得翻案 |
| R7 | W2 Weekly Gate 被仓库既有 workspace 门禁阻断 | `cargo build --workspace` 依赖桌面资源文件缺失；`cargo clippy --workspace -- -D warnings` 命中 `crates/tools` / `crates/octopus-runtime-adapter` 既有 lint | 2026-04-21 已缓解：补 desktop sidecar 占位生成、清零相关 clippy、清理 worktree `target/` 后重跑通过；保留为已发生风险记录 |

---

## 7. 与 `docs/sdk/*` 规范的关系

| docs/sdk | 落地位置 |
|---|---|
| `01-core-loop` | `octopus-sdk-core`（W6） |
| `02-context-engineering` | `octopus-sdk-context`（W4） |
| `03-tool-system` | `octopus-sdk-tools` + `octopus-sdk-mcp`（W3） |
| `04-session-brain-hands` | `octopus-sdk-session` + `octopus-sdk-core`（W1, W6） |
| `05-sub-agents` | `octopus-sdk-subagent`（W5） |
| `06-permissions-sandbox` | `octopus-sdk-permissions` + `octopus-sdk-sandbox`（W4） |
| `07-hooks-lifecycle` | `octopus-sdk-hooks`（W4） |
| `08-long-horizon` | `octopus-sdk-context` + `octopus-sdk-subagent`（W4, W5） |
| `09-observability-eval` | `octopus-sdk-observability`（跨 W1–W8） |
| `10-failure-modes` | 跨所有 crate；作为"风险登记簿"输入 |
| `11-model-system` | `octopus-sdk-model`（W2） |
| `12-plugin-system` | `octopus-sdk-plugin`（W5） |
| `13-contracts-map` | `02-crate-topology.md §契约差异清单`（W1 起持续维护） |
| `14-ui-intent-ir` | `octopus-sdk-ui-intent` + `octopus-sdk-observability`（跨 W4–W6） |

---

## 8. 时间轴甘特（参考）

```
Week:  W1       W2       W3       W4       W5       W6       W7       W8
       ┌────────┐
contrt │████████│
+sess  └────────┘
                 ┌────────┐
model            │████████│
                 └────────┘
                          ┌────────┐
tools+mcp                 │████████│
                          └────────┘
                                   ┌────────┐
perm/hooks/sbx/ctx                 │████████│
                                   └────────┘
                                            ┌────────┐
subagent+plugin                             │████████│
                                            └────────┘
                                                     ┌────────┐
core-loop (E2E)                                      │████████│
                                                     └────────┘
                                                              ┌────────┐
biz cutover                                                   │████████│
                                                              └────────┘
                                                                       ┌────────┐
cleanup+split                                                          │████████│
                                                                       └────────┘
```

> 若 W6 E2E 无法通过，允许将 W7 并入 W8（合并 2 周），但必须新开一份 `10b-cutover-slip.md` 说明并重新登记退出条件。

---

## 9. 下一步交付物（需审核通过本文档后产出）

- `01-ai-execution-protocol.md`（已与本文档同批交付；AI 推进规约）
- `02-crate-topology.md`（W1 起草第一版，随周迭代）
- `03-legacy-retirement.md`（W1 起草骨架，W8 前逐步填满）
- `04-week-1-contracts-session.md` 起每周一份（W1 启动前 24h 内出）

## 10. 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-20 | 首稿（含 10 项取舍、8 周路线、checkpoint 机制、退出条件） | Architect |
| 2026-04-20 | P0+P1 修订：§2.3 被删除 crate 扩为 11 项完整清单；§4 W7/W8 硬门禁与扫描命令同步；§5 DoD 取消"删除列表局部枚举"，改为"11 项清单 + `ls crates/` 校验"；#8 fact-fix 锚点改引 `docs/sdk/README.md` "## Fact-Fix 勘误" 小节 | Architect |
| 2026-04-21 | W2 Weekly Gate 收尾：`05-week-2-model.md` 由 `in_progress` 切为 `done`；为解除工作区门禁补最小 unblocker（desktop sidecar 占位、`crates/tools` / `crates/octopus-runtime-adapter` clippy 清零），并以 fresh `cargo test/build/clippy` 验证通过。 | Codex |
| 2026-04-21 | W3 Weekly Gate 收尾：`06-week-3-tools-mcp.md` 由 `in_progress` 切为 `done`；`octopus-sdk-tools` / `octopus-sdk-mcp` 的公共面与契约差异清单完成回填；`capability_runtime/** + capability bridge` 真删后通过 `cargo build --workspace`、`cargo clippy --workspace -- -D warnings` 与双口径守护扫描。 | Codex |
| 2026-04-21 | W4 Plan 起稿：`07-week-4-permissions-hooks-sandbox-context.md` 由 `pending` 切为 `draft`；4 crate（`octopus-sdk-permissions / octopus-sdk-sandbox / octopus-sdk-hooks / octopus-sdk-context`）公共面与 12-Task Ledger 完成；3 项关键决策（`ToolCategory` 反向下沉到 Level 0 / Compactor 首版 `ClearToolResults` + `Summarize` / Sandbox 3 后端 + Windows Noop fallback）在审核中已确认。 | Architect (Claude) |
| 2026-04-21 | W4 Weekly Gate 收尾：`07-week-4-permissions-hooks-sandbox-context.md` 由 `in_progress` 切为 `done`；4 个 Level 2/3 crate 落地；Level 0 contracts W4 补丁完成；凭据零暴露合同测试与 prompt cache fingerprint 守护通过，且 `cargo build --workspace`、`cargo clippy --workspace -- -D warnings`、`cargo test --workspace` 全绿。 | Codex |
| 2026-04-21 | W5 Plan 起稿：`08-week-5-subagent-plugin.md` 由 `pending` 切为 `draft`；2 个 SDK crate（`octopus-sdk-subagent` Level 3 / `octopus-sdk-plugin` Level 2）公共面与 12-Task Ledger 完成；4 项审核决策（D1 仅 `MockEvaluator` / D2 插件仅本地目录 / D3 `worker_boot` 只取 Orchestrator 通用部分 / D4 `.agents/**` 双 root 发现）在审核中已确认。 | Architect (Claude Opus 4.7) |
| 2026-04-21 | W5 审计追补：W5 出口状态第 3 条改成 `ToolRegistry / HookRunner` 的 executable runtime registration，并明确 `SkillDecl / ModelProviderDecl` 在 W5 仍是 metadata + builder slot；`worker_boot` 更正为 W5 non-source、留到 W7 复核。 | Codex |
| 2026-04-21 | W5 三轮审计追补：`plugins_snapshot` 硬门禁改成“首事件优先，必要时退回 `session.plugins_snapshot` 次事件”的显式双分支；W8/DoD 的 800 行守护命令改成真实按行数检查。 | Codex |
