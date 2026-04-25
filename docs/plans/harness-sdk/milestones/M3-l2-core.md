# M3 · L2 Core · 核心闭环（最小可运行 SDK）

> 状态：待启动 · 依赖：M2 完成 · 阻塞：M4 / M5 / M6 / M7
> 关键交付：tool / hook / context / session 四 crate 完整 + 临时 driver 跑通 E2E
> 预计任务卡：20 张 · 累计工时：AI 25 小时（串行）+ 人类评审 10 小时
> 并行度：1（强制串行 4 步：tool → hook → context → session）

---

## 0. 里程碑级注意事项

1. **强制串行**：session 是聚合者，必须在 tool / hook / context 完成后才能动；context 又依赖 tool 的工具结果路径
2. **本里程碑结束 = 最小可运行 SDK**：通过临时 driver 能跑 "create_session → run_turn → ListDir 工具调用 → 输出"
3. **临时 driver**：在 `crates/octopus-harness-session/tests/e2e_minimal.rs` 中实现（非正式 engine，仅供本里程碑闭环）
4. **L2 同层耦合**：仅允许 `tool-search → tool` 一条（M4），M3 内部无同层耦合
5. **Hook FailOpen 是受控例外**：v1.8.1 P0-1 修订，必须在 hook crate 显式标注

---

## 1. 任务卡总览

| Crate | 任务卡 | 内容 |
|---|---|---|
| **tool** | M3-T01 ~ T05 | Tool trait + Registry + Pool + Orchestrator + 内置 8 工具 |
| **hook** | M3-T06 ~ T10 | Hook 三 transport + Dispatcher + 事务语义 + 20 类事件 |
| **context** | M3-T11 ~ T15 | 5 阶段管线 + ContextProvider + budget + microcompact / autocompact |
| **session** | M3-T16 ~ T20 | 生命周期 + Projection + Fork + HotReload + SteeringQueue + E2E driver |
| **chore** | M3-T21 | M4 / M5 依赖预注入到 `[workspace.dependencies]`（避免后续并行 PR 共改冲突）|

---

## 2. 步骤 1 · `octopus-harness-tool`

### M3-T01 · Tool trait + ToolDescriptor + ToolContext

**SPEC 锚点**：
- `harness-tool.md` §2（Tool trait）
- `api-contracts.md` §9
- ADR-002（Tool 不含 UI）/ ADR-0011（Capability Handle）

**预期产物**：
- `src/tool.rs`：`Tool` trait + `ToolDescriptor` + `ToolContext` + `ToolProperties`（含 `defer_policy / search_hint`）+ `ToolResult` + `ToolStream`
- `src/result_budget.rs`：ResultBudget 三档（Truncate / Offload / Reject）

**关键不变量**：
- Tool trait 是 `dyn-safe + Send + Sync`
- `ToolProperties.is_concurrency_safe: bool`（v1.8.1 P2-6 强调 bool 二档非三桶）
- ToolContext 提供 `permission_cap / sandbox_cap / model_cap / memory_cap / capability_registry` 但不提供 UI

**预期 diff**：< 400 行

---

### M3-T02 · ToolRegistry + ToolPool + Snapshot 机制

**SPEC 锚点**：
- `harness-tool.md` §3（Registry）+ §4（Pool）

**预期产物**：
- `src/registry.rs`：ToolRegistry + ToolRegistryBuilder
- `src/pool.rs`：ToolPool（三分区：AlwaysLoad / Deferred / RuntimeAppended）+ snapshot
- `src/builder.rs`：BuiltinToolset 枚举（Default / Empty / Custom）

**关键不变量**：
- snapshot 不可变（`Arc<ToolPoolSnapshot>`）
- 固定集按名字字典序，追加集按加入序

**预期 diff**：< 350 行

---

### M3-T03 · Orchestrator（并发分桶 + 串行执行）

**SPEC 锚点**：
- `harness-tool.md` §5（Orchestrator）+ §2.7（bool 二档分桶）

**预期产物**：
- `src/orchestrator.rs`：bool 分桶 + 并行/串行执行
- `tests/orchestrator.rs`

**关键不变量**：
- `is_concurrency_safe = true` → 并行执行
- `is_concurrency_safe = false` → 串行（FIFO）
- 每个 tool 必经 PermissionBroker 检查（除非已声明 NoApproval）

