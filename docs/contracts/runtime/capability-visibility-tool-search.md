# CapabilityVisibility and ToolSearch Contract

**状态**: `approved`
**日期**: `2026-03-26`
**所属平面**: `Runtime`
**所属对象**: `CapabilityVisibilityResult`, `ToolSearchResultItem`
**适用切片**: `GA`

## 1. 背景

- 为什么需要这份 contract：Octopus 明确要求“能力可见性”和 “ToolSearch” 进入正式 capability runtime，而不能依赖 prompt 偶然行为。Phase 2 需要先把可见性求值结果和搜索暴露结果固定下来。
- 它约束什么，不约束什么：它约束 capability 可见性求值的最小输出、ToolSearch 可返回的信息边界与治理链路；不约束具体 connector SDK、搜索排序算法或完整 capability catalog schema。

## 2. 关联真相源

- `PRD` 相关章节：
  - `1.2 核心价值主张`
  - `3.2 统一术语表`
  - `3.4 领域不变量`
  - `4.1 Agent Runtime`
  - `6.1 Tenant、Workspace、Role 与 Permission 模型`
  - `#### 授权求值顺序`
- `SAD` 相关章节：
  - `1.3 核心设计原则`
  - `3.1 边界上下文`
  - `4.4 Runtime Plane`
  - `5.1 统一运行时模型`
  - `7.1 认证与授权`
