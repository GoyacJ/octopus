# 03 · 质量闸门（Quality Gates）

> 状态：Accepted · 本文是 PR 合入的**最终判定标准**
> 适用对象：Codex AI（自检）+ CI（自动）+ Reviewer（手工）

---

## 1. 5 道闸门概览

每个 PR 必须**全部通过**以下 5 道闸门才允许 merge：

| 闸门 | 名称 | 工具 | 自动化 | 阻断级别 |
|---|---|---|:---:|---|
| **G1** | 格式与编译 | `cargo fmt` + `cargo check` | ✓ | hard |
| **G2** | 静态检查 | `cargo clippy -- -D warnings` | ✓ | hard |
| **G3** | 测试 | `cargo test` + contract-test | ✓ | hard |
| **G4** | 依赖治理 | `cargo deny check` + `cargo audit` | ✓ | hard |
| **G5** | SPEC 一致性 | grep 模板 + 人类 review | 半 | hard |

> "hard" = 失败必须 reject；"soft" = 失败可豁免（需 reviewer 加签）

---

## 2. G1 · 格式与编译

### 2.1 命令

```bash
# 格式
cargo fmt --all -- --check

# 编译（按 PR 修改的 crate 范围）
cargo check -p octopus-harness-<crate> --features <flags>

# 默认 feature 编译（兜底）
cargo check --workspace --all-targets

# all-features 编译（防 feature 组合冲突）
cargo check --workspace --all-features
```

### 2.2 通过判据

- `cargo fmt --check` 退出码 0
- `cargo check` 在 default + all-features + 当前 PR 启用 features 三种矩阵下都退出码 0
- 编译告警视为失败（`-D warnings`）

### 2.3 常见失败 → 处理

| 失败 | 原因 | 处理 |
|---|---|---|
| 未运行 `cargo fmt` | AI 没自检 | reset 重跑 |
| `cargo check` failed: cyclic dependency | 违反 D2 §3 模块边界 | reset；检查 use 路径 |
| `cargo check --all-features` 失败 | feature 组合矩阵未对齐 | 修 `Cargo.toml [features]` |

---

## 3. G2 · 静态检查（Clippy）

### 3.1 命令

```bash
cargo clippy -p octopus-harness-<crate> --all-targets --all-features -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings
```

### 3.2 lint 配置（已固化在根 `Cargo.toml`）

- `[workspace.lints.rust]`：`unsafe_code = "forbid"`
- `[workspace.lints.clippy]`：`all = warn`、`pedantic = warn`，并允许约 30 项常见的"非问题"lint

参见根 `Cargo.toml` L64-L107（已存在），新 crate 必须继承 `[lints]` workspace = true。

### 3.3 严禁豁免的 lint

以下 lint 一律不允许 `#[allow(...)]` 豁免：

- `unsafe_code`（架构层 P1 内核纯净）
- `clippy::dbg_macro`
- `clippy::print_stderr` / `clippy::print_stdout`（SDK 层禁直接打印）
- `clippy::todo` / `clippy::unimplemented`（任务卡完成态不允许遗留）

### 3.4 可豁免的 lint（需 reviewer 加签）

- `clippy::too_many_lines`（已 `allow`）
- `clippy::type_complexity`（已 `allow`）
- 其它 pedantic 类（按场景）

---

## 4. G3 · 测试

### 4.1 命令

```bash
# 单 crate 测试
cargo test -p octopus-harness-<crate> --features <flags>

# workspace 全测试（每里程碑结束）
cargo test --workspace --all-features --no-fail-fast

# 文档测试
cargo test --doc -p octopus-harness-<crate>
```

### 4.2 测试类型矩阵

| 类型 | 位置 | 必须性 | 工具 |
|---|---|---|---|
| **单元测试** | `src/<module>.rs` 内 `#[cfg(test)] mod tests` | 每模块 ≥ 1 | std |
| **集成测试** | `tests/*.rs` | 每 crate ≥ 1 | std |
| **Contract test** | `tests/contract.rs` | 每 trait ≥ 1 | 见 §4.3 |
| **Mock impl** | `src/mock.rs` 或 `src/testing.rs` | 每 trait ≥ 1 | `#[cfg]` 门控 |
| **Doctest** | `///` 注释中的 `///`-fenced code block | 公开 API 必须 | std |
| **Property test** | `tests/proptest_*.rs` | 选做（推荐 ID/Schema 类）| `proptest` |

