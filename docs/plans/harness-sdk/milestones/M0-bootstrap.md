# M0 · Bootstrap · 工作空间脚手架与旧 SDK 冻结

> 状态：待启动 · 依赖：无 · 阻塞：所有后续里程碑
> 关键交付：旧 SDK freeze 清单 · 19 新 crate 空骨架 · 依赖边界检查 · CI 接入 · `cargo check --workspace` 绿
> 预计任务卡：12 张（T01a/b + T02a/b/c/d + T03-T08）· 预计累计工时：AI 5 小时 + 人类评审 2.5 小时
> 并行度：1（串行，因 workspace 文件互斥）

---

## 0. 里程碑级注意事项

1. **本里程碑不实现任何业务逻辑**：所有任务只动 `Cargo.toml` / `lib.rs` 占位 / CI workflow / `deny.toml` / 边界脚本 / freeze 审计文档
2. **旧 SDK 在 M0~M7 保留但冻结**：业务层继续走现有 `octopus-sdk*` 路径，直到 M8 按 server / desktop / cli 三路切到 `octopus-harness-sdk`
3. **旧 SDK freeze 规则**：
   - 14 个 `octopus-sdk*` crate 不在 M0 删除
   - 允许修编译、安全问题、CI 适配
   - 禁止新增公开 API、业务能力、持久化路径、事件类型
   - 禁止新 `octopus-harness-*` crate 依赖任何 `octopus-sdk*`
4. **删除时机**：旧 `octopus-sdk*` crate 只在 M8 业务全切并通过 Gate 后 `git rm`
5. **保留 crate**：`octopus-core / octopus-persistence / octopus-platform / octopus-infra / octopus-server / octopus-desktop / octopus-cli` 都继续保留；其中 `octopus-platform / octopus-infra` 现有旧 SDK 依赖在 M8 按需重接 `octopus-harness-sdk` 或删除
6. **新 crate 命名规则**：`octopus-harness-<x>`（路径 `crates/octopus-harness-<x>`，包名同），内部 module 用 `harness_<x>`（详见 ADR-008）

---

## 1. 任务卡清单

| ID | 任务 | 依赖 | 可并行 | diff 上限 |
|---|---|---|---|---|
| M0-T01a | 旧 SDK freeze 清单与共存边界登记 | 无 | × | < 150 |
| M0-T01b | harness / legacy 依赖边界检查脚本 | T01a | × | < 150 |
| M0-T02a | 创建 L0 `harness-contracts` 空骨架 | T01b | × | < 100 |
| M0-T02b | 创建 L1 五原语 crate 空骨架 | T02a | × | < 200 |
| M0-T02c | 创建 L2 七复合能力 crate 空骨架 | T02b | × | < 250 |
| M0-T02d | 创建 L3+L4 六 crate 空骨架 | T02c | × | < 250 |
| M0-T03 | workspace `Cargo.toml` 重组（members + dependencies）| T02d | × | < 100 |
| M0-T04 | 根 `deny.toml` 与 `cargo deny` 配置 | T03 | × | < 100 |
| M0-T05 | GitHub Actions CI workflow（fmt/check/clippy/test/deny/boundary）| T03 | × | < 200 |
| M0-T06 | SPEC 与边界自检脚本 | T03 | × | < 150 |
| M0-T07 | `cargo doc` 生成 + 仓库 README 链接更新 | T05 | × | < 50 |
| M0-T08 | M0 Gate 检查：`cargo check --workspace` 绿 + CI 首次绿 | T01a-T07 | × | 0 |

---

## 2. 任务卡详情

### M0-T01a · 旧 SDK freeze 清单与共存边界登记

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | 无 |
| **预期 diff** | < 150 行 |
| **预期工时** | AI 30 min |

**背景说明**：

本 Plan 不在 M0 删除旧 SDK。旧实现作为现有业务运行路径保留到 M8。M0 的责任是把旧 SDK 标成冻结资产，并把新 harness 的边界钉住。

