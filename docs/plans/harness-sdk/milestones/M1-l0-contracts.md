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
- 序列化用 `#[serde(rename_all = "snake_case")]`（与现有事件治理一致）
- `EndReason` 必须含 `Cancelled` 变体

**验收命令**：

```bash
cargo test -p octopus-harness-contracts enums
```

---

### M1-T04a · Event 枚举 · Session(5) + Run(8)

| 字段 | 值 |
|---|---|
| **依赖** | M1-T03 |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `docs/architecture/harness/event-schema.md` §3.0（Session 生命周期，5 个事件）
- `docs/architecture/harness/event-schema.md` §3.1（Run + Turn，8 个事件，含 v1.8.1 P2-5 新增 GraceCallTriggered §3.1.1）

**ADR 锚点**：
- ADR-001（event-sourcing）
- 评审报告 P1-3 / P2-5（Cancelled + GraceCallTriggered）

**预期产物**：

- `crates/octopus-harness-contracts/src/events/mod.rs`：声明 `pub mod session; pub mod run;`，**顶层 `Event` 枚举留空架（仅引入待汇总注释）**，待 T05b 汇总
- `crates/octopus-harness-contracts/src/events/session.rs`：5 个事件结构体 — `SessionCreated / SessionForked / SessionEnded / SessionReloaded / SessionSnapshotCreated`
- `crates/octopus-harness-contracts/src/events/run.rs`：8 个事件结构体 — `RunStarted / RunEnded / IterationStarted / IterationEnded / GraceCallTriggered / AssistantDeltaProduced / AssistantMessageCompleted`（其中 GraceCallTriggered 必须含 `usage_snapshot` 字段，对齐 §3.1.1）

**关键不变量**：

- 每个事件结构体必带 `tenant_id / session_id / run_id?` 三件套
- 所有事件必须 `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]`
- 事件必须实现 `EventEnvelope` trait（提供 `event_id() / occurred_at() / kind()`），`EventEnvelope` trait 在本卡 `events/mod.rs` 内首发定义
- `RunEnded.reason: EndReason` 必须能承载 `Cancelled { initiator }`（对齐 v1.8.1 P1-3）

**禁止行为**：

- 不要省略事件中的"看似冗余"字段（如 `tenant_id`，是反向追踪必备）
- 不要把"派生数据"塞进事件（事件是 fact）
- 本卡不汇总顶层 `Event` 枚举（汇总归 T05b）

**验收命令**：

```bash
cargo test -p octopus-harness-contracts events::session
cargo test -p octopus-harness-contracts events::run
```

---

### M1-T04b · Event 枚举 · Tool(10) + Permission(5)

| 字段 | 值 |
|---|---|
| **依赖** | M1-T04a |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `event-schema.md` §3.4（Tool 执行，10 个事件）
- `event-schema.md` §3.2（Permission 事件 5 个，含 v1.8.1 P1-5 新增 `CredentialPoolSharedAcrossTenants`）

**ADR 锚点**：
- ADR-007（permission-events）
- ADR-009（deferred-tool-loading：ToolDeferredPoolChanged）
- ADR-0010（tool-result-budget：ToolResultBudgetExceeded）

**预期产物**：

- `crates/octopus-harness-contracts/src/events/tool.rs`：10 个事件结构体 — `ToolUseRequested / ToolExecutionStarted / ToolExecutionCompleted / ToolExecutionFailed / ToolResultBudgetExceeded / ToolPoolChanged / ToolDeferredPoolChanged / ToolSearchQueried / ToolSchemaMaterialized / ToolSearchResultMaterialized`
- `crates/octopus-harness-contracts/src/events/permission.rs`：5 个事件结构体 — `PermissionRequested / PermissionResolved / PermissionRuleAdded / PermissionContextElevated / CredentialPoolSharedAcrossTenants`

**关键不变量**：

- `PermissionResolved.decided_by: DecidedBy` 必须区分 User / Hook / Rule / Auto 四类
- `CredentialPoolSharedAcrossTenants` 字段必须含 `from_tenant_id / to_tenant_id_hint` 反向追踪字段（v1.8.1 P1-5）
- `events/mod.rs` 声明 `pub mod tool; pub mod permission;`

**验收命令**：

```bash
cargo test -p octopus-harness-contracts events::tool
cargo test -p octopus-harness-contracts events::permission
```

---

### M1-T05a · Event 枚举 · Steering(3) + ExecuteCode(2) + Memory(6)

| 字段 | 值 |
|---|---|
| **依赖** | M1-T04b |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `event-schema.md` §3.5.1（Steering 事件 3 个）
- `event-schema.md` §3.5.2（ExecuteCode 事件 2 个）
- `event-schema.md` §3.6（Memory 事件 6 个，含 v1.4 新增 4 个）

