# Octopus · GA Implementation Blueprint v2.4

## 1. 文档定位

本文件是 Octopus 首版 GA 的实施蓝图，用于把 PRD 中的产品边界、SAD 中的架构边界，转化为当前阶段可执行、可验证、可迭代推进的开发总纲。

本文件负责回答：

- 首版 GA 到底做什么
- 首版 GA 明确不做什么
- 当前阶段允许启动哪些开发动作
- 各核心模块的实施边界是什么
- 应按什么顺序推进
- 每一阶段的完成判定是什么
- 模块推进时如何与 PRD / SAD / schema-first / phase-gate 协同

本文件不是：

- 最终目标态全量设计
- 逐字段的 schema 细节文档
- 单一模块的一次性详细设计
- 具体 API 路由或数据库表结构定义

对于模块级详细设计，必须在推进该模块时，基于本蓝图 + PRD + SAD 生成任务级设计包后再实现。

---

## 2. 事实源与使用规则

### 2.1 事实源优先级

开发时按以下优先级理解与约束：

1. `README.md`：当前仓库状态与 doc-first rebuild 事实
2. `docs/product/PRD.md`：产品目标、GA / Beta / Later 范围、正式对象语义
3. `docs/architecture/SAD.md`：架构分层、运行时边界、状态机、恢复、治理、协议边界
4. 本文档：首版 GA 的实施优先级、切片顺序、启动条件与完成判定
5. `docs/governance/*.md` 中的相关治理文档：AI 执行规则、schema-first、repo 结构、review、delivery
6. `docs/decisions/` 与 `docs/tasks/` 中的局部详细设计和持久决策

### 2.2 使用规则

- PRD 定义“产品语义与正式范围”
- SAD 定义“架构事实源、边界与约束”
- 本蓝图定义“当前 GA 的实施方式与推进顺序”
- 任务级设计文档可以细化当前模块，但不得悄悄扩张 PRD / SAD / GA 边界
- 若局部设计与 PRD / SAD 冲突，必须以 PRD / SAD 为准，并通过 ADR 或蓝图修订显式处理

---

## 3. 当前仓库阶段与允许启动的工作类型

当前仓库处于 **doc-first rebuild** 阶段。  
因此，当前允许启动的开发动作应按“受控启动”理解，而不是全面铺开实现。

### 3.1 当前允许启动的工作

- 仓库骨架设计与初始化
- `schemas/` 首批共享契约设计
- `crates/` / `packages/` / `apps/` 模块骨架设计
- 首个纵向切片的任务拆分
- 最小运行时闭环的实现准备
- 与 GA 主闭环直接相关的最小实现
- 测试、审计、恢复、幂等等基础治理能力的最小实现

### 3.2 当前不允许作为默认启动方向的工作

- 全面 UI 铺开
- Beta 范围的高阶 Mesh / A2A 深实现
- ResidentAgentSession 正式运行链
- Org Knowledge Graph 正式晋升
- 跨 Hub 正式业务对象自动共享
- 大而全 connector 生态
- 为未来假想需求做的超前重构

### 3.3 当前已落地并受验证的基线

- 本地 SQLite 驱动的受治理 runtime 主闭环已覆盖 Slice 1 到 Slice 5
- `apps/remote-hub`、`apps/desktop`、`packages/schema-ts`、`packages/hub-client` 的 minimum surface foundation 已落地
- GA trigger expansion 已落地，当前 tracked tree 已验证 `manual event`、`cron`、`webhook`、`MCP event` 四类 trigger 进入同一 `TriggerDelivery -> Task -> Run` 主链
- Slice 9 `real MCP transport / credentials` 已落地并通过验证
- Slice 10 `remote-hub persistence / auth` 已落地并通过验证
- `minimum automation surface` 已落地并通过验证
- Slice 11 `GA governance interaction surface` 已落地并通过验证，补齐 approval detail / inbox action / approval-driven knowledge promotion
- Slice 12 `GA governance explainability` 已落地并通过验证，补齐 project-bound capability execution-state explanations 与 Run policy-decision surface consumption
- Slice 13 `desktop local host foundation` 已落地并通过验证，补齐真实 Tauri/Rust 本地宿主、共享本地 transport 契约消费、以及受控 local-mode 引导路径
- Slice 14 `desktop task workbench` 已落地并通过验证，补齐 desktop workbench 的 route split、独立 loaders 与 `Tasks / Runs / Inbox / Notifications / Connections` IA
- Slice 15 `project knowledge index` 已落地并通过验证，补齐 project-scoped Shared Knowledge 只读索引面与 local/remote transport parity
- post-Slice-15 的下一优先级现已冻结为 Slice 16 `desktop remote connection & session surface`；该切片当前处于计划/实施中，尚未计入已验证基线

