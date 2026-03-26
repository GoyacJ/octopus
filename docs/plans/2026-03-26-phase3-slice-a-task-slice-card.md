# Phase 3 Slice A Task Slice Card

## 1. 基本信息

- `task_name`: `phase3-slice-a-chat-run-approval-inbox-trace`
- `request_type`: `design`
- `secondary_type`: `implementation`
- `owner`: `Codex`
- `date`: `2026-03-26`

## 2. 目标

- 本次要解决什么问题：为首个 GA 垂直切片固定交互边界、对象映射和实施顺序，使后续 Phase 4/5 可以直接按 Slice A 进入最小代码骨架和真实实现。
- 为什么现在做：Phase 1 最小 topology 与 Phase 2 首批 contract 已经收口；当前缺的是能把 `Chat -> Run -> Approval -> Inbox -> Trace` 交给实现者的 IA 设计与 implementation plan。

## 3. 作用范围

- `release_slice`: `GA`
- `planes`: `Interaction`, `Runtime`, `Governance`, `Observation`
- `surfaces`: `Desktop` 首轮最小工作面中的 `Chat`、`Inbox`、`Trace`
- `affected_objects`: `Run`, `ApprovalRequest`, `InboxItem`, `TraceEvent`

## 4. 边界检查

- 是否会改变产品范围：`no`
- 是否会改变架构主决策：`no`
- 是否会新增平台表面：`no`
- 是否涉及高风险能力或安全姿态变化：`no`
- 是否需要人工确认：`no`

若任一项为 `yes`，写明原因：

- 无。本切片只把已批准的 GA 范围与 contract 收敛成 Phase 4/5 可执行前置物。

## 5. 前置产物判断

- 是否需要 `ADR`：`no`
- 是否需要 `contract`：`no`，直接引用已批准的首批 contract
- 是否需要 `implementation plan`：`yes`
- 是否需要视觉/IA 设计：`yes`
- 是否需要骨架设计说明：`no`，由 Phase 1 文档覆盖

## 6. 验收条件

- 成功条件 1：Slice A 明确限定在 `Chat`、`Inbox`、`Trace` 三个工作面，不把 `Board`、`Knowledge`、`Remote Hub server` 或 `packages/` 带入当前实现边界。
- 成功条件 2：IA 文档明确描述主链路、页面布局、跨页面跳转和对象状态表达，能够直接映射 `Run`、`ApprovalRequest`、`InboxItem`、`TraceEvent`。
- 成功条件 3：implementation plan 明确说明 Phase 4/5 的实施顺序、最小接口边界、验收条件与验证方式，不留“边做边想”的空间。

## 7. 验证方式

- 当前仓库可实际执行的验证：确认新增文档存在；搜索相关 stale references；审阅 focused diff；运行 `git diff --check`
- 不能声称执行的验证：`pnpm` / `cargo` / app runtime / automated tests 通过

## 8. 风险与停机点

- 主要风险：把 Slice A 误扩成全量 GA 控制台；把审批重新做成弹窗交互；把 Inbox 降级成 Notification feed；把 Trace 写成调试日志而非正式对象时间线。
- 发现以下情况时必须停下：需要引入 `Board` 作为必选主工作面；需要提前恢复 `packages/`、`crates/octopus-server/` 或 `apps/octopus-web/`；需要为切片发明未在 `PRD/SAD/contract` 中定义的新正式对象。

## 9. 输出物

- 预计新增或修改的文档/文件：
  - `docs/plans/2026-03-26-phase3-slice-a-task-slice-card.md`
  - `docs/plans/2026-03-26-phase3-slice-a-ia-design.md`
  - `docs/plans/2026-03-26-phase3-slice-a-implementation-plan.md`
  - `docs/plans/2026-03-26-ga-rebuild-project-development-plan.md`
- 最终汇报需要说明的重点：Slice A 只覆盖哪些工作面；哪些对象直接进入实现；哪些能力明确留到后续阶段；当前验证仍停留在文档层