**ADR 锚点**：
- ADR-0017（steering-queue）
- ADR-0016（programmatic-tool-calling）

**预期产物**：

- `events/steering.rs`：`SteeringPushed / SteeringMerged / SteeringDropped`
- `events/execute_code.rs`：`ExecuteCodeStarted / ExecuteCodeCompleted`
- `events/memory.rs`：6 个事件 — `MemoryRecalled / MemoryUpserted / MemoryForgotten / MemoryThreatDetected / MemoryAccessDenied / MemoryProviderSwitched`

**关键不变量**：

- Steering 事件 `SteeringPushed.merge_behavior: SteeringMergeBehavior` 必填
- ExecuteCode 事件必须能在 `decision_scope: DecisionScope::ExecuteCodeScript` 下被审批

**验收命令**：

```bash
cargo test -p octopus-harness-contracts events::steering
cargo test -p octopus-harness-contracts events::execute_code
cargo test -p octopus-harness-contracts events::memory
```

---

### M1-T05b · Event 枚举 · Hook(5) + MCP(8) + Plugin(7) + Team(3) + 顶层 Event 汇总

| 字段 | 值 |
|---|---|
| **依赖** | M1-T05a |
| **预期 diff** | < 450 行 |

**SPEC 锚点**：
- `event-schema.md` §3.7（Hook 事件 5 个，含 `HookFailed`）
- `event-schema.md` §3.19（MCP 事件 8 个）
- `event-schema.md` §3.20（Plugin Lifecycle 7 个，含 v1.8.1 `ManifestValidationFailed`）
- `event-schema.md` §3.21（Team 事件 3 个）

**预期产物**：

- `events/hook.rs`：`HookInvoked / HookCompleted / HookFailed / HookSkipped / HookTransportTimeout`
- `events/mcp.rs`：`McpServerConnected / McpServerDisconnected / McpToolsListChanged / McpResourceFetched / McpSamplingRequested / McpSamplingApproved / McpElicitationRequested / McpElicitationResolved`
- `events/plugin.rs`：`PluginManifestDiscovered / PluginManifestValidated / ManifestValidationFailed / PluginActivationStarted / PluginActivationCompleted / PluginActivationFailed / PluginRevoked`
- `events/team.rs`：`TeamCreated / TeamMemberJoined / TeamMemberLeft`
- **顶层 `Event` 枚举汇总**：在 `events/mod.rs` 完成 `pub enum Event { Session(...), Run(...), Tool(...), Permission(...), Steering(...), ExecuteCode(...), Memory(...), Hook(...), Mcp(...), Plugin(...), Team(...) }`（`#[serde(tag = "kind", content = "data")]` tagged）

**关键不变量**：

- 顶层 `Event` 是 `#[serde(tag = "kind", content = "data")]` 的 tagged enum
- 必须包含 v1.8.1 修订引入的全部事件（P0/P1/P2 修订点逐一对照）
- `Event` 必须实现 `EventEnvelope` trait 的 dispatch（按变体 forward 到内部事件结构）

**禁止行为**：

- 不要在本卡修改前 4 张子卡定义的事件结构体（如发现 bug 走铁律 1 例外）

**验收命令**：

```bash
cargo test -p octopus-harness-contracts events::hook
cargo test -p octopus-harness-contracts events::mcp
cargo test -p octopus-harness-contracts events::plugin
cargo test -p octopus-harness-contracts events::team
cargo test -p octopus-harness-contracts events::Event   # 顶层 dispatch 用例
```

**SPEC 一致性自检**：