**预期产物**：

- 新增 `docs/plans/harness-sdk/audit/M0-legacy-sdk-freeze.md`
- 文档列出 14 个旧 `octopus-sdk*` crate：
  - `octopus-sdk`
  - `octopus-sdk-contracts`
  - `octopus-sdk-core`
  - `octopus-sdk-model`
  - `octopus-sdk-tools`
  - `octopus-sdk-permissions`
  - `octopus-sdk-sandbox`
  - `octopus-sdk-hooks`
  - `octopus-sdk-context`
  - `octopus-sdk-session`
  - `octopus-sdk-subagent`
  - `octopus-sdk-observability`
  - `octopus-sdk-mcp`
  - `octopus-sdk-plugin`
- 文档登记 freeze 规则：
  - 允许：编译修复、安全修复、CI 适配、M8 切换所需的最小兼容修复
  - 禁止：新增公开 API、新业务能力、新持久化路径、新事件类型、新业务入口
  - 禁止：任何 `octopus-harness-*` crate 依赖旧 `octopus-sdk*`
- 文档登记删除时机：M8-T12 Gate 通过后统一 `git rm`

**关键不变量**：

- 不改旧 SDK 代码
- 不改业务入口依赖
- 不创建 `_octopus-bridge-stub`
- 不引入 `legacy-sdk` feature

**验收命令**：

```bash
test -f docs/plans/harness-sdk/audit/M0-legacy-sdk-freeze.md
ls crates/octopus-sdk* | wc -l
! test -d crates/_octopus-bridge-stub
```

---

### M0-T01b · harness / legacy 依赖边界检查脚本

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T01a |
| **预期 diff** | < 150 行 |
| **预期工时** | AI 45 min |

**预期产物**：

- 新增 `scripts/harness-legacy-boundary.sh`
- 脚本验证：
  - `octopus-harness-*` 的 `Cargo.toml` 不得依赖任何 `octopus-sdk*`
  - `octopus-harness-*` 的 Rust 源码不得 `use octopus_sdk*`
  - 业务 crate 在 M0~M7 可以继续依赖旧 SDK
  - 旧 SDK crate 在 M0~M7 可以存在
- 脚本接入 M0-T05 CI

**脚本核心检查**：

```bash
! grep -rE 'octopus-sdk|octopus_sdk' crates/octopus-harness-* --include='Cargo.toml' --include='*.rs'
! grep -rE '_octopus[-_]bridge[-_]stub|legacy-sdk' crates/ apps/ --include='Cargo.toml' --include='*.rs'
```

**关键不变量**：

- 该脚本只约束新 harness，不要求业务层在 M0 清空旧 SDK 引用
- 旧 SDK 的删除检查放到 M8-T12

**验收命令**：

```bash
bash scripts/harness-legacy-boundary.sh
```

---

### M0-T02a-d 共享模板 · 创建 19 个新 `octopus-harness-*` crate 空骨架

> **拆分理由**（实施前评估 P1-5）：原单卡 ~600 行违反 00-strategy 铁律 2 ≤ 500 行硬上限。按 5 层依赖拆 4 子卡，每卡 ≤ 250 行。

| 子卡 ID | 创建的 crate | 数量 | 依赖 | 预期 diff |
|---|---|:---:|---|---|
| M0-T02a | L0：`harness-contracts` | 1 | M0-T01b | < 100 |
| M0-T02b | L1：`harness-{model, journal, sandbox, permission, memory}` | 5 | T02a | < 200 |
| M0-T02c | L2：`harness-{tool, tool-search, skill, mcp, hook, context, session}` | 7 | T02b | < 250 |
| M0-T02d | L3+L4：`harness-{engine, subagent, team, plugin, observability, sdk}` | 6 | T02c | < 250 |

