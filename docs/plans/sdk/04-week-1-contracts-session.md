# W1 · `octopus-sdk-contracts` + `octopus-sdk-session`

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `02-crate-topology.md §2.1 / §2.2 / §5 / §6` → `03-legacy-retirement.md §0`。

## Goal

产出 **两个零业务语义的新 crate**——`crates/octopus-sdk-contracts`（Level 0，纯数据 IR）与 `crates/octopus-sdk-session`（Level 1，`SessionStore` trait + `SqliteJsonlSessionStore` 默认实现），使之成为 W2–W8 所有后续 SDK crate 与业务侧共用的**唯一数据契约与事件存储抽象**。

## Architecture

- **Level 0 · contracts**：零 I/O、零 async runtime、零 HTTP、零 SQLite；只依赖 `serde / thiserror / uuid / async-trait`（后者仅用于 `SecretVault` trait 定义，不带来运行时状态）。公共面严格等于 `02-crate-topology.md §2.1` 登记的符号集；任何增减必须同批次回填 `02`。
- **Level 1 · session**：`SessionStore` trait 只规定行为（append/stream/snapshot/fork/wake），不规定存储技术。`SqliteJsonlSessionStore` 为**默认实现**，落盘分两路：**SQLite projection** 归档状态索引，**`runtime/events/*.jsonl`** 作为 append-only 真相源；两者都可由上层配置路径。
- **并行保留**：`crates/runtime::session_*`、`crates/octopus-runtime-adapter::session_*` 在 W1 **不动**；W6 前作为双写/只读比对的对照源。
- **持久化治理遵循**：严格按 `/AGENTS.md §Persistence Governance` 执行——`data/main.db` 为结构化索引，`runtime/events/*.jsonl` 为 append-only 事件流，不以 `runtime/sessions/*.json` 为真相源。

## Scope

- In scope：
  - 新建 `crates/octopus-sdk-contracts/` 与 `crates/octopus-sdk-session/` 两个 crate 骨架（`Cargo.toml` / `src/lib.rs` / `tests/`）。
  - `02 §2.1` 全部符号：`SessionId / RunId / ToolCallId / EventId / Role / ContentBlock / Message / Usage / AssistantEvent / StopReason / RenderBlock / RenderKind / AskPrompt / ArtifactRef / RenderLifecycle / SessionEvent / EndReason / SecretVault / SecretValue / VaultError / PromptCacheEvent`（最后一项见本 Plan §Risks R1）。
  - `02 §2.2` 全部符号：`SessionStore / EventRange / EventStream / SessionSnapshot / SessionError / SqliteJsonlSessionStore`。
  - `SessionStarted` 作为会话**首事件**的契约测试（含 `config_snapshot_id` + `effective_config_hash`）。
  - `SessionEvent` / `Usage` / `AssistantEvent` 的**序列化稳定性**（字段顺序、枚举 tagged 策略）守护测试。
  - 与 `contracts/openapi/src/**` 的字段差异登记回 `02-crate-topology.md §5 契约差异清单`。
- Out of scope：
  - `octopus-sdk-model / sdk-tools / sdk-mcp / ...` 等 W2+ crate。
  - 任何业务域概念（Project / Workspace / Team / Task / Deliverable / User / Org）。
  - 旧 crate 的删除或符号迁移（W7 统一执行）。
  - `SqliteJsonlSessionStore::wake` 的 checkpoint 回放全语义（W6 再完整落地；本周仅支持"从头重放 + 最新 snapshot"的最小路径）。
  - UI Intent IR 的 schema validator（`02 §6` 的 kind 登记表维持现状；本周只落 `RenderBlock / RenderKind / AskPrompt / ArtifactRef / RenderLifecycle` 的 Rust 数据签名，不含 JSON Schema 校验器与 `validate_render_block` 实现）。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | `02 §2.1` 中 `AssistantEvent::PromptCache(PromptCacheEvent)` 引用了 `PromptCacheEvent`，但 `§2.1` 未给签名；`AssistantEvent` 要稳定序列化必须先确定该类型。 | W1 内在 contracts 新增 `PromptCacheEvent { cache_read_input_tokens: u32, cache_creation_input_tokens: u32, breakpoint_count: u32 }` 与 `CacheBreakpoint { position: usize, ttl: CacheTtl }`、`CacheTtl { FiveMinutes, OneHour }` 的最小签名；同批次回填 `02 §2.1`。 | 若该签名需要与 OpenAPI 对齐 → #1 或 #3 |
