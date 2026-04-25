# M1 · L0 Contracts · 契约层全量类型

> 状态：待启动 · 依赖：M0 完成 · 阻塞：M2~M9
> 关键交付：`octopus-harness-contracts` 全量类型 + Schema 派生 + 文档
> 预计任务卡：12 张（T04/T05 各拆 2 子卡）· 累计工时：AI 8 小时 + 人类评审 4 小时
> 并行度：1（串行，因 contracts 是单 crate）

---

## 0. 里程碑级注意事项

1. **本里程碑是项目最关键的稳定基线**：所有 L1-L4 crate 的接口最终都引用本 crate 的类型；任何后续返工都会扩散
2. **逐字搬运 SPEC**：`harness-contracts.md` 已经把所有类型 / 字段 / 枚举写成了 Rust 代码块，AI 任务**逐字 copy** 即可，不允许"美化"
3. **schemars 派生不可跳**：所有公开类型必须 `#[derive(JsonSchema)]`（feature `contracts/openapi/schemars` 治理用）
4. **审计修订项必须落地**：v1.8.1 修订（Cancelled / GraceCallTriggered / TenantId::SHARED / CancelInitiator / PluginId newtype 等）必须**全部体现**
5. **Schema 输出**：M1 完成后，`schemas/harness-contracts/*.json` 至少 60 个 schema 文件
6. **Event 卡 diff 上限**：考虑到事件结构密度（每个事件 ~20 行），T04/T05 已拆为 4 张子卡（T04a/T04b/T05a/T05b），每卡 ≤ 450 行
7. **Redactor trait 必须在 contracts 层定义**（实施前评估 P0-D）：T07 同卡产出 `Redactor` trait + `NoopRedactor` 默认实现，供 M2 EventStore 装配槽预留

---

## 1. 任务卡清单

| ID | 任务 | 依赖 | diff 上限 |
|---|---|---|---|
| M1-T01 | `Cargo.toml` 完整化 + `[dependencies]` 落地 | M0 | < 100 |
| M1-T02 | TypedUlid + 21 类 ID + TenantId 哨兵 | T01 | < 250 |
| M1-T03 | 共享 enum（Decision/PermissionMode/Severity/EndReason/CancelInitiator/...）| T02 | < 350 |
| M1-T04a | Event 枚举 · Session(5) + Run(8) | T03 | < 450 |
| M1-T04b | Event 枚举 · Tool(10) + Permission(5) | T04a | < 450 |
| M1-T05a | Event 枚举 · Steering(3) + ExecuteCode(2) + Memory(6) | T04b | < 450 |
| M1-T05b | Event 枚举 · Hook(5) + MCP(8) + Plugin(7) + Team(3) + 顶层 Event 汇总 | T05a | < 450 |
| M1-T06 | TurnInput / Message / Part 系列类型 | T03 | < 350 |
| M1-T07 | BlobStore + ToolCapability + DecisionScope + **Redactor trait** 抽象 | T03 | < 350 |
| M1-T08 | Error 根类型（HarnessError）+ 各 crate 错误类型族 | T03 | < 300 |
| M1-T09 | schemars 派生 + Schema 导出脚本 | T02-T08 | < 200 |
| M1-T10 | M1 Gate 检查 | T01-T09 | 0 |

---

## 2. 任务卡详情

### M1-T01 · `Cargo.toml` + 依赖

| 字段 | 值 |
|---|---|
| **依赖** | M0 完成（M0-T02 已生成空骨架） |
| **预期 diff** | < 100 行 |

**SPEC 锚点**：
- `docs/architecture/harness/crates/harness-contracts.md` §2（依赖列表）

**预期产物**：

- `crates/octopus-harness-contracts/Cargo.toml` 完整化：

```toml
[package]
name = "octopus-harness-contracts"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false
rust-version.workspace = true

[lints]
workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
ulid = { workspace = true }
schemars = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }
bytes = { workspace = true }
secrecy = { workspace = true }

[dev-dependencies]
proptest = "1"
serde_yaml = "0.9"
```

**禁止行为**：

- 不要引入任何其他 crate（contracts 是 L0，零依赖原则）
- 不要在 contracts 引入 `tokio / async-trait`（contracts 不含 IO）

