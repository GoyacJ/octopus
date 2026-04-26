# Octopus Agent Harness SDK · 行动 Plan

> 版本：1.0（首发）
> 状态：**Active · 执行中**
> 上次更新：2026-04-26
> 架构基线：[`docs/architecture/harness/`](../../architecture/harness/) v1.8.1
> 开发模式：**Vibecoding（Codex AI 主导 + 人类把关）**

---

## 0. 本 Plan 是什么

把 [`docs/architecture/harness/`](../../architecture/harness/) 中的 19 个 crate 架构 SPEC 与 18 份 ADR，拆解成 **AI（Codex）单次会话可闭环、CI 可自动验收** 的最小任务卡集合，按 5 层依赖单向收敛交付。

**前置条件**：架构基线 v1.8.1 已 Accepted，所有 trait 签名 / 事件 / Feature 在 SPEC 中已固定。**任何偏离都必须先开 ADR，不得在 plan / 代码层私自决议**。

**非目标**：
- 本 Plan **不重新设计架构**，只把架构落成代码
- 本 Plan **不承担产品层规划**（Desktop UI / Server API 排期由产品 Plan 负责）
- 本 Plan **不替代 ADR**：如发现 SPEC 缺陷，必须回写 ADR + 修订 SPEC，再回流到本 Plan

---

## 1. 关键决策（已确认）

| 决策项 | 取舍 | 理由 |
|---|---|---|
| **范围** | 全量 19 crate + 业务层切换（M0~M9） | SDK 是基础设施，半成品对业务无价值 |
| **AI 并行度** | M2/L1 与 M4/L2 扩展层并行 5~3 路；L0/L4/M3 串行 | L1 五原语正交无耦合；聚合层须串行避免分支冲突 |
| **旧 SDK** | M0 冻结保留旧 SDK；新 `octopus-harness-*` 并行实现；**M3 末 cli 最简入口先行接入 lower-level harness driver（非 facade）**；M8 业务全切后删除旧 SDK | 业务层在 M0~M7 继续可运行；新 harness 不依赖旧 SDK，避免套壳；M3 渐进切换让真集成风险前移到 M3 而非 M8；`octopus-harness-sdk` 门面仍在 M7 交付 |
| **测试** | 严格：每 crate mock + contract-test + ≥1 正反用例 | 对齐 ADR-012；AI 易在边界条件偷工，必须用例兜底 |
| **验证时机** | **3 个 spike 前置（M2/M3 末尾），M9 只做 post-spike 集成回归与端到端验收** | 评审报告 §4.4 三个高风险点的失败代价是 2-4 周返工，必须在 M5/M6/M7 之前用最小 prototype 把假设钉死 |

---

## 2. 阅读顺序

| 角色 | 必读 | 选读 |
|---|---|---|
| **Codex AI（执行者）** | `02-task-template.md` → 当前里程碑 → 当前任务卡指向的 SPEC 锚点 | `04-context-anchoring.md` |
| **架构评审者** | `01-roadmap.md` → 各里程碑入口段 | `00-strategy.md` |
| **新人** | `README.md`（本文） → `00-strategy.md` → `01-roadmap.md` | 全部 |
| **CI 维护者** | `03-quality-gates.md` | `02-task-template.md` |
| **业务层接入方** | M8 + M9 | M3（最小可运行点）|

---

## 3. 文档地图

```
docs/plans/harness-sdk/
├── README.md                       ← 本文（总入口）
├── 00-strategy.md                  ← Vibecoding 五条铁律 + AI 工作流
├── 01-roadmap.md                   ← M0~M9 DAG + 并行矩阵 + Review Gate
├── 02-task-template.md             ← AI 任务卡标准模板 + Codex Prompt 骨架
├── 03-quality-gates.md             ← 5 道质量闸门 + CI 矩阵
├── 04-context-anchoring.md         ← 防 AI 幻觉的上下文锚定规范
└── milestones/
    ├── M0-bootstrap.md             ← 工作空间脚手架 + 旧 SDK 冻结
    ├── M1-l0-contracts.md          ← L0 契约层（harness-contracts）
    ├── M2-l1-primitives.md         ← L1 五原语并行（model/journal/sandbox/permission/memory）
    ├── M3-l2-core.md               ← L2 核心闭环（tool/hook/context/session）
    ├── M4-l2-extensions.md         ← L2 扩展（tool-search/skill/mcp）
    ├── M5-l3-engine.md             ← L3 单 Agent（engine/observability/plugin）
    ├── M6-l3-agents.md             ← L3 多 Agent（subagent/team）
    ├── M7-l4-facade.md             ← L4 门面（harness-sdk）
    ├── M8-business-cutover.md      ← 业务层切换（octopus-server/desktop/cli）
    └── M9-poc-and-acceptance.md    ← post-spike 集成验证 + E2E 验收
```