| R2 | OpenAPI `contracts/openapi/src/**` 已有 `Usage / Message / ContentBlock` 类似类型，与新 contracts 可能字段命名/可选性不一致。 | 本周**不改 OpenAPI**。差异逐项登记到 `02 §5 契约差异清单`，W2 随 `sdk-model` 一起决定 upstream / downstream。 | 若差异涉及 `cache_*` 字段语义分歧 → #1 |
| R3 | `SqliteJsonlSessionStore::wake` 与 W6 的 `Checkpoint` 回放契约未最终确定。 | W1 提供最小版：`wake` = 读取最新 `Checkpoint` 事件（若存在）再从其 `anchor_event_id` 顺序 replay 至末尾；若无 checkpoint 则从头顺序 replay。完整 wake 语义（含 context compaction replay）W6 再扩。 | 若上层 API 需要 "wake without replay" → #8 |
| R4 | `runtime/events/*.jsonl` 的分片策略（按 session / 按天 / 按 run）未统一。 | 本周确定：每 `SessionId` 一个文件 `<jsonl_root>/<session_id>.jsonl`；旋转策略与更细粒度分片交 W8（与 `octopus-persistence` 一并落地）。 | 若需要跨 session 全局事件索引 → 写入 `00-overview.md §6 风险登记簿` 并 Stop #8 |
| R5 | contracts 是否需要 `feature-gated serde`？ | 不需要。contracts 默认启用 serde（它的核心价值就是序列化契约），不设 feature flag。 | — |

## Execution Rules

- 遵循 `01-ai-execution-protocol.md`：三层 Checklist + Stop Conditions 1–11 全部生效。
- 每个 Task 原子、单 PR ≤ 800 行；违反 → 拆 sub-Task。
- 公共面（`pub` 符号）变动 → 同一 PR 必须更新 `02-crate-topology.md §2.1 / §2.2`；违反 → Stop Condition #1 或本目录 AGENTS.md §5 不一致。
- 任何与 `contracts/openapi/src/**` 的字段差异 → 登记到 `02 §5`，**不**直接改 openapi。
- `crates/runtime / crates/octopus-runtime-adapter` 的任何代码本周**禁止改动**。如发现依赖反向、只能改旧代码才能完成 W1 → Stop Condition #8。
- `default-members` 在 W1 结束时追加 `crates/octopus-sdk-contracts` + `crates/octopus-sdk-session`（两者都不是 `02 §8` 的业务 crate，但为了 `cargo build` 能覆盖新 crate；正式的"5 业务 crate 收敛"在 W7/W8）。

---

## Task Ledger

### Task 1：crate 骨架 + workspace 登记

Status: `pending`

Files:
- Create: `crates/octopus-sdk-contracts/Cargo.toml`
- Create: `crates/octopus-sdk-contracts/src/lib.rs`
- Create: `crates/octopus-sdk-session/Cargo.toml`
- Create: `crates/octopus-sdk-session/src/lib.rs`
- Modify: `Cargo.toml`（workspace `default-members` 追加两个新 crate）

Preconditions：`docs/plans/sdk/02-crate-topology.md §1` 已最终确认；`docs/plans/sdk/AGENTS.md` 已合入。