**验收命令**：

```bash
cargo check -p octopus-harness-contracts
cargo doc --no-deps -p octopus-harness-contracts
```

---

### M1-T02 · TypedUlid + 12 类 ID

| 字段 | 值 |
|---|---|
| **依赖** | M1-T01 |
| **预期 diff** | < 250 行 |

**SPEC 锚点**：
- `docs/architecture/harness/crates/harness-contracts.md` §3.1（L29-L100，TypedUlid + ID 全表）
- `docs/architecture/harness/crates/harness-contracts.md` §3.2（L86-L106，Hash ID + 其他作用域）

**ADR 锚点**：
- ADR-0014（TenantId::SHARED 安全契约）
- 评审报告 P1-5 修订（TenantId::SINGLE / SHARED 双哨兵）

**预期产物**：

- `crates/octopus-harness-contracts/src/ids.rs`（新文件）：
  - `pub struct TypedUlid<Scope>`
  - 18 个 `<Scope>` ZST：SessionScope / RunScope / MessageScope / ToolUseScope / SubagentScope / TeamScope / AgentScope / TenantScope / RequestScope / DecisionScope / WorkspaceScope / MemoryScope / CorrelationScope / CausationScope / SnapshotScope / BlobScope / TransactionScope / EventScope / DeltaScope / BreakpointScope / SteeringScope（共 21 个）
  - 21 个 type alias（`SessionId / RunId / ...`）
  - `impl<S> TypedUlid<S>`：`new() / parse() / as_bytes() / timestamp_ms()`
  - `impl TenantId`：`SINGLE` / `SHARED` 常量（按 SPEC §3.1 L70-L80）
- `crates/octopus-harness-contracts/src/lib.rs`：`pub mod ids; pub use ids::*;`
- `tests/ids.rs`：
  - 正向：构造 + parse + roundtrip serde
  - 反向：跨类型 ID 不可互换（编译期）→ `compile_fail` doctest
  - SHARED ≠ SINGLE 保证

**关键不变量**：

- TenantId::SINGLE / SHARED 是常量 ULID，序列化为固定字符串
- TypedUlid 必须 `#[derive(JsonSchema)]`
- TypedUlid 必须 `Hash + Eq + PartialEq + Clone + Copy + Debug + Serialize + Deserialize`
- 不允许从 String 隐式转 TypedUlid（必须 `parse()`）

**禁止行为**：

- 不要使用 `String` 作 ID（违反 D2 §4.2 软禁止 Newtype 模式）
- 不要为 TypedUlid 实现 `Default`（容易误用）
- 不要保留旧 `octopus-sdk-contracts` 的 ID 类型

**验收命令**：

```bash
cargo test -p octopus-harness-contracts ids
cargo test --doc -p octopus-harness-contracts ids
```

**SPEC 一致性自检**：

```bash
# 21 个 type alias 必须存在
for id in SessionId RunId MessageId ToolUseId SubagentId TeamId AgentId TenantId RequestId DecisionId WorkspaceId MemoryId CorrelationId CausationId SnapshotId BlobId TransactionId EventId DeltaId BreakpointId SteeringId; do
    grep -q "pub type $id" crates/octopus-harness-contracts/src/ids.rs || echo "MISSING: $id"
done

# TenantId 双哨兵
grep -q 'TenantId::SINGLE' crates/octopus-harness-contracts/src/ids.rs
grep -q 'TenantId::SHARED' crates/octopus-harness-contracts/src/ids.rs
```

---

### M1-T03 · 共享 enum

| 字段 | 值 |
|---|---|
| **依赖** | M1-T02 |
| **预期 diff** | < 350 行 |

**SPEC 锚点**：
- `docs/architecture/harness/crates/harness-contracts.md` §3.3（L107-L300，共享枚举全表）
- `docs/architecture/harness/crates/harness-contracts.md` §3 EndReason（v1.8.1 修订包含 Cancelled）

**ADR 锚点**：
- ADR-007（permission events）
- ADR-0011（tool capability handle）
- 评审报告 P1-3（EndReason::Cancelled）

**预期产物**：