---

## 4. 首版 GA 范围

根据 PRD，首版 GA 正式范围为：

- `Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`

并带有以下约束：

- 正式执行主线以 `run_type=task`、`run_type=automation`、审批驱动的 `run_type=review` 为主
- 事件触发仅开放 `cron`、`webhook`、`manual event`、`MCP event`
- `Shared Knowledge` 进入 GA
- `Org Knowledge Graph` 正式晋升留在 Beta
- `A2A` 与高阶 Mesh 协作保留架构接口，但不作为首版默认交付承诺

### 4.1 GA 的核心目标

首版 GA 必须证明三件事：

1. Octopus 可以以正式对象模型接收、运行、治理并恢复任务
2. Octopus 可以在授权、预算、审批和审计边界内安全执行
3. Octopus 可以把执行结果沉淀为共享知识，并在后续运行中召回

### 4.2 GA 的完成标准

首版 GA 不以“功能数量”定义完成，而以“最小正式运行闭环成立”定义完成。  
必须至少满足：

- 一条完整的正式运行闭环已稳定打通
- Run / Task / Automation / Approval / Artifact / Knowledge 的主对象与契约稳定
- 授权 / 预算 / 审批最小链路成立
- 审计、追踪、恢复、幂等具备最小可用性
- Shared Knowledge 最小闭环成立
- 模块边界与目录归属稳定
- 测试、说明与开发规则已能支撑后续演进

---

## 5. 首版 GA 明确不做或只保留接口的范围

以下能力在当前阶段不应进入深实现，最多只允许保留接口、占位对象、设计说明或 ADR：

- 高阶 A2A 协作
- Org Knowledge Graph 正式晋升体系
- ResidentAgentSession 正式运行链
- 高阶 Mesh 协作拓扑
- 复杂 Team / Delegation 编排
- Mobile 专属深能力面
- 复杂多租户运营面
- 消费级能力目录复刻
- 更广泛事件源生态和跨系统深治理联动

这些内容的存在方式只能是：

- 架构接口预留
- 契约预留（若必须）
- ADR
- 非默认实现路径
- 后续路线图说明

不得侵入首版 GA 的主闭环和主实现路径。

---

## 6. GA 实施总原则

### 6.1 先做最小闭环，不做目标态全实现

当前阶段必须优先验证闭环完整性，而不是能力广度。

### 6.2 先做运行时内核，再做表面层

实施优先级继续保持“先 runtime / contract / governance，再 transport，再 surface”的原则。

截至当前 tracked tree，以下内容已落地并通过验证：

1. Slice 1 到 Slice 5 的本地 governed runtime
2. minimum `desktop + remote-hub + schema-ts + hub-client` surface foundation
3. trigger expansion foundation + Slice 6 `cron` + Slice 7 `webhook` + Slice 8 `MCP event`
4. Slice 9 `real MCP transport / credentials`
5. Slice 10 `remote-hub persistence / auth`
6. minimum automation surface
7. Slice 11 `GA governance interaction surface`
8. Slice 12 `GA governance explainability`
9. Slice 13 `desktop local host foundation`
10. Slice 14 `desktop task workbench`
11. Slice 15 `project knowledge index`

更深 remote admin / tenant / IdP 能力，以及任何新的 Beta / 扩 scope 工作，必须等新的 task package 与 owner docs 明确后再启动；当前 post-Slice-15 的允许下一步仅冻结到 Slice 16 `desktop remote connection & session surface`。

### 6.3 Shared Contract 先于实现

跨语言共享对象、正式命令、事件、DTO、状态、审批对象、知识对象，必须优先进入 `schemas/`，再进入实现。

### 6.4 蓝图驱动、模块迭代细化

本蓝图只稳定全局方向。  
每推进一个 Slice 或一个核心模块，必须先完成当前模块的：

- Task Definition
- Design Note
- Contract Change
- 必要时 ADR

然后才允许实现。  
模块完成后，若形成稳定结论，必须回写蓝图、ADR 或相关开发治理文档。

### 6.5 恢复、幂等、审计不是后补件

正式语义上，GA 必须把恢复、幂等、审批可追溯、知识写回失败处理、外部输出门控当作主闭环的一部分，而不是后续优化项。

---

## 7. 首版必须打通的最小正式闭环

### 7.1 闭环定义

首版必须打通如下链路：

`Task / Automation -> Run -> Capability Resolve -> Policy / Budget Check -> Optional Approval -> Execution -> Artifact -> Audit / Trace -> Knowledge Candidate -> Shared Knowledge`

### 7.2 这条闭环为什么是首版主线

