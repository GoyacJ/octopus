# W6 · `octopus-sdk-core`（Brain Loop）+ `octopus-sdk` / `octopus-cli` 最小接线

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/01-core-loop.md` → `docs/sdk/04-session-brain-hands.md` → `docs/sdk/03-tool-system.md` → `docs/sdk/07-hooks-lifecycle.md` → `docs/plans/sdk/02-crate-topology.md §2.13 / §2.14 / §2.15 / §8` → `docs/plans/sdk/03-legacy-retirement.md §2.1 / §6 / §7.1 / §7.4` → `crates/runtime/src/conversation.rs` + `crates/runtime/src/conversation/turn_orchestrator.rs` → `crates/octopus-runtime-adapter/src/{lib.rs,agent_runtime_core.rs,execution_service.rs,event_bus.rs,execution_events.rs}` → `crates/octopus-runtime-adapter/tests/runtime_turn_loop.rs`。

## Status

状态：`done`

## Active Work

当前 Task：`W6 complete`

当前 Step：`Weekly Gate 通过；公共面 / legacy 映射 / CLI 最小入口已与实现对齐`

### Pre-Task Checklist（起稿阶段留空模板，Task 1 启动前逐条勾选）

- [x] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [x] 已阅读 `00-overview.md §1 10 项取舍`，且当前任务未违反。
- [x] 已阅读 `docs/sdk/01 / 04 / 03 / 07` 与本 Task 对应的规范章节。
- [x] 已阅读 Task 段落的 `Files` / `Preconditions` / `Step*` 且无歧义。
- [x] 已识别本 Task 涉及的 **SDK 对外公共面** 变更（是）。
  - 若“是”：已确认变更在 `02-crate-topology.md §2.13 / §2.14 / §2.15 / §8` 有登记项（或计划在本批次内新增登记）。
- [x] 已识别是否涉及 `contracts/openapi/src/**` 或 `packages/schema/src/**`（预期否；若需要调整 `/api/v1/runtime/*` 契约，转是）。
- [x] 已识别是否涉及 `docs/sdk/14` UI Intent IR 变更（预期否；W6 只消费现有 `RenderBlock`）。
- [x] Preconditions 已全部满足；未满足项已在 `Open Questions` 中登记。
- [x] 当前 git 工作树干净或有明确切分；本批次计划 diff ≤ 800 行（不含 generated）。
- [x] 已识别所有 `Stop if:` 条款；遇到任一条件 → 立即停止并汇报。

Open Questions：

- 已收口：`StartSessionInput` 在 W6 冻结为 `session_id / working_dir / permission_mode / model / config_snapshot_id / effective_config_hash / token_budget`，并已回填 `02-crate-topology.md §2.14`。
- 已收口：`octopus-cli` 在 W6 只落单会话单回合最小入口（`main_with_args / main_from_env / run_once`）；`commands` parser 与全量子命令迁移明确留给 W7。

### 已确认的审核决策（2026-04-22）

下列 5 项作为 W6 起稿决策先冻结。执行期若需变更，必须在本文件 §变更日志追加专项条目，或回写 `docs/sdk/README.md ## Fact-Fix 勘误`。

| # | 决策点 | 确认结论 | 关联章节 |
|---|---|---|---|
| D1 | W6 crate 边界 | **W6 最小交付 = `octopus-sdk-core` + `octopus-sdk` + `octopus-sdk-observability` + `octopus-cli` 最小 run path**。`server / desktop / api` 真切换留 W7。 | Goal / Task 1 / Task 7 |
| D2 | CLI 范围 | **只迁最小单会话单回合路径**，用于跑 W6 硬门禁；`automation / workspace / project / config` 子命令不在本周做全量平移。 | Scope / Task 7 |
| D3 | Builder 槽位 | W6 builder 不再停留在 `PermissionPolicy` / `SubagentOrchestrator` 这类不完整抽象；公共面直接冻结到可组装现有执行链的输入：`PermissionGate`、`AskResolver`、`PluginsSnapshot`、`TaskFn`、`Tracer`。`EventSink` 由 runtime 内部围绕 `SessionStore` 物化，不外露。 | Architecture / `02 §2.14` / Task 1 / Task 3 / Task 6 |
| D4 | E2E 形态 | **复用 `runtime_turn_loop` 的 scripted model driver 模式**，但测试主体改为 `octopus-sdk`；工具执行走真实 `BashTool`，工作目录走 temp workspace。 | Task 5 / Task 8 |
| D5 | Plugin / Hook 注册时机 | `PluginLifecycle::run(...)` 与 manifest discover 保持在 builder 外部；W6 `build()` 只消费**已注册完成**的 `PluginRegistry` 与稳定 `PluginsSnapshot`，不在 build 阶段扫磁盘。plugin hook 接线前必须先把 hooks crate 的 source order 校回 `session > project > plugin > workspace > defaults`。 | Architecture / Task 6 |

## Goal

产出 `crates/octopus-sdk-core` 作为 **Level 4 Brain Loop**，并补齐 `crates/octopus-sdk` 门面、`crates/octopus-sdk-observability` 最小 tracing/usage 能力、`crates/octopus-cli` 最小命令入口，使以下链路可运行：

`CLI -> AgentRuntimeBuilder::build() -> start_session() -> submit_turn() -> 一轮模型调用 -> BashTool -> end_turn -> SessionStore / EventStream / RenderBlock`

本周完成后，W5 已落好的 `session / model / tools / permissions / hooks / context / subagent / plugin` 不再只是分散 crate，而是能被单个 runtime 真实装配并跑通最小端到端主路径。

## Architecture

- **Level 4 Brain Loop 收口**：`octopus-sdk-core` 是唯一的会话主循环实现。它整合 Level 0–3 crate，但不接入业务域类型，不读业务 config 文件，不持有 `Workspace / Project / Task / Deliverable / User / Team` 概念。业务侧只通过 `AgentRuntimeBuilder` 注入可直接组装当前执行链的输入：`SessionStore / ModelProvider / SecretVault / ToolRegistry / PermissionGate / AskResolver / SandboxBackend / PluginRegistry / PluginsSnapshot / Tracer / TaskFn`。`EventSink` 由 runtime 内部围绕 `SessionStore` 物化，不作为外部注入口。

- **Session-first，不回退到业务 loader**：`start_session()` 的第一件事是写 `session.started`，并把 `config_snapshot_id`、`effective_config_hash` 与 `PluginsSnapshot` 落进 `SessionStore`；`resume()` / `events()` 只从 `SessionStore` 与 runtime 内部状态恢复，不引入 `ConfigLoader`、`RuntimeAdapter`、`runtime/sessions/*.json` 之类 legacy 真相源。`cancel()` 在 W6 明确收窄为**仅取消当前进程内的活动 run**；跨重启恢复后的运行态控制不在本周承诺范围。

- **循环形状严格按 `docs/sdk/01`**：W6 不再围绕旧 `RuntimeAdapter` 的 capability planning 外壳扩散，而是按 stop-reason 驱动的 8 段链路收口：`append user -> build prompt -> model sample -> assistant event projection -> partition_tool_calls -> pre/post hook + permission + sandbox + tool dispatch -> stop hook / inject message -> maybe_compact -> emit events / render blocks`。工具并发继续复用 W3 `partition_tool_calls()`，hook 超时与 stop hook 继续机会在 core 层而不是 hooks crate。

- **插件与子代理由 Builder 组装，不跨层偷依赖**：W5 的 `PluginRegistry / PluginLifecycle / OrchestratorWorkers / AgentRegistry` 不应继续停在 crate 内测试。W6 不引入新的 `SubagentOrchestrator` 空抽象，而是直接沿用现有 `TaskFn` + `OrchestratorWorkers::into_task_fn()` 接口。plugin discover 与 `PluginLifecycle::run(...)` 保持在 builder 外部；builder 只消费已注册完成的 runtime-owned registry/snapshot。`AgentRegistry` 仍只消费业务层传入的 roots，不在 SDK 内硬编码 `.agents/**` 路径。

- **CLI 只是薄壳，不是新 RuntimeAdapter**：`octopus-cli` 只负责构造 builder、启动 session、提交一轮输入、流式打印 `SessionEvent` / `RenderBlock`。它不应重新实现 model loop、tool dispatch、permission mediation，也不允许直连 `runtime / tools / plugins / octopus-runtime-adapter` legacy crate。

## Scope

- In scope：
  - 新建 `crates/octopus-sdk-core`
  - 新建 `crates/octopus-sdk`
  - 新建 `crates/octopus-sdk-observability`
  - 新建 `crates/octopus-cli` 的最小入口
  - 把 `docs/sdk/01` 的主循环真正落成 `AgentRuntime`
  - 把 `PluginRegistry / PluginsSnapshot`、`OrchestratorWorkers::into_task_fn()`、`SessionStore`、`ModelProvider`、`ToolRegistry`、`PermissionGate`、`AskResolver`、`HookRunner`、`Compactor` 真正组装进 runtime
  - 端到端测试 `cargo test -p octopus-sdk --test e2e_min_loop`
  - 回填 `02-crate-topology.md` / `03-legacy-retirement.md` / `README.md`

- Out of scope：
  - `octopus-server` / `octopus-desktop-backend` / `/api/v1/runtime/*` 正式切到 SDK
  - 删除 `crates/runtime`、`crates/tools`、`crates/plugins`、`crates/octopus-runtime-adapter`、`crates/commands`、`crates/rusty-claude-cli`
  - `team / workflow / mailbox / background / handoff` 四套 legacy runtime 的业务态回放完全迁移
  - `commands` 全量子命令迁移
  - 动态 plugin load、marketplace、slot arbitration、真实 observability 面板
  - `octopus-persistence`、大文件拆分、legacy 目录物理退场（归 W7/W8）

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | `StartSessionInput` 的字段边界还没冻结；若把业务 ID 带入 core，会破坏 SDK 窄接口。 | 只允许携带 runtime 自己必须知道的字段：`config_snapshot_id`、`effective_config_hash`、会话级预算/权限/初始消息等；禁止直接塞业务 DTO。 | #2 |
| R2 | W6 容易继续沿用 `with_subagent_orchestrator(...)` 这种计划层空名称，导致 builder 公共面和现有 `TaskFn` 真接口继续分叉。 | Task 1 先把 `§2.14` 冻结到 `with_task_fn(...)`，不再引入新抽象名；不允许“实现先于公共面”。 | #1 |
| R3 | `AgentTool` 当前默认还是 `TaskFn not injected` stub；若 builder 阶段无法安全替换，W6 的 subagent path 会断。 | Builder 必须拥有 runtime 自己的 `ToolRegistry`，并在 build 阶段一次性完成 `task_fn` 注入；不要在执行期动态篡改 registry。 | #8 |
| R4 | 当前 hooks crate 的 `source_rank` 还是 `plugin -> workspace -> defaults -> project -> session`，与 `02` / `docs/sdk/07` 的口径不一致；若直接接 plugin hooks，会把错误优先级带进 W6。 | 继续保持排序实现在 hooks crate，但 **W6 必须先修 hooks source order，再允许 plugin hook 接线**；若本周不修，实现侧必须把 plugin hook 注册留空并登记阻塞。 | #8 |
| R5 | `render.block` 是 W6 硬门禁的一部分，但 Bash path 不天然产生 render。 | W6 统一把 assistant text / markdown 投影成 `RenderBlock::Text` 或 `RenderBlock::Markdown`，不要把 render 只绑定到工具结果。 | #7 |
| R6 | `octopus-sdk-observability` 还不存在；若 scope 失控，容易把 W6 拉成 tracing 平台周。 | 本周只落 `Tracer`、`UsageLedger`、`ReplayTracer` 最小面；业务分析能力不做。 | #6 |
| R7 | 旧 `RuntimeAdapter` 同时承担 config、secret、execution、event bus；W6 只迁 core loop，容易被“顺手带业务”拖偏。 | 只迁 `conversation.rs` / `turn_orchestrator.rs` / `agent_runtime_core.rs` 的 Brain Loop 责任；config/secret 业务服务继续留给 platform/W7。 | #2 / #8 |
| R8 | `octopus-cli` 若追求与 `rusty-claude-cli` 全量行为一致，会把 W6 直接拖进 W7 cutover。 | 先做最小命令路径；`commands` 全量平移只在 W6 gate 通过后再判断是否需要扩。 | #6 |
| R9 | W6 需要真实 `cargo test -p octopus-sdk --test e2e_min_loop`，但 facade crate目前不存在。 | Task 1 同批创建 facade crate；Task 8 的 E2E 只依赖 facade 对外面，不允许测试直接穿透到 `sdk-core` 私有模块。 | #7 |
| R10 | `default-members` 阶段性偏离要和 `02 §8` 保持一致；新增 crate 后容易漏改。 | 本周只把新的 SDK crate 加进 `default-members`；`octopus-cli` 作为业务 crate 不进 `default-members`。 | #11 |

## 承 W5 / 启 W7 的契约链

- **承 W5**：
  - `PluginRegistry` 在 W6 进入 runtime build 路径，不再只停在 crate 内测试；`PluginLifecycle::run(...)` 保持在 build 外部执行，builder 只消费结果。
  - `OrchestratorWorkers` 通过 `AgentTool::with_task_fn(...)` 进入真实 tool dispatch。
  - `AgentRegistry` 继续保持“root 由业务层提供”，W6 不硬编码 discovery path。
  - `PluginsSnapshot` 必须在 `start_session()` 首事件或紧随其后的次事件中稳定回放。

- **启 W7**：
  - `octopus-sdk` 成为 `octopus-server` / `octopus-desktop` / `octopus-cli` 后续接线的唯一入口。
  - `RuntimeAdapter` 在 W6 之后进入“仍是生产路径，但已有 SDK 对等主循环”的状态；W7 再切业务入口。
  - `commands` / `rusty-claude-cli` 在 W6 只做最小入口桥接，不在本周删目录。

## 本周 `02 §2.13 / §2.14 / §2.15 / §8` 公共面修订清单（同批次回填）

> 以下修订必须在 Task 1 / Task 2 / Task 3 / Task 7 的同批 PR 内回填到 `02-crate-topology.md`，否则视为 `Stop Condition #1`。

### `02 §2.13 octopus-sdk-observability`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 1 | `§2.13` | 落实 crate | 新建 `crates/octopus-sdk-observability`，至少落 `TraceSpan / Tracer / UsageLedger / ReplayTracer` 最小公共面。 |
| 2 | `§2.13` | 结构补充 | 若本周需要默认实现，补 `NoopTracer` 或等价空 tracer；不引入业务侧 sink。 |

### `02 §2.14 octopus-sdk-core`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 3 | `§2.14 AgentRuntimeBuilder` | 方法补齐 | `with_tracer(...)`、`with_permission_gate(...)`、`with_ask_resolver(...)`、`with_plugins_snapshot(...)`、`with_task_fn(...)`；删除 `with_permission_policy(...)` 与 `with_subagent_orchestrator(...)` 的 W6 预期。 |
| 4 | `§2.14` | 类型落地 | `StartSessionInput / SessionHandle / RunHandle / RuntimeError` 至少冻结最小字段集与错误分类。 |
| 5 | `§2.14 AgentRuntime` | 语义落实 | `start_session / submit_turn / resume / cancel / events` 对应到真实 `SessionStore` / `EventStream`；其中 `cancel()` 明确为“仅当前进程内 active run best-effort”。 |

### `02 §2.15 octopus-sdk`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 6 | `§2.15` | 落实 crate | 新建 `crates/octopus-sdk`，只做 `pub use` facade；禁止自定义新 trait / struct / fn。 |
| 7 | `§2.15` | re-export 对齐 | facade 对外只暴露 `AgentRuntime`、`AgentRuntimeBuilder`、`StartSessionInput`、`RuntimeError`、`SessionStore`、`SqliteJsonlSessionStore`、`ModelProvider` 等既定窄接口。 |

### `02 §8 workspace / default-members`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 8 | `§8` | 成员更新 | 把 `crates/octopus-sdk-observability`、`crates/octopus-sdk-core`、`crates/octopus-sdk` 加入阶段性 SDK default-members；`octopus-cli` 只进 `members`。 |

## 本周在 `03-legacy-retirement.md` 的状态推进

| 旧位置 | 新位置 | 期望状态推进 |
|---|---|---|
| `crates/runtime/src/conversation.rs` + `conversation/*` | `crates/octopus-sdk-core/**` | `pending -> partial/replaced` |
| `crates/octopus-runtime-adapter/src/{agent_runtime_core.rs,execution_service.rs,event_bus.rs,execution_events.rs}` | `crates/octopus-sdk-core/**` + `crates/octopus-sdk-observability/**` | `pending -> partial` |
| `crates/commands/src/{command_parser.rs,runtime_commands.rs}` | `crates/octopus-cli/**` | `pending -> partial` |
| `crates/rusty-claude-cli/src/{main.rs,init.rs,input.rs,render.rs}` | `crates/octopus-cli/**` | `pending -> partial` |

## Weekly Gate 对齐表（W6）

| `00-overview.md §3` 条目 | 本周落点 | 验证 |
|---|---|---|
| `AgentRuntimeBuilder::new()...build()` 链路可用 | Task 3 | `cargo test -p octopus-sdk-core --test builder_contract` |
| 最小 E2E：CLI -> start_session -> model -> Bash -> end_turn | Task 5 + Task 7 + Task 8 | `cargo test -p octopus-sdk --test e2e_min_loop` |
| 事件流含 `session.started` / `tool.executed` / `assistant.message` / `render.block` | Task 4 + Task 5 + Task 8 | `cargo test -p octopus-sdk --test e2e_min_loop -- --exact test_min_loop_events` |
| W6 SDK crate 进入可编译闭包 | Task 1 + Task 8 | `cargo build --workspace` / `cargo clippy --workspace -- -D warnings` |

## Task Ledger

### Task 1: W6 crate 脚手架与公共面回填

Status: `done`

Files:
- Modify: `Cargo.toml`
- Create: `crates/octopus-sdk-observability/Cargo.toml`
- Create: `crates/octopus-sdk-observability/src/lib.rs`
- Create: `crates/octopus-sdk-core/Cargo.toml`
- Create: `crates/octopus-sdk-core/src/lib.rs`
- Create: `crates/octopus-sdk/Cargo.toml`
- Create: `crates/octopus-sdk/src/lib.rs`
- Create: `crates/octopus-cli/Cargo.toml`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/plans/sdk/README.md`

Preconditions:
- W5 Weekly Gate 已通过，`octopus-sdk-subagent` / `octopus-sdk-plugin` 对外公共面稳定。
- 已确认 W6 不触碰 `contracts/openapi/**` 与 `packages/schema/src/**`。

Step 1:
- Action: 新建 `octopus-sdk-observability` / `octopus-sdk-core` / `octopus-sdk` / `octopus-cli` 四个 crate 骨架；更新 workspace `members` 与 SDK 阶段性 `default-members`。
- Done when: `cargo metadata --no-deps` 成功，且新 crate 名全部可被 workspace 识别。
- Verify: `cargo metadata --no-deps > /tmp/octopus-sdk-w6-metadata.json`
- Stop if: 新 crate 依赖方向会造成 `sdk-core -> business crate` 或 `sdk -> define symbols`。

Step 2:
- Action: 回填 `02-crate-topology.md §2.13 / §2.14 / §2.15 / §8` 与必要的 `docs/sdk/*` 勘误入口，确保 builder 槽位、plugin discover 边界、subagent 注入口与 hook/source 顺序口径一致。
- Done when: `rg -n "octopus-sdk-core|octopus-sdk-observability|octopus-sdk|octopus-cli" docs/plans/sdk/02-crate-topology.md docs/plans/sdk/03-legacy-retirement.md` 命中期望条目。
- Verify: `rg -n "with_task_fn|with_permission_gate|with_ask_resolver|with_plugins_snapshot|with_tracer|octopus-cli|octopus-sdk-core" docs/plans/sdk/02-crate-topology.md docs/sdk/07-hooks-lifecycle.md docs/sdk/README.md`
- Stop if: 公共面需要新增业务字段或 `02` / `03` 之间出现冲突。

### Task 2: `octopus-sdk-observability` 最小 tracing / usage 能力

Status: `done`

Files:
- Modify: `crates/octopus-sdk-observability/src/lib.rs`
- Create: `crates/octopus-sdk-observability/src/tracer.rs`
- Create: `crates/octopus-sdk-observability/src/usage.rs`
- Create: `crates/octopus-sdk-observability/src/replay.rs`
- Create: `crates/octopus-sdk-observability/tests/usage_replay.rs`

Preconditions:
- Task 1 完成；`§2.13` 公共面已登记。
- `SessionStore::stream()` / `EventStream` 形状无需新增 trait。

Step 1:
- Action: 落 `TraceSpan`、`Tracer`、`UsageLedger` 最小实现；提供空 tracer 或等价默认实现，保证 core 可注入而不依赖业务 sink。
- Done when: `octopus-sdk-core` 后续可以只依赖 `dyn Tracer` 与 `UsageLedger`，不回连 `telemetry` 或 `event_bus` legacy 代码。
- Verify: `cargo test -p octopus-sdk-observability --test usage_replay`
- Stop if: 需要把业务 trace envelope 或数据库写入逻辑放进 observability crate。

Step 2:
- Action: 实现 `ReplayTracer`，从 `SessionStore::stream()` 回放 `SessionEvent` 到 tracer / usage ledger，作为 W6 runtime 恢复与验证的最小工具。
- Done when: 单测可从已写入的 session 事件恢复 usage 累计，不依赖 `RuntimeAdapter` 的 event bus。
- Verify: `cargo test -p octopus-sdk-observability --test usage_replay -- --exact test_replay_tracer_usage`
- Stop if: `SessionEvent` 缺字段导致必须先改 contracts。

### Task 3: `AgentRuntimeBuilder` 与 runtime 公共面

Status: `done`

Files:
- Modify: `crates/octopus-sdk-core/src/lib.rs`
- Create: `crates/octopus-sdk-core/src/builder.rs`
- Create: `crates/octopus-sdk-core/src/runtime.rs`
- Create: `crates/octopus-sdk-core/src/types.rs`
- Create: `crates/octopus-sdk-core/tests/builder_contract.rs`

Preconditions:
- Task 1、Task 2 完成。
- `02 §2.14` 已冻结 builder 方法集合与对外类型名。

Step 1:
- Action: 定义 `AgentRuntime`、`AgentRuntimeBuilder`、`StartSessionInput`、`SessionHandle`、`RunHandle`、`RuntimeError` 的最小公共面；builder 接受 `SessionStore / ModelProvider / SecretVault / ToolRegistry / PermissionGate / AskResolver / SandboxBackend / PluginRegistry / PluginsSnapshot / Tracer / TaskFn`，并由 runtime 自己围绕 `SessionStore` 物化 `EventSink`。
- Done when: `octopus-sdk-core` 外部可以只通过 builder 构造 runtime，且没有 `RuntimeAdapter`、`ConfigLoader`、`WorkspacePaths` 等业务依赖。
- Verify: `cargo test -p octopus-sdk-core --test builder_contract`
- Stop if: `StartSessionInput` 需要业务域 DTO 才能成立。

Step 2:
- Action: `build()` 组装 runtime 自有状态，包括 tool registry、hook runner、usage ledger、plugin snapshot、runtime-owned event sink 与 subagent task fn 注入槽位，但不做 plugin discover、manifest load 或其他磁盘扫描 IO。
- Done when: `AgentRuntimeBuilder::build()` 返回的 runtime 已能服务 `start_session()` / `submit_turn()`，且 builder 一次构建后不再修改外部注入对象。
- Verify: `cargo clippy -p octopus-sdk-core -- -D warnings`
- Stop if: 需要在 build 之后依赖外部可变全局状态。

### Task 4: session 启动、恢复、事件流

Status: `done`

Files:
- Modify: `crates/octopus-sdk-core/src/runtime.rs`
- Create: `crates/octopus-sdk-core/src/session_boot.rs`
- Create: `crates/octopus-sdk-core/tests/session_boot.rs`

Preconditions:
- Task 3 完成。
- `SessionStore::append_session_started`、`stream`、`wake`、`fork` 现有能力可直接复用。

Step 1:
- Action: 实现 `start_session()`，写入首条 `session.started`，带 `config_snapshot_id`、`effective_config_hash` 与 builder 物化出的 `PluginsSnapshot`。
- Done when: 启动 session 不再依赖 `RuntimeAdapter::load_persisted_*` 路径，`SessionSnapshot` 可直接反映 W6 runtime 所需元信息。
- Verify: `cargo test -p octopus-sdk-core --test session_boot -- --exact test_start_session_writes_snapshot`
- Stop if: 需要 SDK 自己打开 runtime config 文件。

Step 2:
- Action: 实现 `resume()`、`events()`、`cancel()` 的最小语义，确保 `AgentRuntime` 能从 `SessionStore` 读事件流，并只对**当前进程内 active run**做最小控制。
- Done when: `resume()` 基于 `SessionStore::wake()` 工作；`events()` 直接转发 `EventStream`；`cancel()` 不依赖 legacy event bus，且不会宣称跨重启可恢复。
- Verify: `cargo test -p octopus-sdk-core --test session_boot -- --exact test_resume_reads_session_store`
- Stop if: `SessionStore` trait 缺关键操作且必须跨 crate 补洞。

### Task 5: Brain Loop 主路径

Status: `done`

Files:
- Modify: `crates/octopus-sdk-core/src/runtime.rs`
- Create: `crates/octopus-sdk-core/src/brain_loop.rs`
- Create: `crates/octopus-sdk-core/src/tool_dispatch.rs`
- Create: `crates/octopus-sdk-core/src/assistant_projection.rs`
- Create: `crates/octopus-sdk-core/tests/min_loop.rs`

Preconditions:
- Task 3、Task 4 完成。
- W3 `partition_tool_calls()`、W4 `HookRunner` / `PermissionGate` / `Compactor`、W3 builtin `BashTool` 均可直接使用。

Step 1:
- Action: 实现 stop-reason 驱动的 `submit_turn()` 主路径：构造 model request、消费 `ModelProvider` 响应、把 assistant 内容与 usage 投影回 session / usage ledger。
- Done when: 无 tool_use 的路径可以完整走到 `assistant.message` + `render.block` + `end_turn`。
- Verify: `cargo test -p octopus-sdk-core --test min_loop -- --exact test_end_turn_without_tools`
- Stop if: W2 `ModelProvider` 输出形状无法无损映射到 core loop。

Step 2:
- Action: 落 tool path：`partition_tool_calls -> pre hook -> permission -> sandbox/tool execute -> post hook -> tool_result -> stop hook -> compactor`；并把 `tool.executed` / `assistant.message` / `render.block` 写进 session。
- Done when: 一轮 bash tool use 能跑通，且 compaction 触发点在 core 层，不回退到 legacy `maybe_auto_compact()`。
- Verify: `cargo test -p octopus-sdk-core --test min_loop -- --exact test_bash_tool_round_trip`
- Stop if: tool dispatch 需要重新引入 `runtime::conversation` 或 `RuntimeAdapter` 的执行器。

### Task 6: plugin / subagent 接线

Status: `done`

Files:
- Modify: `crates/octopus-sdk-core/src/builder.rs`
- Create: `crates/octopus-sdk-core/src/plugin_boot.rs`
- Create: `crates/octopus-sdk-core/src/subagent_boot.rs`
- Modify: `crates/octopus-sdk-tools/src/builtin/w5_stubs.rs`（仅当 builder 无法完成 `task_fn` 注入时）
- Create: `crates/octopus-sdk-core/tests/plugin_subagent_integration.rs`

Preconditions:
- Task 3 完成；W5 tests 全绿。
- D5 已确认 plugin discover 保持在 build 外部，builder 只消费预注册完成的 registry/snapshot。

Step 1:
- Action: 在 builder 外部执行 `PluginLifecycle::run(...)` 或等价预注册流程；builder 只接收已注册完成的 `PluginRegistry` 与稳定 `PluginsSnapshot`，并在 build 时校验两者可一致用于 runtime-owned registry/hook runner。
- Done when: runtime 启动后 `SessionStarted` 使用的 snapshot 来自**外部预注册完成**的 plugin registry，且 `build()` 本身不再 discover manifests。
- Verify: `cargo test -p octopus-sdk-core --test plugin_subagent_integration -- --exact test_builder_uses_supplied_plugin_registry`
- Stop if: 想让 `build()` 直接持有业务路径、discover config 或动态 loader。

Step 2:
- Action: 把 `OrchestratorWorkers` 接进 `AgentTool::with_task_fn(...)`；若 runtime 需要消费 `AgentRegistry`，只接受外部传入实例或 roots，不在 SDK 内 discover 路径。
- Done when: `task` tool 不再返回 `TaskFn not injected`，子代理最小路径能在 core 层被调用。
- Verify: `cargo test -p octopus-sdk-core --test plugin_subagent_integration -- --exact test_agent_tool_uses_orchestrator`
- Stop if: 为了注入 subagent 必须让 `plugin` 直接依赖 `subagent`。

### Task 7: `octopus-sdk` facade 与 `octopus-cli` 最小入口

Status: `done`

Files:
- Modify: `crates/octopus-sdk/src/lib.rs`
- Create: `crates/octopus-cli/src/lib.rs`
- Create: `crates/octopus-cli/src/main.rs`
- Create: `crates/octopus-cli/src/run_once.rs`
- Create: `crates/octopus-cli/tests/min_cli.rs`

Preconditions:
- Task 3、Task 4、Task 5 完成。
- `octopus-sdk` facade 的 re-export 面在 `02 §2.15` 已冻结。

Step 1:
- Action: 完成 `octopus-sdk` facade，只做 re-export，不引入新公共符号。
- Done when: E2E 测试与 CLI 都只通过 `octopus-sdk` 使用 runtime，不直接 `use octopus_sdk_core::*`。
- Verify: `cargo test -p octopus-sdk`
- Stop if: facade 需要定义自有 trait / struct 才能用。

Step 2:
- Action: 新建 `octopus-cli` 最小 run path，完成 `start_session -> submit_turn -> print events/render`；只迁最小参数解析，不碰全量 commands 语义。
- Done when: `cargo test -p octopus-cli --test min_cli` 通过，且 `rg "octopus_runtime_adapter|RuntimeAdapter" crates/octopus-cli` 为 0 命中。
- Verify: `cargo test -p octopus-cli --test min_cli`
- Stop if: CLI 要求接入 `commands` 全量业务逻辑才能运行。

### Task 8: W6 E2E、门禁与文档收口

Status: `done`

Files:
- Create: `crates/octopus-sdk/tests/e2e_min_loop.rs`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/00-overview.md`（仅当需要追加 W6 Checkpoint / 变更日志时）
- Modify: `docs/sdk/README.md`（仅当发现新的 Fact-Fix）

Preconditions:
- Task 1–7 完成。
- facade / cli / core 三层均已存在。

Step 1:
- Action: 用 scripted model driver + real `BashTool` 写 `octopus-sdk` E2E，断言 `session.started`、`tool.executed`、`assistant.message`、`render.block` 全链路出现。
- Done when: `cargo test -p octopus-sdk --test e2e_min_loop` 成为 W6 硬门禁，并且测试不依赖 `RuntimeAdapter`。
- Verify: `cargo test -p octopus-sdk --test e2e_min_loop`
- Stop if: E2E 需要 server route / Tauri host 才能成立。

Step 2:
- Action: 执行 W6 Weekly Gate 命令，回填 `03-legacy-retirement.md` 状态推进与本文件 checkpoint；如有规范冲突，同批写 `docs/sdk/README.md ## Fact-Fix 勘误`。
- Done when: `README.md` 中 W6 状态可切到 `in_progress` 或 `done`，并且守护扫描不再命中新建 crate 对 legacy runtime 的直接依赖。
- Verify:
  - `cargo build --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `rg "octopus_runtime_adapter|octopus-runtime-adapter|RuntimeAdapter" crates/octopus-sdk* crates/octopus-cli`
- Stop if: Weekly Gate 任一硬门禁无法归因或无法通过。

### Task 9: `resume()` 真实恢复运行态

Status: `done`

Files:
- Modify: `crates/octopus-sdk-contracts/src/event.rs`
- Modify: `crates/octopus-sdk-session/src/{store.rs,snapshot.rs,sqlite/*}`
- Modify: `crates/octopus-sdk-core/src/runtime.rs`
- Modify: `crates/octopus-sdk-core/tests/session_boot.rs`
- Modify: `crates/octopus-sdk-contracts/tests/{serialization_golden.rs,fixtures/session_event/*}`

Preconditions:
- Task 8 完成；当前 W6 门禁全绿。
- 本批不触碰 `contracts/openapi/**` 与 `packages/schema/**`。

Step 1:
- Action: 扩 `SessionEvent::SessionStarted`、`SessionSnapshot`、`SessionStore::append_session_started(...)`，把 `working_dir / permission_mode / model / token_budget` 持久化到首事件与 SQLite projection。
- Done when: fresh runtime 只靠 `SessionStore::wake()` 就能拿到恢复 `submit_turn()` 所需的全部运行态，不再依赖默认占位值。
- Verify: `cargo test -p octopus-sdk-session && cargo test -p octopus-sdk-contracts --test serialization_golden`
- Stop if: 扩首事件字段后出现无法在同批 golden fixture 中收口的序列化冲突。

Step 2:
- Action: 让 `resume()` 基于持久化 snapshot 重建 `SessionRuntimeState` 并回填 `inner.sessions`，新增跨 runtime 的恢复回归测试。
- Done when: 新 runtime 执行 `resume()` 后可继续 `submit_turn()`，不再报 `SessionStateMissing`。
- Verify: `cargo test -p octopus-sdk-core --test session_boot`
- Stop if: `resume()` 仍需要读业务侧 config/runtime loader 才能工作。

### Task 10: compaction 持久化与 replay

Status: `done`

Files:
- Modify: `crates/octopus-sdk-session/src/{store.rs,sqlite/mod.rs,sqlite/stream.rs}`
- Modify: `crates/octopus-sdk-core/src/{session_boot.rs,brain_loop.rs,tool_dispatch.rs}`
- Modify: `crates/octopus-sdk-core/tests/min_loop.rs`

Preconditions:
- Task 9 完成，fresh runtime 已能仅靠 `SessionStore::wake()` 恢复运行态。
- `SessionEvent::Checkpoint` 已包含 `compaction` 字段，允许把折叠结果写回事件流。

Step 1:
- Action: 在 `SessionStore` 增加 `stream_records()` 最小内部面，SQLite 实现返回真实 `event_id + payload`，保持原 `stream()` 契约不变。
- Done when: core replay 侧能拿到历史消息对应的真实 `event_id`，不再为 compaction anchor 生成临时占位值。
- Verify: `cargo test -p octopus-sdk-session`
- Stop if: 需要打破现有 facade/trait 公共面，导致外部 crate 跟着改接口。

Step 2:
- Action: core transcript 改为维护 `messages + event_ids`，在 `maybe_compact_transcript()` 里把 summarize 结果写成 `Checkpoint { compaction: Some(...) }`，并在 `resume()` replay 时用最新 summarize checkpoint 合成 summary system message、跳过被折叠前缀。
- Done when: fresh runtime `resume()` 后继续 `submit_turn()`，模型请求包含 summary，而不再回放被 fold 的原始前缀消息。
- Verify: `cargo test -p octopus-sdk-core --test min_loop -- --exact test_resume_replays_compaction_summary_instead_of_folded_prefix`
- Stop if: replay 语义需要把 `ClearToolResults` 与 summarize 一并设计，超出 W6 收口范围。

### Task 11: `AskResolver` 接入权限审批链

Status: `done`

Files:
- Modify: `crates/octopus-sdk-core/src/tool_dispatch.rs`
- Modify: `crates/octopus-sdk-core/tests/{min_loop.rs,support/mod.rs}`

Preconditions:
- Task 5 的 tool dispatch 主路径已稳定。
- `octopus-sdk-permissions::ApprovalBroker` 已可独立 emit `SessionEvent::Ask` 并把 resolver 结果映射回 `PermissionOutcome`。

Step 1:
- Action: 在 core tool dispatch 中接住 `PermissionOutcome::{AskApproval,RequireAuth}`，通过 `ApprovalBroker + AskResolver` 完成询问与落库，不再直接报 `RuntimeError::UnresolvedPrompt`。
- Done when: approval/auth prompt 会落成 `SessionEvent::Ask`，resolver 返回 `approve` 时继续执行工具，返回拒绝时转成 denial tool result 并继续 loop。
- Verify: `cargo test -p octopus-sdk-core --test min_loop -- --exact test_permission_approval_executes_tool_after_approve`
- Stop if: 审批链必须引入新的业务态 prompt store，无法在 SDK 内闭合。

Step 2:
- Action: 增加拒绝 / auth 路径回归测试，确认 denied tool 不执行但事件流仍完整。
- Done when: `AskApproval` 与 `RequireAuth` 两条路径都有回归测试，且 denial 结果不会静默丢失。
- Verify: `cargo test -p octopus-sdk-core --test min_loop -- --exact test_permission_denial_returns_tool_error_without_execution`
- Stop if: 测试只能通过修改 builtin tool 权限策略本身成立，而不是 core 审批接线问题。

### Task 12: W6 文档与门禁最终收口

Status: `done`

Files:
- Modify: `docs/plans/sdk/{09-week-6-core-loop.md,03-legacy-retirement.md,README.md}`
- Modify: `crates/octopus-sdk-subagent/src/orchestrator.rs`

Preconditions:
- Task 9–11 完成。
- `cargo test -p octopus-sdk-core --test {session_boot,min_loop}` 已全绿。

Step 1:
- Action: 回填 W6 子 Plan、legacy retirement map、SDK 计划索引，把 `resume / compaction replay / approval chain` 的收口状态写回控制面。
- Done when: `09-week-6-core-loop.md` 不再保留 `pending` 的执行任务，`03-legacy-retirement.md` 与 `README.md` 状态和实际代码一致。
- Verify: `rg -n "Status: \`pending\`|W6" docs/plans/sdk/{09-week-6-core-loop.md,03-legacy-retirement.md,README.md}`
- Stop if: 文档状态与门禁结果冲突，无法给出单一结论。

Step 2:
- Action: 执行 W6 最终门禁，补齐 `Checkpoint.compaction` 相关的 build/clippy 兼容修复，确认 workspace 层面无残留红灯。
- Done when: `cargo build --workspace`、`cargo clippy --workspace -- -D warnings`、`cargo test -p octopus-sdk-core`、`cargo test -p octopus-sdk --test e2e_min_loop`、`cargo test -p octopus-cli --test min_cli` 全绿。
- Verify:
  - `cargo build --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo test -p octopus-sdk-core`
  - `cargo test -p octopus-sdk --test e2e_min_loop`
  - `cargo test -p octopus-cli --test min_cli`
- Stop if: 需要为通过门禁引入 W7 cutover 级别的业务接线。

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-22 | 首稿：新建 W6 计划，冻结 `sdk-core / sdk / observability / octopus-cli` 的最小范围，补 Task Ledger、W6 Gate 对齐表、legacy 映射与公共面修订清单。 | Codex |
| 2026-04-22 | 审计收口：builder 改为直接接 `PermissionGate / AskResolver / PluginsSnapshot / TaskFn`；`PluginLifecycle::run(...)` 从 `build()` 外移；`cancel()` 语义收窄到当前进程内 active run；plugin hook 接线前必须先修正 hooks source order。 | Codex |
| 2026-04-22 | 执行启动：W6 状态切到 `in_progress`，Task 1 进入执行态；首批先修 `HookRunner` source 顺序，作为 plugin hook 接线前置条件。 | Codex |
| 2026-04-22 | 实施完成：落地 `octopus-sdk-observability / octopus-sdk-core / octopus-sdk / octopus-cli`，补齐 builder/runtime/brain loop/plugin-subagent/CLI 最小入口与对应测试；同步回填 `02-crate-topology.md`、`03-legacy-retirement.md`、`README.md`。 | Codex |
| 2026-04-22 | Weekly Gate 收尾：`cargo build --workspace`、`cargo clippy --workspace -- -D warnings`、`cargo test -p octopus-sdk-observability --test usage_replay`、`cargo test -p octopus-sdk-core --test {builder_contract,session_boot,min_loop,plugin_subagent_integration}`、`cargo test -p octopus-sdk --test e2e_min_loop`、`cargo test -p octopus-cli --lib --test min_cli` 全绿；legacy `RuntimeAdapter` 守护扫描对新 SDK crate 为 0 命中。 | Codex |
| 2026-04-22 | 收口补齐：`SessionStarted` / `SessionSnapshot` 持久化补上 `working_dir / permission_mode / model / token_budget`；`resume()` 改为真实恢复运行态；新增 `stream_records()` 与 summarize checkpoint replay；tool dispatch 接入 `ApprovalBroker + AskResolver`；最终通过 `cargo test -p octopus-sdk-contracts --test serialization_golden`、`cargo test -p octopus-sdk-session`、`cargo test -p octopus-sdk-core`、`cargo test -p octopus-sdk --test e2e_min_loop`、`cargo test -p octopus-cli --test min_cli`、`cargo build --workspace`、`cargo clippy --workspace -- -D warnings`。 | Codex |