Step 1：
- Action：创建两个 crate 的最小骨架；`octopus-sdk-contracts/Cargo.toml` 只允许 `[dependencies]` 含 `serde / serde_json / thiserror / uuid / async-trait`；`octopus-sdk-session/Cargo.toml` 额外允许 `tokio / rusqlite / tokio-stream / futures / tracing`。两处 `lib.rs` 保持 ≤ 80 行（遵守文件行数硬约束）。
- Done when：`cargo build -p octopus-sdk-contracts -p octopus-sdk-session` 成功；两个 crate 目录下 `rg 'fn ' -c src/` 均为 0。
- Verify：`cargo build -p octopus-sdk-contracts -p octopus-sdk-session && wc -l crates/octopus-sdk-{contracts,session}/src/lib.rs`
- Stop if：workspace 通配 `crates/*` 未覆盖新目录（理论上不会，但若 `Cargo.toml` 显式 `members` 列表模式下漏登记 → #8）。

Step 2：
- Action：更新 workspace `Cargo.toml` 的 `default-members`，在现有列表追加 `"crates/octopus-sdk-contracts", "crates/octopus-sdk-session"`。
- Done when：`cargo build` 默认构建范围含两个新 crate；`cargo metadata --format-version=1 | jq '.workspace_default_members[]'` 输出包含两者。
- Verify：`cargo build && cargo metadata --format-version=1 --no-deps | jq -r '.workspace_default_members[]' | rg 'octopus-sdk-(contracts|session)'`
- Stop if：已有 `default-members` 约束强制只列业务 crate（查 `00 §2.3 / §W7`）→ 回退该修改，改为 W7 统一调整，并在本任务留 `TODO(W7)` 注释。

Notes：
- Rust 包名 `octopus-sdk-contracts` 与 `octopus-sdk-session`（短横线）；crate 目录同名。
- 对 `Cargo.toml` 唯一允许的修改是 `default-members`；不新增 workspace 级依赖版本。

---

### Task 2：contracts 基础 IR（Id / Role / Message / Usage）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-contracts/src/id.rs`
- Create: `crates/octopus-sdk-contracts/src/message.rs`
- Create: `crates/octopus-sdk-contracts/src/usage.rs`
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`（模块声明 + `pub use`）

Preconditions：Task 1 完成。

Step 1：
- Action：落地 `SessionId / RunId / ToolCallId / EventId`（`#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]`，内部 `String`；提供 `new_v4() -> Self` 工厂）；`Role`（`System / User / Assistant / Tool`，`#[serde(rename_all = "snake_case")]`）；`ContentBlock`（`#[serde(tag = "type", rename_all = "snake_case")]` 标签枚举，四个变体对齐 `02 §2.1`）；`Message { role: Role, content: Vec<ContentBlock> }`。
- Done when：类型签名与 `02-crate-topology.md §2.1` 第 93–111 行完全一致；单元测试：`Message` 序列化 JSON 后的键顺序 = `["role", "content"]`；`ContentBlock::ToolUse` 的 JSON `type` 字段 = `"tool_use"`。
- Verify：`cargo test -p octopus-sdk-contracts message::`
- Stop if：与 `contracts/openapi/src/**` 的 `Message / ContentBlock` 字段名冲突无法消除 → 登记 §4 并 Stop #1。

Step 2：
- Action：落地 `Usage { input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens }`（全 `u32`）；实现 `Default` + `impl Add<&Usage>`（用于后续累加）。
- Done when：`Usage::default()` 四字段均为 0；`&u1 + &u2` 逐字段相加；字段序列化顺序固定。
- Verify：`cargo test -p octopus-sdk-contracts usage::`
- Stop if：OpenAPI 侧字段命名为 `cache_read` / `cache_creation`（无 `_input_tokens` 后缀）→ 以 SDK 命名为准，登记 `02 §5`。

Notes：
- 字段序列化顺序通过 `struct` 声明顺序控制；禁止 `#[serde(flatten)]` 改变顺序。
- Task 2 作为单 PR 提交；规模预估 ~300 行含测试。