**预期 diff**：< 350 行

---

### M3-T04 · 内置工具集（8 个）

**SPEC 锚点**：
- `harness-tool.md` §6（内置工具集）：Read / Write / ListDir / Bash / Grep / WebSearch / Clarify / SendMessage / ReadBlob

**预期产物**：
- `src/builtin/read.rs / write.rs / list_dir.rs / bash.rs / grep.rs`
- `src/builtin/web_search.rs / clarify.rs / send_message.rs / read_blob.rs`
- `tests/builtin.rs`

**关键不变量**：
- 全部需要权限审批（默认）
- Bash 必须接 SandboxBackend
- Grep 默认接 ripgrep（`rg`）

**Cargo feature**：`builtin-toolset`

**预期 diff**：< 500 行

---

### M3-T05 · Tool Contract Test + ResultBudget E2E

**预期产物**：
- `tests/contract.rs`
- `tests/result_budget.rs`：超限场景三档动作

**预期 diff**：< 200 行

---

## 3. 步骤 2 · `octopus-harness-hook`

### M3-T06 · HookHandler trait + 20 类事件 + HookContext

**SPEC 锚点**：
- `harness-hook.md` §2（HookHandler / HookEvent / HookContext / HookSessionView）
- `api-contracts.md` §11

**预期产物**：
- `src/handler.rs`：HookHandler trait + interested_events
- `src/event.rs`：HookEventKind 枚举（20 类，含 v1.6 新增 PreToolSearch / PostToolSearchMaterialize）
- `src/context.rs`：HookContext（五元组）+ HookSessionView（只读）
- `src/outcome.rs`：HookOutcome + PreToolUse 三件套（修改 input / 给 permission decision / block）

**关键不变量**：
- HookContext 不暴露可变 Session（只读）
- 20 类事件必须完整列出
- HookFailureMode 默认 FailOpen，admin 可声明 FailClosed（P6 受控例外，v1.8.1 P0-1）

**预期 diff**：< 400 行

---

### M3-T07 · Hook Dispatcher + Registry + 事务语义

**SPEC 锚点**：
- `harness-hook.md` §3（Dispatcher）+ §2.6.2（Hook 失败的事务语义，v1.8.1 P1-4）

**预期产物**：
- `src/dispatcher.rs`：HookDispatcher（按 kind 路由）
- `src/registry.rs`：HookRegistry + HookRegistryBuilder
- `tests/dispatcher.rs / transaction.rs`

**关键不变量**：
- PreToolUse 链是 all-or-nothing（v1.8.1 P1-4 修订）
- failure_mode = FailClosed → 失败拒绝主流程；FailOpen → 失败放过 + 发 HookFailedEvent

**预期 diff**：< 350 行

---

### M3-T08 · Hook Transport（in-process）

**SPEC 锚点**：`harness-hook.md` §4.1（in-process transport）

**预期产物**：
- `src/transport/in_process.rs`
- `tests/in_process.rs`

**Cargo feature**：`in-process`

**预期 diff**：< 250 行

---

### M3-T09 · Hook Transport（Exec + HTTP）

**SPEC 锚点**：`harness-hook.md` §4.2 / §4.3（Exec + HTTP transport，含 SSRF guard / mTLS）

**预期产物**：
- `src/transport/exec.rs`：HookExecSpec + working_dir / resource_limits / signal_policy
- `src/transport/http.rs`：HookHttpSpec + allowlist / ssrf_guard / max_redirects / max_body_bytes / mTLS
- `tests/exec.rs / http.rs`

**Cargo feature**：`exec / http`

**关键不变量**：
- HTTP 默认 SSRF guard（拒绝 private IP）
- replay 幂等：Hook 必须支持 Live / Audit 两种 ReplayMode

**预期 diff**：< 450 行

---

### M3-T10 · Hook Contract Test + Replay 幂等测试

**预期产物**：
- `tests/contract.rs`
- `tests/replay_idempotent.rs`

**预期 diff**：< 200 行

---

## 4. 步骤 3 · `octopus-harness-context`

### M3-T11 · ContextEngine 5 阶段管线骨架

