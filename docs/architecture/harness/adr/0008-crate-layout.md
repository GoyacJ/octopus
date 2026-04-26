# ADR-008 · Crate 命名与分层

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：整个 `crates/` 目录；业务层 `Cargo.toml`

## 1. 背景与问题

Octopus 现有 `crates/octopus-sdk*` 14 个 crate **职责混乱、层级跨越、缺失核心抽象**（见 `overview.md` §3 工程遗产迁移提示）。必须全部重建。

新架构需回答：

1. Crate 如何命名？
2. 如何分层？
3. 每层含哪些 crate？
4. 命名前缀如何避免与老 crate 混淆？

## 2. 决策

### 2.1 命名前缀

所有新 SDK crate 使用 `octopus-harness-*` 前缀：

- **中性**：强调"基础设施"而非业务
- **可识别**：与 `octopus-*` 业务 crate 区分
- **未来兼容**：当 SDK 独立发版时，直接 publish 为 `octopus-harness-*`

门面 crate 命名为 `octopus-harness-sdk`（而非 `octopus-harness`），避免与内部 crate 混淆。

### 2.2 Rust 模块名

Rust 模块使用 `harness_xxx` 前缀（不带 octopus）：

```rust
use octopus_harness_sdk::prelude::*;
// 实际 use：
use harness_contracts::Event;  // 不带 octopus_ 前缀
```

### 2.3 分层（5 层）

```text
L4 · Facade      ← 对外唯一门面
L3 · Engine      ← 运行引擎与协作
L2 · Composite   ← 复合能力
L1 · Primitives  ← 原语
L0 · Contracts   ← 公共契约
```

### 2.4 19 个 Crate 最终清单（v1.2：新增 `harness-tool-search`）

```text
crates/
  L0: octopus-harness-contracts/
  L1:
    octopus-harness-model/
    octopus-harness-journal/
    octopus-harness-sandbox/
    octopus-harness-permission/
    octopus-harness-memory/
  L2:
    octopus-harness-tool/
    octopus-harness-tool-search/   ← v1.2 新增（ADR-009）
    octopus-harness-skill/
    octopus-harness-mcp/
    octopus-harness-hook/
    octopus-harness-context/
    octopus-harness-session/
  L3:
    octopus-harness-engine/
    octopus-harness-subagent/
    octopus-harness-team/
    octopus-harness-plugin/
    octopus-harness-observability/
  L4:
    octopus-harness-sdk/   ← 业务层唯一依赖
```

> **依赖方向**（ADR-009 §2.1）：`harness-tool-search` 依赖 `harness-contracts` + `harness-tool`（Tool trait） + `harness-model`（ModelCapabilities）。`harness-tool` **不**反向依赖 `harness-tool-search`；`ToolSearchTool` 的注入由 L4 门面 `harness-sdk` 完成，避免循环依赖。

### 2.5 删除清单（14 个老 SDK）

以下 crate 是最终删除清单。M0~M7 期间只冻结保留，不新增能力；M8 业务层完成切换后再按 §6.4 删除。

```text
crates/octopus-sdk/
crates/octopus-sdk-contracts/
crates/octopus-sdk-core/
crates/octopus-sdk-model/
crates/octopus-sdk-session/
crates/octopus-sdk-tools/
crates/octopus-sdk-mcp/
crates/octopus-sdk-permissions/
crates/octopus-sdk-sandbox/
crates/octopus-sdk-hooks/
crates/octopus-sdk-context/
crates/octopus-sdk-subagent/
crates/octopus-sdk-plugin/
crates/octopus-sdk-observability/
```

### 2.6 保留 crate（业务侧基础设施）

```text
crates/octopus-core/         保留（工程通用工具）
crates/octopus-persistence/  保留（业务层数据持久化）
crates/octopus-platform/     保留（平台适配）
crates/octopus-infra/        保留（基础设施）
crates/octopus-server/       保留（HTTP server）
crates/octopus-desktop/      保留（Tauri 适配）
crates/octopus-cli/          保留（CLI）
```

这些 crate **不属于 SDK 范畴**，业务层按需使用。

## 3. 替代方案

### 3.1 A：`octopus-agent-*` / `octopus-agent-sdk`

- ❌ 强调 Agent 领域，但 SDK 本身是 Harness（基础设施）；命名不精确
- ❌ 可能与未来的 Agent 产品混淆

### 3.2 B：沿用 `octopus-sdk-*` 前缀（覆盖老 crate）

- ❌ 需要逐步替换，增加迁移风险
- ❌ Git 历史污染

### 3.3 C：`octopus-harness-*` / `octopus-harness-sdk`（采纳）

- ✅ 中性、清晰
- ✅ 与老 crate 无冲突
- ✅ 整个 SDK 一次性 flip 切换

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| 迁移工作量 | 19 个新 crate + 14 个删除 | 本 SDK 采用**全量重建**策略；一次切换到位 |
| 前缀略长 | `octopus-harness-*` 12 字符 | 业务层只写 `use octopus_harness_sdk` 一个，不直面其他 |
| 多 crate 启动时间 | 首次编译慢 | Feature flag 按需开启 |

## 5. 后果

### 5.1 正面

- 职责清晰，单一职责
- Feature flag 粒度细
- 业务层依赖面最小（仅 `harness-sdk`）
- 删除老 crate 降低维护负担

### 5.2 负面

- 一次性迁移成本
- 文档更新量大（已规划：19 个 crate SPEC + 10 个 SAD + 9 个 ADR）

## 6. 实现指引

### 6.1 工作空间 Cargo.toml

```toml
[workspace]
members = [
    "crates/octopus-harness-contracts",
    "crates/octopus-harness-model",
    "crates/octopus-harness-journal",
    # ... 其余 16 个（含 v1.2 新增的 octopus-harness-tool-search）
    "crates/octopus-harness-sdk",
    # 业务层
    "crates/octopus-core",
    "crates/octopus-persistence",
    # ... 其余 5 个
]
```

### 6.2 命名规范

| 层次 | 命名 |
|---|---|
| crate package name | `octopus-harness-<role>` |
| cargo path | `crates/octopus-harness-<role>/` |
| rust module crate name | `harness_<role>`（使用 `package = "octopus-harness-<role>"` + `crate-name = "harness_<role>"`） |
| 业务层 import | `use octopus_harness_sdk::*` |

### 6.3 公共类型 re-export

`harness-sdk` 在 `prelude` 中 re-export 所有对外类型：

```rust
pub mod prelude {
    pub use octopus_harness_contracts::*;
    pub use crate::{
        Harness, HarnessBuilder, Session, Event, EventStream,
        TurnInput, TurnOutput, Error,
    };
    pub use crate::ext::*;
}
```

### 6.4 删除流程

按层自底向上删除，每批删除后跑 `cargo check --all`：

1. 删除顶层 `crates/octopus-sdk`
2. 删除 L3 `crates/octopus-sdk-subagent / plugin / observability`
3. 删除 L2 `crates/octopus-sdk-tools / mcp / hooks / context / session`
4. 删除 L1 `crates/octopus-sdk-model / sandbox / permissions / core`
5. 删除 L0 `crates/octopus-sdk-contracts`
6. 从 workspace Cargo.toml 移除对应 member

## 7. 相关

- D1 · `overview.md` §4 19 个 Crate 总览
- D2 · `module-boundaries.md`
- ADR-009 Deferred Tool Loading（新增 `harness-tool-search` crate）
- 所有 `crates/harness-*.md` SPEC
