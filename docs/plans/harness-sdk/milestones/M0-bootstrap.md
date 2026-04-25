# M0 · Bootstrap · 工作空间脚手架与旧 SDK 退役

> 状态：待启动 · 依赖：无 · 阻塞：所有后续里程碑
> 关键交付：14 旧 crate 移除 · 19 新 crate 空骨架 · 过渡 stub crate · CI 接入 · `cargo check --workspace` 绿
> 预计任务卡：16 张（T01a/b/c/d + T01.5/T01.6 + T02a/b/c/d + T03-T08）· 预计累计工时：AI 7 小时 + 人类评审 3.5 小时
> 并行度：1（串行，因 workspace 文件互斥）

---

## 0. 里程碑级注意事项

1. **本里程碑不实现任何业务逻辑**：所有任务只动 `Cargo.toml` / `lib.rs` 占位 / CI workflow / `deny.toml` / 过渡 stub
2. **删除旧 SDK 是不可逆操作**：T01a~T01d 完成后业务层若无 stub 会立即编译失败 —— 必须先做 T01.5/T01.6 才能继续
3. **保留 crate 与"反向解耦"**（关键修订，对应实施前评估 P0-A）：
   - 保留 crate：`octopus-core / octopus-persistence / octopus-server / octopus-desktop / octopus-cli`（业务入口）
   - **`octopus-platform / octopus-infra` 名义保留，但其内部对旧 SDK 的依赖必须在 T01.5 内反向解耦**（实际仓库现状显示这两个 crate 深度耦合旧 SDK，详见 T01.5）
4. **过渡 stub crate `_octopus-bridge-stub` 与 `legacy-sdk` feature 都是临时治理产物，M8 必删**（实施前评估 P2-3 统一立场）：
   - `crates/_octopus-bridge-stub/`（带下划线前缀以示临时），不进入 `default-members`，仅供业务层占位
   - `octopus-platform / octopus-infra` 内 `legacy-sdk` feature 仅供 M0~M7 期间隔离旧 SDK
   - **M8 业务切换完成后两者都必须 `git rm` / 删除**，否则 M9 完成定义（README §4「14 个旧 sdk crate 已移除」）不成立
   - 历史回放需求 → 走 `octopus-harness-sdk` adapter（M8 显式重接），**不**作为 live feature 永久保留
5. **新 crate 命名规则**：`octopus-harness-<x>`（路径 `crates/octopus-harness-<x>`，包名同），内部 module 用 `harness_<x>`（详见 ADR-008）
6. **任务卡 diff 治理例外**：T01a~T01d 因为是删除型 PR，diff 上限不适用 §00-strategy 铁律 2 的"≤ 500 行"，但每个 commit 必须 ≤ 500 行（按 crate 维度切 commit）

---

## 1. 任务卡清单

| ID | 任务 | 依赖 | 可并行 | diff 上限 |
|---|---|---|---|---|
| M0-T01.5 | `octopus-platform / octopus-infra` 旧 SDK 反向解耦（feature gate 隔离）| 无 | × | < 300 |
| M0-T01.6 | 过渡 stub crate `_octopus-bridge-stub` 创建（业务层占位类型）| T01.5 | × | < 400 |
| M0-T01a | 删除旧 SDK · 基础组（contracts / core / model）| T01.5 + T01.6 | × | 删除型，分 commit ≤ 500 |
| M0-T01b | 删除旧 SDK · L1-L2 组（tools / permissions / sandbox / hooks）| T01a | × | 同上 |
| M0-T01c | 删除旧 SDK · L2-L3 组（context / session / subagent / observability）| T01b | × | 同上 |
| M0-T01d | 删除旧 SDK · L2-L4 组（mcp / plugin / sdk）+ 业务层 import 全部切到 stub | T01c | × | 同上 |
| M0-T02a | 创建 L0 `harness-contracts` 空骨架 | T01d | × | < 100 |
| M0-T02b | 创建 L1 五原语 crate 空骨架 | T02a | × | < 200 |
| M0-T02c | 创建 L2 七复合能力 crate 空骨架 | T02b | × | < 250 |
| M0-T02d | 创建 L3+L4 六 crate 空骨架 | T02c | × | < 250 |
| M0-T03 | workspace `Cargo.toml` 重组（members + dependencies）| T02d | × | < 100 |
| M0-T04 | 根 `deny.toml` 与 `cargo deny` 配置 | T03 | × | < 100 |
| M0-T05 | GitHub Actions CI workflow（fmt/check/clippy/test/deny）| T03 | × | < 200 |
| M0-T06 | `scripts/spec-consistency.sh`（SPEC grep 自检脚本）| T03 | × | < 100 |
| M0-T07 | `cargo doc` 生成 + 仓库 README 链接更新 | T05 | × | < 50 |
| M0-T08 | M0 Gate 检查：`cargo check --workspace` 绿 + CI 首次绿 | T01.5-T07 | × | 0 |