---

### Task 3：contracts 事件类型（AssistantEvent / SessionEvent / StopReason / EndReason）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-contracts/src/event.rs`
- Create: `crates/octopus-sdk-contracts/src/prompt_cache.rs`
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`

Preconditions：Task 2 完成。

Step 1：
- Action：新增 `PromptCacheEvent { cache_read_input_tokens: u32, cache_creation_input_tokens: u32, breakpoint_count: u32 }` 与 `CacheBreakpoint { position: usize, ttl: CacheTtl }`、`CacheTtl { FiveMinutes, OneHour }`（与 `restored-src/anthropic-api/prompt_caching.ts` 的 5m / 1h 两档对齐）；**同批次**回填 `02 §2.1` 末尾追加这两个类型的签名（原文 §2.1 第 123 行留了 `PromptCache(PromptCacheEvent)` 变体但未定义 `PromptCacheEvent`，这是 §Risks R1）。
- Done when：`cargo check -p octopus-sdk-contracts` 绿；`02 §2.1` 末尾已追加 `PromptCacheEvent` + `CacheBreakpoint` + `CacheTtl` 的签名。
- Verify：`cargo check -p octopus-sdk-contracts && rg 'pub struct PromptCacheEvent' docs/plans/sdk/02-crate-topology.md`
- Stop if：回填 `02` 时发现 `AssistantEvent` 变体命名需改动 → Stop #1（规范冲突）。

Step 2：
- Action：落地 `StopReason`、`AssistantEvent`（5 个变体，内部 tagged 枚举 `#[serde(tag = "kind")]`），`EndReason`、`SessionEvent`（8 个变体）。
- Done when：所有枚举变体与 `02 §2.1` 第 120–155 行一致；单元测试：`SessionEvent::SessionStarted` 序列化后首字段为 `kind = "session_started"`；`AssistantEvent::ToolUse` 的 `input` 字段类型为 `serde_json::Value`。
- Verify：`cargo test -p octopus-sdk-contracts event::`
- Stop if：`SessionEvent` 需要新增变体（如 `RunStarted / RunEnded`）才能满足 W2 → 先在本 Plan §Risks 追加一行并 Stop #8。

Notes：
- 序列化稳定性守护测试延后到 Task 4 统一处理。

---

### Task 4：contracts 序列化稳定性与黄金测试

Status: `pending`

Files:
- Create: `crates/octopus-sdk-contracts/tests/serialization_golden.rs`
- Create: `crates/octopus-sdk-contracts/tests/fixtures/` 目录（JSON 黄金样本）

Preconditions：Task 2–3 完成。

Step 1：
- Action：为 `Usage / AssistantEvent（5 variant）/ SessionEvent（8 variant）/ ContentBlock（4 variant）/ RenderBlock` 写黄金测试：固定输入 struct → 序列化 JSON → 字节级别对比 `fixtures/*.json`。首次运行若缺 fixture 则写入；CI 运行时必须对比相等。
- Done when：`cargo test -p octopus-sdk-contracts --test serialization_golden` 全绿；`ls crates/octopus-sdk-contracts/tests/fixtures/` 不少于 18 个 `.json`。
- Verify：`cargo test -p octopus-sdk-contracts --test serialization_golden && find crates/octopus-sdk-contracts/tests/fixtures -name '*.json' | wc -l`
- Stop if：fixture 内 JSON 字段顺序不稳定（`serde_json::to_string` 在不同 rustc/serde 版本输出不一致）→ 切换为 `serde_json::to_value` + sorted keys 比对并登记此决策。

Notes：
- 这批黄金测试是 `01-ai-execution-protocol §5 Stop Condition #4`（Prompt Cache 稳定性）的一部分依据；后续 W2 添加的"工具顺序守护"会复用同一风格。
- 禁止用 `#[serde(flatten)]` 或运行期构造字段顺序的 trick。

