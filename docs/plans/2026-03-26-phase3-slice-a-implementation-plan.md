# Slice A Implementation Plan

**状态**: `approved`
**日期**: `2026-03-26`
**owner**: `Codex`
**release_slice**: `GA`

## 1. 目标

- 本次切片目标：为 `Chat 发起 task -> Run 执行 / 阻塞 -> ApprovalRequest -> Inbox 处理 -> Trace 可回放` 制定可直接执行的 Phase 4/5 实施步骤。
- 不在本次范围内：`Board`、`Knowledge`、`Remote Hub` server、`packages/`、`apps/octopus-web/`、`Notification` 渠道、`CapabilityVisibility / ToolSearch` 独立工作面。

## 2. 输入依据

- 对应 Task Slice Card：
  - [`docs/plans/2026-03-26-phase3-slice-a-task-slice-card.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/2026-03-26-phase3-slice-a-task-slice-card.md)
- 对应 PRD 章节：
  - `2.2 使用模式`
  - `2.5 发版切片`
  - `3.2 统一术语表`
  - `3.4 领域不变量`
- 对应 SAD 章节：
  - `1.3 核心设计原则`
  - `1.5 架构平面`
  - `4.4 Runtime Plane`
  - `4.8 Observation Layer`
  - `5.1 统一运行时模型`
  - `6.3 状态机`
- 对应 contract / ADR / visual doc：
  - [`docs/plans/2026-03-26-phase1-project-skeleton-design.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/2026-03-26-phase1-project-skeleton-design.md)
  - [`docs/adr/20260326-phase1-minimal-repo-topology.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/adr/20260326-phase1-minimal-repo-topology.md)
  - [`docs/contracts/runtime/run.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/runtime/run.md)
  - [`docs/contracts/governance/approval-request.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/governance/approval-request.md)
  - [`docs/contracts/interaction/inbox-item.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/interaction/inbox-item.md)
  - [`docs/contracts/runtime/trace-event.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/runtime/trace-event.md)
  - [`docs/VISUAL_FRAMEWORK.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VISUAL_FRAMEWORK.md)

## 3. 影响对象

- 正式对象：`Run`, `ApprovalRequest`, `InboxItem`, `TraceEvent`
- 所属平面：`Interaction`, `Runtime`, `Governance`, `Observation`
- 所属交互面：`Chat`, `Inbox`, `Trace`

## 4. 实施步骤

### Step 1

- 要做什么：
  - 建立 Phase 4 最小代码骨架，只创建 `apps/octopus-desktop/` 与 `crates/octopus-hub/`
  - 同步加入 `apps/octopus-desktop/AGENTS.md` 与 `crates/AGENTS.md`
  - 仅创建首个切片所需的最小 manifest、构建入口和验证入口
- 产出物：
  - `apps/octopus-desktop/`
  - `crates/octopus-hub/`
  - 最小 manifest / entrypoint / local AGENTS
- 完成判定：
  - 仓库中不存在额外的 `packages/`、`crates/octopus-server/`、`apps/octopus-web/`
  - 代码骨架足以承载 Slice A 的单机实现

### Step 2

- 要做什么：
  - 在 `crates/octopus-hub/` 中落最小 authority flow
  - 以 `Run` 为权威执行壳，实现 task 提交、审批阻塞、审批结果回写、Trace 追加、Inbox 投影的最小对象流
  - 不在前端壳层重建对象语义；前端只消费 Hub 投影或 application-level view model
- 产出物：
  - `Run` 生命周期最小用例
  - `ApprovalRequest` 最小治理流程
  - `InboxItem` 投影与处理结果
  - `TraceEvent` 追加式时间线
- 完成判定：
  - 单条任务链路可以从创建 `Run` 进入 `running` 或 `waiting_approval`
  - 审批处理能改变 `Run` 结果并同步更新 `Inbox` 与 `Trace`

### Step 3