这条链路同时验证了：

- 系统能够接受正式任务与自动化触发
- 系统能够把输入映射为正式 Run
- 系统能够进行 capability 可见性、预算与审批判断
- 系统能够在正式执行边界内运行并产出 Artifact
- 系统能够保留 Trace / Audit
- 系统能够将结果沉淀为 Shared Knowledge
- 系统能够在失败、恢复、补偿场景下保留正式语义

如果这条链不能成立，Octopus 仍然只是“会调用工具的聊天系统”，而不是统一 Agent Runtime Platform。

---

## 8. GA 最小协作边界

### 8.1 Workspace

GA 必须承认 Workspace 是核心协作边界。  
至少以下对象应在正式语义上归属于 Workspace 或受 Workspace 边界约束：

- Agent
- Team（即使 GA 不深做复杂协作）
- Project
- Shared Knowledge
- Artifact
- Run
- InboxItem
- Notification
- 成员授权与治理边界

### 8.2 Project

GA 中 Project 作为业务上下文边界存在。  
至少承担以下职责：

- 作为 Run 的业务上下文容器
- 作为 Artifact 的业务归属容器
- 作为附着 KnowledgeSpace 视图的载体
- 作为局部 CapabilityBinding / Policy 作用域之一

GA 当前不要求深做复杂项目管理，但不能缺失 Project 这一上下文边界。

### 8.3 KnowledgeSpace

GA 必须承认 Shared Knowledge 的正式归属来自 KnowledgeSpace。  
Project 对 Shared Knowledge 的可见性，应来自其附着的一个或多个 KnowledgeSpace 视图，而不是把 Shared Knowledge 直接挂在 Project 下。

### 8.4 最小业务关系

在 GA 中，至少应成立如下关系：

- Workspace 拥有 Project
- Workspace 拥有 KnowledgeSpace
- Project 附着一个或多个 KnowledgeSpace 视图
- Run 归属于 Project
- Artifact 归属于 Project
- Shared Knowledge 的正式容器是 KnowledgeSpace
- Approval / Inbox / Notification 至少归属于 Workspace，并可引用具体 Project / Run / Artifact / Knowledge 事件

---

## 9. 实施平面与模块边界

### 9.1 Interaction Plane（GA 最小表面）

GA 最小交互表面应覆盖：

- Task 创建入口
- Run 查看与回放入口
- Approval Inbox
- Artifact 查看页
- Shared Knowledge 最小查看与管理页
- Workspace / Project 最小上下文呈现
- Hub Connection 的最小配置与连接状态呈现
- Capability 执行性与审批结果的最小解释能力
- Notification 的最小接收与处理入口

GA 当前不要求：

- 全量复杂 Board / 多视图协作体验
- 深移动端体验
- 复杂跨端继续体验
- 完整消费级 widget 能力

### 9.2 Runtime Plane（GA 核心）

GA 必须最小落地以下组件或等价组件：

- Intent Intake
- Capability Resolver
- Scheduler / Event Bus（最小触发与异步恢复能力）
- Run Orchestrator
- Task Adapter
- Review Adapter
- Automation Manager（GA 子集）
- Execution Adapter / MCP Gateway

GA 当前不要求：

- Discussion Adapter 深实现
- Watch Adapter 深实现
- Delegation Adapter 深实现
- Resident Supervisor 正式实现

### 9.3 Knowledge Plane（GA 子集）

GA 必须落地：

- Shared Knowledge 的权威容器
- Knowledge Candidate 生成
- 人工或规则驱动的晋升
- 后续 Run 可召回
- 最小 lineage 记录

GA 当前不要求：

- Org Knowledge Graph 正式晋升
- 复杂冲突传播
- 图关系深治理

### 9.4 Governance Plane（GA 子集）

GA 必须落地：

- Capability 可见性与执行性解析
- 最小 Policy 判定
- 最小 Budget 判定
- ApprovalRequest 生命周期
- 审批结果的可解释性
- 审计和策略日志
- 越界执行的拒绝 / 升级审批路径

GA 当前不要求：

- 复杂组织级授权图谱
- 高阶弹性预算治理
- A2A 的正式授权传递链

### 9.5 Interop Plane（GA 子集）

GA 正式只开放 MCP：

- Hub 统一注册和治理
- 以 connector-backed capabilities 进入统一 catalog
- 鉴权、健康、失败、信任级别可见
- 输出进入 Artifact / Knowledge 前必须门控

A2A 仅保留架构接口，不进入 GA 主实现路径。

### 9.6 Execution Plane（GA 子集）

#### 正式语义层

GA 必须承认正式的执行环境语义至少包含以下 tier：