- `crates/octopus-harness-contracts/src/enums.rs`：
  - `Decision`（Allow / Deny / AskUser / Defer 等）
  - `PermissionMode`（6 mode）
  - `Severity`
  - `EndReason`（含 `Cancelled { initiator: CancelInitiator }`）
  - `CancelInitiator`（User / Parent / System）
  - `DecidedBy`
  - `DecisionScope`（含 v1.8 新增 `ExecuteCodeScript`）
  - `ToolCapability`（v1.8 新增 `CodeRuntime` 等）
  - `MemoryKind / MemoryVisibility`（D 文档 1.4 二维拆分）
  - `TrustLevel`（PluginManifest）
  - `PromptCacheStyle`
  - `CacheImpact`（reload_with 三档）
  - `ToolPoolChangeSource`
  - `HookFailureMode`（FailOpen / FailClosed）
  - `SteeringMergeBehavior`（DropOldest / Block / Coalesce）
  - 其他 SPEC §3.3 列出的所有 enum

**关键不变量**：

- 全部 `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]`（按需）
- 大型 enum（如 `Decision / DecidedBy / PermissionSubject` 等带判别量需求的）派生 `strum::EnumDiscriminants` + `#[strum_discriminants(name(...), derive(...))]`，**对齐 `harness-contracts.md §3.3 / §3.4` 的派生模板**
- 序列化默认 `#[serde(rename_all = "snake_case")]`（与 SPEC 一致）；如有 tagged enum，`tag` 字段名以 SPEC 为准（事件层固定 `tag = "type"`）
- `EndReason` 必须含 `Cancelled` 变体（v1.8.1 P1-3）

**验收命令**：

```bash
cargo test -p octopus-harness-contracts enums
```

---

> **重要原则（事件名权威源）**：
>
> M1-T04a/b 与 T05a/b 的所有事件名、字段、`#[serde(...)]` 派生属性，**必须逐字 copy 自** `harness-contracts.md §3.3`（顶层 `Event` 枚举）+ `event-schema.md §3.x`（每个事件结构）。
> 任务卡**不得**自行列举或重命名事件（铁律 1）。AI 必须先用 `Read` 工具读 SPEC 行号片段，再生成代码。
> 对齐项：
> - 顶层 `Event` 派生：`#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, strum::EnumDiscriminants)]` + `#[strum_discriminants(name(EventKind), derive(Hash, Eq, PartialEq, Serialize, Deserialize))]` + `#[serde(tag = "type", rename_all = "snake_case")]`（**不是** `tag = "kind", content = "data"`）
> - 事件名清单以 `harness-contracts.md` §3.3 L132-L279 为准（约 80 个变体）

### M1-T04a · Event 枚举 · Session + Run + 消息流

| 字段 | 值 |
|---|---|
| **依赖** | M1-T03 |
| **预期 diff** | < 450 行 |

**SPEC 锚点**（必读，不要省略）：
- `crates/harness-contracts.md` §3.3（L120-L150 的 Session/Run/消息流变体清单）
- `event-schema.md` §3.0 - §3.1（每个事件 struct 字段定义；含 §3.1.1 `GraceCallTriggered`）

**ADR 锚点**：
- ADR-001（event-sourcing）
- 评审报告 P1-3（`EndReason::Cancelled`）/ P2-5（`GraceCallTriggered`）

**预期产物**：

- `events/mod.rs`：声明 `pub mod session; pub mod run; pub mod messages;`（顶层 `Event` 留待 T05b）+ `EventEnvelope` trait 首发定义
- `events/session.rs`：按 SPEC 列出的 Session 生命周期变体（含 `SessionCreated / SessionForked / SessionEnded / SessionReloadRequested / SessionReloadApplied`）
- `events/run.rs`：按 SPEC 列出的 Run 变体（含 `RunStarted / RunEnded / GraceCallTriggered`）
- `events/messages.rs`：按 SPEC 列出的消息流变体（含 `UserMessageAppended / AssistantDeltaProduced / AssistantMessageCompleted`）

**关键不变量**：