**SPEC 锚点**：
- `context-engineering.md` §3（5 阶段固定顺序：tool-result-budget → snip → microcompact → collapse → autocompact）
- `harness-context.md` §2

**预期产物**：
- `src/engine.rs`：ContextEngine + ContextStageId 枚举
- `src/provider.rs`：ContextProvider trait + ContextOutput

**关键不变量**：
- 5 阶段顺序硬编码不可重排（ADR-003 / Prompt Cache）
- 每阶段可插拔 ContextProvider 但 stage 顺序不变

**预期 diff**：< 350 行

---

### M3-T12 · 5 阶段实现（tool-result-budget / snip / collapse）

**SPEC 锚点**：`context-engineering.md` §4-§6（前 3 阶段）

**预期产物**：
- `src/stages/budget.rs`：ToolResultBudgetStage
- `src/stages/snip.rs`：SnipStage（截断）
- `src/stages/collapse.rs`：CollapseStage（折叠）
- `tests/stages.rs`

**预期 diff**：< 350 行

---

### M3-T13 · Microcompact + Autocompact（aux LLM）

**SPEC 锚点**：
- `context-engineering.md` §7-§8（compact 双档）
- `harness-context.md` §3

**预期产物**：
- `src/stages/microcompact.rs`：基于 AuxModelProvider 的中等压缩
- `src/stages/autocompact.rs`：基于 AuxModelProvider 的全压缩
- `tests/compact.rs`：mock AuxProvider 验证

**Cargo feature**：`compact-aux-llm`

**预期 diff**：< 400 行

---

### M3-T14 · Recall 编排 + Memdir 注入

**SPEC 锚点**：
- `context-engineering.md` §11（Recall 编排）
- `harness-memory.md` §6

**预期产物**：
- `src/stages/recall.rs`：每轮至多 1 次 recall + fail-safe
- `src/memdir_inject.rs`：MEMORY.md / USER.md → user message 注入

**关键不变量**：
- Memdir 注入位置在 user message（不动 system prompt）
- `<memory-context>` 栅栏 + 上一轮栅栏剥离

**预期 diff**：< 300 行

---

### M3-T15 · Context Contract Test + Prompt Cache 稳定性

**预期产物**：
- `tests/contract.rs`
- `tests/cache_stability.rs`：连续 5 轮调用，验证 prompt prefix 不变

**预期 diff**：< 200 行

---

## 5. 步骤 4 · `octopus-harness-session`

### M3-T16 · Session 生命周期 + SessionOptions + SessionBuilder（含 workspace_root 注入）

**SPEC 锚点**：
- `harness-session.md` §2（生命周期）
- `harness-sdk.md` §8.2（Session 不走 type-state，运行时 Result 校验，v1.8.1 P2-4）
- `api-contracts.md` §13
- `AGENTS.md` § Persistence Governance（runtime/events / data/blobs / data/main.db / config/runtime 路径治理）

**预期产物**：
- `src/lib.rs`
- `src/session.rs`：Session struct + **SessionOptions 必含 `workspace_root: PathBuf`** + SessionHandle
- `src/builder.rs`：SessionBuilder（运行时 Result 校验，非 type-state）
- `src/lifecycle.rs`：CreateSession / EndSession 流程
- `src/paths.rs`（**新增**，实施前评估 P2-2）：`SessionPaths` helper struct 派生于 `workspace_root`：
  ```rust
  pub struct SessionPaths {
      pub events: PathBuf,        // <workspace_root>/runtime/events/<tenant>/<session>.jsonl
      pub blobs: PathBuf,         // <workspace_root>/data/blobs
      pub db: PathBuf,            // <workspace_root>/data/main.db
      pub memdir: PathBuf,        // <workspace_root>/data/memdir
      pub runtime_sessions: PathBuf, // <workspace_root>/runtime/sessions
  }
  impl SessionPaths {
      pub fn from_workspace(root: &Path, tenant: &TenantId, session: &SessionId) -> Self;
  }
  ```

