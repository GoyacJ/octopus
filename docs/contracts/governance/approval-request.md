# ApprovalRequest Contract

**状态**: `approved`
**日期**: `2026-03-26`
**所属平面**: `Governance`
**所属对象**: `ApprovalRequest`
**适用切片**: `GA`

## 1. 背景

- 为什么需要这份 contract：`ApprovalRequest` 是 Octopus 在越界升级、高风险动作和关键写回上的正式审批对象。Phase 2 必须先固定审批对象语义，后续 Slice A 才不会把审批退化成前端临时弹窗。
- 它约束什么，不约束什么：它约束审批对象的类型、状态、治理求值位置、恢复语义和与 `Run` / `InboxItem` 的关系；不约束具体审批 UI、通知渠道实现或组织角色配置页面。

## 2. 关联真相源

- `PRD` 相关章节：
  - `3.2 统一术语表`
  - `3.4 领域不变量`
  - `4.1 Agent Runtime`
  - `6.1 Tenant、Workspace、Role 与 Permission 模型`
  - `#### 授权求值顺序`
- `SAD` 相关章节：
  - `1.3 核心设计原则`
  - `5.5 Approval 与预算越界恢复`
  - `6.3 ApprovalRequest 状态机`
  - `7.1 认证与授权`
- 其他规范：
  - [`docs/DELIVERY_GOVERNANCE.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/DELIVERY_GOVERNANCE.md)
  - [`docs/VISUAL_FRAMEWORK.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VISUAL_FRAMEWORK.md)

## 3. 对象与职责

- `authority_source`: `Governance Plane / Approval Service`
- `projections_or_cache`: `Inbox` 队列投影、审批详情视图、通知聚合
- `actors`: `runtime orchestrator`, `reviewer`, `knowledge_space_owner`, `workspace_admin`, `tenant_admin`, `requesting actor`
- `scope`: 始终绑定一个明确目标对象，通常是 `Run` 或其触发的高风险动作；审批范围受 `Workspace`、`Project`、角色和策略窗口约束

## 4. 数据结构

### 4.1 核心字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `ApprovalRequestId` | `yes` | 审批对象标识 |
| `approval_type` | `execution | knowledge_promotion | external_delegation | export_sharing` | `yes` | 审批类别，来源于当前已确认的治理路径 |
| `status` | `ApprovalStatus` | `yes` | 审批生命周期状态 |
| `run_ref` | `RunRef` | `no` | 若审批阻塞某个运行，则引用目标 Run |
| `target_object_ref` | `ObjectRef` | `yes` | 本次审批服务的正式目标对象或动作 |
| `requested_by_ref` | `ActorRef` | `yes` | 谁发起了审批 |
| `reviewer_scope_ref` | `ReviewerScopeRef` | `yes` | 这次审批应由哪个治理角色或责任边界处理 |
| `policy_reason` | `string` | `yes` | 触发审批的策略原因、越界原因或风险说明 |
| `risk_level` | `low | medium | high | critical` | `yes` | 风险等级摘要 |
| `resolution_summary` | `ResolutionSummary` | `no` | 审批结果或修改后批准的关键参数 |
| `replacement_grant_ref` | `CapabilityGrantRef` | `no` | 若批准生成新授权窗口，则引用新对象 |
| `replacement_budget_ref` | `BudgetPolicyRef` | `no` | 若批准调整预算，则引用新对象 |
| `idempotency_key` | `string` | `no` | 审批处理去重键 |
| `expires_at` | `timestamp` | `no` | 审批过期时间 |

### 4.2 状态枚举

| 状态 | 含义 | 进入条件 | 退出条件 |
| --- | --- | --- | --- |
| `pending` | 等待处理 | 运行或治理流程创建审批对象 | 审批、拒绝、过期或取消 |
| `approved` | 已批准 | 有权主体批准继续、修改后批准或放行 | 终态 |
| `rejected` | 已拒绝 | 有权主体拒绝 | 终态 |
| `expired` | 已超时 | 到达过期时间且未决 | 终态 |
| `cancelled` | 已取消 | 上游运行终止、请求撤销或审批对象作废 | 终态 |

### 4.3 关键关系

- 与哪些正式对象相关：`Run`、`InboxItem`、`CapabilityGrant`、`BudgetPolicy`、`Policy Decision Log`、`Artifact`
- 谁创建：`Governance Plane`，通常由 `Runtime Plane` 在越界或高风险路径上触发
- 谁更新：具备相应 reviewer scope 的治理主体
- 谁消费：`Run Orchestrator`、`Inbox`、`Trace`、`Audit Log`

## 5. 流程与规则

- 正常路径：运行提交越界动作或高风险动作 -> 生成 `ApprovalRequest` -> 发布 InboxItem / Notification / Policy Decision -> 有权主体批准或拒绝 -> 返回恢复参数、终止命令或新授权对象
- 异常路径：审批过期、请求方撤销、grant 被撤销、不可审批硬禁止命中时，审批对象必须明确结束并阻断运行
- 幂等要求：审批处理必须支持幂等键，避免多端重复点击造成重复放行或重复拒绝
- 恢复要求：Hub 重启后，未决审批必须可恢复为 `pending`，已终态审批不可被重新打开，只能创建新审批对象
- 审计要求：创建原因、审批人、审批时间、放行参数、被拒绝原因和关联目标对象都必须进入 `Audit Log` 与 `Trace`

## 6. 治理约束

- 是否受 `Role / Permission` 影响：`yes`，审批主体必须拥有对应 scope 权限
- 是否受 `CapabilityGrant` 影响：`yes`，审批可能放行或生成新的授权窗口，但不能跳过 grant 体系
- 是否受 `BudgetPolicy` 影响：`yes`，审批可能放行预算越界或创建新的预算对象
- 是否受 `ApprovalRequest` 影响：`n/a`，本对象就是审批治理本身
- 是否涉及 `Knowledge Write Gate`：`yes`，`knowledge_promotion` 类审批直接属于知识写回门控

## 7. 非目标

- 这份 contract 不解决什么：
  - 租户角色矩阵的完整配置模型
  - Notification 的渠道投递 contract
  - 审批页面布局和交互文案
  - 不可审批硬禁止策略的完整策略语言

## 8. 验收与验证

- 验收条件：
  - 审批对象不再被描述为前端交互，而是正式治理对象
  - 可明确区分审批类型、状态、作用域与对 Run 的影响
- 需要的测试或校验：
  - 后续实现阶段需要补审批状态机、幂等处理、过期恢复和硬禁止优先级测试
- 当前仓库可实际执行的验证：
  - 文档存在性检查
  - 与 `PRD/SAD` 审批规则对齐审阅
  - focused diff 审阅
  - `git diff --check`

## 9. 风险与待决项

- 风险：
  - `approval_type` 当前只覆盖已明确的四类主路径，后续若出现新增治理流程需要补 contract
- 待决项：
  - “修改后批准”是否在后续细化为独立决议类型，还是继续保持在 `approved + resolution_summary`
