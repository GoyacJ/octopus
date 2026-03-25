# M06 Discussion Engine 实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-discussion-engine.md`
- Objective: `把 Discussion Engine 从产品功能描述拆成独立的实现合同，并明确它与 TaskEngine 的边界。`

## Inputs

- `docs/PRD.md`
- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/DOMAIN.md`
- `docs/API/DISCUSSIONS.md`

## Contracts To Freeze

- Discussion 模式：`Roundtable / Brainstorm / Debate`。
- 调度与主持：顺序轮流、主持人指定、用户插话、暂停/继续/结束。
- 结论生成、结论编辑、历史检索、续会、导出和记忆写回。
- Discussion 与 Task 的职责边界、共用合同和禁止复用点。

## Repo Reality

- 当前文档同时存在“Discussion 是独立引擎”和“与 Task 共享部分运行时语义”的描述，实施时必须先冻结接口边界。

## Deliverables

- Discussion 对象与状态边界表。
- 调度策略与用户插话流程图。
- 与 TaskEngine 的边界说明和复用清单。

## Verification

- 对 `DiscussionEngine`、模式名、主持人、结论、记忆写回和 `TaskEngine` 边界做一致性 grep。
- 检查 `PRD / SAD / ARCHITECTURE / DOMAIN / API` 的 Discussion 语义一致。
- 检查高风险工具限制与 `M3` 的能力治理合同一致。

## Docs Sync

- `docs/PRD.md`
- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/DOMAIN.md`
- `docs/API/DISCUSSIONS.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-discussion-engine.md`

## Open Risks

- 若不明确与 Task 的边界，Discussion 很容易复用错误的状态机和审批假设。

## Out Of Scope

- Team/Task 主执行链。
- 模型中心和 Provider 管理。