```bash
# 12 个 v1.8.1 修订事件必须全部存在
for ev in GraceCallTriggered SteeringPushed SteeringMerged SteeringDropped ExecuteCodeStarted ExecuteCodeCompleted CredentialPoolSharedAcrossTenants ManifestValidationFailed HookFailed; do
    grep -q "pub struct ${ev}" crates/octopus-harness-contracts/src/events/*.rs || echo "MISSING: $ev"
done
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
| **预期 diff** | < 350 行 |

**SPEC 锚点**：
- `harness-contracts.md` §3.6（BlobStore trait + BlobMeta + BlobRetention + BlobError）
- `harness-contracts.md` §3.4（ToolCapability + CapabilityRegistry + 7 个 *Cap 窄接口）
- `harness-contracts.md` §3.4（DecisionScope）
- `docs/architecture/harness/harness-observability.md` §2.5.0（**Redactor 必经管道契约**，v1.8.1 P2-7）
- `docs/architecture/harness/crates/harness-journal.md` §2.1（EventStore 头注：必装配 `Arc<Redactor>`）

**ADR 锚点**：
- ADR-0011 / ADR-012（capability handle）
- 实施前评估 P0-D（Redactor trait 必须在 contracts 层定义，供 M2 EventStore 装配槽预留）

**预期产物**：

- `src/blob.rs`（BlobStore trait + 三类型）
- `src/capability.rs`（ToolCapability enum + CapabilityRegistry + 7 个 *Cap trait）
- `src/scope.rs`（DecisionScope，含 v1.8 新增 `ExecuteCodeScript`）
- `src/redactor.rs`（**新增**）：
  - `Redactor` trait（`Send + Sync + 'static + dyn-safe`）：
    ```rust
    pub trait Redactor: Send + Sync + 'static {
        fn redact_event(&self, event: &mut Event) -> Result<(), RedactorError>;
        fn redact_message_part(&self, part: &mut MessagePart) -> Result<(), RedactorError>;
        fn redact_tool_input(&self, input: &mut serde_json::Value) -> Result<(), RedactorError>;
        fn redact_tool_output(&self, output: &mut serde_json::Value) -> Result<(), RedactorError>;
        fn redact_model_request(&self, req: &mut ModelRequest) -> Result<(), RedactorError>;
        fn redact_model_response(&self, resp: &mut ModelStreamEvent) -> Result<(), RedactorError>;
    }
    ```
  - `pub struct NoopRedactor;` 实现 Redactor，所有方法返回 `Ok(())`（M2 期 EventStore 默认装配，待 M5 替换）
  - `RedactorError` enum（`thiserror::Error`，含 `Internal / Pattern / Encoding` 三档）

**关键不变量**：

- `BlobStore` trait 必须是 `Send + Sync + 'static + dyn-safe`
- ToolCapability enum 不得使用别名 `ToolCapabilityHandle / ToolCap`（v1.8.1 P2-2 修订）
- 7 个 *Cap trait：`PermissionCap / SandboxCap / ModelCap / MemoryCap / SubagentCap / TeamCap / SkillCap`
- `Redactor` 必须 dyn-safe（M2 EventStore 通过 `Arc<dyn Redactor>` 装配）
- `NoopRedactor` 必须 `Default + Clone + Debug`

**禁止行为**：

- 不要把 `Redactor` 真实现（默认正则规则）写在本 crate（归属 M5-T03 `harness-observability`）
- 本卡 Redactor trait 仅定义契约 + Noop 默认；任何具体规则 30+ 模式都不入 contracts

**验收命令**：

```bash
cargo test -p octopus-harness-contracts blob
cargo test -p octopus-harness-contracts capability
cargo test -p octopus-harness-contracts scope
cargo test -p octopus-harness-contracts redactor
```

**SPEC 一致性自检**：

```bash
# Redactor trait 必经管道 6 个挂钩点全部存在
for hook in redact_event redact_message_part redact_tool_input redact_tool_output redact_model_request redact_model_response; do
    grep -q "fn ${hook}" crates/octopus-harness-contracts/src/redactor.rs || echo "MISSING hook: $hook"
done

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
- ✅ 12 个 v1.8.1 修订项全部体现：
  - TenantId::SINGLE + SHARED 双哨兵
  - EndReason::Cancelled + CancelInitiator
  - GraceCallTriggered Event
  - ToolOrigin::Plugin { plugin_id: PluginId } newtype
  - CredentialPoolSharedAcrossTenants Event
  - ManifestValidationFailed Event
  - HookFailureMode FailOpen / FailClosed
  - SteeringId + 3 Steering Events（Pushed/Merged/Dropped）
  - ExecuteCodeScript / 2 ExecuteCode Events
  - ToolCapability::CodeRuntime
  - ToolResultPart 8 正向白名单
  - DecisionScope::ExecuteCodeScript
- ✅ **Redactor trait + NoopRedactor 已落地**（contracts 层 6 个 hook 完整，对应 v1.8.1 P2-7 必经管道契约；M2 EventStore 据此预留装配槽）

未全绿 → 不得开始 M2。

---

## 3. 索引

- **上一里程碑** → [`M0-bootstrap.md`](./M0-bootstrap.md)
- **下一里程碑** → [`M2-l1-primitives.md`](./M2-l1-primitives.md)
- **D3 api-contracts** → [`docs/architecture/harness/api-contracts.md`](../../../architecture/harness/api-contracts.md)
- **harness-contracts SPEC** → [`docs/architecture/harness/crates/harness-contracts.md`](../../../architecture/harness/crates/harness-contracts.md)
