# M2 · L1 Primitives · 5 原语并行

> 状态：待启动 · 依赖：M1 完成 · 阻塞：M3
> 关键交付：5 个 L1 crate 完整可用（trait + builtin + mock + contract-test）
> 预计任务卡：25 张（每 crate 5 张）· 累计工时：AI 24 小时（5 路并行约 5 小时墙钟）+ 人类评审 12 小时
> 并行度：**5 路并行**（每 crate 一个 codex 会话）

---

## 0. 里程碑级注意事项

1. **5 路完全并行**：`model / journal / sandbox / permission / memory` 五 crate 仅依赖 L0，互相正交
2. **每路 5 张任务卡内部串行**：每 crate 内部 5 卡（trait / 默认实现 / 第二实现 / mock / contract-test）必须按序
3. **L1 间禁止互相 use**：违反 D2 §3.2，CI cargo deny 会拒绝
4. **feature 触发的破窗**：`auto-mode`（permission → model）/ `redactor`（model → observability）默认不开，本里程碑不实现
5. **必须配 mock**：每个 L1 trait 都必须有 `Mock<T>` 实现，存放于 `src/mock.rs` 并 `#[cfg(any(test, feature = "mock"))]` 门控

---

## 1. 任务卡总览（25 张）

| Crate | 任务卡 | 内容 | 并行性 |
|---|---|---|:---:|
| **model** | M2-T01 ~ T05 | trait + AnthropicProvider + Mock + AuxModel + contract-test | 与其他 4 路并行 |
| **journal** | M2-T06 ~ T10 | EventStore trait + Jsonl + Sqlite + InMemory + BlobStore | 与其他 4 路并行 |
| **sandbox** | M2-T11 ~ T15 | trait + Local + Noop + heartbeat + contract-test | 与其他 4 路并行 |
| **permission** | M2-T16 ~ T20 | DirectBroker + StreamBroker + RuleEngine + DangerousPatternLibrary | 与其他 4 路并行 |
| **memory** | M2-T21 ~ T25 | Store + Lifecycle + Memdir + ThreatScanner + contract-test | 与其他 4 路并行 |

---

## 2. 路 L1-A · `octopus-harness-model`

### M2-T01 · ModelProvider trait + 类型骨架

**SPEC 锚点**：
- `harness-model.md` §2.1（ModelProvider trait + InferContext）
- `api-contracts.md` §2.1
- `harness-contracts.md` §3.5（ModelRequest / ModelStreamEvent / ModelDescriptor）

**预期产物**：
- `src/lib.rs` 主结构
- `src/provider.rs`（ModelProvider trait + ModelRequest / ModelResponse / ModelStreamEvent / ModelDescriptor / ModelCapabilities / PromptCacheStyle / HealthStatus）
- `src/credential.rs`（CredentialSource trait + CredentialKey + CredentialValue + CredentialError）
- `src/token_counter.rs`（TokenCounter trait）
- `src/aux.rs`（AuxModelProvider trait + AuxOptions + AuxTask）
- `src/middleware.rs`（InferMiddleware trait）

**关键不变量**：
- `ModelProvider` 是 `dyn-safe`
- `CredentialKey` 必须含 `tenant_id: TenantId`（字段必填，遵循 P1-5 修订）
- `prompt_cache_style()` 默认实现返回 `PromptCacheStyle::None`

**预期 diff**：< 350 行

---

### M2-T02 · AnthropicProvider 实现（拆 2 子卡）

> **拆分理由**（实施前评估 P1-5）：原单卡含 client + streaming + cache + tokenizer + e2e tests，500 行紧。拆为 client/transport 与 cache/streaming/tokenizer 两卡。

#### M2-T02a · AnthropicClient + 重试限流

**SPEC 锚点**：
- `harness-model.md` §3.1（Anthropic 实现详情）

**预期产物**：
- `src/anthropic/mod.rs`
- `src/anthropic/client.rs`（HTTP 客户端 + retry 策略 + 限流）
- `src/anthropic/error.rs`（错误映射）
- `tests/anthropic_client.rs`（mock HTTP，不打实际 API）

**关键不变量**：
- 必须支持 `claude-3-5-sonnet-20241022 / claude-3-7-sonnet-20250219` 等模型
- 必须实现 retry + 限流（按 SPEC §3.1）

**Cargo feature**：`anthropic`

**预期 diff**：< 350 行