---

### Task 5：contracts UI Intent IR 签名 + SecretVault

Status: `pending`

Files:
- Create: `crates/octopus-sdk-contracts/src/ui_intent.rs`
- Create: `crates/octopus-sdk-contracts/src/secret.rs`
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`

Preconditions：Task 3 完成。

Step 1：
- Action：落地 `RenderBlock / RenderKind / AskPrompt / ArtifactRef / RenderLifecycle` 签名，字段与 `docs/sdk/14-ui-intent-ir.md` 一致（枚举变体用 snake_case JSON tag）。`RenderKind` 有 10 个变体；`RenderMeta` 先落最小字段：`id: EventId, parent: Option<EventId>, ts_ms: i64`。`AskPrompt / ArtifactRef` 内部字段若 `docs/sdk/14` 未锁 → 本 Plan §Risks 追加一行，在 Task 5 提前和 14 对齐（14 已产出终稿，应可直接复制）。
- Done when：所有 IR 类型通过 rustc 类型检查并被 `SessionEvent::Render { block, lifecycle }` 正确嵌入；`RenderKind` 的 10 个变体与 `02 §6 UI Intent IR 登记表` 的 10 个既有 kind 保持一致，W1 不额外新增 kind 行。
- Verify：`cargo check -p octopus-sdk-contracts && rg -n '^\| 10 \| `raw`' docs/plans/sdk/02-crate-topology.md`
- Stop if：`docs/sdk/14-ui-intent-ir.md` 与 `02 §2.1` 签名冲突 → 登记 Fact-Fix（`docs/sdk/README.md` 末尾的勘误小节）。

Step 2：
- Action：落地 `SecretVault` trait、`SecretValue`、`VaultError`。`SecretValue` 内部持 `zeroize::Zeroizing<Vec<u8>>`；`impl !Debug`（显式 `impl fmt::Debug` 输出 `SecretValue(REDACTED)`）；`Drop` 被 `Zeroizing` 接管。
- Done when：`cargo test -p octopus-sdk-contracts secret::` 绿；包含断言：`format!("{:?}", SecretValue::new(b"sk-xxx")) == "SecretValue(REDACTED)"`。
- Verify：`cargo test -p octopus-sdk-contracts secret::`
- Stop if：引入 `zeroize` 依赖需 workspace 级登记，且该依赖未在 `Cargo.toml [workspace.dependencies]` 中 → 先加 workspace 依赖 + 单 PR；超过规模则拆 sub-Task。

Notes：
- 守卫：`rg 'impl.*Debug.*for SecretValue' crates/octopus-sdk-contracts/src/secret.rs` 必须命中显式实现；否则 derive(Debug) 会泄漏明文。

---

### Task 6：session trait + 错误类型

Status: `pending`

Files:
- Create: `crates/octopus-sdk-session/src/store.rs`
- Create: `crates/octopus-sdk-session/src/error.rs`
- Create: `crates/octopus-sdk-session/src/snapshot.rs`
- Modify: `crates/octopus-sdk-session/src/lib.rs`

Preconditions：Task 2–3 完成（trait 依赖 `SessionId / SessionEvent / EventId / Usage`）。

Step 1：
- Action：落地 `SessionStore` trait、`EventRange`、`EventStream`（`Pin<Box<dyn Stream<Item = Result<SessionEvent, SessionError>> + Send>>`）、`SessionSnapshot`（与 `02 §2.2` 字段完全一致）、`SessionError`（含 `NotFound / Corrupted / Io(std::io::Error) / Sqlite(rusqlite::Error) / Serde(serde_json::Error)` 五个主变体，`thiserror::Error` derive）。
- Done when：`cargo check -p octopus-sdk-session` 绿；trait 对象安全（`Arc<dyn SessionStore>` 可构造）。
- Verify：`cargo check -p octopus-sdk-session && cargo test -p octopus-sdk-session store::trait_object`
- Stop if：trait 方法签名需要 `&mut self` 或泛型参数破坏 `dyn` 兼容 → Stop #1。

Notes：
- `tokio-stream` 作为 `Stream` 依赖来源；不引入 `async-stream` 等额外 crate。

---

### Task 7：session SqliteJsonlSessionStore（append / stream / snapshot）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-session/src/sqlite/mod.rs`
- Create: `crates/octopus-sdk-session/src/sqlite/schema.rs`
- Create: `crates/octopus-sdk-session/src/sqlite/append.rs`
- Create: `crates/octopus-sdk-session/src/sqlite/stream.rs`
- Create: `crates/octopus-sdk-session/src/jsonl.rs`
- Create: `crates/octopus-sdk-session/tests/sqlite_jsonl.rs`