- 要做什么：
  - 在 `apps/octopus-desktop/` 中实现 `Chat`、`Inbox`、`Trace` 三个工作面
  - `Chat` 负责发起 task、显示当前 `Run` 状态和审批阻塞 callout
  - `Inbox` 负责处理审批待办
  - `Trace` 负责展示当前 `Run` 的时间线与事件详情
- 产出物：
  - 最小应用壳层
  - `Chat` 页面
  - `Inbox` 页面
  - `Trace` 页面
  - 围绕正式对象的领域组件
- 完成判定：
  - 用户能沿 `Chat -> Inbox -> Trace` 看见同一条对象链路
  - `Inbox` 没有被做成 `Notification` feed
  - `Trace` 没有被做成散乱 debug log

### Step 4

- 要做什么：
  - 补齐切片级验证与文档同步
  - 当实现树进入 tracked tree 后，运行真实可支撑的构建、测试和状态回归
  - 若实现中发现对象语义漂移，先回到 contract / plan 修正，再继续
- 产出物：
  - 当前仓库条件下的验证记录
  - 必要的文档同步
- 完成判定：
  - Phase 5 的完成结论建立在 fresh verification 上
  - 没有把目标态能力描述成已实现现实

## 5. 实现级接口边界

本切片需要的只是最小实现级接口边界，不构成公共 API 承诺：

- Desktop 侧至少需要：
  - task 提交入口
  - 当前 `Run` 详情读取
  - `InboxItem` 列表与详情读取
  - 审批处理动作
  - `TraceEvent` 列表读取
- Hub 侧至少需要：
  - 创建 task 型 `Run`
  - 推进 `Run` 状态
  - 创建与处理 `ApprovalRequest`
  - 创建与更新 `InboxItem`
  - 追加 `TraceEvent`

约束：

- 这些接口边界是 Phase 4/5 的实现缝，不是对外 HTTP API 承诺
- 若远程模式需要 wire API，留到 Phase 6 与 `crates/octopus-server/` 一并定义

## 6. 文档同步

- 需要同步哪些文档：
  - `README.md`：若实现树首次进入 tracked tree，需补入口导航
  - `AGENTS.md`：仅在仓库级规则变化时更新
  - `docs/SAD.md`：仅在实现反推出必须修正的架构边界时更新
  - `docs/plans/2026-03-26-ga-rebuild-project-development-plan.md`：根据 Phase 4/5 完成度更新进度
- 为什么需要同步：
  - 防止文档仍停留在“只有文档、没有实现”的旧状态
  - 防止代码实现与已批准的 contract / IA / topology 漂移

## 7. 验收条件

- 验收条件 1：`Run` 仍是权威执行壳，审批与 Trace 没有被前端局部交互替代
- 验收条件 2：用户能完成单条主链路 `Chat -> Run -> Approval -> Inbox -> Trace`
- 验收条件 3：实现没有提前引入 `packages/`、`crates/octopus-server/`、`apps/octopus-web/` 或其他与 Slice A 无关的目录

## 8. 验证计划

- 当前仓库可执行验证：
  - 当前阶段仅能验证文档存在性、stale references、focused diff、`git diff --check`
- 后续实现阶段需要补充的验证：
  - 最小构建验证
  - `Run` / `ApprovalRequest` / `InboxItem` / `TraceEvent` 状态与回放测试
  - 单条主链路的交互回归
  - 权威对象与前端投影的一致性检查

## 9. 风险与回退

- 主要风险：
  - 为了“完整感”提前恢复过多目录
  - 让 UI 先行并反向发明对象语义
  - 把审批或 Trace 做成不可审计的临时交互
- 若失败如何收敛问题边界：
  - 先判断问题属于 topology、contract、Hub authority flow 还是前端投影
  - 只修正最小边界问题，再重新验证主链路
- 需要人工确认的点：
  - 只有当 Phase 4/5 需要引入新平台表面、变更 `GA/Beta/Later` 边界或改变核心对象语义时，才需要重新请求确认