- `local_trusted`
- `tenant_sandboxed`
- `ephemeral_restricted`
- `external_delegated`

#### 首批实现层

GA 首批主路径重点落：

- `local_trusted`
- `tenant_sandboxed`
- `ephemeral_restricted`

`external_delegated` 当前只保留占位与治理接口，不进入 GA 主实现路径。

#### 其他要求

GA 必须支持最小执行环境语义：

- 环境租约
- 心跳与过期
- resume token / 恢复语义
- 与 Policy / Budget / Approval 结合
- 必要时补偿或安全终止

---

## 10. 核心对象与 GA 首批对象清单

本节定义 GA 首批必须优先稳定的正式对象，不展开到字段级。

### 10.1 第一优先级对象

必须首先进入 `schemas/` 与模块设计：

- Run
- Task
- Automation
- Trigger
- TriggerDelivery
- ApprovalRequest
- Artifact
- KnowledgeSpace
- KnowledgeCandidate
- KnowledgeAsset（GA 只用到 Shared Knowledge 相关子集）
- Workspace
- Project
- CapabilityDescriptor
- CapabilityBinding
- CapabilityGrant
- BudgetPolicy
- EnvironmentLease
- InboxItem
- Notification
- AuditRecord
- TraceRecord
- PolicyDecisionLog
- KnowledgeLineageRecord

### 10.2 第二优先级对象

在首个闭环稳定后进入：

- InteractionPrompt
- MessageDraft
- ConversationRecallRef
- HubConnection 相关最小对象
- ArtifactSessionState
- Attachment

### 10.3 只保留接口或占位的对象

当前不做深实现，但允许保留占位或契约准备：

- DiscussionSession
- ResidentAgentSession
- DelegationGrant（若当前切片未直接需要，可仅保留语义说明）
- A2APeer
- ExternalAgentIdentity
- Org Knowledge Graph 相关晋升对象

---

## 11. GA 必须遵守的状态机与可裁剪状态机

### 11.1 必须严格遵守正式语义的状态机

以下对象在 GA 中必须保持正式状态机语义，不得在局部实现中任意改义：

- Run
- Automation
- TriggerDelivery
- EnvironmentLease
- ApprovalRequest
- KnowledgeAsset（至少 Shared Knowledge 子集）
- InboxItem
- Notification

### 11.2 允许当前只落最小子集，但不得改义的状态机

以下对象可以在 GA 当前切片只实现最小状态子集，但状态语义不得与 SAD 冲突：

- Automation
- TriggerDelivery
- EnvironmentLease
- InboxItem
- Notification
- KnowledgeAsset（不含 Org Graph 晋升链）

### 11.3 典型强约束

- `ApprovalRequest` 至少应保留 `pending`、`approved`、`rejected`、`expired`、`cancelled` 的正式语义
- `Run` 的最小状态机必须支持创建、运行、等待审批 / 阻塞、完成、失败、取消 / 终止、恢复语义
- `TriggerDelivery` 必须支持去重、投递、结果可追踪与重试 / 恢复
- `EnvironmentLease` 必须支持 heartbeat、expiry、resume token 或等价恢复语义
- `InboxItem` 与 `Notification` 必须支持状态推进与去重键

---

## 12. 模块级实施边界

### 12.1 Run Orchestrator

#### 责任

- 创建和推进 Run
- 维护 Run 生命周期与状态机
- 记录 checkpoint、恢复信息和关键事件
- 协调 Task / Automation / Review 的正式执行流程

#### 首批必须实现

- 最小 Run 状态机
- 状态迁移事件
- checkpoint 语义
- 与 Audit / Trace 的记录钩子
- 与 ApprovalRequest 的挂起 / 恢复连接
- 与 EnvironmentLease / action idempotency key 的最小集成接口

#### 当前不做

- 复杂长驻会话
- 多 Agent 调度器
- 复杂 delegation 网格

### 12.2 Task / Automation

#### Task

- 手动创建的正式执行入口
- 有明确输入、目标、触发上下文
- 以 Task 语义映射到 Run

#### Automation

- 定时 / 事件触发的正式定义
- 支持最小触发器集合：`cron`、`webhook`、`manual event`、`MCP event`
- 需要幂等投递与基础恢复语义

#### 首批必须实现

- Task 创建与最小字段集
- Automation 创建与最小触发定义
- TriggerDelivery 最小状态推进
- 自动化生成 Run
- 幂等键与失败重试基础

#### 当前不做

- 复杂自动化 DSL
- 复杂多事件聚合
- 长驻自主建 Run

### 12.3 Capability / Policy / Budget / Approval

#### 责任

