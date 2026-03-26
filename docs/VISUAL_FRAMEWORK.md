# Octopus · GA 视觉框架与交互语法

**状态**: 基线建立版 | **日期**: 2026-03-26
**适用范围**: 首版 `GA` 核心交互面

---

## 1. 文档目标

本框架将 `PRD` 中定义的核心交互面与 `SAD` 中定义的 `Interaction Plane` 转换为可复用、可审查、可指导 AI 实现的视觉与页面语法规则。

本框架只覆盖首版 `GA`：

- `Desktop + Remote Hub`
- `Chat`
- `Board`
- `Trace`
- `Inbox`
- `Knowledge`
- `Workspace / Project`
- `Hub Connections`

以下内容不在首轮覆盖范围内：

- `Mobile`
- `A2A` 全景协作体验
- `ResidentAgentSession` 的高阶监控界面
- `Org Knowledge Graph` 的 Beta 级高级图谱工作台

## 2. 体验设计原则

Octopus 的 GA 界面必须同时体现四种气质：

1. **可信**：像可审计系统，不像玩具聊天应用
2. **可治理**：用户能看见权限、预算、审批和降级，而不是被动接受结果
3. **高信息密度**：适合长时间工作，不依赖大面积空白和营销式卡片
4. **对象优先**：界面围绕 `Run`、`ApprovalRequest`、`KnowledgeAsset` 等正式对象组织，而不是围绕松散的聊天片段组织

设计上明确禁止：

- 消费级 IM 风格气泡聊天作为整体主风格
- 用纯装饰动画掩盖状态复杂度
- 把高风险状态埋进 hover、tooltip 或二级页面
- 为了“更现代”而牺牲审计信息密度

## 3. 视觉基础令牌

以下令牌是 GA 默认基线，后续实现可在不破坏语义的前提下细化。

### 3.1 字体

- `font-ui`: `IBM Plex Sans`, `PingFang SC`, `Segoe UI`, sans-serif
- `font-mono`: `IBM Plex Mono`, `SFMono-Regular`, monospace

使用规则：

- 页面标题、区块标题、表单与表格使用 `font-ui`
- Trace、事件 ID、能力名、预算值、状态枚举使用 `font-mono`

### 3.2 颜色语义

| 语义 | 令牌 | 默认值 |
| --- | --- | --- |
| 画布背景 | `color.canvas` | `#F3F6F8` |
| 主面板背景 | `color.surface` | `#FFFFFF` |
| 次级面板背景 | `color.surface-muted` | `#E9EEF2` |
| 主文本 | `color.text-primary` | `#0F172A` |
| 次文本 | `color.text-secondary` | `#475569` |
| 边框 | `color.border` | `#D7DEE7` |
| 主强调色 | `color.accent` | `#0F766E` |
| 信息态 | `color.info` | `#2563EB` |
| 成功态 | `color.success` | `#15803D` |
| 预警态 | `color.warning` | `#B45309` |
| 危险态 | `color.danger` | `#B91C1C` |

规则：

- 正常业务操作使用中性基底 + 少量强调色
- 风险、审批、阻塞、越界、离线、降级必须使用稳定的状态色语义
- 禁止在不同页面给同一状态换色

### 3.3 尺寸与节奏

- 间距体系：4 / 8 / 12 / 16 / 24 / 32
- 圆角体系：6 / 10 / 14
- 边框优先于重阴影
- 卡片、表格、抽屉、详情面板应共享统一边距与标题节奏

默认原则：

- 8px 倍数优先
- 信息块之间以分组和层级区分，不靠大面积留白

## 4. 应用壳层语法

GA 默认采用“控制台式工作界面”：

- 左侧是稳定导航区
- 中央是主工作区
- 右侧是上下文检查器或详情面板
- 顶部保留 Workspace / Project / Hub 上下文、健康与全局动作

壳层规则：

- 重要上下文必须长期可见，例如当前 Hub、Workspace、Project
- 高风险动作不使用无上下文的全屏弹层作为唯一入口
- 详情优先放在右侧 inspector 或二级 pane，而不是频繁打断主流程