#### M2-T02b · SSE 流解析 + Prompt Cache 注入 + Tokenizer

**SPEC 锚点**：
- `harness-model.md` §3.1
- ADR-003（Prompt Cache style `system_and_3`）

**预期产物**：
- `src/anthropic/streaming.rs`（SSE 解析 → ModelStreamEvent）
- `src/anthropic/cache.rs`（Prompt Cache breakpoint 注入）
- `src/anthropic/tokenizer.rs`（基于 anthropic-tokenizer 或近似）
- `tests/anthropic_streaming.rs`
- `tests/anthropic_cache.rs`

**关键不变量**：
- `prompt_cache_style()` 返回 `SystemAnd3`
- `cache_breakpoints` 字段按 SPEC 注入到 ModelRequest
- `cache_creation` / `cache_read` 字段按 SPEC 解析到 UsageSnapshot

**Cargo feature**：`anthropic`

**预期 diff**：< 400 行

---

### M2-T03 · CredentialPool + CostCalculator

**SPEC 锚点**：
- `harness-model.md` §2.4（CredentialKey 多租户安全契约）
- `harness-model.md` §3.4（CostCalculator + ModelPricing）

**预期产物**：
- `src/credential_pool.rs`
- `src/cost.rs`

**关键不变量**：
- `CredentialPool::pick_strategy` 四档（fill / round / random / least）
- `CredentialKey.tenant_id` 必填，反向使用 `TenantId::SHARED` 必发 `CredentialPoolSharedAcrossTenants` 事件

**预期 diff**：< 250 行

---

### M2-T04 · MockProvider + MockCredentialSource

**SPEC 锚点**：
- ADR-012（capability-testing-boundary）

**预期产物**：
- `src/mock.rs`：`MockProvider`、`MockCredentialSource`、`ScriptedProvider`（按用户脚本回放）
- Cargo feature：`mock`

**关键不变量**：
- mock 实现必须满足 `ContractTest` 套件（M2-T05）
- mock 不允许有真实 IO（仅内存）

**预期 diff**：< 200 行

---

### M2-T04.5 · 5 个 provider feature stub（占位，对齐 D10 §2.2）

| 字段 | 值 |
|---|---|
| **依赖** | M2-T04 |
| **预期 diff** | < 200 行 |

**背景**（实施前评估 P2-1 修订）：

D10 §2.2 要求 `harness-model` 提供 `openai / anthropic / gemini / openrouter / bedrock / codex / local-llama / mock` 共 8 个 feature。M2 的 T01-T05 仅落地 `anthropic + mock`。M7-T04 builtin 模块在 `#[cfg(feature = "provider-openai")]` 等时会因 `octopus_harness_model::openai` 不存在导致编译失败。

本卡为 5 个未实现的 provider 落 feature stub（可编译的占位 module），M7 builtin re-export 不破。后续 v1.1 才填入真实现。

**SPEC 锚点**：
- `feature-flags.md` §2.2（harness-model features 列表）
- `harness-model.md` §3（provider 实现章节）

**预期产物**：

- `crates/octopus-harness-model/src/openai.rs`：feature gate `#[cfg(feature = "openai")]`，仅含一个 `pub struct OpenAiProvider;` + `impl ModelProvider for OpenAiProvider { ... }` 全 `unimplemented!("TODO(v1.1): implement OpenAI provider")`
- 同模式生成 `gemini.rs / openrouter.rs / bedrock.rs / codex.rs / local_llama.rs`
- `src/lib.rs`：5 行 `#[cfg(feature = "...")] pub mod ...;`
- `Cargo.toml`：5 个 feature 项 `openai = []` / `gemini = []` / ...（不引入真实 dep）

**关键不变量**：

- 5 个 stub provider 编译通过（feature on）
- 不引入新 dep（`reqwest / google-cloud-sdk / aws-sdk` 等留给 v1.1）
- 所有方法 `unimplemented!()`，且通过 contract test（必须显式 fail：M2-T05 contract test 加 `#[should_panic]` 用例验证 stub 正确"假装失败"）

**禁止行为**：

- 不要在本卡引入实际 HTTP 依赖
- 不要让 stub default 启用（对齐 D10 §3.5：default 集只含 anthropic）

**验收命令**：