- 其他规范：
  - [`docs/AI_ENGINEERING_PLAYBOOK.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/AI_ENGINEERING_PLAYBOOK.md)

## 3. 对象与职责

- `authority_source`: `Capability Resolver` 与 `Tool Search Service`
- `projections_or_cache`: Chat 中的工具发现结果、能力摘要面板、运行计划辅助上下文
- `actors`: `agent`, `user`, `runtime orchestrator`, `capability resolver`, `tool search service`
- `scope`: 在 `platform`、connector 状态、`Workspace / Project` policy、`CapabilityGrant` 和 `BudgetPolicy` 的共同上下文中求值；结果绑定主体、范围与当前运行上下文

## 4. 数据结构

### 4.1 核心字段

#### CapabilityVisibilityResult

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `capability_id` | `string` | `yes` | 正式 capability 标识 |
| `subject_ref` | `ActorRef` | `yes` | 当前主体 |
| `scope_ref` | `WorkspaceRef | ProjectRef | RunRef` | `yes` | 当前求值范围 |
| `platform` | `string` | `yes` | 当前平台上下文 |
| `connector_state` | `string` | `no` | 相关 connector 的可用性摘要 |
| `visibility_state` | `hidden | visible` | `yes` | 是否在当前上下文中对主体可见 |
| `search_state` | `hidden | searchable` | `yes` | 是否允许被 ToolSearch 暴露 |
| `execution_state` | `blocked | executable | approval_required` | `yes` | 是否允许直接执行、被阻断或需升级审批 |
| `blocked_by` | `string[]` | `no` | 命中的策略、缺失前提或阻断原因 |
| `fallback_summary` | `string` | `no` | 当前能力不可执行时的 fallback 摘要 |
| `resolved_at` | `timestamp` | `yes` | 求值时间 |

#### ToolSearchResultItem

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `capability_id` | `string` | `yes` | 被返回的 capability 标识 |
| `descriptor_summary` | `string` | `yes` | 返回给搜索调用方的描述摘要 |
| `schema_ref` | `SchemaRef` | `yes` | 当前可见的 schema 引用 |
| `risk_level` | `string` | `yes` | 风险等级标签 |
| `governance_tags` | `string[]` | `yes` | 可解释的治理标签 |
| `fallback_summary` | `string` | `no` | 若不可直接执行，可返回的 fallback 摘要 |
| `visibility_ref` | `CapabilityVisibilityResultRef` | `yes` | 必须可追溯到可见性求值结果 |

### 4.2 状态枚举

| 状态 | 含义 | 进入条件 | 退出条件 |
| --- | --- | --- | --- |
| `hidden` | 当前上下文不可见 | connector 不可用、策略不允许、grant 不存在等 | 上下文重新求值为 `visible/searchable/executable` |
| `visible` | 当前上下文可见 | 满足最小可见性条件 | 若允许搜索，则进入 `searchable` 视角；若条件失效则回到 `hidden` |
| `searchable` | 允许被 ToolSearch 暴露 | 已可见且允许搜索暴露 | 若暴露条件失效则回到 `hidden` |
| `blocked` | 已知能力但当前不可执行 | 可见或可搜索，但执行条件不满足 | grant/budget/approval 条件满足后进入 `executable` 或 `approval_required` |
| `approval_required` | 执行需要升级审批 | 属于可升级高风险路径 | 审批放行后进入 `executable`；若被拒绝则回到 `blocked` |
| `executable` | 当前上下文允许执行 | 平台、connector、policy、grant、budget 条件都满足 | 条件失效后回到 `blocked` 或 `hidden` |

### 4.3 关键关系

- 与哪些正式对象相关：`CapabilityDescriptor`、`CapabilityBinding`、`CapabilityGrant`、`BudgetPolicy`、`ApprovalRequest`、`Run`
- 谁创建：`Capability Resolver` 计算可见性，`Tool Search Service` 生成搜索结果项
- 谁更新：不通过手工编辑更新；每次上下文变化重新求值
- 谁消费：`Agent Runtime`、`Chat`、计划生成、能力说明面板、审计与诊断

## 5. 流程与规则

- 正常路径：`CapabilityCatalog -> CapabilityBinding -> CapabilityResolver -> CapabilityVisibilityResult -> ToolSearchResultItem`
- 异常路径：connector 离线、platform 不匹配、Workspace / Project policy 阻断、grant 缺失、budget 受限时，系统必须返回可解释阻断原因而不是把能力静默隐藏成“像是不存在”
- 幂等要求：相同主体、相同范围、相同上下文下的求值结果应稳定；上下文变化时必须重新求值
- 恢复要求：恢复后不能直接沿用过期 visibility 结果，必须重新校验 platform、connector、policy、grant 和 budget freshness
- 审计要求：ToolSearch 命中、可见性求值、策略命中和由此触发的 `InteractionPrompt` / `MessageDraft` 必须可追溯到 run context 与 policy decision

## 6. 治理约束

- 是否受 `Role / Permission` 影响：`yes`
- 是否受 `CapabilityGrant` 影响：`yes`
- 是否受 `BudgetPolicy` 影响：`yes`
- 是否受 `ApprovalRequest` 影响：`yes`，仅在 `approval_required` 路径上体现
- 是否涉及 `Knowledge Write Gate`：`indirect`，搜索与可见性本身不写知识，但后续执行动作可能触发知识门控

## 7. 非目标

- 这份 contract 不解决什么：
  - `CapabilityDescriptor` 全量 schema
  - 搜索相关性排序、召回算法
  - 具体 connector 配置格式
  - UI 上的 capability card 样式

## 8. 验收与验证

- 验收条件：
  - `ToolSearch` 不再被理解为自动授权，而是可见性求值后的搜索暴露结果
  - 能明确表达 “不可见 / 可见但不可执行 / 仅可搜索 / 可执行 / 需审批” 的差异
- 需要的测试或校验：
  - 后续实现阶段需要补 capability visibility matrix、policy 优先级、connector 离线、审批升级与审计回归测试
- 当前仓库可实际执行的验证：
  - 文档存在性检查
  - 与 `PRD/SAD` 治理优先级和 `ToolSearch` 不变量对齐审阅
  - focused diff 审阅
  - `git diff --check`

## 9. 风险与待决项

- 风险：
  - `execution_state` 目前采用最小三态，后续若出现更细的降级执行语义，需扩 contract
- 待决项：
  - 是否在后续 contract 中把 `CapabilityVisibilityResult` 拆成单独文档，还是继续与 `ToolSearchResultItem` 并列维护