## 5. 页面层级与最小页面集合

### 5.1 一级工作面

以下是首轮 GA 的一级工作面：

1. `Chat`
2. `Board`
3. `Trace`
4. `Inbox`
5. `Knowledge`
6. `Workspace / Project`

### 5.2 支撑型工作面

以下是首轮 GA 的支撑型界面：

- `Hub Connections`

规则：

- 一级工作面服务核心工作流
- 支撑型工作面服务连接、配置和连续性
- 不得把支撑型配置页做成主导航中心

## 6. 核心交互面模板

### 6.1 Chat

定位：

- `Chat` 是任务发起、结构化澄清、计划确认和流式结果观察的入口
- 它不是普通即时通讯界面

推荐布局：

- 中央主流：会话与运行输出
- 左侧上下文：当前 Agent / Team / Project / 最近 Run
- 右侧 inspector：grant、budget、capabilities、knowledge refs、approval 状态

必须可见的信息：

- 当前作用对象
- 当前运行状态
- 是否触发结构化澄清
- 是否存在预算/审批/能力边界
- 引用到的 Artifact / Knowledge / Trace 入口

禁止事项：

- 仅用气泡区分所有系统信息
- 把审批、预算、风险埋进消息正文里

### 6.2 Board

定位：

- `Board` 是运行编排和状态总览界面

推荐布局：

- 顶部过滤与视图切换
- 中央是按状态或阶段组织的 Run/Automation 列表
- 右侧是对象详情与阻塞说明

每个卡片或行项至少展示：

- 对象类型
- 当前状态
- 阶段或下一步
- 负责人或归属范围
- 预算占用
- 是否阻塞 / 是否待审批

优先模式：

- 默认使用列表或分组列表
- 看板泳道只在确有阶段管理价值时使用

### 6.3 Trace

定位：

- `Trace` 是回放、审计和诊断界面

推荐布局：

- 顶部过滤器：时间、对象、事件类型、风险级别
- 中央时间线：按因果顺序展示 event
- 右侧详情面板：payload、policy 命中、关联对象

表现规则：

- 工具调用、审批、预算命中、异常、委托必须有不同但稳定的事件样式
- 事件时间、对象 ID、actor、source 必须高可读
- 详情面板优先展示结构化字段，而不是一大段原始 JSON

### 6.4 Inbox

定位：

- `Inbox` 是待处理事实，不是通知流

推荐布局：

- 左侧队列列表
- 中间详情
- 右侧相关对象上下文

每个 InboxItem 至少展示：

- 需要谁处理
- 为什么需要处理
- 来源对象
- 风险等级
- 截止感知或优先级
- 建议动作

禁止事项：

- 与 Notification 混为一个 feed
- 无来源对象的待办

### 6.5 Knowledge

定位：

- `Knowledge` 是知识条目、来源、使用情况和治理状态工作面

推荐布局：

- 搜索与筛选
- 列表或表格
- 详情面板
- lineage / usage / promotion 区块

默认展示重点：

- 来源
- trust level
- 所属 KnowledgeSpace
- 当前状态
- 最近使用
- 晋升与撤销历史

规则：

- 图谱可视化不是默认首屏
- 首轮 GA 更适合“表格/列表 + 详情 + lineage”而不是炫技式关系图

### 6.6 Workspace / Project

定位：

- `Workspace / Project` 负责边界配置、成员、Agent、Knowledge、Artifact 和策略视图

推荐布局：

- 顶部摘要头
- Tab 或二级导航承载成员、项目、知识空间、Artifacts、策略
- 详情操作使用抽屉或 inspector，减少跳页

重点：

- 边界、权限、预算、负责人必须比装饰更突出
- 配置表单应追求清晰和密度，不追求营销式排版

### 6.7 Hub Connections

定位：

- 管理本地 Hub、远程 Hub、认证状态、缓存状态和切换动作

推荐布局：

- Hub 列表
- 连接详情
- 健康/认证/缓存模式状态

必须区分：