- 每个事件 struct 字段、字段顺序、字段类型 **逐字** 对齐 `event-schema.md §3.x`
- 全部 `#[non_exhaustive]`（与顶层 `Event` 一致）
- 全部事件 struct 派生 `Debug + Clone + Serialize + Deserialize + JsonSchema`
- `RunEnded.reason: EndReason` 必须能承载 `Cancelled { initiator: CancelInitiator }`（对齐 v1.8.1 P1-3，已在 M1-T03 落地）
- `GraceCallTriggered` 必须含 `current_iteration / max_iterations / usage_snapshot` 三字段

**禁止行为**：

- 禁止用计划文档列举的别名（如 `IterationStarted / IterationEnded / SessionReloaded / SessionSnapshotCreated` —— 这些**不是** SPEC 名，纯属上一轮 plan bug）；以 SPEC 为准
- 禁止省略事件中的"看似冗余"字段
- 禁止把"派生数据"塞进事件
- 本卡不汇总顶层 `Event` 枚举

**SPEC 一致性自检**：

```bash
# Session/Run/Message 关键事件名以 SPEC 为准（grep 命中 == 通过）
for ev in SessionCreatedEvent SessionForkedEvent SessionEndedEvent SessionReloadRequestedEvent SessionReloadAppliedEvent RunStartedEvent RunEndedEvent GraceCallTriggeredEvent UserMessageAppendedEvent AssistantDeltaProducedEvent AssistantMessageCompletedEvent; do
    grep -q "pub struct ${ev}" crates/octopus-harness-contracts/src/events/{session,run,messages}.rs || echo "MISSING: $ev"
done

# 反向验证：禁止出现 plan v1 误命名
! grep -rE 'pub struct (IterationStartedEvent|IterationEndedEvent|SessionReloadedEvent|SessionSnapshotCreatedEvent)' crates/octopus-harness-contracts/src/events/
```

---

### M1-T04b · Event 枚举 · Tool 执行 + Permission 审批

| 字段 | 值 |
|---|---|
| **依赖** | M1-T04a |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `crates/harness-contracts.md` §3.3（L146-L165 Tool / L158-L165 Permission 变体清单）
- `event-schema.md` §3.4（Tool 执行 - ToolUse* 系列）
- `event-schema.md` §3.2（Permission 审批 - 含 v1.8.1 P1-5 新增 `CredentialPoolSharedAcrossTenants`）

**ADR 锚点**：
- ADR-007（permission-events）
- ADR-009（ToolDeferredPoolChanged / ToolSearchQueried / ToolSchemaMaterialized）
- ADR-0010（ToolResultOffloaded）

**预期产物**：

- `events/tool.rs`：按 SPEC 列出（包含但不限于 `ToolUseRequested / ToolUseApproved / ToolUseDenied / ToolUseCompleted / ToolUseFailed / ToolUseHeartbeat / ToolResultOffloaded / ToolRegistrationShadowed / ToolDeferredPoolChanged / ToolSearchQueried / ToolSchemaMaterialized`）
- `events/permission.rs`：按 SPEC 列出（含 `PermissionRequested / PermissionResolved / PermissionPersistenceTampered / PermissionRequestSuppressed / CredentialPoolSharedAcrossTenants`）

**关键不变量**：

- 事件名采用 SPEC 的 `ToolUse*` 形态（不是 `ToolExecution*` —— 那是 plan v1 误命名）
- `PermissionResolved.decided_by: DecidedBy` 区分多 4 类（按 SPEC §3.4）
- `events/mod.rs` 声明 `pub mod tool; pub mod permission;`

**SPEC 一致性自检**：

```bash
# Tool / Permission 关键事件名以 SPEC 为准
for ev in ToolUseRequestedEvent ToolUseApprovedEvent ToolUseDeniedEvent ToolUseCompletedEvent ToolUseFailedEvent ToolUseHeartbeatEvent ToolResultOffloadedEvent PermissionRequestedEvent PermissionResolvedEvent CredentialPoolSharedAcrossTenantsEvent; do
    grep -q "pub struct ${ev}" crates/octopus-harness-contracts/src/events/{tool,permission}.rs || echo "MISSING: $ev"
done

# 反向：plan v1 误命名禁出现
! grep -rE 'pub struct (ToolExecutionStartedEvent|ToolExecutionCompletedEvent|ToolExecutionFailedEvent|ToolPoolChangedEvent)' crates/octopus-harness-contracts/src/events/
```