Preconditions：Task 6 完成。

Step 1：
- Action：设计最小 SQLite schema（`sessions(session_id PK, config_snapshot_id, effective_config_hash, head_event_id, usage_json, created_at, updated_at)`、`events(event_id PK, session_id FK, seq INTEGER, kind TEXT, payload TEXT, created_at)`，对 `(session_id, seq)` 建唯一索引）。在 `schema.rs` 用 `CREATE TABLE IF NOT EXISTS` 幂等初始化。
- Done when：首次 `SqliteJsonlSessionStore::open` 生成两张表；`cargo test -p octopus-sdk-session sqlite::schema::` 绿。
- Verify：`cargo test -p octopus-sdk-session sqlite::schema::`
- Stop if：schema 必须与 `crates/octopus-infra` 已有 sessions/events 表结构兼容 → 本周不兼容（并行路径，不共享表），登记 `00-overview.md §6 风险登记簿` 并继续；若被要求共享 → Stop #8。

Step 2：
- Action：实现 `append(&self, id, event)`：`BEGIN → 写 JSONL（fsync）→ UPSERT events 行（seq = max+1）→ UPDATE sessions.head_event_id → COMMIT`；失败时保证 SQLite 与 JSONL 一致（先写 JSONL 再写 DB；DB 写入失败时 JSONL 已落但 head 未更新，启动时用"JSONL 尾部 vs DB head" 一致性检查修复）。JSONL 路径为 `<jsonl_root>/<session_id>.jsonl`。
- Done when：`tests/sqlite_jsonl.rs::test_append_roundtrip` 绿：10 个 `SessionEvent` append 后，`stream(id, EventRange::all())` 产出的序列与原序列严格相等。
- Verify：`cargo test -p octopus-sdk-session --test sqlite_jsonl test_append_roundtrip`
- Stop if：`rusqlite` 在 tokio 运行时下死锁（调度问题）→ 改 `tokio::task::spawn_blocking` 包装；若超过 Task 规模 → 拆 sub-Task。

Step 3：
- Action：实现 `stream(&self, id, range)` 与 `snapshot(&self, id)`。`stream` 基于 `EventRange::{ after, limit }` 从 SQLite 分页读；`snapshot` 从 `sessions` 表直接读出 `SessionSnapshot`；`Usage` 的累加交由 §7 契约测试覆盖。
- Done when：`test_stream_after_cursor`、`test_snapshot_matches_last_event` 两个测试绿。
- Verify：`cargo test -p octopus-sdk-session --test sqlite_jsonl`
- Stop if：事件数超过 `i64::MAX` 的 `seq` 风险（不会触发，但需显式留 TODO）→ 忽略。

Notes：
- JSONL 文件旋转（大小 / 时间）**本周不做**；本周固定每 session 单文件 `<session_id>.jsonl`，登记 §Risks R4。
- `append` 的 durability：JSONL 写入 + `fsync`；SQLite 走默认 journal；不引入 WAL 配置变更（留 W8 `octopus-persistence` 决定）。

---