- `offline-cache`
- `auth-expired`
- `hub-unreachable`
- `connected-readonly`

## 7. 组件分层

### 7.1 Primitive Components

应建立稳定基础组件层：

- Button
- Input / Textarea / Select
- Tabs
- Badge
- Table
- Drawer / Sheet
- Dialog
- Tooltip
- Empty State
- Skeleton

### 7.2 Domain Components

应围绕正式对象建设领域组件：

- `RunStatusPill`
- `BudgetMeter`
- `ApprovalCallout`
- `PolicyHitBadge`
- `CapabilityList`
- `KnowledgeLineagePanel`
- `ArtifactRefCard`
- `HubHealthCard`
- `TraceEventRow`
- `InboxActionCard`

### 7.3 Page Patterns

GA 页面应尽量复用以下模式：

- 列表 + 详情
- 时间线 + inspector
- 队列 + 动作详情
- 主工作流 + 上下文侧栏
- 管理摘要 + 分组设置

禁止每个页面单独发明新的主布局语法。

## 8. 状态语义

| 状态 | 含义 | 默认表现 |
| --- | --- | --- |
| `loading` | 数据或对象尚未就绪 | skeleton + 占位标题 |
| `streaming` | 正在流式生成或接收 | 明确流式指示，不闪烁 |
| `blocked` | 无法继续，需要外部条件满足 | 预警色 + 原因说明 + 下一步 |
| `approval-needed` | 需要人工审批 | 预警色 + 强行动按钮 |
| `degraded` | 系统降级但可继续 | 次强调色 + 降级说明 |
| `offline-cache` | 仅缓存可读，不允许远程写 | 明确信息条 + 禁用写动作 |
| `policy-hit` | 命中策略限制 | 信息条或 badge + 解释 |
| `error` | 当前操作失败 | 危险态 + 重试/查看详情 |
| `done-with-warning` | 主流程完成但有治理或写回告警 | 成功态结合预警说明 |

规则：

- 所有关键状态必须同时具备颜色、文本和图标，不得只依赖颜色
- 同一状态在所有 GA 页面保持同一语义

## 9. 关键可视化约束

### 9.1 预算

- 预算必须显示剩余量与阈值，不只显示累计数字
- 越界前预警，越界后阻塞语义清晰
- 预算异常应关联到具体 `Run`、`Workspace` 或 `Automation`

### 9.2 审批

- 审批卡必须显示触发原因、风险等级、建议动作、来源对象
- 审批不是普通提示；必须在视觉层级上高于一般通知

### 9.3 Trace

- 时间线优先于自由图谱
- 事件关系应可过滤、可折叠、可定位到对象详情

### 9.4 Knowledge Lineage

- 必须能看出“从哪来、被谁用、何时晋升、何时撤销”
- 首轮使用结构化 lineage 面板优先，不默认依赖复杂知识图谱可视化

## 10. 信息密度与可读性

GA 页面应采用“高密度但不拥挤”的基线：

- 表格和列表优先于巨型卡片墙
- 重要对象详情用分组标题和清晰标签组织
- 面板内字段优先成组显示，而不是无序散列
- 空态要给下一步动作，不给营销文案

## 11. 交互与无障碍要求

- 键盘导航必须覆盖主工作流
- 对比度优先满足可读性
- 所有状态变化应有文本说明
- 高风险动作必须有明确确认语义
- 不依赖 hover 作为唯一信息承载方式

## 12. 反模式

以下做法在 GA 默认禁止：

- 把 Chat 做成普通社交聊天窗口
- 把 Board 做成纯装饰 Kanban
- 把 Inbox 做成 Notification feed
- 把 Knowledge 首屏做成难以读懂的炫技图谱
- 使用大量渐变、玻璃拟态、强发光和无意义动效
- 各页面各自定义 status badge 颜色与含义

## 13. 结论

Octopus 的 GA 界面不是消费级 AI 助手外观，而是面向长期工作流、治理与审计的控制台式工作界面。视觉框架的作用不是“美化”，而是让不同实现者和不同 AI 产出同一种对象语法、状态语法和风险表达。