---

## 2. 任务卡详情

### M0-T01.5 · `octopus-platform / octopus-infra` 旧 SDK 反向解耦

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | 无 |
| **预期 diff** | < 300 行 |
| **预期工时** | AI 30 min |

**背景说明**（必读）：

仓库现状抽样显示，名义保留 crate `octopus-platform` 与 `octopus-infra` 内部仍深度耦合旧 SDK：

- `crates/octopus-platform/Cargo.toml` 引用 7 个旧 SDK：`octopus-sdk / sdk-context / sdk-observability / sdk-plugin / sdk-session / sdk-subagent / sdk-tools`
- `crates/octopus-platform/src/runtime_sdk/` 目录（11 个文件，约 60+ 处旧 SDK 引用）整个绑死在旧 SDK
- `crates/octopus-infra/src/{agent_assets.rs, resources_skills.rs}` 同样依赖旧 SDK

如不先反向解耦，T01a~T01d 删除旧 SDK 后两 crate 立即编译失败，M0 Gate 永远不通过。

**SPEC 锚点**：
- `AGENTS.md` 仓库根规则（保留 crate 列表）
- `docs/plans/harness-sdk/milestones/M0-bootstrap.md` §0（保留 crate 与反向解耦声明）
- 实施前评估报告 P0-A

**ADR 锚点**：
- ADR-008（crate-layout）

**预期产物**：

- `crates/octopus-platform/Cargo.toml`：把 7 个旧 SDK 依赖全部加 `optional = true` 并归到 `[features] legacy-sdk = [...]` 之下；`default = []`（不再默认启用）
- `crates/octopus-platform/src/runtime_sdk/mod.rs` 顶部加 `#![cfg(feature = "legacy-sdk")]`，整个目录都仅在 `legacy-sdk` feature 下编译
- `crates/octopus-platform/src/lib.rs`：`pub mod runtime_sdk;` 改为 `#[cfg(feature = "legacy-sdk")] pub mod runtime_sdk;`
- `crates/octopus-infra/Cargo.toml`：同上 `legacy-sdk` feature 化
- `crates/octopus-infra/src/{agent_assets.rs, resources_skills.rs}`：模块声明加 `#[cfg(feature = "legacy-sdk")]` 门控
- 修改 `crates/octopus-server / octopus-desktop / octopus-cli / apps/desktop/src-tauri` 的 `Cargo.toml`：移除对 `octopus-platform / octopus-infra` 的 `legacy-sdk` feature 启用（如有显式启用）

**关键不变量**：

- 关 feature 后 `cargo check -p octopus-platform / -p octopus-infra` 必须通过
- `legacy-sdk` feature 不进 default、不在 `feature-flags.md` D10 矩阵登记（这是**临时治理产物**，非 SDK feature）
- M0~M7 期间这两个 crate 的 `runtime_sdk / agent_assets / resources_skills` 模块**不可调用**（业务层迁移期不应继续走这条路径）

**禁止行为**：

- 不要直接 `git rm` 这两个 crate（其它非 SDK 模块仍在用）
- 不要把 `legacy-sdk` 加进 `feature-flags.md`（治理污染）
- 不要在本卡修改 `runtime_sdk` 内部代码（仅改门控）

**验收命令**：

```bash
cargo check -p octopus-platform               # 必须绿
cargo check -p octopus-infra                  # 必须绿
cargo check -p octopus-platform --features legacy-sdk  # 此时仍会失败（因为 sdk 未删），可接受
```

**PR 描述模板要点**：本卡是"反向解耦"，不是"删除"。`legacy-sdk` feature 在 M0~M7 期间作为旧路径开关，M8 后删除。

---

