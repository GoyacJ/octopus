# Octopus Agent Harness SDK · 架构文档集

> 版本：1.8
> 状态：Accepted · 本文档集为开发基线，任何偏离需先提交 ADR
> 上次更新：2026-04-25

## 0. 当前基线索引

本 README 只承载当前架构基线索引。历史修订记录见 `CHANGELOG.md`；如历史摘要与 D1-D10、ADR 或 crate SPEC 冲突，以当前 Accepted 文档为准。

本目录存放 **Octopus Agent Harness SDK**（19 个 crate）的**正式架构设计文档**，作为项目治理与开发的唯一基线。所有新增模块、接口变更、破坏性修改，必须先更新对应文档并补充 ADR。

### 0.1 阅读顺序推荐

| 读者 | 推荐入口 | 覆盖时间 |
|---|---|---|
| 新成员/评审者 | `overview.md` → `module-boundaries.md` → 感兴趣的 crate SPEC | 30~60 分钟 |
| 接入业务方 | `overview.md` → `api-contracts.md` → `feature-flags.md` → `crates/harness-sdk.md` | 20~30 分钟 |
| 内核开发者 | `overview.md` → `module-boundaries.md` → `api-contracts.md` → 全部 ADR → 相关 crate SPEC | 2~4 小时 |
| 安全/合规 | `security-trust.md` → `permission-model.md` → ADR-006/007 | 30 分钟 |

### 0.2 文档组织

```
docs/architecture/harness/
├── README.md                       ← 本文（当前基线索引）
├── CHANGELOG.md                    ← 架构变更日志
├── overview.md                     ← D1 · 架构总览（SAD）
├── module-boundaries.md            ← D2 · 模块边界与依赖矩阵
├── api-contracts.md                ← D3 · 接口契约规范
├── event-schema.md                 ← D4 · Event Schema 与 Replay 语义
├── permission-model.md             ← D5 · 权限模型
├── agents-design.md                ← D6 · Subagent + Team 详解
├── extensibility.md                ← D7 · 扩展性规范（Tool/Hook/Plugin/Skill/MCP）
├── context-engineering.md          ← D8 · 上下文工程
├── security-trust.md               ← D9 · 安全与信任域
├── feature-flags.md                ← D10 · Feature Flag 手册
├── adr/                            ← 架构决策记录
│   ├── 0001-event-sourcing.md
│   ├── 0002-tool-no-ui.md
│   ├── 0003-prompt-cache-locked.md
│   ├── 0004-agent-team-topology.md
│   ├── 0005-mcp-bidirectional.md
│   ├── 0006-plugin-trust-levels.md
│   ├── 0007-permission-events.md
│   ├── 0008-crate-layout.md
│   ├── 0009-deferred-tool-loading.md
│   ├── 0010-tool-result-budget.md
│   ├── 0011-tool-capability-handle.md
│   ├── 0012-capability-testing-boundary.md
│   ├── 0013-integrity-signer.md
│   ├── 0014-plugin-manifest-signer.md
│   ├── 0015-plugin-loader-capability-handles.md
│   ├── 0016-programmatic-tool-calling.md
│   ├── 0017-steering-queue.md
│   └── 0018-no-loop-intercepted-tools.md
└── crates/                         ← 19 个 crate 的 SPEC
    ├── harness-contracts.md
    ├── harness-model.md
    ├── harness-journal.md
    ├── harness-sandbox.md
    ├── harness-permission.md
    ├── harness-memory.md
    ├── harness-tool.md
    ├── harness-tool-search.md
    ├── harness-skill.md
    ├── harness-mcp.md
    ├── harness-hook.md
    ├── harness-context.md
    ├── harness-session.md
    ├── harness-engine.md
    ├── harness-subagent.md
    ├── harness-team.md
    ├── harness-plugin.md
    ├── harness-observability.md
    └── harness-sdk.md
```

## 1. 治理规则

1. **变更流程**：接口/边界变更 → 先开 ADR → 经评审后更新 SPEC → 再改代码
2. **证据引用**：所有设计主张必须引用 `docs/architecture/reference-analysis/evidence-index.md` 中的 ID（`HER-xxx` / `OC-xx` / `CC-xx`）
3. **命名规范**：crate 名用 `octopus-harness-*`（工作空间内路径）/ crate 内模块使用 `harness_xxx` 前缀（Rust module 命名）
4. **强制约束**：本文档集中所有 `MUST / MUST NOT / SHOULD` 与 Prompt Cache 硬约束（ADR-003）不得违反

## 2. 交叉引用矩阵

| 主文档 | 强依赖 ADR | 强依赖 SPEC |
|---|---|---|
| overview.md | 全部 | 全部 |
| module-boundaries.md | ADR-008 | — |
| api-contracts.md | ADR-002, ADR-007, ADR-0017 | 全部 |
| event-schema.md | ADR-001, ADR-007, ADR-0016, ADR-0017 | harness-contracts, harness-journal |
| permission-model.md | ADR-007 | harness-permission |
| agents-design.md | ADR-003, ADR-004, ADR-0016 | harness-engine, harness-subagent, harness-team |
| extensibility.md | ADR-002, ADR-005, ADR-006, ADR-0015, ADR-0016 | harness-tool, harness-hook, harness-plugin, harness-skill, harness-mcp, harness-sandbox |
| context-engineering.md | ADR-003 | harness-context, harness-memory |
| security-trust.md | ADR-006, ADR-007, ADR-0014, ADR-0016, ADR-0017 | harness-permission, harness-sandbox, harness-plugin |
| feature-flags.md | ADR-008, ADR-0016, ADR-0017 | harness-sdk |

## 3. 工程遗产迁移提示

本 SDK 采用**全部重建策略**。现 `crates/octopus-sdk*` 系列（14 个）**全部删除**，见 `crates/harness-sdk.md` §6 与 ADR-008。

保留的业务基础设施 crate（不在 SDK 范畴）：`octopus-core / octopus-persistence / octopus-platform / octopus-infra / octopus-server / octopus-desktop / octopus-cli`。

## 4. 质量标准

- **正确性**：每个 trait 签名、Event 字段、配置项必须可编译（语法正确）、可实现（无歧义）
- **可维护性**：文档粒度与 crate 粒度对齐，一个 crate 一份 SPEC；避免多处重复描述同一事实，相互引用
- **可审计性**：所有关键决策有 ADR 记录；所有证据有索引

## 5. 变更日志

历史修订记录已移至 `CHANGELOG.md`。README 不再承载长篇版本叙事。

### 5.1 版本索引

- 1.8（2026-04-25）：PTC、Steering Queue、Loop-Intercepted Tools 反向决议。
- 1.7（2026-04-25）：Plugin signer、Loader 二分、capability-scoped activation。
- 1.6（2026-04-25）：Hook 能力矩阵、transport 安全字段、replay 幂等。
- 1.5（2026-04-25）：MCP 三维来源/信任/生命周期、sampling 与重连治理。
- 1.4（2026-04-25）：Memory store/lifecycle 拆分、Memdir、Recall 与威胁扫描。
- 1.3.x（2026-04-25）：ToolStream、ResultBudget、Capability Handle、内置工具扩展。
- 1.2（2026-04-25）：Deferred Tool Loading 与 `harness-tool-search`。
- 1.1（2026-04-24）：首轮架构审计后的 P0-P3 文档修订。
- **1.0（2026-04-24）**：初版定稿。
