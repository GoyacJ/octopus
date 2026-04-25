# D2 · 模块边界与依赖矩阵

> 依赖 ADR：ADR-008（Crate 命名与分层）
> 状态：Accepted

## 1. 目的

定义 18 个 crate 之间**允许的依赖方向**与**禁止的耦合模式**。任何违反本文的代码评审**必须拒绝**。

## 2. 分层编号约定

| 层 | 编号 | Crate |
|---|---|---|
| L0 契约 | `contracts` | harness-contracts |
| L1 原语 | `P1..P5` | harness-model / journal / sandbox / permission / memory |
| L2 复合能力 | `C1..C6` | harness-tool / skill / mcp / hook / context / session |
| L3 引擎与协作 | `E1..E5` | harness-engine / subagent / team / plugin / observability |
| L4 门面 | `sdk` | harness-sdk |

## 3. 允许的依赖方向（白名单）

### 3.1 L0 · harness-contracts

- **对外依赖**：无（仅依赖标准库 / serde / ulid / schemars 等**契约级基础库**）
- **对内开放**：所有其他 crate 都可以 `use harness_contracts::*;`

### 3.2 L1 · 原语层

| Crate | 可依赖 | 禁止依赖 |
|---|---|---|
| `harness-model` | `contracts` | L1 其他 / L2 / L3 / L4 |
| `harness-journal` | `contracts` | L1 其他 / L2 / L3 / L4 |
| `harness-sandbox` | `contracts` | L1 其他 / L2 / L3 / L4 |
| `harness-permission` | `contracts` | L1 其他 / L2 / L3 / L4 |
| `harness-memory` | `contracts` | L1 其他 / L2 / L3 / L4 |

> 例外：L1 crate 可互相通过 `contracts` 定义的**共享类型**通信，但**不得直接 use 另一个 L1 crate**。

### 3.3 L2 · 复合能力层

| Crate | 可依赖 | 说明 |
|---|---|---|
| `harness-tool` | `contracts`, `permission`, `sandbox` | 工具执行需要权限检查与沙箱；**不依赖 `model`**（避免 L2 交叉：Tool 本身不做推理，ToolContext 里的 model 通过 trait 对象间接访问） |
| `harness-skill` | `contracts`, `memory`(读) | 无需沙箱、无需模型 |
| `harness-mcp` | `contracts`, `tool`(trait 抽象注册) | MCP 动态工具接入 `ToolRegistry` |
| `harness-hook` | `contracts` | Hook 事件分发 |
| `harness-context` | `contracts`, `memory`, `journal`(trait), `model`(trait，用于 aux LLM compact) | Context 从 Memory 与 Journal 读；Microcompact/Autocompact 通过 `ModelProvider` trait 调辅助 LLM |
| `harness-session` | `contracts`, `journal`, `context`, `permission`, `memory`, `tool`, `skill`, `mcp`, `hook` | Session 是 L2 的"聚合者" |

### 3.4 L3 · 引擎与协作层

| Crate | 可依赖 | 说明 |
|---|---|---|
| `harness-engine` | L0, 全部 L1, 全部 L2 | Engine 是"最大聚合者" |
| `harness-subagent` | `engine`（via trait）, `contracts`, `session`, `tool`, `permission` | 独立 crate，不直接引用 `engine` 实现 |
| `harness-team` | `engine`（via trait）, `contracts`, `session`, `journal` | 独立 crate，Team 协调器 |
| `harness-plugin` | `contracts`, `tool`(trait), `hook`(trait), `mcp`(trait), `skill`(trait) | 只通过 trait 注册，不实例化 |
| `harness-observability` | `contracts`, `journal`(读) | 只读不写 Journal |

### 3.5 L4 · 门面

| Crate | 可依赖 | 说明 |
|---|---|---|
| `harness-sdk` | 全部 L0/L1/L2/L3 | 聚合对外门面 |

### 3.6 业务层

| 消费路径 | 规则 |
|---|---|
| 业务层 → `harness-sdk` | **唯一允许**的依赖形态（95% 场景覆盖） |
| 业务层 → 单个内部 crate | **例外**场景（如只复用 `harness-contracts` 类型定义），需 PR 显式说明 |
| 业务层 → 多个内部 crate | **禁止** |

## 4. 禁止的耦合模式

### 4.1 硬禁止（Critical）

| 反模式 | 原因 |
|---|---|
| L1 crate 直接 `use` 另一个 L1 crate | 违反原语正交性 |
| L2 crate 直接 `use` 另一个 L2 具体实现（除非 trait） | 违反 Bounded Context |
| L2/L3 crate `use` L4 (`harness-sdk`) | 反向依赖 |
| 任何 crate `use std::sync::Mutex`（应使用 `tokio::sync` 或 `parking_lot`）| 阻塞异步运行时 |
| 任何 crate 在 trait 方法签名中暴露 UI 类型（`React.*` / `egui::*` / `tauri::*`）| 违反 ADR-002 |
| 任何 crate 依赖具体的持久化后端（`sqlite` / `postgres` / `redis`）而非 trait | 违反 P2 依赖倒置 |