### M0-T01.6 · 过渡 stub crate `_octopus-bridge-stub` 创建

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T01.5 |
| **预期 diff** | < 400 行 |
| **预期工时** | AI 45 min |

**背景说明**：

业务层（server / desktop / cli / apps/desktop/src-tauri）当前调用旧 SDK 的类型（`SessionId / TurnInput / ToolUseRequested / ...`）+ 函数。仅删 import 后留 `unimplemented!()` 不能编译——类型签名仍需类型存在。

需要创建临时桩 crate 提供"占位类型 + `unimplemented!()` 函数体"，让业务层在 M0~M7 期间持续可编译，M8 切换时整体 `git rm` 该 crate。

**SPEC 锚点**：
- 实施前评估报告 P0-A / P0-E

**预期产物**：

- 创建 `crates/_octopus-bridge-stub/` 目录（注意下划线前缀）
- `crates/_octopus-bridge-stub/Cargo.toml`：
  ```toml
  [package]
  name = "_octopus-bridge-stub"
  version.workspace = true
  edition.workspace = true
  publish = false
  rust-version.workspace = true

  [lints]
  workspace = true

  [dependencies]
  # 仅依赖 std + serde（不引入业务依赖）
  serde = { workspace = true }
  ```
- `crates/_octopus-bridge-stub/src/lib.rs`：
  ```rust
  //! `_octopus-bridge-stub`
  //!
  //! 临时桥接桩，M0~M7 期间业务层占位用。
  //! 状态：M8 业务切换完成后 `git rm` 整个 crate。
  //!
  //! 治理来源：docs/plans/harness-sdk/milestones/M0-bootstrap.md §0
  //!          实施前评估报告 P0-A

  pub type SessionId = String;
  pub type RunId = String;
  pub type TenantId = String;
  // ...其它业务层占位类型
  ```
  详细类型清单需在本卡 PR 描述中由 AI 通过 grep 业务层提取（约 30~50 个最小占位类型）
- `crates/_octopus-bridge-stub/src/stub.rs`：放置 `unimplemented!()` 函数桩
  ```rust
  pub async fn create_session(_: SessionId) -> Result<(), String> {
      unimplemented!("TODO(M8): replace with octopus_harness_sdk::Harness::create_session")
  }
  ```
- 根 `Cargo.toml`：`[workspace] members` 加 `crates/_octopus-bridge-stub`；`default-members` **不加**（仅按需编译）
- 业务层 `Cargo.toml` (server / desktop / cli / apps/desktop/src-tauri) 加 `_octopus-bridge-stub = { path = "../_octopus-bridge-stub" }`

**关键不变量**：

- crate 名以 `_` 开头，便于 ls / cargo metadata 识别为临时
- `lib.rs` 顶部必须有 "M8 必删" 警示注释
- 桩函数必须 `unimplemented!("TODO(M8-Txx): ...")` 包含具体的 M8 任务卡 ID 引用
- 该 crate 不在 `[workspace.lints]` 之外另立 lint 规则
- 不引入除 `std + serde` 之外的依赖

**禁止行为**：

- 不要在 stub 内实现真业务逻辑（所有函数体都应 `unimplemented!()`）
- 不要为 stub 写测试（M8 必删，测试浪费）
- 不要把 stub 类型公开到 `octopus-harness-*` crate（污染最终交付）

**验收命令**：

```bash
cargo check -p _octopus-bridge-stub
cargo doc --no-deps -p _octopus-bridge-stub
ls crates/_octopus-bridge-stub  # 必须存在
```

**SPEC 一致性自检**：

```bash
# stub 内部不允许真实现
! grep -rE 'fn .*\{[^}]*[a-z]' crates/_octopus-bridge-stub/src --include='*.rs' | grep -v 'unimplemented' | grep -v 'pub type' | grep -v '^//'

# 必须在 src/lib.rs 顶部有 M8 必删警示
grep -q 'M8.*git rm' crates/_octopus-bridge-stub/src/lib.rs
```

---

### M0-T01a · 删除旧 SDK · 基础组（contracts / core / model）

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T01.5 + M0-T01.6 |
| **预期 diff** | 删除型，分 commit 每个 ≤ 500 行 |
| **预期工时** | AI 25 min |

**SPEC 锚点**：
- `docs/architecture/harness/README.md` §3
- `docs/architecture/harness/crates/harness-sdk.md` §6
- ADR-008

**预期产物**：