- 判断 capability 是否可见、可搜索、可执行
- 判断是否越权、超预算或需审批
- 生成 deny / escalate / approve 的结构化结果
- 形成可解释的治理结果

#### 首批必须实现

- CapabilityDescriptor / Binding / Grant 的最小结构
- Capability Resolver 最小求值链
- Policy 最小判定
- Budget 最小判定
- ApprovalRequest 生命周期
- 审批后的 Run 恢复
- capability 执行性解释输出
- PolicyDecisionLog 的最小记录能力

#### 当前不做

- 复杂组织级授权图谱
- 高阶预算弹性模型
- A2A 委托授权深实现

### 12.4 Execution Adapter / MCP Gateway

#### 责任

- 以统一执行边界对接 MCP
- 封装外部调用
- 提供命名空间、健康、鉴权、失败与信任信息
- 将外部结果送入 Artifact / Knowledge 前做门控

#### 首批必须实现

- MCP connector 最小注册模型
- 统一调用封装
- 错误归一化
- 信任等级最小表达
- 输出门控接口
- Trace / Audit 接口

#### 当前不做

- 复杂 provider 市场化体系
- 过度 provider-specific 抽象扩张

### 12.5 Artifact / Audit / Trace / Observe

#### 责任

- 保存执行结果
- 保存关键执行痕迹
- 支持审计、回放与问题定位
- 保留治理和知识相关的最小观测能力

#### 首批必须实现

- Artifact 基本对象
- Trace / Audit 基本对象
- PolicyDecisionLog 最小落点
- KnowledgeLineageRecord 最小落点
- Artifact 与 Run 关联
- 审批、执行、恢复、失败等关键事件记录
- 失败事件与重试入口记录

#### 当前不做

- 复杂 Artifact 工作台
- 高级多版本治理
- 全量成本账本与高级分析面板

### 12.6 Shared Knowledge

#### 责任

- 承接执行结果中的可沉淀知识
- 提供最小共享召回能力
- 保留 lineage 与可信度最小信息

#### 首批必须实现

- Knowledge Candidate 生成
- Shared Knowledge 写入
- 人工或规则驱动晋升
- 后续 Run 最小召回
- lineage 最小记录
- 写回失败不阻塞主结果，但留下事件与重试入口

#### 当前不做

- Org Knowledge Graph 正式晋升
- 复杂冲突传播与图计算

### 12.7 Inbox / Notification

#### 责任

- 承接审批、越界、知识晋升、异常和关键通知
- 为 GA 的治理闭环提供正式交互入口

#### 首批必须实现

- Approval Inbox 最小模型
- Notification 最小模型
- 去重键
- 与 ApprovalRequest / Run / KnowledgeCandidate 的最小关联
- 最小处理状态推进

#### 当前不做

- 复杂通知编排
- 全渠道通知矩阵
- 高级优先级调度与 SLA 体系

---

## 13. 当前切片推进顺序与已完成基线

### 13.1 Slice 1：Task -> Run -> Artifact -> Audit

#### 范围

- Workspace / Project 最小上下文落位
- Task 创建
- Run 创建与最小状态推进
- 一个最小执行动作
- Artifact 产出
- Audit / Trace 写入

#### Slice 1 目标

证明“Octopus 已经有正式执行外壳”，而不是纯交互式对话流。

#### Slice 1 最小验收

- 能在 Workspace / Project 上下文中创建 Task
- 能创建并完成一个最小 Run
- 能看到 Run 状态变化
- 能拿到 Artifact
- 能查到 Trace / Audit
- 至少一个失败路径可被记录和重试或显式终止

### 13.2 Slice 2：Approval + Inbox / Notification

#### 范围

- Policy / Budget 最小判定
- ApprovalRequest
- Approval Inbox
- Notification 最小链
- 审批后恢复 Run

#### Slice 2 目标

证明“Octopus 的正式执行不是静默自治，而是受治理的自治”。

#### Slice 2 最小验收

- 高风险动作能触发审批
- 审批通过与拒绝能正确改变 Run 路径
- 审计链完整
- 客户端离线不导致审批被静默跳过
- Inbox 可见可处理
- Notification 具备最小可达与去重能力

### 13.3 Slice 3：Automation

#### 范围

- 一个最小 Automation 触发器
- TriggerDelivery 最小状态机
- 幂等投递
- 失败重试或恢复

#### Slice 3 目标

证明“Octopus 可以正式接纳非手动入口，并保持运行时语义一致”。

#### Slice 3 最小验收

- Automation 能生成 Run
- 触发去重与幂等语义成立
- 失败可被记录并重试或恢复

### 13.4 Slice 4：Shared Knowledge

#### 范围