### 4.3 Contract Test 规范

**Contract Test = 验证任意实现都满足 trait 文档约束的测试**。模板：

```rust
// crates/octopus-harness-permission/tests/contract.rs

use octopus_harness_permission::*;

fn run_contract_tests<B: PermissionBroker>(broker: B) {
    fail_closed_default(&broker);
    permission_context_required(&broker);
    no_state_across_calls(&broker);
}

#[test] fn contract_direct_broker() {
    let broker = DirectBroker::new(|_, _| async { Decision::Allow });
    run_contract_tests(broker);
}

#[test] fn contract_stream_broker() {
    let broker = StreamBasedBroker::new(/* ... */);
    run_contract_tests(broker);
}

#[test] fn contract_mock_broker() {
    let broker = MockBroker::default();
    run_contract_tests(broker);
}
```

**强制规则**：

- 每个 trait 至少 3 个 contract-test 子用例（fail-closed / 关键不变量 / 无状态）
- 所有实现（含 builtin + business + mock）必须接入同一组 contract-test
- ADR-012 规定的 testing-boundary：mock 必须验证"业务实现失败时 SDK 行为可观察"

### 4.4 通过判据

- 全部测试通过（含 doctest）
- 测试覆盖率：每个 crate 整体 ≥ 60%（建议 80%+，但不强制）
- 关键 trait（PermissionBroker / EventStore / SandboxBackend / ModelProvider / MemoryProvider）覆盖率 ≥ 90%

> 覆盖率工具：`cargo llvm-cov` 或 `cargo tarpaulin`（CI 二选一）

---

## 5. G4 · 依赖治理

### 5.1 cargo deny 配置

仓库根目录维护 `deny.toml`，强制：

```toml
[graph]
all-features = true
targets = ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"]

[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Unicode-DFS-2016", "Zlib"]
copyleft = "deny"

[bans]
multiple-versions = "warn"
# cargo-deny [bans] 只能禁 crate（不是 std 类型）。
# `std::sync::Mutex` / `std::sync::RwLock` 阻塞型 Mutex 在异步中是反模式，
# 由 `[workspace.lints.clippy] disallowed_types` 在编译期拒绝（见根 Cargo.toml），
# 并由 scripts/spec-consistency.sh 在 CI 期 grep 兜底。
deny = []
```

> `[bans] deny` 中**不要**写 `{ name = "std-sync-mutex" }` 这类条目 —— cargo-deny 不识别 std 类型，会变成静默无效；此前任务卡模板曾有该错误，**已废弃**。

### 5.2 命令

```bash
cargo deny check
cargo deny check bans
cargo deny check advisories
cargo deny check licenses
cargo deny check sources
```

### 5.3 D2 §10 例外登记的 feature 矩阵（权威集中地）

> **本节是 cargo-deny 例外矩阵的唯一权威来源**（实施前评估 P2-3）。
> 任务卡 / CI workflow / `deny.toml` 应**仅引用本节行号**，不复述命令；
> 如新增破窗 → 必须先开 ADR 修订 D2 §3.7 / §10，再回流到本节。

每次 PR 涉及新 feature 启用 → CI 必须跑：

```bash
# Default
cargo deny check --features default

# 已登记的破窗 feature 单开
cargo deny check --features auto-mode
cargo deny check --features redactor
cargo deny check --features subagent-tool

# 全开
cargo deny check --all-features
```

每条对应 D2 §3.7 / §10 的破窗登记。

**新增破窗的流程**（治理硬约束）：

1. 在 `module-boundaries.md` §10 例外登记表新增一行（含 ADR 链接）
2. 在 `module-boundaries.md` §3.7 feature 触发依赖附录登记
3. 在本节 §5.3 命令矩阵新增 `cargo deny check --features <new>` 行
4. 在对应 CI workflow（`03-quality-gates §7`）matrix 段加 entry
5. 在所有相关任务卡更新"feature 矩阵"段（仅引用本节行号）

