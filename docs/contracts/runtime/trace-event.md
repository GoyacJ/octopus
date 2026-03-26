# TraceEvent Contract

**状态**: `approved`
**日期**: `2026-03-26`
**所属平面**: `Runtime + Observation`
**所属对象**: `TraceEvent`
**适用切片**: `GA`

## 1. 背景

- 为什么需要这份 contract：`Trace` 是 Octopus 的回放、审计和诊断界面，而 `TraceEvent` 是它的最小事件 envelope。Phase 2 必须先定义事件 contract，后续 Slice A 才能让 `Run`、审批、工具调用和异常进入同一可回放语义。
- 它约束什么，不约束什么：它约束 `TraceEvent` 的最小 envelope、关联关系和事件类别；不约束具体日志存储实现、事件压缩策略或前端时间线组件。

## 2. 关联真相源

- `PRD` 相关章节：
  - `1.3 产品设计原则`
  - `4.1 Agent Runtime`
  - `5.1 核心交互面`
- `SAD` 相关章节：
  - `1.3 核心设计原则`
  - `4.8 Observation Layer`
  - `5.1 统一运行时模型`
  - `5.5 Approval 与预算越界恢复`
  - `9.1 观测模型`
- 其他规范：
  - [`docs/VISUAL_FRAMEWORK.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VISUAL_FRAMEWORK.md)

## 3. 对象与职责

- `authority_source`: `Observation Layer`
- `projections_or_cache`: `Trace` 工作面时间线、调试详情、聚合摘要、只读缓存
- `actors`: `runtime orchestrator`, `tool runtime`, `governance services`, `user`, `agent`, `external system`
- `scope`: 必须至少绑定一个 `Run` 上下文；可扩展关联审批、策略命中、委托、知识写回或 Artifact 变更

## 4. 数据结构

### 4.1 核心字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `TraceEventId` | `yes` | 事件标识 |
| `run_ref` | `RunRef` | `yes` | 所属运行上下文 |
| `sequence` | `integer` | `yes` | 同一 Run 内的事件顺序 |
| `event_kind` | `TraceEventKind` | `yes` | 事件类别 |
| `occurred_at` | `timestamp` | `yes` | 事件发生时间 |
| `actor_ref` | `ActorRef` | `no` | 谁触发了该事件 |
| `source_ref` | `ObjectRef` | `no` | 事件来源对象 |
| `target_object_ref` | `ObjectRef` | `no` | 事件作用到的对象 |
| `status_snapshot` | `string` | `no` | 事件发生时的重要状态快照，例如 Run 状态 |
| `summary` | `string` | `yes` | 可读摘要 |
| `duration_ms` | `integer` | `no` | 事件耗时 |
| `error_code` | `string` | `no` | 异常事件的错误分类 |
| `policy_decision_ref` | `PolicyDecisionRef` | `no` | 若事件与策略命中有关，则引用对应记录 |
| `approval_request_ref` | `ApprovalRequestRef` | `no` | 若事件与审批有关，则引用对应审批对象 |

### 4.2 状态枚举

`TraceEvent` 在当前文档基线中是 append-only 事件 envelope，不定义独立业务生命周期。实现侧只要求事件一旦进入权威观测流，即视为 `recorded`。

| 状态 | 含义 | 进入条件 | 退出条件 |
| --- | --- | --- | --- |
| `recorded` | 已写入正式 Trace 流 | 运行、治理或执行环节产出可审计事件 | 不适用；事件为追加式记录 |

### 4.3 关键关系

- 与哪些正式对象相关：`Run`、`ApprovalRequest`、`Policy Decision Log`、`Artifact`、`KnowledgeCandidate`、`DelegationGrant`
- 谁创建：`Runtime Plane`、`Governance Plane`、`Execution Plane`、`Observation Layer`
- 谁更新：不更新事件主体；后续只能追加新事件
- 谁消费：`Trace` 工作面、运维诊断、审计和恢复分析

## 5. 流程与规则

- 正常路径：运行状态变化、工具调用、审批流转、政策命中、异常、委托、知识写回等关键节点产出 `TraceEvent`
- 异常路径：敏感字段需要脱敏展示；错误事件必须保留分类而不是只留自然语言描述
- 幂等要求：同一来源在同一顺序位置的事件写入必须可去重，避免恢复或重放产生重复展示
- 恢复要求：Hub 重启后的恢复流程应继续向同一 Run 追加事件，而不是覆写历史时间线
- 审计要求：`TraceEvent` 必须可追溯到 run context、actor、source 和关键治理对象；不同事件种类必须在 UI 中保持稳定语义

## 6. 治理约束

- 是否受 `Role / Permission` 影响：`yes`，不同主体能看到的 Trace 细节需要受权限控制
- 是否受 `CapabilityGrant` 影响：`indirect`，能力执行会产生日志，但 Trace 本身不授予能力
- 是否受 `BudgetPolicy` 影响：`indirect`，预算命中与越界记录应作为事件来源
- 是否受 `ApprovalRequest` 影响：`yes`，审批创建、批准、拒绝、过期都应形成 Trace 事件
- 是否涉及 `Knowledge Write Gate`：`yes`，知识候选、晋升、撤销应有可追溯事件

## 7. 非目标

- 这份 contract 不解决什么：
  - 事件存储引擎与保留策略细节
  - 前端时间线的筛选和折叠交互
  - 完整 observability taxonomy；当前只锁定最小 envelope

## 8. 验收与验证

- 验收条件：
  - `TraceEvent` 作为正式事件 envelope，可承载 Slice A 的状态变化、工具调用、审批和异常回放
  - 文档保持 append-only 事件语义，不把 Trace 误写成可变业务对象
- 需要的测试或校验：
  - 后续实现阶段需要补事件顺序、重放、脱敏和去重测试
- 当前仓库可实际执行的验证：
  - 文档存在性检查
  - 与 `PRD/SAD/VISUAL_FRAMEWORK` 一致性审阅
  - focused diff 审阅
  - `git diff --check`

## 9. 风险与待决项

- 风险：
  - 当前只定义最小 envelope，详细 `event_kind` taxonomy 仍需在 Slice A / 后续 observability 文档中补细
- 待决项：
  - 是否在后续 contract 中把 `TraceEventKind` 单独提升为共享枚举文档