- Knowledge Candidate 生成
- Shared Knowledge 晋升
- 后续 Run 召回
- KnowledgeLineageRecord 最小落点

#### Slice 4 目标

证明“Octopus 可以把执行沉淀为组织共享资产，而不只是产出一次性结果”。

#### Slice 4 最小验收

- 执行结果可产生知识候选
- 候选可晋升为共享知识
- 后续执行可命中召回
- 写回失败不影响主结果，但有失败事件与重试入口

### 13.5 Slice 5 之后的 minimum surface foundation（已完成）

#### 范围

- 冻结最小 visual framework
- 增加 `schema-ts` 与 `hub-client`
- 增加 thin `remote-hub`
- 增加 minimum `desktop` shell

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 该层只证明最小表面消费边界，不等于 automation management surface 已完成

### 13.6 Trigger Expansion Foundation（已完成）

#### 范围

- 将 `Trigger` 从 `manual_event` 单点扩展为 GA 四类 trigger 的判别联合
- 保持一个 `Automation` 只绑定一个 `Trigger`
- 将 trigger-specific ingress 统一收口到同一 `TriggerDelivery -> Task -> Run` 主链

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 兼容入口 `dispatch_manual_event` 仍保留

### 13.7 Slice 6：`cron` Trigger（已完成）

#### 范围

- 暴露 `tick_due_triggers(now)` 显式 runtime API
- 在 `remote-hub` 壳层增加最小轮询 loop
- 以 `trigger_id + scheduled_at` 形成幂等投递键

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 仍是本地单进程 poller，不是独立 scheduler service

### 13.8 Slice 7：`webhook` Trigger（已完成）

#### 范围

- 提供单一路由 `POST /api/triggers/{trigger_id}/webhook`
- 强制 `Idempotency-Key` 与 shared-secret header
- 保持 ingress 只做验签、去重、投递

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- secret 仍按 create-time reveal + persisted hash 的最小模型运行

### 13.9 Slice 8：`MCP event` Trigger（已完成）

#### 范围

- 提供 governed MCP event ingress API
- 事件必须绑定到已登记的 `McpServer`
- 保持复用既有 `TriggerDelivery -> Run -> Artifact -> Knowledge gate` 主链

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 该 slice 不包含真实凭证化 transport

### 13.10 Slice 9：real MCP transport / credentials（已完成）

#### 范围

- 以真实 HTTP / JSON-RPC transport 驱动 credentialed MCP 调用
- 引入 `McpCredentialRef` 与最小 credential lookup
- 在 runtime 主链中记录真实 invocation、health、lease、retry 语义

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 该 slice 不包含 remote-hub 远程身份、会话或成员鉴权

### 13.11 Slice 10：remote-hub persistence / auth（已完成）

#### 范围

- 新增专门的 access/auth Rust 边界承载远程用户、工作区成员关系与 JWT 会话
- 在 `remote-hub` SQLite 中持久化 remote user、membership、session 记录
- 为 remote REST / SSE 路径增加登录、鉴权、登出、会话过期与 workspace membership 校验
- 扩展 shared contracts、hub-client 与 desktop，使其可区分 `authenticated`、`auth_required`、`token_expired`

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 当前是 bootstrap user + persisted membership + JWT session 的最小模型，不包含 full tenant / RBAC admin surface 或 external IdP

### 13.12 Slice 11：GA governance interaction surface（已完成）

#### 范围

- 扩展 `ApprovalRequest`、`InboxItem`、`Notification` 的共享契约，引入 `target_ref` 并把 `approval_type` 扩展到 `execution | knowledge_promotion`
- 新增 `RequestKnowledgePromotionCommand`，让 `KnowledgeCandidate` 进入正式审批驱动的 shared-knowledge 晋升路径
- 扩展 runtime / remote-hub / hub-client / desktop，使 Workspace / Run surface 可以查看审批详情、inline resolve approval、发起 knowledge promotion request

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 当前是 approval-centric 的最小治理交互面，不包含独立 Inbox / Board 路由、full notification center、tenant / RBAC / IdP admin、vector retrieval 或 Org Graph promotion

### 13.13 Slice 12：GA governance explainability（已完成）

#### 范围

- 用 `CapabilityResolution` 取代 visible-only capability explanation contract，并保持 project-bound capability surface，不扩成全局 catalog
- 在 runtime / governance 中增加只读 capability resolution 求值入口，复用 binding、grant、budget、risk 的现有治理真值，并支持 `estimated_cost`
- 扩展 `remote-hub`、`hub-client`、`desktop`，让 Workspace 页展示 project-bound capability execution-state explanations，让 Run 页展示已持久化的 `policy_decisions`

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 当前是 read-only governance explainability slice，不包含 grant / budget 编辑、tenant / RBAC / IdP、独立 Inbox / Notification center、vector retrieval、Org Graph promotion 或更广泛 capability catalog

