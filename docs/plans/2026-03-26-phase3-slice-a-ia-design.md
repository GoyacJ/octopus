# Octopus Phase 3 Slice A IA Design

**状态**: `approved`
**日期**: `2026-03-26`
**owner**: `Codex`
**release_slice**: `GA`

## 1. 目标

本设计只服务 Slice A：

`Chat 发起 task -> Run 执行 / 阻塞 -> ApprovalRequest -> Inbox 处理 -> Trace 可回放`

它的目标是把首个 GA 垂直切片的页面边界、对象呈现和跨工作面跳转固定下来，供 Phase 4/5 直接实现。

本设计明确不做以下事情：

- 不扩出 `Board`、`Knowledge`、`Workspace / Project` 管理页或 `Hub Connections` 的完整工作流
- 不引入 `Remote Hub` server、`packages/`、`Web` 或 `Mobile`
- 不定义 API wire 形状、数据库 schema 或测试代码
- 不把 `Notification` 作为正式待处理事实入口替代 `Inbox`

## 2. 输入依据

- [`docs/plans/2026-03-26-phase3-slice-a-task-slice-card.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/2026-03-26-phase3-slice-a-task-slice-card.md)
- [`docs/plans/2026-03-26-phase1-project-skeleton-design.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/2026-03-26-phase1-project-skeleton-design.md)
- [`docs/contracts/runtime/run.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/runtime/run.md)
- [`docs/contracts/governance/approval-request.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/governance/approval-request.md)
- [`docs/contracts/interaction/inbox-item.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/interaction/inbox-item.md)
- [`docs/contracts/runtime/trace-event.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/contracts/runtime/trace-event.md)
- [`docs/PRD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md)
- [`docs/SAD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md)
- [`docs/VISUAL_FRAMEWORK.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/VISUAL_FRAMEWORK.md)

## 3. 切片边界

### 3.1 进入本切片的对象

- `Run`
- `ApprovalRequest`
- `InboxItem`
- `TraceEvent`

### 3.2 进入本切片的工作面

- `Chat`：任务发起、运行流观察、阻塞解释入口
- `Inbox`：审批待办处理工作面
- `Trace`：回放、审计和诊断工作面

### 3.3 明确不进入本切片

- `Board` 作为独立编排总览页
- `Knowledge` 工作面
- `Hub Connections` 的完整连接管理流程
- `CapabilityVisibility / ToolSearch` 的独立 UI 工作面
- `Notification` 渠道体验
- `Remote Hub` 专属远程模式差异表达

## 4. 应用壳层

Slice A 继续遵守 GA 视觉框架的“控制台式工作界面”：

- 顶部上下文栏：显示当前 `Hub`、`Workspace`、`Project` 与连接健康
- 左侧稳定导航：当前切片只暴露 `Chat`、`Inbox`、`Trace`
- 中央主工作区：当前页面的主内容
- 右侧 inspector：展示当前对象上下文与关键治理信息

约束：

- 当前切片不应为了“看起来完整”而把所有未来工作面都提前放进导航
- 若实现层必须保留未来导航占位，未实现工作面只能显示为不可进入状态，不能伪装成可工作界面

## 5. 工作面设计

### 5.1 Chat

定位：

- Slice A 的唯一任务发起入口
- 展示当前 `Run` 的实时状态、阻塞原因与下一步动作

推荐布局：

- 顶部：当前上下文与当前 Run 摘要
- 中部主流：任务输入区 + 运行输出区
- 右侧 inspector：当前 `Run`、审批状态、预算/风险摘要、相关 `Trace` / `Inbox` 入口

必须可见的信息：

- 当前 `Run` 的 `status`
- 当前任务是否进入 `planning`、`running`、`waiting_approval`、`completed`、`failed`
- 若被阻塞，明确显示阻塞原因和关联 `ApprovalRequest`
- 跳转到对应 `InboxItem` 与 `Trace`

禁止事项：

- 用普通聊天气泡掩盖正式状态
- 把审批要求埋在消息正文而不形成独立 callout

### 5.2 Inbox

定位：

- Slice A 中所有审批动作的正式处理面

推荐布局：

- 左侧：待办队列
- 中部：当前 `InboxItem` 详情
- 右侧：关联 `Run`、`ApprovalRequest`、最近 `TraceEvent`

每个审批待办至少展示：

- `owner_ref`
- `target_object_ref`
- `source_object_ref`
- `priority`
- `risk_level`
- `policy_reason`
- 建议动作：`approve` / `reject`

约束：

- `Inbox` 展示的是待处理事实，不是提醒 feed
- 审批处理完成后，`InboxItem` 必须进入终态并保留关联对象

### 5.3 Trace

定位：

- Slice A 的回放与诊断工作面

推荐布局：

- 顶部：当前 `Run` 上下文与过滤器
- 中部：按 `sequence` 排列的事件时间线
- 右侧：当前事件 inspector

每个事件至少展示：

- `sequence`
- `occurred_at`
- `event_kind`
- `actor_ref`
- `source_ref` 或 `target_object_ref`
- `summary`

详情面板至少展示：

- `status_snapshot`
- `approval_request_ref`
- `error_code`
- 结构化字段，而不是大段原始 JSON

## 6. 跨工作面主链路

### 6.1 Happy Path

1. 用户在 `Chat` 输入任务并提交
2. 系统创建 `Run`
3. `Chat` 展示 `Run` 从 `queued/planning` 进入 `running`
4. 若任务可直接执行，则 `Run` 完成，`Trace` 展示完整时间线

### 6.2 Approval Path

1. `Run` 进入 `waiting_approval`
2. `Chat` 显示阻塞 callout，并给出“前往 Inbox 处理”的明确入口
3. 系统创建 `ApprovalRequest` 和对应 `InboxItem`
4. 用户在 `Inbox` 处理审批
5. 处理结果写回 `Run`
6. `Chat` 显示 `Run` 恢复、终止或失败
7. `Trace` 可回放审批创建、审批处理和运行恢复结果

### 6.3 Error Path

1. `Run` 失败或终止
2. `Chat` 显示失败摘要与 `Trace` 入口
3. `Trace` 能看到失败事件、相关对象和状态快照
4. 若该失败产生正式待处理事实，可通过 `Inbox` 查看对应对象；否则不强制创建待办

## 7. 对象到 UI 的状态映射

### 7.1 Run

| `Run.status` | Chat 表达 | Inbox 表达 | Trace 表达 |
| --- | --- | --- | --- |
| `queued` / `planning` | 顶部状态 pill + 进度文案 | 不默认出现 | 时间线事件 |
| `running` | 主状态 | 不默认出现 | 时间线事件 |
| `waiting_approval` | 高风险阻塞 callout | 创建或关联审批待办 | 审批触发事件 |
| `completed` | 成功状态 + 结果摘要 | 相关待办应已终态 | 完整时间线 |
| `failed` / `terminated` / `cancelled` | 失败或终止状态 + `Trace` 入口 | 仅在存在正式待办时显示 | 失败/终止事件 |

### 7.2 ApprovalRequest

| `ApprovalRequest.status` | Chat 表达 | Inbox 表达 | Trace 表达 |
| --- | --- | --- | --- |
| `pending` | 阻塞中 | 待处理主卡片 | 审批创建事件 |
| `approved` | 运行恢复或已放行 | 待办进入终态 | 审批放行事件 |
| `rejected` | 明确拒绝与运行结果 | 待办进入终态 | 审批拒绝事件 |
| `expired` / `cancelled` | 阻塞结束但未放行 | 待办进入终态 | 对应终态事件 |

### 7.3 InboxItem

- `open`：默认可处理状态
- `acknowledged`：可选；若实现首轮不需要“已接收未完成”，可以不在 UI 首版开放主动切换动作，但对象语义保留
- `resolved` / `dismissed` / `expired`：只在历史或详情中显示，不留在待处理主队列

## 8. 最小组件集合

Slice A 实现至少需要以下领域组件：

- `RunStatusPill`
- `ApprovalCallout`
- `InboxActionCard`
- `TraceEventRow`
- `ObjectLink`
- `ContextInspectorSection`

这些组件必须直接围绕正式对象语义组织，不能以“消息卡片”“提醒卡片”这类弱语义命名替代。

## 9. 验收条件

- 实现者可以明确知道 Slice A 只需要做 `Chat`、`Inbox`、`Trace` 三个工作面
- `ApprovalRequest` 在交互上表现为正式对象，而不是局部弹窗
- `InboxItem` 在交互上表现为待处理事实，而不是通知流
- `TraceEvent` 在交互上表现为时间线事件，而不是调试日志堆叠

## 10. 风险与后续

- 若 Phase 4/5 试图顺手加入 `Board`、`Knowledge`、`Remote Hub` server，会破坏当前切片最小边界
- `CapabilityVisibility / ToolSearch` 已有 contract，但不属于 Slice A 的必需工作面；若实现需要展示，最多允许作为右侧 inspector 的只读摘要，不单独扩页
- 远程模式、本地缓存与连接状态的完整表达留到 Phase 6 Slice B