```bash
cargo check -p octopus-harness-model --features openai
cargo check -p octopus-harness-model --features gemini
cargo check -p octopus-harness-model --features openrouter
cargo check -p octopus-harness-model --features bedrock
cargo check -p octopus-harness-model --features codex
cargo check -p octopus-harness-model --features local-llama
cargo check -p octopus-harness-model --all-features  # 不能有冲突
```

---

### M2-T05 · ModelProvider Contract Test 套件

**SPEC 锚点**：
- ADR-012
- `03-quality-gates.md` §4.3

**预期产物**：
- `tests/contract.rs`：`fn run_contract_tests<P: ModelProvider>(p: P)`
- 用例：
  - `provider_id` 非空且稳定
  - `supported_models` 至少 1 个
  - `infer` 流式产出 ≥ 1 个 event
  - `health` 默认 Healthy
  - 超时被 `cancel` 触发后立刻返回 Err
- 接入：`MockProvider / AnthropicProvider`（用 mock HTTP）

**预期 diff**：< 200 行

---

## 3. 路 L1-B · `octopus-harness-journal`

### M2-T06 · EventStore trait + 接口骨架（Redactor 装配点契约）

**SPEC 锚点**：
- `harness-journal.md` §2.1（EventStore trait + Redactor 必经管道契约 v1.8.1）
- `api-contracts.md` §5（EventStore trait）
- `api-contracts.md` §18.2（Redactor 单 method 签名 `fn redact(&str, &RedactRules) -> String`）
- `harness-observability.md` §2.5.0（Redactor 6 行挂钩点表 = **装配点（call site）契约**，不是 trait 多 method）

**ADR 锚点**：
- ADR-001（event-sourcing）

**前置任务产物**（必读 PR）：
- M1-T07 PR：`octopus-harness-contracts` `Redactor` trait（单 method，与 SPEC 一致）+ `NoopRedactor` 默认实现

**预期产物**：
- `src/store.rs`：
  - `EventStore` trait（dyn-safe + async）—— 与 `api-contracts.md §5` **逐字一致**
  - **EventStore 实现**（由 T07/T08/T09 各自提供）的构造函数必须接受 `redactor: Arc<dyn Redactor>` 参数，并把它存入实例
  - 写路径（`append / append_batch / append_with_blob`）在序列化事件**前**：把每个事件的字符串字段（如 `tool_input / message_text / error_message` 等）逐一过 `redactor.redact(field, &RedactRules::default())`，再写入存储
  - 测试默认采用 `Arc::new(NoopRedactor::default())`
- `src/projection.rs`：`Projection` trait + `SessionProjection / UsageProjection / ToolPoolProjection`
- `src/snapshot.rs`：`Snapshot` 类型 + `SnapshotStore` trait
- `src/blob.rs`：BlobStore impl 入口
- `src/version.rs`：`VersionedEventStore` 装饰器 + `Migrator` trait
- `src/retention.rs`：`RetentionEnforcer`

**关键不变量**：
- 写事件前必经 Redactor（v1.8.1 P2-7 强制）—— M2 期使用 `NoopRedactor` 占位，M5-T03 替换为 `DefaultRedactor`
- append-only：禁止 `delete / update_event`
- 必须支持 `query_after(after: EventId, limit: usize)`
- 任何 `EventStore` 实现的构造函数**必须**接受 `Arc<dyn Redactor>` 参数（编译期强制；contract-test 会验证 redact 调用次数）
- "必经管道"是装配点（call site）契约：调用方在 6 行挂钩点处把字符串过 `redact()`，**不是** Redactor trait 多 method

**禁止行为**：
- 不允许构造方法默认提供 NoopRedactor（必须由调用方显式传入）
- 不允许在 trait 内部维护 redactor field（每个实现各自持有）
- 不允许把 Redactor 当作"事件改写器"（它仅处理字符串）；事件结构本身由调用方自行裁剪

**预期 diff**：< 400 行

---

### M2-T07 · JsonlEventStore（默认 builtin）

