# Run Contract

**状态**: `approved`
**日期**: `2026-03-26`
**所属平面**: `Runtime`
**所属对象**: `Run`
**适用切片**: `GA`

## 1. 背景

- 为什么需要这份 contract：`Run` 是 Octopus 所有正式执行的权威外壳。Phase 2 必须先固定 `Run` 的最小 contract，才能让后续 Slice A、审批、Trace 和最小代码骨架都围绕同一对象语义展开。
- 它约束什么，不约束什么：它约束 `Run` 的对象职责、最小字段、生命周期、治理约束和恢复要求；不约束具体 API 路由、数据库表结构、前端 view model 或模型计划算法。

## 2. 关联真相源

- `PRD` 相关章节：
  - `2.2 使用模式`
  - `3.2 统一术语表`
  - `3.4 领域不变量`
  - `4.1 Agent Runtime`
- `SAD` 相关章节：
  - `1.3 核心设计原则`
  - `3.1 边界上下文`
  - `4.4 Runtime Plane`
  - `5.1 统一运行时模型`
  - `5.2 人工发起 Run`
  - `6.3 Run 状态机`
  - `6.4 版本、幂等与恢复策略`
- 其他规范：
  - [`docs/ENGINEERING_STANDARD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/ENGINEERING_STANDARD.md)
  - [`docs/CODING_STANDARD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/CODING_STANDARD.md)

## 3. 对象与职责

- `authority_source`: `Hub` 中的 `Run Orchestration`
- `projections_or_cache`: Client 侧运行快照、Board/Trace/Inbox 视图投影、只读缓存
- `actors`: `user`, `agent`, `scheduler`, `trigger source`, `reviewer`, `runtime orchestrator`
- `scope`: 至少绑定 `Workspace`；可选绑定 `Project`；在当前上下文中与 `Agent`、`Team`、`Automation`、`Trigger`、`ApprovalRequest`、`Artifact`、`Knowledge`、`EnvironmentLease` 发生关系

## 4. 数据结构

### 4.1 核心字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `RunId` | `yes` | Run 权威标识 |
| `run_type` | `task | discussion | automation | watch | delegation | review` | `yes` | 正式运行类型；首版 GA 重点交付 `task`、`automation`、`review`、受控 `watch` |
| `status` | `RunStatus` | `yes` | 生命周期状态 |
| `workspace_ref` | `WorkspaceRef` | `yes` | 运行所属协作边界 |
| `project_ref` | `ProjectRef` | `no` | 运行所属业务上下文；允许为空以支持非项目级运行 |
| `initiator_ref` | `ActorRef` | `yes` | 本次运行由谁或什么触发 |
| `trigger_ref` | `TriggerRef` | `no` | 若运行来自 Automation / event，则引用正式触发器 |
| `subject_ref` | `TaskRef | DiscussionSessionRef | AutomationRef | DelegationRef | ReviewRef` | `no` | 运行服务的业务对象；并非所有 run_type 都要求独立业务对象 |
| `plan_summary` | `PlanSummary` | `no` | 规划阶段产出的当前计划摘要；不要求等于完整执行计划全文 |
| `checkpoint_ref` | `CheckpointRef` | `no` | 最近恢复点 |
| `environment_lease_ref` | `EnvironmentLeaseRef` | `no` | 当前受控执行环境租约 |
| `budget_policy_ref` | `BudgetPolicyRef` | `no` | 当前资源与时窗约束 |
| `active_approval_ref` | `ApprovalRequestRef` | `no` | 当前正在阻塞运行的审批对象 |
| `artifact_refs` | `ArtifactRef[]` | `no` | 与本次运行相关的正式结果对象 |
| `knowledge_effect_refs` | `KnowledgeCandidateRef[] | KnowledgeAssetRef[]` | `no` | 运行引发的候选知识或写回结果引用 |
| `idempotency_key` | `string` | `no` | 触发或恢复链路的去重键 |

### 4.2 状态枚举

