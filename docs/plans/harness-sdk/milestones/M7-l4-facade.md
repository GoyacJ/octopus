# M7 · L4 Facade · `octopus-harness-sdk` 门面收口

> 状态：待启动 · 依赖：M4 + M5 + M6 全部完成 · 阻塞：M8
> 关键交付：业务层唯一门面 + Builder type-state + prelude / builtin / ext / testing 四模块
> 预计任务卡：6 张 · 累计工时：AI 8 小时 + 人类评审 6 小时
> 并行度：1（串行，因 sdk 是 single-writer）

---

## 0. 里程碑级注意事项

1. **本里程碑后业务层应仅依赖 `octopus-harness-sdk`**（除 `octopus-harness-contracts`）
2. **HarnessBuilder type-state**：缺必填依赖编译失败（核心亮点保护，不可弱化）
3. **SessionBuilder 不走 type-state**：v1.8.1 P2-4 修订，运行时 Result 校验
4. **feature flag 必须与 D10 完全对齐**：包括 default 集合
5. **prelude / builtin / ext / testing 四个模块必须独立**：用户清晰的 import 路径

---

## 1. 任务卡总览

| ID | 任务 | 依赖 | diff 上限 |
|---|---|---|---|
| M7-T01 | sdk crate Cargo.toml + features 矩阵（D10 §2.1）| M4-M6 | < 200 |
| M7-T02 | Harness + HarnessBuilder type-state | T01 | < 400 |
| M7-T03 | prelude + ext 模块 | T02 | < 250 |
| M7-T04 | builtin 模块（feature-gated re-export）| T02 | < 350 |
| M7-T05 | testing 模块（mock 汇总）| T02 | < 200 |
| M7-T06 | M7 Gate + 文档输出 | T01-T05 | < 100 |

---

## 2. 任务卡详情

### M7-T01 · `octopus-harness-sdk/Cargo.toml` 完整化

**SPEC 锚点**：
- `harness-sdk.md` §2-§3
- `feature-flags.md` §2.1

**预期产物**：
- `crates/octopus-harness-sdk/Cargo.toml`：
  - `[features]` 段与 `feature-flags.md` §2.1 完全一致（约 50 个 feature）
  - default 集合：`sqlite-store / jsonl-store / local-sandbox / interactive-permission / mcp-stdio / provider-anthropic / tool-search / steering-queue`
  - 所有 feature 触发的 dep:* 必须在根 `deny.toml` 与 D2 §10 例外表登记

**关键不变量**：
- default 集合与 D10 §2.1 字面量一致
- 所有 feature 都有对应的内部 crate feature 转发

**预期 diff**：< 200 行

---

### M7-T02 · Harness + HarnessBuilder type-state

**SPEC 锚点**：
- `harness-sdk.md` §4-§5
- `overview.md` §6.1（Builder Type-State）

**预期产物**：
- `src/lib.rs`
- `src/harness.rs`：Harness struct + HarnessOptions
- `src/builder.rs`：HarnessBuilder<ModelState, StoreState, SandboxState>
  - `pub struct Unset; pub struct Set<T>(T);`
  - `with_model<M: ModelProvider>(self, m: M) -> HarnessBuilder<Set<M>, S, SB>`
  - `with_store<S: EventStore>(self, s: S) -> ...`
  - `with_sandbox<SB: SandboxBackend>(self, sb: SB) -> ...`
  - `impl<M: ModelProvider, S: EventStore, SB: SandboxBackend> HarnessBuilder<Set<M>, Set<S>, Set<SB>> { pub async fn build(self) -> Result<Harness> }`
- `src/error.rs`：HarnessError（再导出 contracts）
- `src/api.rs`：Harness 公开方法
  - `create_workspace`
  - `create_session`
  - `create_team`
  - `resolve_permission`
  - `resolve_elicitation`
  - `enabled_features`