### 5.4 通过判据

- `cargo deny check --all-features` 退出码 0
- 任何新增 dependency 必须在 `deny.toml [bans] allow` 表中登记
- 任何新增 license 必须 review 后追加到 `[licenses] allow`

---

## 6. G5 · SPEC 一致性

### 6.1 自动 grep 模板（AI 自检 · 文本层）

每张任务卡的"SPEC 一致性自检"段会列出**针对该卡**的 grep 命令。通用模板：

```bash
# 1. trait 签名一致性（关键 trait 的方法签名必须与 SPEC 一致）
grep -E '^\s*(async )?fn (decide|infer|append|exec|recall)' crates/octopus-harness-*/src/**/*.rs

# 2. 错误类型未自定义新族
! grep -E 'enum (My|Custom)?(Tool|Sandbox|Model)Error' crates/octopus-harness-*/src/**/*.rs | grep -v 'use harness_contracts'

# 3. 无 std::sync::Mutex / RwLock（D2 §4.1 硬禁止；clippy::disallowed_types 兜底）
! grep -E 'std::sync::(Mutex|RwLock)' crates/octopus-harness-*/src/**/*.rs

# 4. 无 unsafe（workspace.lints 已 forbid，二次确认）
! grep -E '^\s*unsafe ' crates/octopus-harness-*/src/**/*.rs

# 5. 无 UI 类型暴露（D2 §4.1 + ADR-002）
! grep -E '(React|Tauri|egui|ratatui|crossterm)' crates/octopus-harness-*/src/**/*.rs
```

> grep 不能抓到的(别名导入 / 间接依赖 / feature 触发依赖 / 跨 crate 反向依赖)由 §6.2 依赖图脚本兜底。

### 6.2 依赖图与边界检查（CI 自检 · 结构层）

文本 grep 不能覆盖以下场景：

| 场景 | 文本 grep 漏检 | 用何工具 |
|---|---|---|
| `use foo::Foo as Bar` 别名导入 | 漏 | cargo metadata 解析 + crate 名维度白名单 |
| `feature = "X"` 触发的 dep | 漏（grep 看 use 不看 features 矩阵）| feature 矩阵脚本 |
| 间接依赖（A → B → C，C 是黑名单）| 漏 | cargo tree --duplicates / cargo-depgraph |
| L1 反向依赖 L3 | 漏 | 边界白名单 + cargo metadata |

补 3 个脚本（在 M0-T06 落地，与 spec-consistency.sh 同级）：

```bash
# scripts/dep-boundary-check.sh
# 解析 cargo metadata，验证每个 crate 的依赖列表都在 D2 §3 / §10 白名单中
cargo metadata --format-version 1 --no-deps \
    | jq '.packages[] | select(.name | startswith("octopus-harness-")) | {name, deps: [.dependencies[].name]}' \
    | python3 scripts/check_layer_boundaries.py

# scripts/feature-matrix.sh
# 跑 D10 所有 typical profile 组合的 cargo check
cargo check --workspace
cargo check --workspace --features "sqlite-store,local-sandbox,interactive-permission,provider-anthropic"
cargo check --workspace --features all-providers
cargo test --workspace --features all-providers
cargo check --workspace --all-features

# scripts/depgraph-snapshot.sh
# 生成依赖图 SVG 并与 D2 §5 期望图比对（MD5 / 图同构）
test -f docs/architecture/harness/expected-depgraph.dot || {
    echo "FAIL: missing docs/architecture/harness/expected-depgraph.dot"
    exit 1
}
cargo depgraph --workspace-only > target/depgraph.dot
diff <(sort target/depgraph.dot) <(sort docs/architecture/harness/expected-depgraph.dot) || {
    echo "FAIL: 依赖图与 D2 §5 不一致"
    exit 1
}
```

PR 流水线必须接入这 3 个脚本（见 §7.1 matrix.deny / boundary 段）。

### 6.2 人类 reviewer checklist

每个 PR 必须由人类 reviewer 勾选：