### 4.2 软禁止（Warning，需评审豁免）

| 反模式 | 豁免条件 |
|---|---|
| 跨 crate 的 `thiserror` 错误链 | 顶层用 `anyhow` 或专属 `HarnessError` |
| crate 内部 `pub(crate)` 但被其他 crate 间接访问 | 明确走 `pub` 公开面 |
| 动态字符串作为类型名（如 `String` 表示工具名而非 `ToolName`） | 新类型模式（Newtype）必选 |

## 5. 依赖图可视化

```text
                          harness-sdk (L4)
                               │
           ┌───────────────────┼───────────────────┐
           ▼                   ▼                   ▼
    harness-engine      harness-subagent    harness-team
    harness-plugin      harness-observability
           │                   │                   │
           └───────────┬───────┴──────┬────────────┘
                       ▼              ▼
                 L2 复合能力层（共享 trait 接口）
           ┌──────┬──────┬──────┬──────┬──────┐
         tool  skill  mcp   hook  context  session
           │                                 │
           └────────────┬────────────────────┘
                        ▼
                 L1 原语层（trait + 默认实现）
           ┌──────┬──────────┬─────────┬──────────┬──────┐
         model  journal   sandbox  permission  memory
           │      │          │         │          │
           └──────┴──────────┴─────────┴──────────┘
                        ▼
                 harness-contracts (L0)
                  （types / events / errors）
```

## 6. 循环依赖检查清单

| 场景 | 检查方式 |
|---|---|
| `harness-engine` ↔ `harness-subagent` | Subagent 通过 `trait EngineRunner` 注入，不直接引用 `engine` 实现 |
| `harness-tool` ↔ `harness-mcp` | MCP 客户端只实现 `trait Tool` 并注册到 `ToolRegistry`，`tool` crate 不感知 MCP |
| `harness-session` ↔ `harness-context` | Session 拥有 `ContextEngine`，但 `ContextEngine` 不回调 Session 状态（读-only Projection） |
| `harness-plugin` ↔ 被加载能力 | Plugin crate 只依赖各能力的 **trait**，不实例化业务实现 |
| `harness-plugin` ↔ Loader（ADR-0015） | `PluginManifestLoader::enumerate` 返回 `Vec<ManifestRecord>`，**类型禁止**产出 `Arc<dyn Plugin>`；`PluginRuntimeLoader::load` 仅由 `PluginRegistry::activate` 调用。Loader 实现不得反向依赖 `PluginRegistry` 内部状态 |
| `harness-plugin` ↔ TrustedSignerStore（ADR-0014） | `harness-plugin` 仅依赖 `TrustedSignerStore` trait；默认实现 `StaticTrustedSignerStore` 在 `harness-plugin` 内部，与 ADR-0013 `IntegritySigner`（`harness-permission`）**不共享**密钥与配置 |

## 7. 代码所有权

| Crate | 所有者（TL） | 评审者 |
|---|---|---|
| `harness-contracts` | Arch | All |
| L1 原语 | Core | Arch + Consumer crate TL |
| L2 复合能力 | Core | Arch |
| L3 引擎与协作 | Agent Runtime | Arch |
| L4 门面 | API Surface | Arch + Consumer（业务层 TL）|

> **所有者**负责 API 稳定性与文档维护；**评审者**对跨层变更强制加签。

## 8. 变更治理

1. 新增 crate → 必须先提交 ADR
2. 新增 trait 或公开类型 → 必须同步更新该 crate SPEC + `api-contracts.md` 总表
3. 破坏性修改 → 必须同步更新 `overview.md` §10 决策一览 + 对应 ADR
4. Feature flag 调整 → 必须同步更新 `feature-flags.md`

## 9. 编译期强制

在 CI 中加入 `cargo-deny` 规则：

```toml
[graph]
targets = ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"]

[deny]
# 禁止 L1 crate 间相互依赖（示例）
multiple-versions = "deny"
```

此外，使用 `cargo-depgraph` 或 `cargo-modules` 定期生成实际依赖图，与本文档图对比（CI 产物存档）。

## 10. 例外登记

如需打破本文档规则的例外情况，必须在此表登记：

| 日期 | Crate A | Crate B | 原因 | ADR 链接 |
|---|---|---|---|---|
| — | — | — | — | — |

> 空表即为"目前无例外"。每新增一行必须附 ADR 链接。