---

## 4. 成功判据（Definition of Done）

整个 Plan 完成判据：

1. ✅ 19 个 `octopus-harness-*` crate 全部进入 workspace，`cargo check --workspace --all-features` 通过
2. ✅ M8 业务切换完成后，14 个旧 `octopus-sdk*` crate 已从仓库移除
3. ✅ `octopus-server / octopus-desktop / octopus-cli` 已切到 `octopus-harness-sdk`
4. ✅ `cargo test --workspace --all-features` 全部 green
5. ✅ `cargo clippy --workspace --all-targets -- -D warnings` 零警告
6. ✅ `cargo deny check` 通过（含 D2 §10 例外登记表的 feature 矩阵）
7. ✅ M9 三个 post-spike 集成验证报告归档
8. ✅ `apps/desktop` Tauri 端可启动并跑通"用户提问 → 工具调用 → 流式输出"完整闭环

---

## 5. 治理规则

| 规则 | 说明 |
|---|---|
| **Plan 不变更政策** | 本目录文档**仅描述执行计划**；任何架构层规则变更必须先改 [`docs/architecture/harness/`](../../architecture/harness/) 与 ADR，再回流到本 Plan（对齐 [`docs/AGENTS.md`](../../AGENTS.md)）|
| **任务卡幂等** | 每张任务卡描述"实现一次后即冻结"；后续修改走新任务卡，不改老卡 |
| **Plan 滚动更新** | 每完成一个里程碑，更新 `01-roadmap.md` 进度块 + 关闭对应 milestone 的所有任务卡 |
| **失败回滚** | 任何任务卡 5 道闸门未通过 → 任务卡作废 → 重新派发新卡，不允许"修复后强行合并"|

---

## 6. 进度跟踪

> 由 maintainer 在 `01-roadmap.md` 进度块手动更新，本 README 不做实时同步以避免维护成本。

| 里程碑 | 状态 | 关键交付 |
|---|---|---|
| M0 Bootstrap | 已完成 | workspace 共存整理 + 旧 SDK 冻结 + 19 crate 空骨架 |
| M1 L0 Contracts | 已完成 | `harness-contracts` 全量类型 + Redactor trait + NoopRedactor |
| M2 L1 Primitives | 已完成 | 5 原语 trait + 全量 model Provider + builtin（EventStore 装配 Redactor） |
| M3 L2 Core | 进行中 | M3-T01 本地提交待评审；M3-T02 本地实现待评审 |
| M4 L2 Extensions | 待开始 | tool-search / skill / mcp |
| M5 L3 Engine | 待开始 | 单 Agent 主循环（DefaultRedactor 替换 Noop）|
| M6 L3 Agents | 待开始 | Subagent + Team |
| M7 L4 Facade | 待开始 | `harness-sdk` 整合 |
| M8 Business Cutover | 待开始 | 业务层完全迁移 + 14 个旧 `octopus-sdk*` crate `git rm` |
| M9 Integration Verification + Acceptance | 待开始 | 集成回归 + 端到端验收报告 |

---

## 7. 索引

- **架构基线** → [`docs/architecture/harness/README.md`](../../architecture/harness/README.md)
- **架构评审报告** → [`docs/architecture/harness/audit/2026-04-25-architecture-review.md`](../../architecture/harness/audit/2026-04-25-architecture-review.md)
- **仓库治理** → [`AGENTS.md`](../../../AGENTS.md)
- **持久化治理** → `AGENTS.md` § Persistence Governance