---

### M1-T05a · Event 枚举 · Steering + ExecuteCode + Memory + Hook

| 字段 | 值 |
|---|---|
| **依赖** | M1-T04b |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `crates/harness-contracts.md` §3.3（L259-L275 Steering/ExecuteCode；L229-L236 Memory；L167-L182 Hook）
- `event-schema.md` §3.5.1（Steering）/ §3.5.2（ExecuteCode）/ §3.6（Memory）/ §3.7（Hook）

**ADR 锚点**：
- ADR-0016 / ADR-0017（execute_code / steering-queue）

**预期产物**：

- `events/steering.rs`：以 SPEC 为准（**应为** `SteeringMessageQueued / SteeringMessageApplied / SteeringMessageDropped`，**不是** plan v1 误命名 `SteeringPushed/Merged/Dropped`）
- `events/execute_code.rs`：以 SPEC 为准（应为 `ExecuteCodeStepInvoked / ExecuteCodeWhitelistExtended`）
- `events/memory.rs`：以 SPEC 为准（含 `MemoryUpserted / MemoryRecalled / MemoryRecallDegraded / MemoryRecallSkipped / MemoryThreatDetected / MemdirOverflow / MemoryConsolidationRan`）
- `events/hook.rs`：以 SPEC 为准（含 `HookTriggered / HookRewroteInput / HookContextPatchEvent / HookFailed / HookReturnedUnsupported / HookOutcomeInconsistent / HookPanicked / HookPermissionConflict`）

**关键不变量**：

- Steering 名采用 SPEC `SteeringMessage*`（plan v1 写过的 `SteeringPushed/Merged/Dropped` 是误命名，已废弃）
- ExecuteCode 必须连接 `parent_tool_use_id`（嵌入式工具调用追溯链）
- Hook 事件 `HookFailedEvent` 字段必须含 `failure_mode / cause_kind`（对齐 v1.8.1 P0-1 受控例外审计）

**SPEC 一致性自检**：

```bash
for ev in SteeringMessageQueuedEvent SteeringMessageAppliedEvent SteeringMessageDroppedEvent ExecuteCodeStepInvokedEvent ExecuteCodeWhitelistExtendedEvent MemoryUpsertedEvent MemoryRecalledEvent MemoryThreatDetectedEvent HookTriggeredEvent HookFailedEvent HookPanickedEvent; do
    grep -q "pub struct ${ev}" crates/octopus-harness-contracts/src/events/{steering,execute_code,memory,hook}.rs || echo "MISSING: $ev"
done

# 反向：plan v1 误命名禁出现
! grep -rE 'pub struct (SteeringPushedEvent|SteeringMergedEvent|SteeringDroppedEvent|ExecuteCodeStartedEvent|ExecuteCodeCompletedEvent)' crates/octopus-harness-contracts/src/events/
```

---

### M1-T05b · Event 枚举 · MCP + Plugin + Team + Sandbox + Subagent + Context + 其他 + 顶层 Event 汇总

| 字段 | 值 |
|---|---|
| **依赖** | M1-T05a |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `crates/harness-contracts.md` §3.3（L186-L279 全部剩余变体清单）
- `event-schema.md` §3.8 - §3.21（每个事件 struct 字段定义）

**预期产物**：