- 删除目录：
  ```
  crates/octopus-sdk-contracts/
  crates/octopus-sdk-core/
  crates/octopus-sdk-model/
  ```
- 修改根 `Cargo.toml`：从 `[workspace] members` / `default-members` 移除上述 3 项
- 业务层（server / desktop / cli / apps/desktop/src-tauri / octopus-platform / octopus-infra）的 `Cargo.toml`：移除对这 3 个 crate 的 path 依赖；如代码层 `use octopus_sdk_contracts::*` / `octopus_sdk_core::*` / `octopus_sdk_model::*` 全部替换为 `use _octopus_bridge_stub::*`（结合 T01.6 的占位类型）
- 每个 crate 的删除 + 引用替换在独立 commit（commit 题目：`chore(M0-T01a): remove octopus-sdk-contracts`）

**关键不变量**：

- 删除后 `cargo check --workspace` 必须通过（依赖 stub crate）
- 不留 `use octopus_sdk_{contracts,core,model}` 残余引用

**禁止行为**：

- 不要"重命名"旧 crate（应直接 git rm）
- 不要保留 README / examples / benchmarks
- 不要在本卡碰其它 11 个旧 SDK（T01b~T01d 处理）

**验收命令**：

```bash
! ls crates/ | grep -E '^octopus-sdk-(contracts|core|model)$'
cargo check --workspace
```

---

### M0-T01b · 删除旧 SDK · L1-L2 组（tools / permissions / sandbox / hooks）

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T01a |
| **预期 diff** | 删除型，分 commit 每个 ≤ 500 行 |

**预期产物**：删除目录与替换引用同 T01a 模式：
- `crates/octopus-sdk-tools/`
- `crates/octopus-sdk-permissions/`
- `crates/octopus-sdk-sandbox/`
- `crates/octopus-sdk-hooks/`

**验收命令**：

```bash
! ls crates/ | grep -E '^octopus-sdk-(tools|permissions|sandbox|hooks)$'
cargo check --workspace
```

---

### M0-T01c · 删除旧 SDK · L2-L3 组（context / session / subagent / observability）

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T01b |
| **预期 diff** | 删除型，分 commit 每个 ≤ 500 行 |

**预期产物**：
- `crates/octopus-sdk-context/`
- `crates/octopus-sdk-session/`
- `crates/octopus-sdk-subagent/`
- `crates/octopus-sdk-observability/`

**验收命令**：

```bash
! ls crates/ | grep -E '^octopus-sdk-(context|session|subagent|observability)$'
cargo check --workspace
```

---

### M0-T01d · 删除旧 SDK · L2-L4 组（mcp / plugin / sdk）+ 业务层 import 收口

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T01c |
| **预期 diff** | 删除型 + 业务层 import 收尾，分 commit 每个 ≤ 500 行 |

**预期产物**：

- 删除目录：
  ```
  crates/octopus-sdk-mcp/
  crates/octopus-sdk-plugin/
  crates/octopus-sdk/
  ```
- 业务层（server / desktop / cli / apps/desktop/src-tauri）剩余所有 `use octopus_sdk*::...` import 全部清零（替换为 `use _octopus_bridge_stub::*`）
- 业务层每个被替换的函数体内部加注释：`// TODO(M8-Txx): replace with octopus_harness_sdk::xxx`（其中 `Txx` 引用 M8 的具体任务卡 ID）

**关键不变量**：

- 全 workspace `grep -rE 'octopus[-_]sdk' crates/ apps/ --include='*.rs' --include='*.toml'` 应仅命中 `_octopus-bridge-stub` 与 `legacy-sdk` feature 描述
- `cargo check --workspace` 通过
- M0 后 `cargo build` 业务层产物可启动但 runtime 不可用（按 Plan 设计预期）

**SPEC 一致性自检**：

```bash
# 不应再出现旧 crate 名（除 stub 与 legacy-sdk 描述）
! grep -rE 'octopus[-_]sdk' --include='*.rs' --include='*.toml' \
    --exclude-dir='.git' --exclude-dir='target' --exclude-dir='docs' \
    --exclude-dir='_octopus-bridge-stub' \
    crates/ apps/ \
    | grep -v 'legacy-sdk' \
    | grep -v 'octopus-harness'

# 业务层应有 TODO(M8-Txx) 标记
grep -rE 'TODO\(M8-T[0-9]+' crates/octopus-server crates/octopus-desktop crates/octopus-cli apps/desktop/src-tauri --include='*.rs'
```

