# W7 · 业务侧切到 SDK + 删除 legacy runtime 路径

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/04-session-brain-hands.md` → `docs/sdk/13-contracts-map.md` → `docs/plans/sdk/02-crate-topology.md §3.2 / §3.3 / §3.4 / §3.5 / §8` → `docs/plans/sdk/03-legacy-retirement.md §6 / §7.1 / §7.4 / §7.5` → `crates/octopus-platform/src/runtime.rs` → `crates/octopus-server/src/workspace_runtime.rs` → `crates/octopus-desktop-backend/src/main.rs` → `apps/desktop/src-tauri/src/backend.rs` → `crates/octopus-cli/src/run_once.rs`。

## Status

状态：`in_progress`

## Active Work

当前 Task：`Task 7 · workspace 收口与 11 个 legacy crate 删除`

当前 Step：`Step 1 已审计：`crates/octopus-infra` 仍直接依赖 legacy `runtime` / `tools`，Task 7 暂时 blocked`

### Pre-Task Checklist（起稿阶段）

- [x] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [x] 已阅读 `00-overview.md §1 10 项取舍`，且当前任务未违反。
- [x] 已阅读 `docs/sdk/04 / 13` 与本 Task 对应的规范章节。
- [x] 已阅读 Task 段落的 `Files` / `Preconditions` / `Step*` 且无歧义。
- [x] 已识别本 Task 涉及的 **SDK 对外公共面** 变更（预期否；W7 默认不新增 SDK 公共符号）。
  - 若转为“是”：必须同批回填 `02-crate-topology.md §2.*`。
- [x] 已识别是否涉及 `contracts/openapi/src/**` 或 `packages/schema/src/**`（可能；只在 `/api/v1/runtime/*` 传输形状无法保持兼容时触发）。
- [x] 已识别是否涉及 `docs/sdk/14` UI Intent IR 变更（预期否；W7 只透传既有 `RenderBlock / AskPrompt / ArtifactRef`）。
- [x] Preconditions 已全部满足；未满足项已在 `Open Questions` 中登记。
- [x] 当前 git 工作树状态已知；本文件只做文档改动。
- [x] 已识别所有 `Stop if:` 条款；遇到任一条件 → 立即停止并汇报。

Open Questions：

- `10-week-7-business-cutover.md` 在本批次前不存在；本文件先完成文档冻结，再进入代码执行。
- `/api/v1/runtime/*` 预期保持 transport-compatible；若首批桥接后做不到，必须先走 OpenAPI 源文件更新，再继续 cutover。
- Task 7 审计新增阻塞项：`crates/octopus-infra/Cargo.toml` 仍声明 `runtime` / `tools` path 依赖，且 `src/resources_skills.rs` 仍使用 legacy runtime config loader、MCP discovery types 与 `tools::mvp_tool_specs()`；在未先迁走这条生产依赖链前，不能删除 `crates/runtime` / `crates/tools`。

### 已确认的审核决策（2026-04-22）

下列 4 项作为 W7 起稿决策先冻结。执行期若需变更，必须在本文件 §变更日志追加专项条目，或回写 `docs/sdk/README.md ## Fact-Fix 勘误`。

| # | 决策点 | 确认结论 | 关联章节 |
|---|---|---|---|
| D1 | 业务接线边界 | **`octopus-platform` 持有 SDK 组装权**。`octopus-server` / `octopus-desktop` 继续只消费 `PlatformServices`，不直接持有 `AgentRuntime`。 | Architecture / Task 1 / Task 2 / Task 3 |
| D2 | `/api/v1/runtime/*` 契约策略 | **默认保持传输兼容**。只有在 SDK 输出无法安全映射回当前 transport DTO 时，才允许改 `contracts/openapi/src/**`，并且必须走 `openapi:bundle → schema:generate`。 | Scope / Task 3 / Task 6 |
| D3 | 桌面宿主形态 | **继续保留 loopback HTTP sidecar 模型**。W7 只把 sidecar 从 `octopus-desktop-backend` 改到 `octopus-desktop`，不把 Tauri 宿主改成直持 `AgentRuntime`。 | Architecture / Task 4 |
| D4 | CLI 范围 | `octopus-cli` 的最小 SDK run path 已在 W6 完成。W7 只补 **删除 `commands` / `rusty-claude-cli` 所需的剩余命令与渲染面**，不再实现第二套 runtime。 | Scope / Task 5 |

## Goal

让 `octopus-server` / `octopus-desktop` / `octopus-cli` 的业务入口全部切到 `octopus-sdk`，删除 `octopus-runtime-adapter` 与其余 10 个 legacy crate，使 `/api/v1/runtime/*` 在新 SDK 主循环下继续保持 W6 已验证的行为与事件语义。

## Architecture

- **平台层持有 SDK 组装，不把 SDK 直接抬进 transport / host**：W6 已经完成 `AgentRuntimeBuilder -> start_session -> submit_turn -> events` 的最小闭环。W7 不再把这套装配散落到 `server`、`desktop`、`cli` 三处，而是在 `octopus-platform` 内建立 SDK-backed bridge：它负责持有 `SessionStore / ModelProvider / SecretVault / ToolRegistry / PermissionGate / AskResolver / SandboxBackend / PluginRegistry / PluginsSnapshot / Tracer / TaskFn` 的实际装配，再把结果包成现有 `RuntimeSessionService / RuntimeExecutionService / RuntimeConfigService / ModelRegistryService`。

- **HTTP transport 继续在 `octopus-server`，宿主一致性继续成立**：`docs/sdk/04-session-brain-hands.md` 已固定 “本地 IPC 到 Rust 后端 / 浏览器 HTTP 到服务端” 的同形接口约束。W7 只切后端实现来源，不改变 `apps/desktop/src-tauri` 的 sidecar + HTTP 模型，也不让前端直接触碰 SDK。这样 `workspace-client.ts` 与 Tauri host 仍通过同一 `/api/v1/runtime/*` 面工作。

- **优先 cutover，再删除 legacy**：`octopus-runtime-adapter` 当前仍同时承担 session、execution、config、registry、secret store、event bus。W7 的顺序必须是：
  1. 在 `octopus-platform` 补齐 SDK-backed bridge；
  2. 切 `octopus-server`、`octopus-desktop`、`octopus-cli`；
  3. 守护扫描确认无生产引用；
  4. 再删 11 个 legacy crate 目录。
  反过来做会让 hidden dependency 无处收口。

- **W7 默认零新增 SDK 公共面**：本周重点是业务接线和删除 legacy，不是继续扩 `octopus-sdk-*`。如果 bridge 过程中发现 SDK 缺口，先停下来写明缺口，再决定是否要回补到 `sdk-core` 或 `sdk-session`，不能在 server/platform 本地打补丁绕过去。

## Scope

- In scope：
  - 在 `octopus-platform` 新建 SDK-backed runtime bridge，替代 `RuntimeAdapter` 的业务装配职责。
  - 切换 `octopus-server` 的 `/api/v1/runtime/*` 实现与测试夹具到 platform bridge。
  - 把 `crates/octopus-desktop-backend` 改成 `crates/octopus-desktop`，并切换到 platform bridge。
  - 更新 Tauri sidecar 构建、启动与测试路径到 `octopus-desktop`。
  - 把 `commands` / `rusty-claude-cli` 中仍未迁移、但删除 legacy 所需的 CLI 入口平移到 `octopus-cli`。
  - 在必要时更新 `contracts/openapi/src/**`、`packages/schema/src/**`、桌面 adapter / store / tests。
  - 删除 11 个 legacy crate 目录：`runtime`、`tools`、`plugins`、`api`、`octopus-runtime-adapter`、`commands`、`compat-harness`、`mock-anthropic-service`、`rusty-claude-cli`、`octopus-desktop-backend`、`octopus-model-policy`。
  - 更新 `Cargo.toml` `default-members` 与 `02-crate-topology.md §8`、`03-legacy-retirement.md`、`README.md` 状态。

- Out of scope：
  - `octopus-persistence` 新 crate 上线。
  - `octopus-core` / `octopus-infra` / `octopus-server` 超长文件拆分到 ≤ 800 行。
  - 新的业务功能或 UI 功能。
  - 改变前端调用模型为“直接 SDK / 直接 Tauri invoke”。
  - 再扩 `octopus-sdk-core` 的 Brain Loop 语义，除非 cutover 过程中发现硬缺口并单独登记。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | `octopus-platform::Runtime*Service` 现在全部基于 `octopus_core` 的 legacy DTO。SDK session/run/event 形状与这些 DTO 不一一对应。 | 先在 platform bridge 内做 DTO 投影，保持 server transport 面不变；不要直接把 SDK 事件泄漏到 server handler。 | #1 / #8 |
| R2 | `runtime_config / registry / secret_store` 仍主要活在 `octopus-runtime-adapter`，而 `03-legacy-retirement.md` 明确这些责任在 W7 进入 `octopus-platform`。 | W7 第一批先把 ownership 提到 platform，再切 server/backend。不要把这些逻辑临时塞回 `octopus-server`。 | #2 |
| R3 | `octopus-desktop-backend` 改名会牵动 Tauri sidecar build、capability、测试和本地 debug binary 路径。 | 维持 sidecar 机制不变，只替换 crate/binary 名和依赖来源；任何 release 打包层面的未知改动都先停。 | #7 |
| R4 | 删除 `runtime / tools / plugins / api` 目录前，可能还存在隐式测试或 build-script 引用。 | 所有物理删除必须放到 cutover 绿后，先跑守护扫描，再删目录。 | #9 / #10 |
| R5 | `commands` / `rusty-claude-cli` 语义面可能大于 W7 删除所需。 | 只迁“删除 legacy 所必需”的命令与渲染路径；不在 W7 追求 100% CLI 体验翻版。 | #6 |
| R6 | `/api/v1/runtime/*` 若因 SDK-backed output 需要新增字段，必须同步 OpenAPI 和 schema。 | 任何 HTTP payload 差异都先改 `contracts/openapi/src/**`，禁止直接改生成物或先改 server 再补文档。 | #3 |
| R7 | `Cargo.toml default-members` 当前仍含 `octopus-runtime-adapter` 与 `octopus-desktop-backend`。 | 只有在 legacy crate 目录真正删除后，才一次性更新到 W7 目标形态；中间状态不要半切。 | #11 |
| R8 | `octopus-cli` 已无 legacy runtime 依赖，但 legacy commands 目录仍在 workspace 中。 | CLI 迁移与 crate 删除要绑定在同一周完成；不要留下“功能在新 crate，源码还在旧 crate”的双写状态。 | #8 |

## 承 W6 / 启 W8 的契约链

- **承 W6**：
  - `octopus-sdk` 已是最小运行入口；W7 不再新增第二套 runtime loop。
  - `SessionStarted + config_snapshot_id + effective_config_hash + plugins_snapshot + render.block` 的事件语义已冻结；W7 只能投影、不能重定义。
  - `octopus-cli` 已确认不依赖 `RuntimeAdapter`；W7 继续沿这个方向完成 CLI 收口。

- **启 W8**：
  - `octopus-persistence` 只在 W8 上线，W7 不抢跑。
  - W7 删除 legacy crate 后，W8 才对 `octopus-core / octopus-infra / octopus-server` 做大文件拆分。
  - 如果 W7 cutover 暴露新的持久化责任边界，先记进 `00-overview.md §6` 或 `02 §5`，不在本周顺手开 `octopus-persistence`。

## 本周 `02 §3.2 / §3.3 / §3.4 / §3.5 / §8` 公共面修订清单（同批次回填）

> W7 默认不新增 SDK 公共面；本周文档回填重点是**业务 crate 责任边界与 workspace 收口**。

### `02 §3.2 octopus-platform`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 1 | `§3.2` | 责任补齐 | 增加 “platform 持有 SDK-backed runtime bridge / secret vault / config bridge / registry bridge 的组装权”，并明确 server / desktop 不直接持有 `AgentRuntime`。 |

### `02 §3.3 octopus-server`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 2 | `§3.3` | 依赖收口 | 明确 `octopus-server` 不再依赖 `octopus-runtime-adapter`；runtime HTTP transport 只消费 `PlatformServices`。 |

### `02 §3.4 octopus-desktop`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 3 | `§3.4` | crate 改名 | 把 `octopus-desktop-backend` 正式替换为 `octopus-desktop`，保留 sidecar + loopback HTTP 形态。 |

### `02 §3.5 octopus-cli`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 4 | `§3.5` | CLI 收口 | 明确 `commands` / `rusty-claude-cli` 的剩余命令面在 W7 并入 `octopus-cli`，删除 legacy CLI crate。 |

### `02 §8 workspace / default-members`

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 5 | `§8` | 成员更新 | 把 `crates/octopus-desktop` 纳入 W7 目标列表；删除 `octopus-runtime-adapter` 与 `octopus-desktop-backend` 的 `default-members` 引用。 |
| 6 | `§8` | 目录删除 | 把 11 个 legacy crate 的目录删除状态从“计划”推进到“执行完成”。 |

## 本周在 `03-legacy-retirement.md` 的状态推进

| 旧位置 | 新位置 | 期望状态推进 |
|---|---|---|
| `crates/octopus-runtime-adapter/src/lib.rs` + `config_service.rs` + `registry*.rs` + `secret_store.rs` | `crates/octopus-platform/src/runtime_sdk/**` | `pending -> done/replaced` |
| `crates/octopus-desktop-backend/**` | `crates/octopus-desktop/**` | `pending -> done/replaced` |
| `crates/commands/src/**` | `crates/octopus-cli/src/**` | `partial -> done/replaced` |
| `crates/rusty-claude-cli/src/**` | `crates/octopus-cli/src/**` | `partial -> done/replaced` |
| `crates/runtime` / `crates/tools` / `crates/plugins` / `crates/api` | SDK / platform / server 已切完后的物理删除 | `pending -> done` |

## Weekly Gate 对齐表（W7）

| `00-overview.md §3` 条目 | 本周落点 | 验证 |
|---|---|---|
| `octopus-server` / `octopus-desktop` / `octopus-cli` 不再依赖任何 legacy crate | Task 3 / Task 4 / Task 5 / Task 7 | `rg "(octopus_runtime_adapter|octopus-runtime-adapter|octopus_model_policy|rusty_claude_cli|octopus_desktop_backend|compat_harness|mock_anthropic_service)" crates/ apps/` |
| 11 个 legacy crate 目录整体删除 | Task 7 | `ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'` |
| `/api/v1/runtime/*` 在新 SDK 下行为与 W6 一致 | Task 3 / Task 6 | `cargo test -p octopus-server` + `pnpm -C apps/desktop exec vitest run test/runtime-store.test.ts` |
| workspace build / clippy / desktop suite 通过 | Task 7 / Task 8 | `cargo build --workspace` / `cargo clippy --workspace -- -D warnings` / `pnpm -C apps/desktop test` |

## Task Ledger

### Task 1: `octopus-platform` SDK bridge 骨架

Status: `done`

Files:
- Modify: `crates/octopus-platform/src/lib.rs`
- Modify: `crates/octopus-platform/src/runtime.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/mod.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/builder.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/session_bridge.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/execution_bridge.rs`
- Create: `crates/octopus-platform/tests/runtime_sdk_bridge.rs`

Preconditions:
- W6 Weekly Gate 已通过。
- 已确认 W7 默认不新增 SDK 公共面。

Step 1:
- Action: 在 `octopus-platform` 冻结 `RuntimeSdkDeps` / `RuntimeSdkFactory` 之类的组装入口，明确 `PlatformServices` 如何拿到 `AgentRuntime` 与配套依赖，而不把 `AgentRuntime` 暴露给 server / desktop。
- Done when: `octopus-platform` 内有单一 SDK 组装入口；`crates/octopus-server` 和 `crates/octopus-desktop*` 后续不需要自己 new `AgentRuntimeBuilder`。
- Verify: `cargo test -p octopus-platform --test runtime_sdk_bridge`
- Stop if: 组装入口需要把业务域类型塞进 `octopus-sdk-*`，或必须让 handler 直接持有 `AgentRuntime`。

Step 2:
- Action: 实现 SDK-backed 的 `RuntimeSessionService` / `RuntimeExecutionService` 最小投影，先覆盖 `create_session / get_session / list_sessions / submit_turn / list_events` 这些 W7 必经路径。
- Done when: `octopus-platform` 可以在不链接 `octopus-runtime-adapter` 的前提下返回与当前 server 兼容的 runtime DTO。
- Verify: `cargo test -p octopus-platform --test runtime_sdk_bridge`
- Stop if: DTO 投影无法保持 transport-compatible，且需要直接改 `/api/v1/runtime/*` 契约。

### Task 2: config / registry / secret ownership 提升到 `octopus-platform`

Status: `done`

Files:
- Modify: `crates/octopus-platform/src/lib.rs`
- Modify: `crates/octopus-platform/src/runtime.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/config_bridge.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs`
- Create: `crates/octopus-platform/src/runtime_sdk/secret_vault.rs`
- Create: `crates/octopus-platform/tests/runtime_config_bridge.rs`

Preconditions:
- Task 1 已冻结 SDK bridge 入口。
- `docs/api-openapi-governance.md` 已重读，确认 transport 不下沉到 platform。

Step 1:
- Action: 把 `runtime_config`、`model registry`、`secret store` 的 ownership 从 `octopus-runtime-adapter` 平移到 platform bridge，对齐根 `AGENTS.md` 的 file-first config / secret redaction 规则。
- Done when: `octopus-platform` 可以提供 `RuntimeConfigService`、`ModelRegistryService` 与 `SecretVault` 实现，而不依赖 `octopus-runtime-adapter`。
- Verify: `cargo test -p octopus-platform --test runtime_config_bridge`
- Stop if: secret 持久化实现需要新建 `octopus-persistence` 才能继续。

Step 2:
- Action: 守护扫描 `octopus-platform`，确认没有继续保留 `RuntimeAdapter` 依赖或 `use octopus_runtime_adapter::*`。
- Done when: `rg "octopus_runtime_adapter|octopus-runtime-adapter|RuntimeAdapter" crates/octopus-platform` 为 0 命中。
- Verify: `rg "octopus_runtime_adapter|octopus-runtime-adapter|RuntimeAdapter" crates/octopus-platform`
- Stop if: 平台层还需要 adapter 暴露的行为，但其真正所有权不清楚。

### Task 3: `octopus-server` runtime transport 切到 platform bridge

Status: `done`

Files:
- Modify: `crates/octopus-server/Cargo.toml`
- Modify: `crates/octopus-server/src/lib.rs`
- Create: `crates/octopus-server/src/test_runtime_sdk.rs`
- Modify: `crates/octopus-server/src/workspace_runtime.rs`
- Modify: `crates/octopus-server/src/handlers.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/mod.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/session_bridge.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/execution_bridge.rs`
- Modify: `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs`

Preconditions:
- Task 1 / Task 2 已通过。
- 已确认 `octopus-server` 继续只消费 `PlatformServices`。

Step 1:
- Action: 把 server 生产装配和测试夹具里的 `RuntimeAdapter` 替换成 `octopus-platform` 的 SDK-backed services。
- Done when: `crates/octopus-server` 不再声明 `octopus-runtime-adapter` 依赖；runtime route 和测试都走 platform bridge。
- Verify: `cargo test -p octopus-server`
- Stop if: handler 为了跑通而直接 new `AgentRuntimeBuilder` 或直接依赖 `octopus-sdk-core` 细节。

Step 2:
- Action: 保持 `/api/v1/runtime/*` 的 transport payload 与现有 contract 一致；若发现字段差异，立即转 Task 6 走 OpenAPI 源链。
- Done when: server 侧 runtime transport 测试通过，且无需手改生成物。
- Verify: `cargo test -p octopus-server`
- Stop if: 返回体变化已超出“兼容映射”范围，但又无法明确应该改哪些 OpenAPI 源文件。

### Task 4: `octopus-desktop-backend` 改为 `octopus-desktop`

Status: `done`

Files:
- Modify: `Cargo.toml`
- Move: `crates/octopus-desktop-backend` -> `crates/octopus-desktop`
- Modify: `crates/octopus-platform/Cargo.toml`
- Modify: `crates/octopus-platform/src/runtime_sdk/builder.rs`
- Modify: `crates/octopus-desktop/Cargo.toml`
- Modify: `crates/octopus-desktop/src/main.rs`
- Modify: `apps/desktop/src-tauri/src/backend.rs`
- Modify: `apps/desktop/src-tauri/build.rs`
- Modify: `apps/desktop/src-tauri/tauri.conf.json`
- Modify: `apps/desktop/src-tauri/capabilities/default.json`
- Modify: `apps/desktop/src-tauri/tests/shell_contract.rs`
- Modify: `scripts/prepare-desktop-sidecar.mjs`
- Modify: `scripts/prepare-desktop-backend.mjs`
- Modify: `scripts/run-desktop-backend.mjs`

Preconditions:
- Task 3 已完成 server cutover。
- 已确认 sidecar + loopback HTTP 模型保持不变。

Step 1:
- Action: 把 desktop sidecar crate/binary 从 `octopus-desktop-backend` 改名到 `octopus-desktop`，并切换 runtime service 装配到 platform bridge。
- Done when: 新 sidecar crate 不再依赖 `octopus-runtime-adapter`，并能正常启动 HTTP backend。
- Verify: `cargo build -p octopus-desktop && cargo test -p octopus-desktop`
- Stop if: 改名导致 sidecar packaging / binary 发现路径无法在本地验证。

Step 2:
- Action: 更新 Tauri build / sidecar / capability / shell contract 路径，保证桌面宿主仍能拉起新 binary。
- Done when: `apps/desktop/src-tauri` 内不再引用 `octopus-desktop-backend` 字样。
- Verify: `cargo test -p octopus-desktop-shell --test shell_contract`
- Stop if: 宿主层需要改成直连 SDK 或直连 Tauri invoke 才能继续。

### Task 5: `octopus-cli` 收口 `commands` / `rusty-claude-cli`

Status: `done`

Files:
- Modify: `crates/octopus-cli/Cargo.toml`
- Modify: `crates/octopus-cli/src/lib.rs`
- Modify: `crates/octopus-cli/src/run_once.rs`
- Create: `crates/octopus-cli/src/automation.rs`
- Create: `crates/octopus-cli/src/workspace.rs`
- Create: `crates/octopus-cli/src/project.rs`
- Create: `crates/octopus-cli/src/config.rs`
- Create: `crates/octopus-cli/src/init.rs`
- Create: `crates/octopus-cli/src/render.rs`
- Create: `crates/octopus-cli/src/input.rs`
- Modify: `crates/octopus-cli/tests/min_cli.rs`

Preconditions:
- W6 最小 CLI run path 已通过。
- 不允许重新引入任何 legacy runtime 依赖。

Step 1:
- Action: 只迁“删除 `commands` / `rusty-claude-cli` 所需”的剩余命令入口、输入解析与渲染逻辑到 `octopus-cli`。
- Done when: `octopus-cli` 能覆盖 legacy CLI 目录删除所需的最小命令面，而不复制第二套 runtime 实现。
- Verify: `cargo test -p octopus-cli`
- Stop if: 某条命令必须复用旧 adapter/runtime 逻辑而不是走 SDK / platform。

Step 2:
- Action: 守护扫描 CLI，确认 `commands` / `rusty-claude-cli` / `octopus-runtime-adapter` 字样为 0 命中。
- Done when: CLI 生产代码彻底摆脱 legacy crate。
- Verify: `rg "commands::|rusty_claude_cli|rusty-claude-cli|octopus_runtime_adapter|RuntimeAdapter" crates/octopus-cli`
- Stop if: 仍有命令面缺口，但其期望行为没有明确来源。

### Task 6: `/api/v1/runtime/*` transport parity 审计与条件式契约更新

Status: `done`

Files:
- Modify: `contracts/openapi/src/**`（如需）
- Modify: `packages/schema/src/**`（如需）
- Modify: `apps/desktop/src/tauri/workspace-client.ts`（如需）
- Modify: `apps/desktop/src/tauri/shared.ts`（如需）
- Modify: `apps/desktop/test/**`（如需）

Preconditions:
- Task 3 的 SDK-backed server 路径已可运行。
- 已确认 OpenAPI-first 规则。

Step 1:
- Action: 比对 SDK-backed `/api/v1/runtime/*` 实际 payload 与现有 OpenAPI/schema；能兼容则不改源，不能兼容则先改源文件。
- Done when: runtime transport 的契约差异被明确归类为 “unchanged” 或 “需要源更新并已落地” 两种之一。
- Verify: `pnpm openapi:bundle && pnpm schema:generate && pnpm schema:check`
- Stop if: 变化面已经超出 runtime path，但无法判断 canonical source 属于哪个路径/组件文件。

Step 2:
- Action: 复核 desktop adapter/store/runtime tests，确认前端 transport 不因后端切源而破。
- Done when: 关键桌面 transport suite 通过。
- Verify: `pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts && pnpm -C apps/desktop exec vitest run test/runtime-store.test.ts && pnpm -C apps/desktop exec vitest run test/tauri-client-runtime.test.ts`
- Stop if: 前端失败暴露的是旧 contract 历史债，而不是本批 cutover 引入的回归。

### Task 7: workspace 收口与 11 个 legacy crate 删除

Status: `blocked`

Files:
- Modify: `Cargo.toml`
- Delete: `crates/runtime`
- Delete: `crates/tools`
- Delete: `crates/plugins`
- Delete: `crates/api`
- Delete: `crates/octopus-runtime-adapter`
- Delete: `crates/commands`
- Delete: `crates/compat-harness`
- Delete: `crates/mock-anthropic-service`
- Delete: `crates/rusty-claude-cli`
- Delete: `crates/octopus-desktop-backend`
- Delete: `crates/octopus-model-policy`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/plans/sdk/README.md`

Preconditions:
- Task 3 / Task 4 / Task 5 / Task 6 已全部通过。
- 守护扫描已证明生产代码不再引用 legacy crate。

Step 1:
- Action: 更新 workspace `default-members` 与业务 crate 名称；然后一次性删除 11 个 legacy crate 目录。
- Done when: `Cargo.toml` 和磁盘目录都达到 `02 §8` 的 W7 目标形态。
- Verify: `cargo build --workspace`
- Stop if: 任一非测试生产路径仍引用待删 crate。

Step 2:
- Action: 跑 W7 守护扫描，确认 legacy 依赖和 legacy 目录都已清零，并同步回填 `02` / `03` / `README`。
- Done when: legacy grep 与 `ls crates/` 守护命令全绿。
- Verify: `rg "(octopus_runtime_adapter|octopus-runtime-adapter|octopus_model_policy|rusty_claude_cli|octopus_desktop_backend|compat_harness|mock_anthropic_service)" crates/ apps/ && ls crates/ | rg '^(runtime|tools|plugins|api|octopus-runtime-adapter|commands|compat-harness|mock-anthropic-service|rusty-claude-cli|octopus-desktop-backend|octopus-model-policy)$'`
- Stop if: legacy crate 删除后出现无法归因到本批次的 workspace 失败。

### Task 8: W7 Weekly Gate 与文档收口

Status: `pending`

Files:
- Modify: `docs/plans/sdk/10-week-7-business-cutover.md`
- Modify: `docs/plans/sdk/README.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`
- Modify: `docs/plans/sdk/03-legacy-retirement.md`
- Modify: `docs/plans/sdk/00-overview.md`

Preconditions:
- Task 1–7 全部完成或明确 blocked。

Step 1:
- Action: 按 `01-ai-execution-protocol.md` 跑 W7 Weekly Gate，全量补 checkpoint、状态、变更日志与总控摘要。
- Done when: W7 出口状态与硬门禁逐条勾选完成，`README` 状态切到 `done`。
- Verify: `cargo build --workspace && cargo clippy --workspace -- -D warnings && pnpm -C apps/desktop test`
- Stop if: 任一 Weekly Gate 条目只能以“人工脑补”为 pass 条件。

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-22 | 首稿：新建 W7 计划，冻结“platform 持有 SDK bridge、server/desktop/cli 逐层 cutover、最后删除 11 个 legacy crate”的执行顺序；补 Task Ledger、`02` / `03` 回填点与 Weekly Gate 对齐表。 | Codex |
| 2026-04-22 | Task 1 完成：在 `octopus-platform` 新增 `runtime_sdk/{builder,session_bridge,execution_bridge}`，冻结 `RuntimeSdkDeps / RuntimeSdkFactory / RuntimeSdkBridge` 入口，并补 `runtime_sdk_bridge` 集成测试。 | Codex |
| 2026-04-22 | Task 2 完成：在 `octopus-platform` 新增 `runtime_sdk/{config_bridge,registry_bridge,secret_vault}`，把 runtime config / model registry / secret ownership 提升到 platform，并补 `runtime_config_bridge` 集成测试。 | Codex |
| 2026-04-22 | Task 3 完成：`octopus-server` 删除 `octopus-runtime-adapter` dev 依赖，测试夹具改走 platform SDK bridge；同时补 platform bridge 的 permission mode、single-shot generation 与最小 approval flow 映射，保持 server runtime transport 测试绿。 | Codex |
| 2026-04-22 | Task 4 完成：`octopus-desktop-backend` 改名为 `octopus-desktop`，desktop sidecar 改为调用 `RuntimeSdkFactory::build_live`，并同步更新 Tauri sidecar 路径、shell contract 与本地 sidecar 准备脚本。 | Codex |
| 2026-04-22 | Task 5 完成：`octopus-cli` 吸收 `commands / rusty-claude-cli` 中剩余的 parser/help、`init / input / render`、agents/skills 发现与安装、最小 direct CLI 分派；同时保持 run path 仍只走 SDK，不重新引入任何 legacy runtime 依赖。 | Codex |
| 2026-04-22 | Task 6 完成：发现 SDK-backed runtime bridge 已发出新的 session/message/render/checkpoint/plugins snapshot eventType；据此更新 OpenAPI runtime event contract、重新生成 `packages/schema/src/generated.ts`，并用 desktop transport/runtime suite 锁住新枚举与 `kind` 的 string 化。 | Codex |

## Checkpoint 2026-04-22 12:37

- Week: W7
- Batch: Task 1 Step 1 → Task 1 Step 2
- Completed:
  - 在 `octopus-platform` 建立单一 SDK 组装入口 `RuntimeSdkFactory`，把 `AgentRuntimeBuilder` 收口到 platform 内部。
  - 新增 `RuntimeSdkBridge`，覆盖 `create_session / get_session / list_sessions / submit_turn / list_events` 的最小 DTO 投影。
  - 补 `runtime_sdk_bridge` 测试，验证 SDK 事件流能投影到现有 runtime DTO。
- Files changed:
  - `crates/octopus-platform/Cargo.toml` (modified)
  - `crates/octopus-platform/src/lib.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (created)
  - `crates/octopus-platform/src/runtime_sdk/builder.rs` (created)
  - `crates/octopus-platform/src/runtime_sdk/session_bridge.rs` (created)
  - `crates/octopus-platform/src/runtime_sdk/execution_bridge.rs` (created)
  - `crates/octopus-platform/tests/runtime_sdk_bridge.rs` (created)
  - `docs/plans/sdk/02-crate-topology.md` (modified)
- Verification:
  - `cargo test -p octopus-platform --test runtime_sdk_bridge` → pass
  - `cargo clippy -p octopus-platform --test runtime_sdk_bridge -- -D warnings` → pass
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 2 Step 1：把 `runtime_config / model registry / secret store` 的 ownership 从 `octopus-runtime-adapter` 提升到 `octopus-platform::runtime_sdk/*`

## Checkpoint 2026-04-22 13:10

- Week: W7
- Batch: Task 2 Step 1 → Task 2 Step 2
- Completed:
  - 在 `octopus-platform` 新增 `runtime_sdk/{config_bridge,registry_bridge,secret_vault}`，把 runtime config / model registry / secret ownership 收口到 platform。
  - `RuntimeSdkFactory` 现在在 platform 内部自持 workspace layout 和 secret vault，不再依赖 `octopus-runtime-adapter`。
  - 补 `runtime_config_bridge` 测试，覆盖 managed credential 持久化、scope merge 和 catalog snapshot token usage。
- Files changed:
  - `crates/octopus-platform/Cargo.toml` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/builder.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/config_bridge.rs` (created)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs` (created)
  - `crates/octopus-platform/src/runtime_sdk/secret_vault.rs` (created)
  - `crates/octopus-platform/tests/runtime_sdk_bridge.rs` (modified)
  - `crates/octopus-platform/tests/runtime_config_bridge.rs` (created)
- Verification:
  - `cargo test -p octopus-platform --test runtime_config_bridge` → pass
  - `cargo test -p octopus-platform --test runtime_sdk_bridge` → pass
  - `cargo clippy -p octopus-platform --test runtime_config_bridge -- -D warnings` → pass
  - `rg "octopus_runtime_adapter|octopus-runtime-adapter|RuntimeAdapter" crates/octopus-platform` → 0 matches
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 3 Step 1：把 `octopus-server` 的 runtime transport 与测试夹具从 `RuntimeAdapter` 切到 platform SDK bridge

## Checkpoint 2026-04-22 14:05

- Week: W7
- Batch: Task 3 Step 1 → Task 3 Step 2
- Completed:
  - `octopus-server` 删除 `octopus-runtime-adapter` dev 依赖，新增测试专用 `test_runtime_sdk` helper，把 `handlers` 和 `workspace_runtime` 的 runtime 夹具统一切到 `RuntimeSdkFactory`。
  - platform bridge 补齐 `read-only / workspace-write / danger-full-access` 到 SDK permission mode 的映射，并把 configured model surface 校验接到 session create / generation run。
  - platform bridge 增加最小 approval flow 投影，覆盖 `waiting_approval -> completed`、`waiting_approval -> waiting_input` 和 chained approval 的第二个 pending approval。
  - `gemini-2.5-flash` 的内建 registry surface 调整为 single-shot generation，保持 server 侧 generation route 现有测试语义。
- Files changed:
  - `crates/octopus-server/Cargo.toml` (modified)
  - `crates/octopus-server/src/lib.rs` (modified)
  - `crates/octopus-server/src/test_runtime_sdk.rs` (created)
  - `crates/octopus-server/src/handlers.rs` (modified)
  - `crates/octopus-server/src/workspace_runtime.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/mod.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/session_bridge.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/execution_bridge.rs` (modified)
  - `crates/octopus-platform/src/runtime_sdk/registry_bridge.rs` (modified)
- Verification:
  - `cargo test -p octopus-server` → pass
  - `rg "octopus_runtime_adapter|octopus-runtime-adapter|RuntimeAdapter" crates/octopus-server` → 0 matches
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 4 Step 1：把 `octopus-desktop-backend` 的 sidecar crate/binary 改名为 `octopus-desktop`，并切到 platform bridge

## Checkpoint 2026-04-22 14:22

- Week: W7
- Batch: Task 4 Step 1 → Task 4 Step 2
- Completed:
  - `crates/octopus-desktop-backend` 已改名为 `crates/octopus-desktop`，sidecar 入口不再依赖 `octopus-runtime-adapter`，改为通过 `RuntimeSdkFactory::build_live` 从 `octopus-platform` 获取 SDK-backed runtime bridge。
  - `octopus-platform` 补了 live 默认装配：共用 `SqliteJsonlSessionStore`、runtime secret vault、内建 tool registry、默认 protocol adapters 和 host sandbox backend，避免 desktop sidecar 自己持有 `AgentRuntimeBuilder` 细节。
  - Tauri sidecar 启动、build placeholder、capability、shell contract 和本地 sidecar 准备脚本全部切到新 binary 名 `octopus-desktop`。
- Files changed:
  - `Cargo.toml` (modified)
  - `crates/octopus-platform/Cargo.toml` (modified)
  - `crates/octopus-platform/src/runtime_sdk/builder.rs` (modified)
  - `crates/octopus-desktop/Cargo.toml` (modified)
  - `crates/octopus-desktop/src/main.rs` (modified)
  - `apps/desktop/src-tauri/src/backend.rs` (modified)
  - `apps/desktop/src-tauri/build.rs` (modified)
  - `apps/desktop/src-tauri/tauri.conf.json` (modified)
  - `apps/desktop/src-tauri/capabilities/default.json` (modified)
  - `apps/desktop/src-tauri/tests/shell_contract.rs` (modified)
  - `scripts/prepare-desktop-sidecar.mjs` (modified)
  - `scripts/prepare-desktop-backend.mjs` (modified)
  - `scripts/run-desktop-backend.mjs` (modified)
  - `crates/octopus-desktop-backend/Cargo.toml` (deleted via move)
  - `crates/octopus-desktop-backend/src/main.rs` (deleted via move)
- Verification:
  - `cargo build -p octopus-desktop` → pass
  - `cargo test -p octopus-desktop` → pass
  - `cargo test -p octopus-desktop-shell --test shell_contract` → pass
  - `cargo fmt --all` → pass
  - `rg -n "octopus-desktop-backend" apps/desktop/src-tauri crates/octopus-desktop scripts` → 0 matches
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 5 Step 1：盘点 `octopus-cli` 仍依赖 `commands` / `rusty-claude-cli` 的命令入口、输入解析与渲染逻辑

## Checkpoint 2026-04-22 12:40

- Week: W7
- Batch: Task 5 Step 1 → Task 5 Step 2
- Completed:
  - `octopus-cli` 新增 `automation / workspace / config / init / input / render / project` 模块，吸收 legacy CLI 的 slash command parser/help、agents/skills 发现与安装、以及终端输入/渲染能力。
  - `run_once` 补了最小 direct CLI 分派，保留既有 SDK single-turn run path，同时新增 `init`、`agents`、`skills` 和 `slash` 入口；未迁完的 legacy slash command 只保留识别和明确提示，不复制第二套 runtime。
  - `octopus-cli` crate 增加对应依赖与测试，CLI 生产代码里对 `commands / rusty-claude-cli / octopus-runtime-adapter` 的文本引用已清零。
- Files changed:
  - `crates/octopus-cli/Cargo.toml` (modified)
  - `crates/octopus-cli/src/lib.rs` (modified)
  - `crates/octopus-cli/src/run_once.rs` (modified)
  - `crates/octopus-cli/src/automation.rs` (created)
  - `crates/octopus-cli/src/workspace.rs` (created)
  - `crates/octopus-cli/src/project.rs` (created)
  - `crates/octopus-cli/src/config.rs` (created)
  - `crates/octopus-cli/src/init.rs` (created)
  - `crates/octopus-cli/src/input.rs` (created)
  - `crates/octopus-cli/src/render.rs` (created)
  - `crates/octopus-cli/tests/min_cli.rs` (modified)
- Verification:
  - `cargo test -p octopus-cli` → pass
  - `cargo fmt --all` → pass
  - `rg "commands::|rusty_claude_cli|rusty-claude-cli|octopus_runtime_adapter|RuntimeAdapter" crates/octopus-cli` → 0 matches
- Exit state vs plan:
  - matches
- Blockers:
  - none
- Next:
  - Task 6 Step 1：比对 SDK-backed `/api/v1/runtime/*` payload 与现有 OpenAPI/schema，确认是否需要源文件更新

## Checkpoint 2026-04-22 12:47

- Week: W7
- Batch: Task 6 Step 1 → Task 6 Step 2
- Completed:
  - 审计确认 SDK-backed runtime bridge 已发出 `runtime.session.started`、`runtime.message.user`、`runtime.message.assistant`、`runtime.tool.executed`、`runtime.render.block`、`runtime.ask`、`runtime.checkpoint.created`、`runtime.session.ended`、`runtime.session.plugins_snapshot` 等新 eventType，现有 OpenAPI `RuntimeEventKind` 枚举已不足以覆盖。
  - `contracts/openapi/src/components/schemas/runtime.yaml` 已补 runtime event 枚举，并把 `RuntimeEventEnvelope.kind` 从错误的同枚举约束放宽为 `string`，对齐实际 transport 里 `session.started`、`message.user` 这类 kind alias。
  - 已重新生成 `contracts/openapi/octopus.openapi.yaml` 与 `packages/schema/src/generated.ts`，并在 desktop contract test 中补断言锁住新增 runtime eventType 和 `kind?: string`。
- Files changed:
  - `contracts/openapi/src/components/schemas/runtime.yaml` (modified)
  - `contracts/openapi/octopus.openapi.yaml` (modified)
  - `packages/schema/src/generated.ts` (modified)
  - `apps/desktop/test/openapi-transport.test.ts` (modified)
- Verification:
  - `pnpm openapi:bundle` → pass
  - `pnpm schema:generate` → pass
  - `pnpm schema:check` → pass
  - `pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts` → pass
  - `pnpm -C apps/desktop exec vitest run test/runtime-store.test.ts` → pass
  - `pnpm -C apps/desktop exec vitest run test/tauri-client-runtime.test.ts` → pass
- Exit state vs plan:
  - needs source update and landed
- Blockers:
  - none
- Next:
  - Task 7 Step 1：守护扫描 legacy crate 剩余引用，确认可以更新 workspace members/default-members 并删除 11 个 legacy crate 目录