### 13.14 Slice 13：desktop local host foundation（已完成）

#### 范围

- 在 `apps/desktop/src-tauri` 下落地真实 Tauri 2 Rust 本地宿主，并注册为 Cargo workspace member
- 通过 `schemas/interop/local-hub-transport.*` 冻结本地 transport command / event 真值，并让 Rust 与 TypeScript 共用
- 用 deterministic demo seed 与现有 runtime 真值打通 local desktop 启动路径
- 保持 desktop 继续消费现有 `HubClient` 边界，而不是引入 app-local runtime 语义

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 当前是受控 local-host foundation，不包含 full local auth/session、tenant / RBAC admin、chat-first desktop redesign 或更深 local onboarding

### 13.15 Slice 14：desktop task workbench（已完成）

#### 范围

- 将当前 desktop 主入口收敛为明确的 workbench IA：`Tasks`、`Runs`、`Inbox`、`Notifications`、`Connections`
- 保留现有 `RunView` 作为 run policy / artifact / trace / approval / knowledge follow-up 的 authoritative detail page
- 仅新增一个共享读取面：`HubClient.listRuns(workspaceId, projectId)`，并在 local Tauri 与 remote-hub 两条 transport 上保持对等
- 把当前过载的 mixed workspace page 拆成 focused route views 与独立 loaders，但不扩成 chat surface、board aggregation 或新的治理写路径

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 当前 slice 不包含 retry/terminate run、streaming/chat、DiscussionSession、Agent/Team onboarding、tenant / RBAC / IdP admin、vector retrieval 或 Org Graph promotion

### 13.16 Slice 15：project knowledge index（已完成）

#### 范围

- 新增 project-scoped 的 Shared Knowledge 只读索引面，复用 `KnowledgeSummary` 作为 mixed entry DTO
- 在 `schemas/observe`、`schemas/interop`、`packages/schema-ts`、`packages/hub-client`、`crates/runtime`、`apps/remote-hub`、`apps/desktop/src-tauri`、`apps/desktop` 之间保持 contract-first 与 local/remote parity
- 把 desktop workbench IA 扩展为 `Tasks`、`Runs`、`Knowledge`、`Inbox`、`Notifications`、`Connections`
- 保持 `RunView` 作为 promotion request 的 authoritative surface，`Inbox` 作为 approval resolve 的 authoritative surface；知识页只负责读取与跳转

#### 当前状态

- 已在 tracked tree 中落地并通过验证
- 当前 slice 是 project-scoped、read-only、pull-first 的 Shared Knowledge read surface，不包含 workspace-wide board、vector retrieval、Org Graph promotion、tenant / RBAC / IdP、retry/terminate run 或新的知识治理写路径

---

## 14. 当前阶段的开发启动条件

在本蓝图下，只有满足以下条件，才允许进入某个 Slice 或模块的实现阶段：

### 14.1 通用前置条件

- 当前模块目标明确
- Scope 与 Out of Scope 已明确
- 受影响模块与层已明确
- 测试与验证方式已明确
- 若涉及共享契约，schema 影响已明确
- 若涉及架构边界，ADR 需求已明确
- 目录归属已明确

### 14.2 模块实施前必须完成的设计包

必须至少产出：

- Task Definition
- Design Note
- Contract Change（如涉及 schema / 事件 / DTO / 公共接口 / 状态）
- 必要时 ADR

### 14.3 禁止直接进入实现的情况

- 目标不清
- 层归属不清
- schema 影响不清
- 兼容性影响不清
- 测试计划不清
- 试图在局部任务中偷渡 Beta 能力
- 试图绕过 `schemas/` 直接定义共享对象
- 试图用 app 层临时逻辑冒充共享核心能力

---

## 15. 与 repo 结构的映射

### 15.1 `schemas/`

优先放置：

- Workspace / Project / KnowledgeSpace 基本契约
- Run / Task / Automation / Trigger / TriggerDelivery / EnvironmentLease
- ApprovalRequest / InboxItem / Notification
- Capability / Policy / Budget / Approval 相关共享契约
- Artifact / Audit / Trace / PolicyDecisionLog / KnowledgeLineageRecord 基本契约
- KnowledgeCandidate / KnowledgeAsset 的 GA 子集
- 对前后端共同可见的状态枚举、命令、事件、DTO

### 15.2 `crates/`

优先放置：

- Run Orchestrator
- Workspace / Project / Knowledge 领域逻辑
- Task / Automation 核心逻辑
- Policy / Budget / Approval 核心逻辑
- Artifact / Audit / Trace 核心逻辑
- MCP Gateway 与执行适配
- 基础设施实现