---

## 2.7 M0-T02a-d 共享模板 · 创建 19 个新 `octopus-harness-*` crate 空骨架

> **拆分理由**（实施前评估 P1-5）：原单卡 ~600 行违反 00-strategy 铁律 2 ≤ 500 行硬上限。按 5 层依赖拆 4 子卡，每卡 ≤ 200 行。

| 子卡 ID | 创建的 crate | 数量 | 依赖 | 预期 diff |
|---|---|:---:|---|---|
| M0-T02a | L0：`harness-contracts` | 1 | M0-T01d | < 100 |
| M0-T02b | L1：`harness-{model, journal, sandbox, permission, memory}` | 5 | T02a | < 200 |
| M0-T02c | L2：`harness-{tool, tool-search, skill, mcp, hook, context, session}` | 7 | T02b | < 250 |
| M0-T02d | L3+L4：`harness-{engine, subagent, team, plugin, observability, sdk}` | 6 | T02c | < 250 |

**SPEC 锚点**：
- `docs/architecture/harness/overview.md` §4.1（19 crate 清单）
- `docs/architecture/harness/module-boundaries.md` §3（依赖白名单）
- `docs/architecture/harness/feature-flags.md` §2.2（每 crate 自身 features）

**ADR 锚点**：ADR-008（crate-layout）

**预期产物**（创建 19 个目录 + 每个 3 文件）：

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
- 每个 crate `Cargo.toml` 中的 `[dependencies]` 必须**仅声明 D2 §3 白名单允许的 crate**
- 每个 crate `Cargo.toml` 中的 `[features]` 必须**与 `feature-flags.md` §2.2 一致**
- 每个 crate `lib.rs` 必须包含 SPEC 文档路径注释
- crate 名包含连字符（`-`），module 名用下划线（`_`）：导入用 `use octopus_harness_permission::...`

**禁止行为**：

- 不要在本卡实现具体逻辑
- 不要新增 D2 / feature-flags 之外的 dependency

**验收命令**：

```bash
ls -d crates/octopus-harness-* | wc -l   # 应输出 19
cargo check --workspace                   # 必须绿
cargo doc --no-deps --workspace           # 必须绿
```

**SPEC 一致性自检**：

```bash
# 验证 19 crate 清单完整
for c in contracts model journal sandbox permission memory tool tool-search skill mcp hook context session engine subagent team plugin observability sdk; do
    test -d "crates/octopus-harness-$c" || echo "MISSING: octopus-harness-$c"
done
```

### M0-T02a · 创建 L0 `harness-contracts` 空骨架

**依赖**：M0-T01d

**预期产物**：按本节共享模板创建 `crates/octopus-harness-contracts/`。

**验收命令**：

```bash
test -d crates/octopus-harness-contracts
cargo check -p octopus-harness-contracts
```

### M0-T02b · 创建 L1 五原语 crate 空骨架

**依赖**：M0-T02a

**预期产物**：按本节共享模板创建 `model / journal / sandbox / permission / memory` 五个 crate。

**验收命令**：

```bash
for c in model journal sandbox permission memory; do
    test -d "crates/octopus-harness-$c"
done
cargo check -p octopus-harness-model -p octopus-harness-journal -p octopus-harness-sandbox -p octopus-harness-permission -p octopus-harness-memory
```

### M0-T02c · 创建 L2 七复合能力 crate 空骨架

**依赖**：M0-T02b

**预期产物**：按本节共享模板创建 `tool / tool-search / skill / mcp / hook / context / session` 七个 crate。

**验收命令**：

```bash
for c in tool tool-search skill mcp hook context session; do
    test -d "crates/octopus-harness-$c"
done
cargo check -p octopus-harness-tool -p octopus-harness-tool-search -p octopus-harness-skill -p octopus-harness-mcp -p octopus-harness-hook -p octopus-harness-context -p octopus-harness-session
```

### M0-T02d · 创建 L3+L4 六 crate 空骨架

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
| **依赖** | M0-T02 |
| **预期 diff** | < 100 行 |

**SPEC 锚点**：
- 当前 `Cargo.toml`（v1.8.1 状态）
- `docs/architecture/harness/feature-flags.md` §2.2

**预期产物**：