- `events/mcp.rs`：以 SPEC 为准（`McpToolInjected / McpConnectionLost / McpConnectionRecovered / McpElicitationRequested / McpElicitationResolved / McpToolsListChanged / McpResourceUpdated / McpSamplingRequested`）
- `events/plugin.rs`：以 SPEC 为准（`PluginLoaded / PluginRejected / ManifestValidationFailed`）
- `events/sandbox.rs`：以 SPEC 为准（`SandboxExecutionStarted / SandboxExecutionCompleted / SandboxActivityHeartbeat / SandboxActivityTimeoutFired / SandboxOutputSpilled / SandboxBackpressureApplied / SandboxSnapshotCreated / SandboxContainerLifecycleTransition`）
- `events/subagent.rs`：以 SPEC 为准（`SubagentSpawned / SubagentAnnounced / SubagentTerminated / SubagentSpawnPaused / SubagentPermissionForwarded / SubagentPermissionResolved`）
- `events/team.rs`：以 SPEC 为准（`TeamCreated / TeamMemberJoined / TeamMemberLeft / TeamMemberStalled / AgentMessageSent / AgentMessageRouted / TeamTurnCompleted / TeamTerminated`）
- `events/context.rs`：以 SPEC 为准（`CompactionApplied / ContextBudgetExceeded / ContextStageTransitioned`）
- `events/observability.rs`：以 SPEC 为准（`UsageAccumulated / TraceSpanCompleted`）
- `events/error.rs`：以 SPEC 为准（`EngineFailed / UnexpectedError`）
- **顶层 `Event` 枚举汇总** 在 `events/mod.rs`：
  - `#[non_exhaustive]`
  - `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, strum::EnumDiscriminants)]`
  - `#[strum_discriminants(name(EventKind), derive(Hash, Eq, PartialEq, Serialize, Deserialize))]`
  - `#[serde(tag = "type", rename_all = "snake_case")]`
  - 变体数量：以 `harness-contracts.md` L132-L279 为准（约 80 个变体）

**关键不变量**：

- 顶层 `Event` 是 `tag = "type"`（**不是** `tag = "kind"`），与 SPEC 一字不差
- 派生 `strum::EnumDiscriminants` 自动生成 `EventKind`（业务方不得另立判别枚举）
- 必须包含全部 v1.8.1 修订引入的事件（`GraceCallTriggered / CredentialPoolSharedAcrossTenants / ManifestValidationFailed / HookFailed / SteeringMessage* / ExecuteCode* / SandboxBackpressureApplied / SubagentSpawnPaused / ...`）

**禁止行为**：

- 禁止使用任何 plan v1 误命名（`IterationStarted/Ended` / `SessionReloaded` / `SessionSnapshotCreated` / `ToolExecution*` / `SteeringPushed/Merged/Dropped` / `ExecuteCodeStarted/Completed`）
- 禁止改 serde tag 为 `kind` 或 `content` 字段名

**SPEC 一致性自检**：

```bash
# 顶层 Event 派生与 serde 属性
grep -E '#\[serde\(tag = "type", rename_all = "snake_case"\)\]' crates/octopus-harness-contracts/src/events/mod.rs
grep -E 'strum::EnumDiscriminants' crates/octopus-harness-contracts/src/events/mod.rs

# 不允许 tag = "kind" 这个 plan v1 误写
! grep -rE 'tag = "kind"|content = "data"' crates/octopus-harness-contracts/

# 顶层 Event 变体计数应 ≥ 75（SPEC 实际约 80）
test "$(grep -E '^    [A-Z][a-zA-Z]+\(' crates/octopus-harness-contracts/src/events/mod.rs | wc -l)" -ge 75
```

---

### M1-T06 · 消息 / Part 类型

| 字段 | 值 |
|---|---|
| **依赖** | M1-T03 |
| **预期 diff** | < 350 行 |

**SPEC 锚点**：
- `harness-contracts.md` §3.5（MessagePart / ToolResultPart / TurnInput / Message / ImageMeta / ...）
- ADR-002（Tool 不含 UI；ToolResultPart 8 个白名单变体）

**预期产物**：

- `src/messages.rs`（或 `src/parts.rs`）：
  - `MessagePart` 枚举（Text / Image / FileRef / ...）
  - `ToolResultPart` 枚举（v1.8 8 个正向白名单变体）
  - `TurnInput` 结构（user / system_message / metadata 等）
  - `Message` 结构
  - `ImageMeta`
  - `BlobRef`

**关键不变量**：

- ToolResultPart 必须仅含 SPEC §3.5 列出的 8 个变体（ADR-002 正向白名单）
- 不允许 React.* / 任何 UI 类型

---

### M1-T07 · BlobStore / ToolCapability / DecisionScope / **Redactor** 抽象

| 字段 | 值 |
|---|---|
| **依赖** | M1-T03 |
| **预期 diff** | < 300 行 |