### 15.3 `packages/`

优先放置：

- 前端 schema 消费层
- API client
- Approval Inbox、Artifact 查看、Knowledge 管理、Trace 回放等共享前端能力
- 前端状态模型和 UI 组件

### 15.4 `apps/`

优先放置：

- Desktop / Hub / Web / CLI 的装配层
- 当前切片所需最小页面与入口
- 不应承载共享运行时真相

---

## 16. 测试、恢复、幂等与补偿要求

### 16.1 测试要求

每个 Slice 至少要覆盖：

- 核心路径
- 失败路径
- 边界条件
- 共享契约行为（如涉及）
- 恢复或补偿的最小验证

### 16.2 恢复与幂等要求

GA 必须在正式语义上支持：

- `run checkpoint`
- `action idempotency key`
- Trigger / Approval / 外部回执 的幂等键
- `EnvironmentLease` 的 heartbeat / expiry / resume token 语义
- 失败后保留恢复入口

### 16.3 补偿原则

- 已确认的外部写操作，不依赖“假回滚”
- 内部未确认动作，可基于幂等键安全重放
- 知识写回失败不影响主结果，但必须留下失败事件和重试入口

---

## 17. 安全、信任与边界要求

### 17.1 Hub 与 Client

- Hub 是正式执行与治理事实源
- Client 只负责交互、连接、缓存与呈现
- Client 不作为远程业务事实源

### 17.2 外部结果门控

- 外部协议结果默认低信任
- MCP 输出进入 Artifact 或 Knowledge 前必须门控
- 低信任结果进入长期知识前必须经过隔离、摘要或人工确认

### 17.3 审批与越界执行

- 高风险审批不得因客户端离线被跳过
- 越界执行必须显式进入审批或拒绝路径
- 审批结果必须可审计、可回放

---

## 18. 产品验收标准与工程完成标准

### 18.1 产品验收标准

首版 GA 至少应让用户或管理员可以：

- 在统一对象语义下理解 Task、Run、Artifact、Shared Knowledge、Approval 的关系
- 看到并处理审批与关键通知
- 解释 capability 在当前 `platform + connector + policy + grant + budget` 条件下的可见性与执行性
- 感知 Hub Connection、Run 状态、审批结果、Artifact 结果和 Shared Knowledge 沉淀
- 在 Workspace / Project 上下文中理解结果归属和知识可见范围
- 感知外部输出被门控，而非被静默写入长期知识

### 18.2 工程完成标准

首版 GA 至少应满足：

- 一条完整纵向闭环稳定打通
- 核心对象与共享契约稳定
- 核心状态机最小集合稳定
- 恢复、幂等、审批、审计、门控最小链路成立
- Shared Knowledge 最小闭环成立
- 模块边界与目录归属稳定
- 测试与说明达到可持续演进水平

---

## 19. 蓝图维护规则

### 19.1 何时更新本蓝图

以下情况发生时，应更新本蓝图：

- Slice 顺序或优先级发生稳定变化
- 核心模块边界发生稳定调整
- GA 范围的实施解释需要收紧或澄清
- 当前阶段允许启动的工作类型发生变化
- 某些“占位接口”正式进入 GA 主路径
- 某些模块完成后形成新的长期团队共识

### 19.2 何时不更新本蓝图

以下情况不应更新本蓝图，而应写入任务包或 ADR：

- 单次实现细节
- 临时 workaround
- 局部命名争议
- 具体字段级 schema 细节
- 某个函数、crate、package 的具体拆分方式

---

## 20. 最终结论

Octopus 首版 GA 不是“把目标态平台全部做出来”，而是：

- 以 PRD 定义的正式对象模型为产品边界
- 以 SAD 定义的运行时、治理、恢复、互操作边界为架构约束
- 以本蓝图定义的最小正式运行闭环为实施主线
- 通过 Slice 1 -> Slice 2 -> Slice 3 -> Slice 4 -> Slice 5 -> minimum surface foundation -> trigger expansion foundation -> Slice 6 -> Slice 7 -> Slice 8 -> Slice 9 -> Slice 10 -> minimum automation surface -> Slice 11 -> Slice 12 -> Slice 13 -> Slice 14 -> Slice 15 的顺序稳步推进
- 当前 tracked tree 已推进到 Slice 15；post-Slice-15 的下一优先级尚未在 tracked owner docs 中冻结
- 在每次模块推进前先完成局部设计包，再实现，再验证，再回写全局文档

只有这样，Octopus 才能在不丢失整体方向的前提下，让 AI 主导开发同时保持可控、可审计、可维护。