- 修改 `Cargo.toml`：
  - `[workspace] members` 增加 19 个新 crate
  - `[workspace] default-members` 也加（默认编译）
  - `[workspace.dependencies]` 增加跨 crate 路径声明（便于子 crate 用 `workspace = true`）
  - 增加 `[workspace.dependencies]` 项：`async-stream / bytes / chrono / schemars / ulid / secrecy / parking_lot / dashmap / regex / globset / sha2 / blake3 / hex` 等已知会用的库（仅声明，不强制使用）

**`[workspace.dependencies]` 新增项示例**：

```toml
async-stream = "0.3.6"
bytes = { version = "1", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
schemars = { version = "0.8", features = ["chrono", "ulid", "derive"] }
ulid = { version = "1", features = ["serde"] }
secrecy = { version = "0.8", features = ["serde"] }
parking_lot = "0.12"
dashmap = "6"
regex = "1"
globset = { workspace = true }   # 已存在
sha2 = "0.10"
blake3 = "1"
hex = "0.4"
futures = "0.3"
futures-util = "0.3"
```

**关键不变量**：

- workspace.members 顺序：现有保留 crate（octopus-core / octopus-persistence / ...）→ 19 个 harness-*
- 不要改变 `[workspace.lints]`（已锁定 P1 / pedantic 规则）
- 现有 `[workspace.dependencies]` 中的版本不要变更

**验收命令**：

```bash
cargo metadata --format-version 1 | jq '.workspace_members | length'   # ≥ 19 + 7 保留 = 26
cargo check --workspace
```

---

### M0-T04 · 根 `deny.toml` 与 `cargo deny` 配置

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T03 |
| **预期 diff** | < 100 行 |

**SPEC 锚点**：
- `docs/plans/harness-sdk/03-quality-gates.md` §5（cargo deny 配置示例）
- `docs/architecture/harness/module-boundaries.md` §3.7 + §10（feature 矩阵 + 例外登记表）

**预期产物**：

- 创建 `deny.toml`（参考 `03-quality-gates.md` §5.1 模板）
- 创建 `.github/workflows/deny.yml`（PR + push 触发）
- 安装 `cargo deny` 到本地：`cargo install --locked cargo-deny`

**关键不变量**：

- `[advisories] vulnerability = "deny"`
- `[licenses] allow` 至少包含：MIT / Apache-2.0 / BSD-3-Clause / BSD-2-Clause / ISC / Unicode-DFS-2016 / Zlib
- 必须配置 feature 矩阵检查（含 `auto-mode / redactor / subagent-tool` 三个已登记破窗）
- **`std::sync::Mutex` 不属于 cargo-deny 职责**（cargo-deny `[bans]` 只能禁 crate 名）；该禁令由两条独立约束承担：
  1. `clippy::disallowed_types`（在 `[workspace.lints.clippy]` 配置）—— 编译期拒绝
  2. `scripts/spec-consistency.sh` 的 grep —— PR 期 CI 拒绝（已在 M0-T06 模板）

**验收命令**：

```bash
cargo deny check
cargo deny check --features auto-mode
cargo deny check --features redactor
cargo deny check --features subagent-tool
```

---

### M0-T05 · GitHub Actions CI workflow

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T03 |
| **预期 diff** | < 200 行 |

**SPEC 锚点**：
- `docs/plans/harness-sdk/03-quality-gates.md` §7

**预期产物**：

- `.github/workflows/ci.yml`（PR 流水线：fmt / check / clippy / test / coverage）
- `.github/workflows/nightly.yml`（每日全 feature 矩阵）
- `.github/workflows/release.yml`（占位，M9 完善）

**关键不变量**：

- ubuntu-latest + macos-latest 双 runner
- Rust toolchain 锁定（`rust-toolchain.toml` 锁 `1.78`，与 workspace.package.rust-version 一致）
- 缓存 `~/.cargo/registry / target/` 加速

**验收命令**：

```bash
# 本地 act 模拟（可选）
gh workflow list
# 推送 PR 后观察 CI
```

---

### M0-T06 · SPEC 一致性自检脚本

| 字段 | 值 |
|---|---|
| **状态** | 待派发 |
| **依赖** | M0-T03 |
| **预期 diff** | < 100 行 |

