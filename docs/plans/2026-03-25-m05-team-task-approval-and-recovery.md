# M05 Team、Task、Approval 与 Recovery 实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-team-task-approval-and-recovery.md`
- Objective: `把多 Agent 任务执行链路拆成正式可实现合同，并明确依赖的前置冻结项。`

## Inputs

- `docs/PRD.md`
- `docs/SAD.md`
- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/API/TEAM.md`
- `docs/API/EVENTS.md`

## Contracts To Freeze

- `Team`、`LeaderPlanningService`、路由规则和成员边界。
- Task DAG、Decision 队列、审批模式、高风险强制审批和终止/恢复语义。
- 等待、恢复、时间线、结果汇总、trace/audit 与导出边界。
- 本里程碑对 `M1-M4` 的依赖字段和引用方式。

## Repo Reality

- 任务执行链当前只存在文档级流程和领域对象，没有事件流或状态机实现。
- 审批、恢复和 trace 语义依赖 `M1` 的事件模型与 `M3` 的治理合同，不能越级实现。

## Deliverables

- Team 与 Task 的正式对象清单。
- Task 状态迁移与审批/恢复流程图。
- 依赖矩阵：本里程碑消费哪些前置冻结合同。

## Verification

- 对 `LeaderPlanningService`、`depends_on`、审批、恢复和时间线关键词做一致性 grep。
- 检查 `DOMAIN / DATA_MODEL / PRD / API` 对 Task 和 Team 的对象关系没有冲突。
- 检查高风险动作与 `M3` 的 capability governance 联动清晰可追溯。

## Docs Sync

- `docs/PRD.md`
- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/API/TEAM.md`
- `docs/API/EVENTS.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-team-task-approval-and-recovery.md`

## Open Risks

- 若不显式写出对 `M1-M4` 的依赖，任务执行会重新定义运行时对象、Agent 合同或能力治理字段。

## Out Of Scope

- Discussion 专属调度与结论机制。
- Blueprint 与模板导入导出。