**SPEC 锚点**：
- `docs/architecture/harness/overview.md` §4.1（19 crate 清单）
- `docs/architecture/harness/module-boundaries.md` §3（依赖白名单）
- `docs/architecture/harness/feature-flags.md` §2.2（每 crate 自身 features）

**ADR 锚点**：ADR-008（crate-layout）

**预期产物**：

```text
crates/octopus-harness-contracts/
├── Cargo.toml
├── src/lib.rs
└── README.md

crates/octopus-harness-model/
├── Cargo.toml
├── src/lib.rs
└── README.md

[…依此类推 19 个 crate…]
```

**`Cargo.toml` 模板**（以 `octopus-harness-permission` 为例）：

```toml
[package]
name = "octopus-harness-permission"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false
rust-version.workspace = true

[lints]
workspace = true

[features]
default = []
interactive = []
stream = []
"rule-engine" = []
mock = []
"auto-mode" = ["dep:octopus-harness-model"]   # D2 §10 例外破窗

[dependencies]
octopus-harness-contracts = { path = "../octopus-harness-contracts" }
serde = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
octopus-harness-model = { path = "../octopus-harness-model", optional = true }
```

**`src/lib.rs` 模板**：

```rust
//! `octopus-harness-permission`
//!
//! 权限模型 + DirectBroker + StreamBroker + 规则引擎。
//!
//! SPEC: docs/architecture/harness/crates/harness-permission.md
//! 状态：M0 空骨架；具体实现见 M2 任务卡。

#![cfg_attr(not(any(test, feature = "mock")), forbid(unsafe_code))]
```

**关键不变量**：

- 19 个 crate 全部生成
- 每个 crate `Cargo.toml` 的依赖必须符合 D2 §3 白名单
- 每个 crate `Cargo.toml` 的 features 必须与 `feature-flags.md` §2.2 一致
- 每个 crate `lib.rs` 必须包含 SPEC 文档路径注释
- 新 harness 不得依赖旧 `octopus-sdk*`

**禁止行为**：

- 不要在本卡实现具体逻辑
- 不要新增 D2 / feature-flags 之外的 dependency
- 不要从 `octopus-sdk*` 复制实现代码到新 harness

**验收命令**：

```bash
ls -d crates/octopus-harness-* | wc -l   # 应输出 19
cargo check --workspace
cargo doc --no-deps --workspace
bash scripts/harness-legacy-boundary.sh
```

#### M0-T02a · 创建 L0 `harness-contracts` 空骨架

**依赖**：M0-T01b

**预期产物**：按本节共享模板创建 `crates/octopus-harness-contracts/`。

**验收命令**：

```bash
test -d crates/octopus-harness-contracts
cargo check -p octopus-harness-contracts
```

#### M0-T02b · 创建 L1 五原语 crate 空骨架

**依赖**：M0-T02a

**预期产物**：按本节共享模板创建 `model / journal / sandbox / permission / memory` 五个 crate。

**验收命令**：

```bash
for c in model journal sandbox permission memory; do
    test -d "crates/octopus-harness-$c"
done
cargo check -p octopus-harness-model -p octopus-harness-journal -p octopus-harness-sandbox -p octopus-harness-permission -p octopus-harness-memory
```

#### M0-T02c · 创建 L2 七复合能力 crate 空骨架

**依赖**：M0-T02b

**预期产物**：按本节共享模板创建 `tool / tool-search / skill / mcp / hook / context / session` 七个 crate。

**验收命令**：

```bash
for c in tool tool-search skill mcp hook context session; do
    test -d "crates/octopus-harness-$c"
done
cargo check -p octopus-harness-tool -p octopus-harness-tool-search -p octopus-harness-skill -p octopus-harness-mcp -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session
```

#### M0-T02d · 创建 L3+L4 六 crate 空骨架

**依赖**：M0-T02c

**预期产物**：按本节共享模板创建 `engine / subagent / team / plugin / observability / sdk` 六个 crate。

**验收命令**：