### Task 8：fork / wake 最小实现 + SessionStarted 契约测试

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-session/src/sqlite/mod.rs`（追加 fork / wake）
- Create: `crates/octopus-sdk-session/tests/contract_session_started.rs`
- Create: `crates/octopus-sdk-session/tests/fork_wake.rs`

Preconditions：Task 7 完成。

Step 1：
- Action：实现 `fork(&self, id, from_event_id) -> Result<SessionId, _>`：生成新 `SessionId`，在 sessions 表写入 `config_snapshot_id` = 源会话同值，`head_event_id` = `from_event_id`；在 events 表**复制** `seq ≤ from_event.seq` 的所有行到新 session（简单方案，W6 再优化为共享前缀）。`wake(&self, id)`：返回最新 `SessionSnapshot`；若检测到事件序列中有 `Checkpoint` 变体，则从最新 `Checkpoint::anchor_event_id` 之后的事件准备好 replay（真正的 replay 在 W6 实现；本周只验证 snapshot 返回值 & `anchor_event_id` 可读）。
- Done when：`fork_wake.rs` 两个测试通过：`test_fork_preserves_prefix`、`test_wake_returns_latest_snapshot`。
- Verify：`cargo test -p octopus-sdk-session --test fork_wake`
- Stop if：fork 需要重写 `event_id`（避免 UNIQUE 冲突）→ 允许；若需要改变 `SessionEvent` 的结构 → Stop #1。

Step 2：
- Action：契约测试 `contract_session_started.rs`：构造一个新 session，首次 `append` 非 `SessionStarted` 事件必须返回 `SessionError::Corrupted { reason: "first_event_must_be_session_started" }`；首次 `append(SessionStarted { config_snapshot_id, effective_config_hash })` 成功后，该两字段可从 `snapshot()` 读出且与输入一致。
- Done when：测试绿；并在 `02-crate-topology.md §2.2` 末尾追加一条 **"首事件必须为 `SessionStarted`"** 的不变量说明（登记到该节"契约不变量"小节，若不存在则同批次新增）。
- Verify：`cargo test -p octopus-sdk-session --test contract_session_started && rg '首事件必须为 `SessionStarted`' docs/plans/sdk/02-crate-topology.md`
- Stop if：业务侧（现有 `crates/runtime`）已在别处写入非 `SessionStarted` 首事件 → 旧代码不改（本周 Out of scope），但在 §Risks 追加"双轨一致性"并 Stop #9（legacy 隐式依赖）如真阻断。

Notes：
- `Checkpoint` 事件本周允许 append，但 wake 只做最简 snapshot 返回；W6 再落完整 replay + context compaction。

---

### Task 9：公共面冻结 + 契约差异登记 + Weekly Gate 收尾

Status: `pending`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md`（§2.1 / §2.2 / §5 / §6 / §10）
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（若本周动了 legacy 记录 → §7 登记，正常情况下为无改动）
- Modify: `docs/plans/sdk/README.md`（W1 行状态 `draft` → `done`）
- Modify: `docs/plans/sdk/04-week-1-contracts-session.md`（本文件；Task 全 `done`、追加 Checkpoint、变更日志）

Preconditions：Task 1–8 全 `done`；`cargo test -p octopus-sdk-contracts -p octopus-sdk-session` 全绿；`cargo clippy -p octopus-sdk-contracts -p octopus-sdk-session -- -D warnings` 全绿。

Step 1：
- Action：把本周新增 / 修改的所有 `pub` 符号与 `02 §2.1 / §2.2` 最终状态 diff 为零（即 §2.1/§2.2 完整描述 = 代码 `pub` 面）。补 `PromptCacheEvent / CacheBreakpoint / CacheTtl` 签名到 §2.1。
- Done when：`rg 'pub (struct|enum|trait|fn|type|const) ' crates/octopus-sdk-contracts/src crates/octopus-sdk-session/src` 的符号集合 ⊆ `02 §2.1 / §2.2` 登记的符号集合。
- Verify：作者自核对 + Checkpoint 记录 diff 行数。
- Stop if：实际代码出现 `02` 未登记的 `pub` 符号 → 要么在 `02` 登记，要么收回 `pub`；二选一必须在本 Task 内闭环。