**关键不变量**：
- `SessionOptions.workspace_root: PathBuf` **必填**（SessionBuilder 验证），不允许默认 `cwd`
- 所有 store / blob / memdir 路径必须经由 `SessionPaths` 派生，业务层不允许手拼路径
- workspace_root 必须是已存在的目录（builder 期 `try_canonicalize` 校验）
- SessionBuilder 配置不全 → `build()` 返回 `Err(SessionError::Incomplete)`（**不是** type-state 编译期）
- system_prompt / tools / memory 三件套创建期可写、运行期 PromptCacheLocked

**禁止行为**：
- 不允许 `EventStore` / `BlobStore` 实现内部使用 `std::env::current_dir()` 或环境变量取路径
- 不允许 SessionOptions 默认 `workspace_root` = "/"（会与多租户测试夹具冲突）

**SPEC 一致性自检**：

```bash
# SessionPaths helper 必须存在且派生 5 个标准路径
grep -q 'pub struct SessionPaths' crates/octopus-harness-session/src/paths.rs
for p in events blobs db memdir runtime_sessions; do
    grep -q "pub ${p}: PathBuf" crates/octopus-harness-session/src/paths.rs || echo "MISSING path: $p"
done

# workspace_root 必填
grep -E 'pub workspace_root: PathBuf' crates/octopus-harness-session/src/session.rs
```

**预期 diff**：< 450 行（比原 400 多出的部分用于 paths 模块 + 校验逻辑）

---

### M3-T17 · SessionProjection + Fork + Snapshot

**SPEC 锚点**：`harness-session.md` §3 / §4

**预期产物**：
- `src/projection.rs`：SessionProjection + replay
- `src/fork.rs`：Session::fork（生成新 SessionId + 拷贝 history）
- `src/snapshot.rs`：snapshot_id() 接口

**预期 diff**：< 350 行

---

### M3-T18 · Hot Reload（reload_with）三档

**SPEC 锚点**：
- `harness-session.md` §5（Hot Reload）
- ADR-003（CacheImpact）

**预期产物**：
- `src/reload.rs`：reload_with(ConfigDelta) → ReloadOutcome
- 三档：AppliedInPlace { CacheImpact } / ForkedNewSession / Rejected
- `tests/reload.rs`

**关键不变量**：
- AppliedInPlace 中：permission_rule_patch → NoInvalidation；其它 → OneShotInvalidation
- ForkedNewSession → CacheImpact::FullReset
- 删工具 / 改 system prompt / 切 model → 必须 fork

**Cargo feature**：`hot-reload-fork`

**预期 diff**：< 400 行

---

### M3-T19 · SteeringQueue（ADR-0017）

**SPEC 锚点**：
- `harness-session.md` §6
- ADR-0017（Steering Queue）

**预期产物**：
- `src/steering.rs`：SteeringQueue + push_steering / steering_snapshot / drain_and_merge
- `tests/steering.rs`

**关键不变量**：
- 默认 capacity 8 / TTL 60s / DropOldest
- drain_and_merge 仅在主循环安全检查点（不在 model inference 中途）

**Cargo feature**：`steering`

**预期 diff**：< 300 行

---

### M3-T20 · M3 E2E 临时 Driver + Gate 检查

**预期产物**：
- `crates/octopus-harness-session/tests/e2e_minimal.rs`：临时 mini-engine（不是 M5 的真 engine）
  - **文件头必须有警示注释**（实施前评估 P1-2）：
    ```rust
    //! 临时 mini-engine，M3 期专用 E2E 闭环验证脚手架。
    //!
    //! TODO(M5-T15): 完成真 engine 后由 `crates/octopus-harness-engine/tests/e2e_engine.rs` 替代；
    //!               M5-T15 任务卡完成后必须 `git rm` 本文件。
    //!
    //! 治理来源：docs/plans/harness-sdk/milestones/M3-l2-core.md M3-T20
    ```
  - 模拟 LLM 响应（用 MockProvider）
  - create_session → run_turn → ListDir 工具调用 → 输出
  - 流程：UserPromptSubmit hook → context.assemble → mock LLM → tool call → permission allow → tool execute → result → context.after_turn → assistant message → RunEnded
- `docs/plans/harness-sdk/audit/M3-mvp-gate.md`（人类填写）