```bash
for c in engine subagent team plugin observability sdk; do
    test -d "crates/octopus-harness-$c"
done
cargo check -p octopus-harness-engine -p octopus-harness-subagent -p octopus-harness-team -p octopus-harness-plugin -p octopus-harness-observability -p octopus-harness-sdk
```

---

### M0-T03 · workspace `Cargo.toml` 重组

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T02d |
| **预期 diff** | < 100 行 |

**预期产物**：

- `[workspace] members` 增加 19 个新 `octopus-harness-*` crate
- `[workspace] default-members` 增加 19 个新 crate
- 保留现有 `octopus-sdk*` members，直到 M8 删除
- `[workspace.dependencies]` 增加新 harness 会用到的共享依赖

**关键不变量**：

- workspace.members 顺序：现有保留 crate → 旧 `octopus-sdk*` crate → 19 个 harness crate
- 不要改变 `[workspace.lints]`
- 现有 `[workspace.dependencies]` 中的版本不要变更，除非 M0-T04/M0-T05 需要

**验收命令**：

```bash
cargo metadata --format-version 1 >/tmp/octopus-metadata.json
cargo check --workspace
bash scripts/harness-legacy-boundary.sh
```

---

### M0-T04 · 根 `deny.toml` 与 `cargo deny` 配置

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T03 |
| **预期 diff** | < 100 行 |

**预期产物**：

- 创建 `deny.toml`（参考 `03-quality-gates.md` §5.1 模板）
- 创建 `.github/workflows/deny.yml`（PR + push 触发）
- cargo deny feature 组合覆盖默认、typical、all-providers、all-features

**关键不变量**：

- `[advisories] vulnerability = "deny"`
- 新 license 必须由 reviewer 追加到 allowlist
- `cargo-deny` 不负责识别新旧 SDK 边界；边界由 `scripts/harness-legacy-boundary.sh` 和 dep-boundary 脚本处理

**验收命令**：

```bash
cargo deny check
```

---

### M0-T05 · GitHub Actions CI workflow

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T03 |
| **预期 diff** | < 200 行 |

**预期产物**：

- `.github/workflows/ci.yml`（PR 流水线：fmt / check / clippy / test / coverage / boundary）
- `.github/workflows/nightly.yml`（每日全 feature 矩阵）
- `.github/workflows/release.yml`（占位，M9 完善）

**关键不变量**：

- `bash scripts/harness-legacy-boundary.sh` 必须进入 PR 阻断检查
- `cargo check --workspace` 在 M0~M7 包含旧 SDK 与新 harness
- 不要求业务层在 M0 清空旧 SDK 引用

---

### M0-T06 · SPEC 与边界自检脚本

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T03 |
| **预期 diff** | < 150 行 |

**预期产物**：

- `scripts/spec-consistency.sh`
- `scripts/feature-matrix.sh`
- `scripts/dep-boundary-check.sh` + `scripts/check_layer_boundaries.py`
- `scripts/depgraph-snapshot.sh`
- `docs/architecture/harness/expected-depgraph.dot`

**关键不变量**：

- `spec-consistency.sh` 检查 trait 签名、错误类型、`std::sync::Mutex`、`unsafe`、UI 类型
- `dep-boundary-check.sh` 检查 harness 层级依赖白名单
- `harness-legacy-boundary.sh` 检查新 harness 不依赖旧 SDK
- 4 个脚本可独立运行

**验收命令**：

```bash
bash scripts/spec-consistency.sh
bash scripts/feature-matrix.sh
bash scripts/dep-boundary-check.sh
bash scripts/depgraph-snapshot.sh
bash scripts/harness-legacy-boundary.sh
```

---

### M0-T07 · `cargo doc` 与仓库 README 更新

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T05 |
| **预期 diff** | < 50 行 |

**预期产物**：