**SPEC 锚点**：
- `harness-contracts.md` §3.6（BlobStore trait + BlobMeta + BlobRetention + BlobError）
- `harness-contracts.md` §3.4（ToolCapability + CapabilityRegistry + 7 个 *Cap 窄接口）
- `harness-contracts.md` §3.4（DecisionScope）
- `api-contracts.md` §18.2（**Redactor trait 权威定义**：`fn redact(&self, input: &str, rules: &RedactRules) -> String`）
- `harness-observability.md` §2.5.0（"必经管道"是**装配点契约**，6 个挂钩点由调用方负责把字符串过 `redact()`，不是 trait 多 method）

**ADR 锚点**：
- ADR-0011 / ADR-012（capability handle）

**预期产物**：

- `src/blob.rs`（BlobStore trait + 三类型）
- `src/capability.rs`（ToolCapability enum + CapabilityRegistry + 7 个 *Cap trait）
- `src/scope.rs`（DecisionScope，含 v1.8 新增 `ExecuteCodeScript`）
- `src/redactor.rs`（与 `api-contracts.md §18.2` 完全一致）：
  ```rust
  pub trait Redactor: Send + Sync + 'static {
      fn redact(&self, input: &str, rules: &RedactRules) -> String;
  }

  pub struct RedactRules { /* 字段以 SPEC 为准 */ }

  pub struct NoopRedactor;
  impl Redactor for NoopRedactor {
      fn redact(&self, input: &str, _: &RedactRules) -> String { input.to_owned() }
  }
  ```
  - **不允许** trait 内多 method、不允许引用 L1 类型（`Event / MessagePart / ModelRequest / ModelStreamEvent` 等）
  - "6 行挂钩点"是**装配点（call site）契约**，由 EventStore / Hook / MCP / Model in-out 各自调用 `redact()` 处理需要脱敏的字符串字段，不是 trait 形态

**关键不变量**：

- `BlobStore` trait 必须是 `Send + Sync + 'static + dyn-safe`
- ToolCapability enum 不得使用别名 `ToolCapabilityHandle / ToolCap`（v1.8.1 P2-2 修订）
- 7 个 *Cap trait：`PermissionCap / SandboxCap / ModelCap / MemoryCap / SubagentCap / TeamCap / SkillCap`
- `Redactor` 必须 dyn-safe，签名**逐字** copy 自 `api-contracts.md §18.2`
- contracts crate **零依赖** L1 类型（dependency 必须保持 §M1-T01 列表，不许引入 `octopus-harness-model` 等）
- `NoopRedactor` 必须 `Default + Clone + Debug`

**禁止行为**：

- **不要**让 Redactor trait 引用 `Event / MessagePart / ModelRequest / ModelStreamEvent`（违反 L0→L1 依赖方向）
- **不要**自创 `redact_event / redact_message_part / redact_tool_input / ...` 多 method 形态（plan v1 误命名，已废弃）
- **不要**把 `Redactor` 真实现（默认正则规则）写在本 crate（归属 M5-T03 `harness-observability`）
- 本卡 Redactor trait 仅定义契约 + Noop 默认

**验收命令**：

```bash
cargo test -p octopus-harness-contracts blob
cargo test -p octopus-harness-contracts capability
cargo test -p octopus-harness-contracts scope
cargo test -p octopus-harness-contracts redactor
```

**SPEC 一致性自检**：

```bash
# Redactor trait 单 method（与 api-contracts §18.2 一致）
grep -E '^\s*fn redact\(&self, input: &str, rules: &RedactRules\) -> String' crates/octopus-harness-contracts/src/redactor.rs

# 反向：禁出现 plan v1 多 method 误命名
! grep -E 'fn redact_(event|message_part|tool_input|tool_output|model_request|model_response)' crates/octopus-harness-contracts/src/redactor.rs

# contracts 不依赖 L1 类型
! grep -rE 'use octopus_harness_(model|journal|sandbox|permission|memory)' crates/octopus-harness-contracts/src/

# NoopRedactor 默认实现存在
grep -q 'pub struct NoopRedactor' crates/octopus-harness-contracts/src/redactor.rs
```

---

### M1-T08 · 错误类型族

| 字段 | 值 |
|---|---|
| **依赖** | M1-T03 |
| **预期 diff** | < 300 行 |

**SPEC 锚点**：
- `harness-contracts.md` §3.7（HarnessError + ToolError + SandboxError + ModelError + ...）

**预期产物**：

