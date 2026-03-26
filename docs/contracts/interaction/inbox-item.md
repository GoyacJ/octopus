# InboxItem Contract

**状态**: `approved`
**日期**: `2026-03-26`
**所属平面**: `Interaction`
**所属对象**: `InboxItem`
**适用切片**: `GA`

## 1. 背景

- 为什么需要这份 contract：`InboxItem` 是 Octopus 面向用户或 Agent 的正式待处理事实，不是通知流。Phase 2 必须先固定其最小 contract，才能在 Slice A 中让审批、异常和恢复待办落在同一对象语义上。
- 它约束什么，不约束什么：它约束待办对象的来源、归属、状态、优先级与去重规则；不约束消息推送渠道、具体列表排序算法或页面组件实现。

## 2. 关联真相源

- `PRD` 相关章节：
  - `3.2 统一术语表`
  - `4.5 Artifacts & Workspaces`
  - `3.4 领域不变量`
- `SAD` 相关章节：
  - `3.1 边界上下文`
  - `6.3 InboxItem 状态机`
  - `6.4 版本、幂等与恢复策略`
- 其他规范：
  - [`docs/VISUAL_FRAMEWORK.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VISUAL_FRAMEWORK.md)

## 3. 对象与职责

- `authority_source`: `Hub` 中的 `Artifact & Inbox` 上下文
- `projections_or_cache`: `Inbox` 工作面列表、详情侧栏、通知聚合、只读客户端缓存
- `actors`: `runtime orchestrator`, `governance services`, `knowledge write gate`, `automation services`, `user`, `agent`
- `scope`: 至少隶属某个 `Workspace`；可选绑定 `Project`；必须引用一个可归因的目标对象

## 4. 数据结构

### 4.1 核心字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `InboxItemId` | `yes` | 待处理项标识 |
| `item_type` | `approval | clarification | exception | delegation | knowledge_promotion | automation_alert | recovery_task` | `yes` | 待办类别，覆盖当前已确认的主要来源 |
| `state` | `InboxItemState` | `yes` | 生命周期状态 |
| `owner_ref` | `OwnerRef` | `yes` | 待办归属人或处理主体 |
| `workspace_ref` | `WorkspaceRef` | `yes` | 所属协作边界 |
| `project_ref` | `ProjectRef` | `no` | 若有业务上下文，则引用对应 Project |
| `target_object_ref` | `ObjectRef` | `yes` | 必须可归因的目标对象 |
| `source_object_ref` | `ObjectRef` | `yes` | 触发该待办的来源对象 |
| `priority` | `low | medium | high | urgent` | `yes` | 排队优先级 |
| `risk_level` | `low | medium | high | critical` | `no` | 风险摘要；审批或异常待办建议提供 |
| `dedupe_key` | `string` | `yes` | 去重键，防止同一事件多次进入队列 |
| `suggested_actions` | `SuggestedAction[]` | `no` | 建议用户采取的操作 |
| `notification_refs` | `NotificationRef[]` | `no` | 与该待办相关的提醒对象 |

### 4.2 状态枚举

| 状态 | 含义 | 进入条件 | 退出条件 |
| --- | --- | --- | --- |
| `open` | 待处理 | 新待办创建 | 被确认、解决、忽略或过期 |
| `acknowledged` | 已确认但未完成 | 处理主体已接收任务 | 解决或忽略 |
| `resolved` | 已完成处理 | 目标动作已完成或问题已关闭 | 终态 |
| `dismissed` | 被主动忽略 | 处理主体明确忽略或关闭 | 终态 |
| `expired` | 已过期 | 超过处理时窗且未完成 | 终态 |

### 4.3 关键关系

- 与哪些正式对象相关：`ApprovalRequest`、`Run`、`Notification`、`KnowledgeCandidate`、`Automation`、`Artifact`
- 谁创建：`Governance Plane`、`Runtime Plane`、`Knowledge Write Gate`、`Automation services`
- 谁更新：待办归属主体或与其关联的系统服务
- 谁消费：`Inbox` 工作面、通知聚合、`Trace`、治理诊断流程

## 5. 流程与规则

- 正常路径：系统检测需要处理的审批、澄清、异常或恢复事项 -> 创建 `InboxItem` -> 可选产生 `Notification` -> 处理主体确认、执行建议动作并将其解决
- 异常路径：无来源对象的待办不得创建；重复事件必须通过 `dedupe_key` 聚合或丢弃；来源对象失效时待办可被取消、解决或过期，但不能悬空存在
- 幂等要求：相同来源对象和相同待办语义必须稳定映射到同一 `dedupe_key`
- 恢复要求：Hub 重启后，未终态待办必须恢复；通知投递失败不能影响待办事实本身
- 审计要求：待办创建来源、归属人、状态变化和最终处置都必须可追溯到目标对象

## 6. 治理约束

- 是否受 `Role / Permission` 影响：`yes`，用户是否可查看和处理待办受其角色和边界影响
- 是否受 `CapabilityGrant` 影响：`indirect`，待办本身不授予能力，但其建议动作可能受 grant 约束
- 是否受 `BudgetPolicy` 影响：`indirect`，待办来源动作可能由预算命中触发
- 是否受 `ApprovalRequest` 影响：`yes`，审批类待办直接引用审批对象
- 是否涉及 `Knowledge Write Gate`：`yes`，知识晋升与撤销待办属于其处理范围

## 7. 非目标

- 这份 contract 不解决什么：
  - Notification 渠道协议
  - Inbox 列表排序、筛选和分页算法
  - 具体交互组件、卡片视觉样式和信息布局

## 8. 验收与验证

- 验收条件：
  - `InboxItem` 能与 `Notification` 清晰区分，且具备来源对象、owner/assignee、state、priority
  - Slice A 可把审批、异常和恢复待办落到统一 contract 上
- 需要的测试或校验：
  - 后续实现阶段需要补状态机、去重键、来源对象约束和通知解耦测试
- 当前仓库可实际执行的验证：
  - 文档存在性检查
  - 与 `PRD/SAD/VISUAL_FRAMEWORK` 一致性审阅
  - focused diff 审阅
  - `git diff --check`

## 9. 风险与待决项

- 风险：
  - `item_type` 当前是首轮 GA 语义集合，后续若新增治理或恢复场景，需扩充 contract
- 待决项：
  - 是否在后续交互 contract 中单独定义 `SuggestedAction`