- 修改根 `README.md`：增加 SDK 链接段，指向 `docs/architecture/harness/` + `docs/plans/harness-sdk/`
- 添加 `cargo doc --no-deps --workspace` 到 CI（产物可选发布到内部 doc 站）

---

### M0-T08 · M0 Gate 检查

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | T01a-T07 |
| **预期 diff** | 0（仅验证）|

**预期产物**：

- 一份 `docs/plans/harness-sdk/audit/M0-bootstrap-gate.md`（**人类**填写）：
  - 19 crate 列表
  - 14 个旧 `octopus-sdk*` crate freeze 清单链接
  - `cargo check --workspace --all-features` 输出
  - `cargo deny check` 输出
  - 边界脚本输出
  - CI 首次绿截图链接
  - 已知未解决问题（应为空，否则 reject）

**Gate 通过判据**：

- ✅ `ls crates/octopus-harness-*` 输出 19 个目录
- ✅ `ls crates/octopus-sdk*` 仍能列出 14 个旧 SDK crate
- ✅ `test ! -d crates/_octopus-bridge-stub`
- ✅ 不存在 `legacy-sdk` feature
- ✅ `cargo check --workspace` 退出 0
- ✅ `cargo check --workspace --all-features` 退出 0
- ✅ `cargo deny check` 退出 0
- ✅ GitHub Actions PR 流水线全绿
- ✅ `bash scripts/spec-consistency.sh` 退出 0
- ✅ `bash scripts/harness-legacy-boundary.sh` 退出 0
- ✅ 新 `octopus-harness-*` 对旧 `octopus-sdk*` 零依赖

未全绿 → 不得开始 M1。

---

## 3. 完成后状态

完成 M0 后仓库状态：

```text
crates/
├── octopus-core/                  保留
├── octopus-persistence/           保留
├── octopus-platform/              保留
├── octopus-infra/                 保留
├── octopus-server/                保留（继续走旧 SDK，M8 切换）
├── octopus-desktop/               保留（继续走旧 SDK，M8 切换）
├── octopus-cli/                   保留（继续走旧 SDK，M3 可切 run --once spike）
├── octopus-sdk*/                  ◆ 旧 SDK 冻结保留（M8 Gate 后删除）
├── octopus-harness-contracts/     ★ 新（空）
├── octopus-harness-model/         ★ 新（空）
├── octopus-harness-journal/       ★ 新（空）
├── octopus-harness-sandbox/       ★ 新（空）
├── octopus-harness-permission/    ★ 新（空）
├── octopus-harness-memory/        ★ 新（空）
├── octopus-harness-tool/          ★ 新（空）
├── octopus-harness-tool-search/   ★ 新（空）
├── octopus-harness-skill/         ★ 新（空）
├── octopus-harness-mcp/           ★ 新（空）
├── octopus-harness-hook/          ★ 新（空）
├── octopus-harness-context/       ★ 新（空）
├── octopus-harness-session/       ★ 新（空）
├── octopus-harness-engine/        ★ 新（空）
├── octopus-harness-subagent/      ★ 新（空）
├── octopus-harness-team/          ★ 新（空）
├── octopus-harness-plugin/        ★ 新（空）
├── octopus-harness-observability/ ★ 新（空）
└── octopus-harness-sdk/           ★ 新（空）
```

M0 后业务层继续可运行。新 harness 开始独立实现，不从旧 SDK 获取依赖或实现继承。

**M8 收尾时必删项**：

- 14 个旧 `octopus-sdk*` crate
- 业务层全部旧 SDK import / Cargo dependency
- `octopus-platform / octopus-infra` 中只服务旧 SDK 的模块；如仍需功能，基于 `octopus-harness-sdk` 重接

---

## 4. 索引

- **下一里程碑** → [`M1-l0-contracts.md`](./M1-l0-contracts.md)
- **路线图** → [`../01-roadmap.md`](../01-roadmap.md)
- **任务卡模板** → [`../02-task-template.md`](../02-task-template.md)