```markdown
- [ ] SPEC 锚点已读，trait 签名 / 字段 / 枚举与 SPEC 完全一致
- [ ] 关键不变量已实现（按任务卡列出）
- [ ] 无禁止行为（按任务卡列出）
- [ ] 测试覆盖正向 + 反向用例
- [ ] Contract test 接入（如有 trait）
- [ ] PR diff ≤ 500 行（含测试）
- [ ] 任务卡 ID 在 PR title
- [ ] 5 道闸门日志全部贴在 PR description
```

### 6.3 失败处理

| 类型 | 处理 |
|---|---|
| AI 输出与 SPEC 不一致 | reset 任务卡，重发 |
| AI 发现 SPEC bug 并标记 `[SPEC-CLARIFY-REQUIRED]` | maintainer 评估：开 ADR or 直接修订 SPEC，本任务卡延期 |
| reviewer 发现 SPEC bug（AI 未发现） | 同上 + 任务卡 reset 重发 |

---

## 7. CI 流水线（GitHub Actions）

### 7.1 PR 流水线

```yaml
on: pull_request
jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - run: cargo fmt --all -- --check

  check:
    strategy:
      matrix:
        profile: [default, all-providers, all-features, typical]
    steps:
      - run: |
          case "${{ matrix.profile }}" in
            default) cargo check --workspace ;;
            all-providers) cargo check --workspace --features all-providers ;;
            all-features) cargo check --workspace --all-features ;;
            typical) cargo check --workspace --features "sqlite-store,local-sandbox,interactive-permission,provider-anthropic" ;;
          esac

  clippy:
    steps:
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    strategy:
      matrix:
        profile: [default, all-providers, all-features, testing]
    steps:
      - run: |
          case "${{ matrix.profile }}" in
            default) cargo test --workspace ;;
            all-providers) cargo test --workspace --features all-providers ;;
            all-features) cargo test --workspace --all-features ;;
            testing) cargo test --workspace --features testing ;;
          esac

  deny:
    steps:
      - run: cargo deny check

  spec-grep:
    steps:
      - run: bash scripts/spec-consistency.sh

  coverage:
    if: github.event.pull_request.draft == false
    steps:
      - run: cargo llvm-cov --workspace --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v4
```

### 7.2 nightly 流水线（每日跑全量）

```yaml
on:
  schedule: [{ cron: '0 16 * * *' }]
jobs:
  full-matrix:
    strategy:
      matrix:
        # 16 种 feature 组合（含全 provider / 全 sandbox / 全 mcp transport）
    steps:
      - run: cargo test --workspace --features ${{ matrix.features }}

  audit:
    steps:
      - run: cargo audit
      - run: cargo deny check
```

### 7.3 release 流水线

`v1.0.0-rc.x` tag 触发：

- 跑全 PR + nightly 流水线
- 跑 M9 post-spike 集成验证用例
- 生成 SDK 用户文档（`cargo doc`）
- 发布到内部 registry

---

## 8. 度量指标

| 指标 | 目标 | 工具 |
|---|---|---|
| PR 通过率（不 reset 比率）| ≥ 70% | git history |
| 任务卡平均 diff 行数 | ≤ 300 | PR diff stats |
| 测试覆盖率（关键 trait）| ≥ 90% | llvm-cov |
| 测试覆盖率（整体）| ≥ 65% | llvm-cov |
| 编译时间（default feature）| ≤ 120s（冷）| CI metrics |
| 编译时间（all-features）| ≤ 240s（冷）| CI metrics |
| `cargo deny` 警告数 | 0 | deny output |

度量周期：每里程碑结束写入 `audit/<date>-implementation-metrics.md`。

---

## 9. 索引

- **任务卡模板** → [`02-task-template.md`](./02-task-template.md)
- **执行策略** → [`00-strategy.md`](./00-strategy.md)
- **路线图** → [`01-roadmap.md`](./01-roadmap.md)
- **D2 模块边界** → [`docs/architecture/harness/module-boundaries.md`](../../architecture/harness/module-boundaries.md)
- **D10 Feature flags** → [`docs/architecture/harness/feature-flags.md`](../../architecture/harness/feature-flags.md)