**Gate 通过判据**：
- ✅ 4 crate 各自 `cargo test --all-features` 全绿
- ✅ E2E 用例跑通：模拟用户提问"list current dir"，session 完成 ListDir 调用并输出文件清单
- ✅ Prompt cache 稳定性测试连续 5 轮 prompt prefix 不变
- ✅ 所有 contract test 接入
- ✅ HookFailureMode 默认 FailOpen 已显式标注 (P0-1 文档落地代码)
- ✅ DirectBroker 签名带 PermissionContext (P0-2 已落地)
- ✅ `tests/e2e_minimal.rs` 文件头含 `TODO(M5-T15)` 警示注释

未全绿 → 不得开始 M4 / M5。

---

### M3-T21 · 依赖预注入 chore（M4 / M5 共用）

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M3-T20 完成 |
| **可并行** | × |
| **预期 diff** | < 200 行 |
| **预期工时** | AI 30 min + 人类评审 15 min |

**背景说明**（实施前评估 P1-3）：

M4 三路（tool-search/skill/mcp）+ M5 三步（observability/plugin/engine）会引入十几个新依赖（`opentelemetry / opentelemetry-otlp / openidconnect / fs2 / blake3 / globset / regex / notify / once_cell / prometheus / prost / ...`）。如果各任务卡各自加根 `Cargo.toml [workspace.dependencies]`，多 PR 共改根文件会引爆冲突。

本卡一次性把 M4/M5 已知会用的所有 dep 预注入根 `Cargo.toml`，后续 M4/M5 任务卡**只能**在 crate 内部 `Cargo.toml` 用 `xxx.workspace = true`，禁止再改根。

**SPEC 锚点**：
- `00-strategy.md` §3.2（并行任务卡协调）
- `M4-l2-extensions.md` §0（M4 / M5 并行约束）
- 实施前评估报告 P1-3

**预期产物**：

- 修改根 `Cargo.toml [workspace.dependencies]`，预注入：
  - **M4 用**：`openidconnect = "3"`（OAuth），`reqwest-eventsource = "0.6"`（SSE transport），`tokio-tungstenite = "0.24"`（websocket），`globset` 已在，`yaml-rust2 = "0.8"`（frontmatter 解析），`notify = "7"`（rule watch）
  - **M5 用**：`opentelemetry = "0.27"`，`opentelemetry-otlp = "0.27"`，`opentelemetry_sdk = "0.27"`，`tracing-opentelemetry = "0.28"`，`prometheus = "0.13"`，`fs2 = "0.4"`（advisory lock），`blake3 = "1"`（哈希），`regex = "1"`（已可能在），`once_cell = "1"`，`libloading = "0.8"`（dynamic-load）
  - **M6 用**（如未在 M5 加）：`tokio = { features += ["broadcast"] }`（broadcast 已在 default features）
- 不预注入 cargo-deny 例外登记表外的依赖
- 同步更新 `deny.toml [licenses] allow` 如新增 license

**关键不变量**：

- 仅修改 `[workspace.dependencies]`，**不**修改各 crate 的 `[dependencies]`
- 每个新增 dep 必须在 PR 描述中标注"M4-Tyy / M5-Tzz 任务卡将使用"
- 任何在本卡未列出的依赖，M4/M5 任务卡**不许使用**（如真有需要必须先开 hot-fix 卡 `M3-Hxx`）

**禁止行为**：

- 不要修改 `[workspace.lints]` / `[workspace.package]`
- 不要在本卡启动 `cargo update`（锁文件刷新归独立 chore PR）

**验收命令**：

```bash
cargo metadata --format-version 1 | jq '.workspace_members | length'
cargo check --workspace
cargo deny check
```

**SPEC 一致性自检**：

```bash
# 新增 dep 全部声明在 [workspace.dependencies]
grep -E '^(openidconnect|opentelemetry|tracing-opentelemetry|prometheus|fs2|blake3|libloading|notify|reqwest-eventsource|tokio-tungstenite|yaml-rust2)' Cargo.toml
```

**PR 描述模板要点**：列出每个新增 dep 对应的 M4/M5 任务卡 ID，作为后续审计追溯依据。

---

## 6. 索引

- **上一里程碑** → [`M2-l1-primitives.md`](./M2-l1-primitives.md)
- **下一里程碑（可并行）** → [`M4-l2-extensions.md`](./M4-l2-extensions.md) / [`M5-l3-engine.md`](./M5-l3-engine.md)