- `src/errors.rs`：
  - `HarnessError`（顶层错误，`#[derive(Debug, thiserror::Error)]`）
  - `ToolError / SandboxError / ModelError / PermissionError / MemoryError / McpError / HookError / PluginError / ContextError / SessionError / EngineError / SteeringError`
  - `From<*Error> for HarnessError` 实现

**关键不变量**：

- 不允许 `anyhow::Error` 进入公开 API
- 错误类型必须可 serialize（事件流场景）
- 必须有 `PromptCacheLocked` 错误（运行期改 system prompt 时返回）

---

### M1-T09 · schemars 派生 + Schema 导出

| 字段 | 值 |
|---|---|
| **依赖** | T02-T08 |
| **预期 diff** | < 200 行 |

**SPEC 锚点**：
- `harness-contracts.md` §3.8（Schema 导出策略）

**预期产物**：

- `crates/octopus-harness-contracts/src/schema_export.rs`（提供 `export_all_schemas()` fn）
- `examples/export_schemas.rs`：CLI 工具，输出到 `schemas/harness-contracts/*.json`
- 添加到 CI：`cargo run --example export_schemas` 后 diff 检查

**关键不变量**：

- 输出 schema 文件 ≥ 60 个（Event 各变体 + ID + enum + struct）
- schema 文件命名：snake_case（如 `tool_use_requested.json`）
- schema 输出对齐 OpenAPI（`contracts/openapi/`）治理路径（M9 时复用）

**验收命令**：

```bash
cargo run --example export_schemas
ls schemas/harness-contracts/*.json | wc -l   # ≥ 60
```

---

### M1-T10 · M1 Gate

| 字段 | 值 |
|---|---|
| **依赖** | T01-T09 |
| **预期 diff** | 0 |

**Gate 通过判据**：

- ✅ `cargo doc --no-deps -p octopus-harness-contracts` 干净生成（无警告）
- ✅ `cargo test -p octopus-harness-contracts --all-features` 全绿
- ✅ schemas 输出 ≥ 60 文件
- ✅ 与 D3 `api-contracts.md` trait/struct 列表 100% 对齐（人工 grep 比对）
- ✅ 12 个 v1.8.1 修订项全部体现（事件名以 `harness-contracts.md §3.3` 与 `event-schema.md §3` 为权威源）：
  - `TenantId::SINGLE` + `TenantId::SHARED` 双哨兵
  - `EndReason::Cancelled { initiator }` + `CancelInitiator` 枚举
  - `GraceCallTriggeredEvent`（`event-schema.md §3.1.1`）
  - `ToolOrigin::Plugin { plugin_id: PluginId, trust }` newtype
  - `CredentialPoolSharedAcrossTenantsEvent`
  - `ManifestValidationFailedEvent`
  - `HookFailureMode::FailOpen / FailClosed` + `HookFailedEvent`
  - `SteeringId` + 3 Steering 事件（**以 SPEC 为准**：`SteeringMessageQueuedEvent / SteeringMessageAppliedEvent / SteeringMessageDroppedEvent`）
  - `ExecuteCodeStepInvokedEvent / ExecuteCodeWhitelistExtendedEvent`
  - `ToolCapability::CodeRuntime`
  - `ToolResultPart` 8 正向白名单
  - `DecisionScope::ExecuteCodeScript`
- ✅ **Redactor trait + NoopRedactor 已落地**（contracts 层签名对齐 `api-contracts.md §18.2`：`fn redact(&self, input: &str, rules: &RedactRules) -> String`；不引用 L1 类型；M2/M5 EventStore 装配 redactor 时调用方负责把 6 行挂钩点的字符串过 `redact()`）

未全绿 → 不得开始 M2。

---

## 3. 索引

- **上一里程碑** → [`M0-bootstrap.md`](./M0-bootstrap.md)
- **下一里程碑** → [`M2-l1-primitives.md`](./M2-l1-primitives.md)
- **D3 api-contracts** → [`docs/architecture/harness/api-contracts.md`](../../../architecture/harness/api-contracts.md)
- **harness-contracts SPEC** → [`docs/architecture/harness/crates/harness-contracts.md`](../../../architecture/harness/crates/harness-contracts.md)