**关键不变量**：
- 缺 model / store / sandbox 任意一个 → `build()` 编译失败（type-state）
- 其他依赖（permission / memory / tool / hook / ...）走 SessionOptions 或 builder 链式（运行时 Result）

**预期 diff**：< 400 行

---

### M7-T03 · `prelude` + `ext` 模块

**SPEC 锚点**：`harness-sdk.md` §3 / §4

**预期产物**：
- `src/prelude.rs`：业务层最小可用面 re-export
- `src/ext.rs`：所有外部 trait 集中导出（ModelProvider / EventStore / SandboxBackend / ...）

**关键不变量**：
- prelude 必须涵盖 95% 业务场景所需类型（不必全部）
- ext 必须列出**所有**业务可能实现的 trait（业务私有 provider 入口）

**预期 diff**：< 250 行

---

### M7-T04 · `builtin` 模块（feature-gated re-export）

**SPEC 锚点**：`harness-sdk.md` §5

**预期产物**：
- `src/builtin.rs`：根据 feature flag re-export 内置实现
  - `#[cfg(feature = "provider-anthropic")] pub use octopus_harness_model::anthropic::AnthropicProvider;`
  - `#[cfg(feature = "sqlite-store")] pub use octopus_harness_journal::sqlite::SqliteEventStore;`
  - `#[cfg(feature = "local-sandbox")] pub use octopus_harness_sandbox::local::LocalSandbox;`
  - …约 30 条 re-export

**关键不变量**：
- 每条 re-export 都必须有对应 feature gate
- 编译期：仅启用 feature 的 re-export 才生效

**预期 diff**：< 350 行

---

### M7-T05 · `testing` 模块（mock 汇总）

**SPEC 锚点**：`harness-sdk.md` §6

**预期产物**：
- `src/testing.rs`：feature `testing` 启用时汇总 mock
  - `MockProvider / MockBroker / MockMemoryProvider / MockEventStore / NoopSandbox / MockHookHandler / ...`
- 测试用例：`tests/builder_compile_fail.rs`（`compile_fail` 检查缺必填依赖）

**关键不变量**：
- testing feature 不进 production（`[features] testing = [...]` 不在 default）
- 每个 mock 必须满足对应 contract test

**Cargo feature**：`testing`

**预期 diff**：< 200 行

---

### M7-T06 · M7 Gate + 文档输出

**预期产物**：
- 修改 `crates/octopus-harness-sdk/README.md`：业务层调用示例（参照 overview.md §9）
- `cargo doc --no-deps -p octopus-harness-sdk` 干净生成
- 一份 `docs/plans/harness-sdk/audit/M7-facade-gate.md`（人类填写）

**Gate 通过判据**：
- ✅ `cargo build --workspace --release` 通过
- ✅ `cargo test -p octopus-harness-sdk --all-features` 全绿
- ✅ `cargo test -p octopus-harness-sdk --features testing` 全绿
- ✅ `cargo doc --no-deps -p octopus-harness-sdk` 无警告
- ✅ Builder type-state 编译期约束验证（`compile_fail` doctest 通过）
- ✅ `enabled_features()` 运行时返回正确
- ✅ feature 矩阵 CI（含 §3.1-§3.4 四种典型 profile）全绿
- ✅ 业务层 demo（`crates/octopus-harness-sdk/examples/quickstart.rs`）跑通：
  ```rust
  use octopus_harness_sdk::prelude::*;
  use octopus_harness_sdk::builtin::*;
  // 模仿 overview.md §9 的代码片段
  ```

未全绿 → 不得开始 M8。

---

## 3. 索引

- **上一里程碑** → [`M6-l3-agents.md`](./M6-l3-agents.md)
- **下一里程碑** → [`M8-business-cutover.md`](./M8-business-cutover.md)
- **harness-sdk SPEC** → [`docs/architecture/harness/crates/harness-sdk.md`](../../../architecture/harness/crates/harness-sdk.md)
- **D10 Feature Flags** → [`docs/architecture/harness/feature-flags.md`](../../../architecture/harness/feature-flags.md)