| 状态 | 含义 | 进入条件 | 退出条件 |
| --- | --- | --- | --- |
| `queued` | 运行已创建，等待调度 | 意图或触发器已被接受 | 开始规划 |
| `planning` | 正在形成或修订计划 | `queued` 后进入规划 | 进入 `running`，或规划失败 |
| `running` | 正在执行动作或推进流程 | 规划完成、恢复成功、输入补齐、审批放行、依赖满足后继续 | 进入等待、暂停、恢复、完成、失败、终止或取消 |
| `waiting_input` | 缺少用户或外部明确输入 | 运行需要补充信息 | 输入满足后回到 `running` |
| `waiting_approval` | 运行被审批门控阻塞 | 产生 `ApprovalRequest` | 审批批准后回到 `running`；或被拒绝/过期后转失败或终止 |
| `waiting_dependency` | 运行在等待依赖完成 | 依赖对象尚未满足 | 依赖满足后回到 `running` |
| `paused` | 被人工或系统暂停 | 需要暂停推进 | 恢复为 `running`，或转 `terminated` |
| `recovering` | 正在根据 checkpoint / freshness check 恢复 | Hub 重启、执行环境抖动或恢复流程启动 | 恢复成功回到 `running` |
| `completed` | 主流程成功完成 | 已达成运行结束条件 | 终态 |
| `failed` | 主流程失败且未恢复 | 规划、执行、恢复等环节失败 | 终态，或由新的 Run 承接重试 |
| `terminated` | 被显式终止 | grant 撤销、人工终止或不可继续 | 终态 |
| `cancelled` | 在开始或执行中被取消 | 请求方取消且允许取消 | 终态 |

### 4.3 关键关系

- 与哪些正式对象相关：`Task`、`Automation`、`Trigger`、`ApprovalRequest`、`Artifact`、`KnowledgeCandidate`、`EnvironmentLease`、`TraceEvent`
- 谁创建：`Runtime Plane`
- 谁更新：`Run Orchestrator`；在治理动作放行或阻断时受 `Governance Plane` 驱动
- 谁消费：`Chat`、`Board`、`Trace`、`Inbox`、`Knowledge`、`Observation Layer`

## 5. 流程与规则

- 正常路径：`Intent -> Trigger -> Run -> Plan -> Action -> Policy Check -> Artifact / Knowledge -> completed`
- 异常路径：规划失败、环境租约失败、审批阻塞、依赖阻塞、恢复失败、政策拒绝都必须显式进入 `status` 变化或关联治理对象，不能隐式吞没
- 幂等要求：来自 Trigger、审批处理、外部回执或恢复链路的关键动作必须可通过 `idempotency_key` 去重
- 恢复要求：`Run` 必须支持 checkpoint；Hub 重启后只续跑未确认完成的动作，并对 grant、budget、approval、peer、environment 做 freshness check
- 审计要求：关键状态变化、工具调用、审批命中、预算命中、异常与恢复都必须产出 `TraceEvent` 和相关观测记录

## 6. 治理约束

- 是否受 `Role / Permission` 影响：`yes`，创建、查看、终止和恢复都受主体权限影响
- 是否受 `CapabilityGrant` 影响：`yes`，运行中的真实动作不能绕过 grant
- 是否受 `BudgetPolicy` 影响：`yes`，预算只约束资源和时窗，不授予新能力
- 是否受 `ApprovalRequest` 影响：`yes`，越界或高风险动作通过审批决定是否继续
- 是否涉及 `Knowledge Write Gate`：`yes`，仅在运行结果写回知识时生效；不是 Run 创建本身的前置条件

## 7. 非目标

- 这份 contract 不解决什么：
  - `Task`、`DiscussionSession`、`Automation` 各自的完整业务对象 schema
  - 计划生成算法与模型编排
  - UI 上如何排版 Run 状态
  - 数据库存储表与 API wire 格式

## 8. 验收与验证

- 验收条件：
  - `Run` 的状态机、actor、scope、治理约束和恢复边界不再依赖聊天上下文解释
  - Slice A 可直接引用本 contract 表达 `Chat -> Run -> Approval -> Inbox -> Trace`
- 需要的测试或校验：
  - 后续实现阶段需要补 `Run` 状态机测试、恢复测试、幂等与审批阻塞测试
- 当前仓库可实际执行的验证：
  - 文档存在性检查
  - 与 `PRD/SAD` 术语和状态一致性人工审阅
  - focused diff 审阅
  - `git diff --check`

## 9. 风险与待决项

- 风险：
  - 当前 contract 只定义对象级语义，尚未定义 `Task Adapter`、`Review Adapter` 等子流程接口
- 待决项：
  - 是否在后续 contract 中单独定义 `RunPlan` / `Checkpoint` 对象
  - `project_ref` 是否最终会成为某些 run_type 的必填条件，留到切片级实现再决定