**SPEC 锚点**：
- `harness-journal.md` §3.2（jsonl 实现）
- `AGENTS.md` Persistence Governance（runtime/events/*.jsonl）

**预期产物**：
- `src/jsonl/mod.rs`
- `src/jsonl/writer.rs`（atomic append + fsync）
- `src/jsonl/reader.rs`（streaming read）
- `tests/jsonl.rs`

**关键不变量**：
- 路径默认对齐 `runtime/events/<tenant>/<session>.jsonl`
- 单文件按 size 切片（默认 100MB）

**Cargo feature**：`jsonl`

**预期 diff**：< 350 行

---

### M2-T08 · SqliteEventStore + BlobStore（拆 2 子卡）

> **拆分理由**（实施前评估 P1-5）：sqlite store + 2 类 blob store 含测试 ≤ 500 行紧；按 EventStore vs BlobStore 拆 2 卡。

#### M2-T08a · SqliteEventStore + Migration

**SPEC 锚点**：
- `harness-journal.md` §3.3
- `AGENTS.md` Persistence Governance（data/main.db）

**预期产物**：
- `src/sqlite/mod.rs`：SqliteEventStore + Migration
- `tests/sqlite_event_store.rs`

**关键不变量**：
- WAL 模式 + FTS5 索引（按 SPEC）
- 写路径调用 `redactor.redact(...)`（对齐 M2-T06 装配点契约）

**Cargo feature**：`sqlite`

**预期 diff**：< 350 行

#### M2-T08b · FileBlobStore + SqliteBlobStore

**SPEC 锚点**：
- `harness-journal.md` §3.4
- `AGENTS.md` Persistence Governance（data/blobs）

**预期产物**：
- `src/sqlite/blob.rs`：SqliteBlobStore（小 blob）
- `src/blob_file.rs`：FileBlobStore（大 blob）
- `tests/blob.rs`

**关键不变量**：
- BlobStore 选择策略：> 1MB 走 file，否则走 sqlite（业务侧可配置）

**Cargo feature**：`blob-file / blob-sqlite`

**预期 diff**：< 350 行

---

### M2-T09 · InMemoryEventStore（testing）

**SPEC 锚点**：`harness-journal.md` §3.5

**预期产物**：
- `src/memory/mod.rs`
- `tests/memory.rs`

**Cargo feature**：`in-memory`

**预期 diff**：< 200 行

---

### M2-T10 · EventStore Contract Test + Replay 测试

**预期产物**：
- `tests/contract.rs`：3 实现（jsonl / sqlite / memory）通过同一 contract
- `tests/replay.rs`：构造一段事件流 → SessionProjection 还原 → 与原状态比对

**预期 diff**：< 250 行

---

## 4. 路 L1-C · `octopus-harness-sandbox`

### M2-T11 · SandboxBackend trait + 类型骨架

**SPEC 锚点**：`harness-sandbox.md` §2.1 / `api-contracts.md` §7

**预期产物**：
- `src/lib.rs`
- `src/backend.rs`（SandboxBackend trait + ExecRequest / ExecResponse / SandboxEvent / Heartbeat）
- `src/policy.rs`（SandboxPolicy + SandboxMode 枚举）
- `src/cwd.rs`（CWD marker FD 协议）
- `src/code_sandbox.rs`（CodeSandbox trait，预留 ADR-0016）

**关键不变量**：
- `SandboxPolicy` 不替代 PermissionBroker（HER-041 显式不采纳）
- CWD marker 用独立 FD 而非 stdout 解析

**预期 diff**：< 300 行

---

### M2-T12 · LocalSandbox（默认 builtin）

**SPEC 锚点**：`harness-sandbox.md` §3.1

**预期产物**：
- `src/local/mod.rs`
- `src/local/exec.rs`（spawn + heartbeat + timeout）
- `tests/local.rs`

**Cargo feature**：`local`

**预期 diff**：< 350 行

---

### M2-T13 · NoopSandbox + Stubs for Docker/SSH

**预期产物**：
- `src/noop.rs`（直接 reject 所有 exec，用于 testing）
- `src/docker.rs`（占位：`unimplemented!()` + TODO M5+ 完善）
- `src/ssh.rs`（占位）

**Cargo feature**：`noop / docker / ssh`

**预期 diff**：< 150 行

---

### M2-T14 · Heartbeat + DangerousPatternLibrary 默认 30+ 命令

**SPEC 锚点**：`harness-sandbox.md` §4 + §5（DangerousPatternLibrary）

**预期产物**：
- `src/dangerous.rs`：`default_unix() / default_windows() / default_all()`
- `tests/dangerous.rs`：30+ 模式正反测试

**预期 diff**：< 250 行

---

### M2-T15 · Sandbox Contract Test

**预期产物**：
- `tests/contract.rs`
- 验证 LocalSandbox / NoopSandbox 行为一致性

**预期 diff**：< 150 行

---

## 5. 路 L1-D · `octopus-harness-permission`

### M2-T16 · PermissionBroker trait + 类型骨架

**SPEC 锚点**：
- `harness-permission.md` §3（PermissionBroker / PermissionContext / PermissionRequest）
- `api-contracts.md` §8
- `permission-model.md` §5.1（v1.8.1 P0-2 修订带 PermissionContext）

**预期产物**：
- `src/lib.rs`
- `src/broker.rs`（PermissionBroker trait + PermissionRequest + PermissionContext）
- `src/decision.rs`（Decision / DecidedBy）
- `src/rule.rs`（PermissionRule / RuleProvider trait）

**关键不变量**：
- `decide(req: PermissionRequest, ctx: PermissionContext)` 必带 ctx 参数（P0-2）
- Fail-Closed 默认（broker 故障 → Deny）

**预期 diff**：< 300 行

---

### M2-T17 · DirectBroker + StreamBasedBroker

**SPEC 锚点**：`harness-permission.md` §3.1 + §3.2

**预期产物**：
- `src/direct.rs`：DirectBroker（同步回调）
- `src/stream.rs`：StreamBasedBroker（事件驱动，配合 SDK `resolve_permission`）
- `tests/direct.rs / stream.rs`

**Cargo feature**：`interactive / stream`

**预期 diff**：< 350 行

---

### M2-T18 · 4 个内置 RuleProvider + RuleEngineBroker

**SPEC 锚点**：`harness-permission.md` §4（4 个 builtin RuleProvider）

**预期产物**：
- `src/rule_engine.rs`：RuleEngineBroker
- `src/providers/file.rs`：FileRuleProvider
- `src/providers/inline.rs`：InlineRuleProvider
- `src/providers/admin.rs`：AdminRuleProvider
- `src/providers/memory.rs`：InMemoryRuleProvider
- `tests/rule_engine.rs`

**关键不变量**：
- 规则 watch + reload 必须通过 `notify` crate 实现（路径变更触发热加载，但仅生成 PermissionRuleAdded 事件）

**Cargo feature**：`rule-engine`

**预期 diff**：< 400 行

---

### M2-T19 · IntegritySigner + DangerousPatternLibrary（permission 侧）

**SPEC 锚点**：
- `harness-permission.md` §5（IntegritySigner + ADR-0013）
- `harness-permission.md` §6（DangerousPatternLibrary 与 sandbox 共用）

**预期产物**：
- `src/integrity_signer.rs`：IntegritySigner trait + StaticSignerStore
- `src/dangerous.rs`：与 sandbox 共用的 pattern library 引用（不重复定义）

**关键不变量**：
- IntegritySigner 与 ManifestSigner（plugin crate）KeyStore 完全独立（ADR-0013）

**预期 diff**：< 250 行

---

### M2-T20 · MockBroker + Permission Contract Test

**预期产物**：
- `src/mock.rs`：MockBroker（按预设序列回放 Decision）
- `tests/contract.rs`：3 broker 通过同 contract（fail-closed / context-required / no-state）

**预期 diff**：< 200 行

---

## 6. 路 L1-E · `octopus-harness-memory`

### M2-T21 · MemoryStore + MemoryLifecycle 二分 trait

**SPEC 锚点**：
- `harness-memory.md` §2（MemoryStore + MemoryLifecycle 二分，v1.4 拆分）
- `api-contracts.md` §6

**预期产物**：
- `src/lib.rs`
- `src/store.rs`（MemoryStore trait：recall / upsert / forget / list）
- `src/lifecycle.rs`（MemoryLifecycle trait：on_session_start / on_pre_compress / on_delegation / on_session_end / 7 个 hook）
- `src/types.rs`（MemoryEntry + MemoryMetadata + MemoryKind + MemoryVisibility）

**关键不变量**：
- MemoryStore 实现必须支持租户隔离
- MemoryLifecycle hook 是可选实现（默认空 impl）

**预期 diff**：< 350 行

---

### M2-T22 · Memdir 默认实现（builtin）

**SPEC 锚点**：`harness-memory.md` §3（Memdir 详细实现）

**预期产物**：
- `src/memdir/mod.rs`
- `src/memdir/file.rs`：MEMORY.md / USER.md / projects/<id>.md 文件管理
- `src/memdir/lock.rs`：跨进程 advisory lock + atomic-rename
- `src/memdir/fence.rs`：`<memory-context>` 栅栏 + escape_for_fence
- `tests/memdir.rs`

**关键不变量**：
- 写磁盘**立即生效**，但**系统提示下一 Session 才生效**（与 ADR-003 交叉）
- 跨进程 lock 必须用文件 advisory lock（`fs2` crate）

**Cargo feature**：`builtin`

**预期 diff**：< 500 行

---

### M2-T23 · MemoryThreatScanner

**SPEC 锚点**：`harness-memory.md` §4（威胁扫描 + 三档动作）

**预期产物**：
- `src/scanner.rs`：MemoryThreatScanner trait + DefaultScanner（30+ 默认正则）
- `tests/scanner.rs`

**关键不变量**：
- 默认 30+ 模式（含 prompt injection / credential leak / SSRF / shell injection）
- 三档动作：Warn / Redact / Block

**Cargo feature**：`threat-scanner`

**预期 diff**：< 300 行

---

### M2-T24 · External MemoryProvider Slot

**SPEC 锚点**：`harness-memory.md` §5（外部 provider slot）

**预期产物**：
- `src/external.rs`：ExternalMemoryProvider trait + 注册机制
- `src/mock.rs`：MockMemoryProvider

**Cargo feature**：`external-slot`

**预期 diff**：< 200 行

---

### M2-T25 · Memory Contract Test + Recall 编排

**预期产物**：
- `tests/contract.rs`：跨 Memdir / External / Mock 一致性
- `tests/recall.rs`：每轮至多 1 次 + fail-safe 默认行为

**预期 diff**：< 250 行

---

## 7. M2-Spike-1 · Anthropic Prompt Cache 命中率（前置 POC）

### M2-S01 · Spike-1 · Anthropic Prompt Cache 命中率最小验证

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M2-T05（Anthropic provider + contract test 完成） |
| **预期 diff** | < 250 行 |
| **预期工时** | AI 1.5h + 人类评审 30min |

**背景**（实施前评估 P1-3 修订）：

评审报告 §4.4 第 1 项指出 Prompt Cache 假设是高风险设计，POC 失败会触发 ADR-003 重设计（2-4 周返工）。M2 完成 Anthropic provider 后即可做最小 spike：在 contracts + model 两层验证 `system_and_3` cache breakpoint 注入正确性 + 真实 API 命中率。**不**等到 M9 才发现假设不成立。

**SPEC 锚点**：
- `crates/harness-model.md` §3.1（Anthropic 实现）
- ADR-003（Prompt Cache `system_and_3`）
- `audit/2026-04-25-architecture-review.md` §4.4 第 1 项

**预期产物**：
- `crates/octopus-harness-model/tests/spike_prompt_cache.rs`：
  - mock SSE 路径验证 `cache_breakpoints` 字段被正确注入到 `ModelRequest`
  - `cache_creation` / `cache_read` 字段被正确解析到 `UsageSnapshot`
  - `#[ignore] live` 用例：连接真实 Anthropic（如有 `ANTHROPIC_API_KEY`）跑 3 轮对话，验证第 2-3 轮 `cache_read > 0`
- `docs/architecture/harness/audit/M2-spike-prompt-cache.md`（人类填写实测报告）

**通过判据**：
- ✅ Mock 路径：cache breakpoint 注入次数 = 期望、cache_read/creation 字段解析正确
- ⚠️ Live 路径（应跑）：第 2 轮 `cache_read > 0`；如限流则记 partial-pass

**失败处理**：
- Mock 失败 → AnthropicProvider 实现 bug，M2-T02 reset
- Live 全失败 → ADR-003 假设受质疑，召开架构 review；spike 报告归档供 M9 完整 POC 比对

---

## 8. M2 Gate 检查

完成后须通过以下检查（人类 reviewer）：

- ✅ 5 crate 各自 `cargo test --all-features` 全绿
- ✅ 5 crate 各自 contract-test 至少 3 个用例
- ✅ `cargo deny check --features auto-mode`（破窗 feature）通过
- ✅ M0 设置的 spec-consistency 脚本继续通过
- ✅ feature 矩阵 CI（含 `interactive,stream,rule-engine` 等组合）全绿
- ✅ **M2-S01 spike 通过**（mock 必跑、live 应跑）

未全绿 → 不得开始 M3。

---

## 9. 索引

- **上一里程碑** → [`M1-l0-contracts.md`](./M1-l0-contracts.md)
- **下一里程碑** → [`M3-l2-core.md`](./M3-l2-core.md)