Step 2：
- Action：与 `contracts/openapi/src/**` 对齐——逐项列出 W1 新增 contracts 与 openapi 的字段差异（如 `Usage` 的 `cache_*_input_tokens` vs openapi 侧命名、`ContentBlock` 的 `tool_use_id` vs openapi 等），按 `02-crate-topology.md §5` 既有列（`# / 日期 / 来源 / 目标 / 字段/枚举 / 差异描述 / 处理方式 / 状态`）逐行追加。
- Done when：§5 至少新增一行非占位数据，覆盖所有 W1 新类型与 openapi 的交集，不另起第二套表头。
- Verify：`rg -n 'align-openapi|align-sdk|dual-carry|no-op' docs/plans/sdk/02-crate-topology.md`
- Stop if：openapi 侧根本无对应字段，但业务层前端/后端已使用它 → Stop #3。

Step 3：
- Action：执行 `01-ai-execution-protocol.md §4 Weekly Gate` 全部勾选；与 `00-overview.md §3 W1` 的出口状态 / 硬门禁逐条对齐；更新本文件"任务状态 + Checkpoint + 变更日志"；把 `README.md` 的 W1 行状态从 `draft` 改为 `done`。
- Done when：
  - `cargo test -p octopus-sdk-contracts -p octopus-sdk-session` 全绿；
  - `cargo clippy -p octopus-sdk-contracts -p octopus-sdk-session -- -D warnings` 全绿；
  - `find crates/octopus-sdk-{contracts,session} -type f -name '*.rs' -size +800c` 为空（单文件行数硬约束；注意 `-size` 单位；若用 wc -l 核对则以 800 行为上限）；
  - `rg 'config_snapshot_id' crates/octopus-sdk-session/src/` 至少 3 处（`snapshot` + `append` + 契约测试）。
- Verify：执行上述 4 条命令。
- Stop if：任一硬门禁失败 → Weekly Gate 未通过；W1 保持 `in_progress`，不切 W2。

Notes：
- Step 3 是本 Plan 的唯一出口；未执行不得声明本周完成。

---

## Batch Checkpoint Format

执行时按 `PLAN_TEMPLATE.md §Batch Checkpoint Format` 追加到本文件末尾（§Checkpoints 小节之后），每个批次一条。

## Checkpoints

（执行期间在此追加）

---

## Exit State 对齐表（与 `00-overview.md §3 W1`）

| `00-overview.md §3 W1` 出口状态 | 本 Plan 对应 Task | 验证方式 |
|---|---|---|
| `octopus-sdk-contracts` 导出全部 IR 类型 | T2 / T3 / T5 | `02 §2.1` 签名集 ⊆ `cargo doc` 生成的 `pub` 符号集 |
| `octopus-sdk-session` 提供 `SessionStore` trait + `SqliteJsonlSessionStore` | T6 / T7 | `cargo test -p octopus-sdk-session` 绿 |
| 单元测试验证 append / stream / snapshot | T7 Step 3 | `tests/sqlite_jsonl.rs` 三个核心用例 |
| `config_snapshot_id` + `effective_config_hash` 首事件契约测试 | T8 Step 2 | `tests/contract_session_started.rs` |
| 00 硬门禁 1（`cargo test -p ...` 全绿） | T9 Step 3 | 执行并记录 |
| 00 硬门禁 2（与 OpenAPI 对齐并登记 §5） | T9 Step 2 | `02 §5` 契约差异表存在 |

---

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-20 | 首稿（9 个 Task、Exit State 对齐表、Risks R1–R5） | Architect |