**SPEC 锚点**：
- `docs/plans/harness-sdk/03-quality-gates.md` §6.1（grep 模板）
- `docs/plans/harness-sdk/03-quality-gates.md` §6.2（依赖图与边界检查脚本）

**预期产物**：

- `scripts/spec-consistency.sh`（详细见 03-quality-gates §6.1 模板：trait 签名 / 错误类型 / std::sync::Mutex / unsafe / UI 类型 grep）
- `scripts/feature-matrix.sh`（cargo check 全 feature profile 跑一遍，对齐 D10 §3.1-§3.4）
- `scripts/dep-boundary-check.sh` + `scripts/check_layer_boundaries.py`（cargo metadata 解析 + crate 维度白名单校验，覆盖别名导入、间接依赖、feature 触发依赖、L1 反向依赖等 grep 漏检场景）
- `scripts/depgraph-snapshot.sh`（cargo depgraph + 与 D2 §5 期望图差异检查）
- `docs/architecture/harness/expected-depgraph.dot`（D2 §5 的期望依赖图快照；缺失时 `depgraph-snapshot.sh` 必须失败）

**关键不变量**：

- 退出码：0 = 全通过；非 0 = 失败
- 输出失败原因 + 失败位置文件路径
- 4 个脚本可独立运行（CI matrix 各自一格）

**验收命令**：

```bash
bash scripts/spec-consistency.sh && echo "OK spec"
bash scripts/feature-matrix.sh && echo "OK features"
bash scripts/dep-boundary-check.sh && echo "OK boundary"
bash scripts/depgraph-snapshot.sh && echo "OK depgraph"
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
| **依赖** | T01.5 / T01.6 / T01a-T01d / T02-T07 |
| **预期 diff** | 0（仅验证）|

**预期产物**：

- 一份 `docs/plans/harness-sdk/audit/M0-bootstrap-gate.md`（**人类**填写）：
  - 19 crate 列表 + `_octopus-bridge-stub` 一项
  - `cargo check --workspace --all-features` 输出
  - `cargo deny check` 输出
  - CI 首次绿截图链接
  - 已知未解决问题（应为空，否则 reject）

**Gate 通过判据**：

- ✅ `ls crates/octopus-harness-*` 输出 19 个目录
- ✅ `ls crates/octopus-sdk*` 输出空
- ✅ `ls crates/_octopus-bridge-stub` 存在（M0~M7 脚手架）
- ✅ `cargo check --workspace` 退出 0
- ✅ `cargo check --workspace --all-features` 退出 0（不含 `legacy-sdk` feature；如开启会失败属预期）
- ✅ `cargo deny check` 退出 0
- ✅ GitHub Actions PR 流水线全绿
- ✅ `bash scripts/spec-consistency.sh` 退出 0
- ✅ `apps/desktop/src-tauri` 编译通过（业务层 import 全部走 `_octopus-bridge-stub`）
- ✅ `octopus-platform / octopus-infra` 关 `legacy-sdk` feature 时编译通过

未全绿 → 不得开始 M1。

---

## 3. 完成后状态

完成 M0 后仓库状态：

```text
crates/
├── octopus-core/                  保留
├── octopus-persistence/           保留
├── octopus-platform/              保留（runtime_sdk/* 仅 legacy-sdk feature 编译，默认关）
├── octopus-infra/                 保留（agent_assets / resources_skills 仅 legacy-sdk feature 编译）
├── octopus-server/                保留（业务调用走 _octopus-bridge-stub）
├── octopus-desktop/               保留（同上）
├── octopus-cli/                   保留（同上）
├── _octopus-bridge-stub/          ◆ 临时（M0~M7 脚手架，M8 必删）
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

业务层（server / desktop / cli）暂时编译通过但 runtime 不可用 —— 等待 M8 切换。

**M8 收尾时必删项**（在 M8 任务卡显式登记）：

- `crates/_octopus-bridge-stub/` 整个目录
- `octopus-platform / octopus-infra` 的 `legacy-sdk` feature 与对应模块（`runtime_sdk / agent_assets / resources_skills`）—— 由业务负责人在 M8 决定是否重新接入 `octopus-harness-sdk`

---

## 4. 索引

- **下一里程碑** → [`M1-l0-contracts.md`](./M1-l0-contracts.md)
- **路线图** → [`../01-roadmap.md`](../01-roadmap.md)
- **任务卡模板** → [`../02-task-template.md`](../02-task-template.md)
