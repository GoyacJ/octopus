# Octopus · 产品需求文档（PRD）

**版本**: v2.1 | **状态**: 重构定稿版 | **日期**: 2026-03-26
**对应 SAD**: v2.1

---

## 1. 产品定义与设计原则

### 1.1 产品定位

Octopus 是一个统一的 Agent Runtime Platform。它不是单个 AI 助手，也不是只服务某一类用户的工作流工具，而是一个让个人、团队与企业在同一对象模型下创建、运行、治理和扩展 Agent 体系的平台。

平台同时覆盖三类能力面：

- **个人能力面**：本地或远程 Hub 下的个人 Agent、个人知识、个人自动化和本地执行环境。
- **团队能力面**：共享 Agent、共享知识空间、协作型 Run、审阅与审批、跨端继续。
- **企业治理能力面**：多租户、RBAC、预算与策略授权、模型中心、审计、协议互操作与运维控制。

Octopus 的核心不是“聊天窗口”，而是将以下对象变成正式系统能力：

- Agent
- Team
- Run
- Knowledge
- Grant / Budget
- Artifact
- Protocol Interop

### 1.2 核心价值主张

| 用户痛点 | Octopus 的解决方式 |
| --- | --- |
| 单个 AI 助手难以承担多角色复杂工作 | 以 Agent、Team、Mesh 协作和 Run 编排为正式能力 |
| AI 缺乏长期记忆和组织知识沉淀 | 建立私有记忆、共享知识空间、组织知识图谱三层知识系统 |
| 无法信任高自治 AI 执行动作 | 用 CapabilityGrant、BudgetPolicy、ApprovalRequest、Audit 形成治理闭环 |
| 多端、多环境、多协议协作时容易失控 | Hub 统一作为事实源和治理中心，Client 负责交互和连续性 |
| 外部工具、MCP、外部 Agent 接入后风险难控 | 将 MCP 与 A2A 都纳入一等互操作层，统一身份、授权、信任和审计 |
| 任务执行、讨论、自动化、监控分散在不同系统 | 统一到 Run 体系，支持手动发起、计划触发、事件触发和常驻代理 |

### 1.3 产品设计原则

1. **统一平台，不拆产品线**：个人、团队、企业是能力组合和治理深度的差异，不是不同产品。
2. **简单优先，复杂受控**：优先使用最小可解释运行模型，只有在结果显著改善时才引入多 Agent、Mesh 或外部委托。
3. **基于环境真值运行**：Agent 不能只依赖语言推理，必须基于工具结果、运行状态、外部反馈和环境观测持续校正。
4. **自治必须先有边界**：任何自治执行都建立在作用域、预算、时间窗、工具集、环境边界和撤销机制之上。
5. **知识是系统资产**：私有记忆、共享知识和组织知识图谱都必须具备来源、权限、晋升、删除和审计语义。
6. **协议优先于私有耦合**：对外工具接入以 MCP 为主，对外 Agent 协作以 A2A 为主，内部模型需与协议边界兼容。
7. **跨表面连续性**：桌面端、Web、移动端对同一 Hub 共享统一对象语义、统一状态机和统一通知语义。
8. **结果与过程都可治理**：Artifact、Trace、Audit、Policy Decision、Knowledge Lineage 都是一等对象。

### 1.4 非目标

本版 PRD 明确不包含以下内容：

- 实现排期、里程碑、人力拆分和项目管理安排
- 代码目录、模块命名、函数签名、数据库表字段和接口样板
- 公共多租户 SaaS 控制平面作为首要交付形态
- 消费级移动设备专属能力作为正式默认能力面，例如短信、定位、提醒事项
- 按 Claude.ai 的消费级工具全集复刻平台能力，例如 `alarm`、`reminder`、`recipes`、`weather`、`places` 或 Google 专属连接器
- 无治理前提下的自由自治网格或任意跨 Hub 自动同步

### 1.5 核心主线

Octopus 的正式核心主线固定为六条：

1. **Agent Runtime**
2. **Knowledge System**
3. **Autonomy Governance**
4. **Collaboration Mesh**
5. **Artifacts & Workspaces**
6. **Protocol Interop**

---

## 2. 用户角色与使用模式

### 2.1 主要用户角色

| 角色 | 典型规模 | 核心诉求 |
| --- | --- | --- |
| 个人操作者 | 1 人 | 用本地或远程 Agent 完成知识工作、自动化和长期协作 |
| 团队负责人 | 2-30 人 | 共享 Agent、共享知识、管理 Run、审阅成果与策略窗口 |
| 审批者 / Reviewer | 1-20 人 | 处理执行类审批、结果复核和异常恢复 |
| KnowledgeSpace 负责人 | 1-N | 负责某个 KnowledgeSpace 的知识晋升、冲突裁决和删除确认 |
| Workspace 管理员 | 1-10 人 | 管理工作区成员、知识边界、Team、工具授权和项目配置 |
| Tenant 管理员 | 1-5 人 | 管理租户用户、模型策略、协议接入、配额、审计和基础治理 |
| 只读观察者 | 若干 | 查看结果、Trace、Artifact、Knowledge 和运行状态 |

### 2.2 使用模式

Octopus 的目标态支持以下运行模式；首版 GA / Beta 切片如下：

| 模式 | 触发方式 | 典型对象 | 首版切片 | 说明 |
| --- | --- | --- | --- | --- |
| 人工任务 | 用户显式输入 | `Run(task)` | `GA` | 最基础的交互型运行 |
| 计划任务 | Cron / 日程 | `Automation` -> `Run(automation)` | `GA` | 到点执行，带预算和审批策略 |
| 事件驱动 | Webhook / Manual / MCP 事件 | `Trigger` -> `Run(watch)` | `GA` | 基于受控事件源启动响应 |
| 审阅治理 | 审批、复核、回放 | `Run(review)` / `ApprovalRequest` | `GA` | 面向人工治理和结果确认 |
| 讨论协作 | 用户显式发起讨论 | `DiscussionSession` -> `Run(discussion)` | `Beta` | 面向多角色讨论与结论沉淀 |
| 常驻代理 | 长时驻留 | `ResidentAgentSession` | `Beta` | 持续监听、分析、必要时自主建 Run |
| 委托协作 | 内部或外部 Agent | `Run(delegation)` / `DelegationGrant` | `Beta` | 用于跨 Agent、跨 Team 或 A2A 协作 |

### 2.3 典型场景

1. **桌面任务与审批（GA）**
   用户在桌面端发起 Task，系统在授权与预算窗口内执行，必要时创建审批并沉淀 Shared Knowledge。

2. **计划任务与 MCP 事件（GA）**
   用户配置基于 `cron`、`webhook`、`manual event` 或 `MCP event` 的 Automation，由系统在 Workspace 主预算下持续运行。

3. **共享知识沉淀（GA 到 Shared Knowledge / Beta 到 Org Graph）**
   Task 和 Automation 的可靠结果先进入候选知识，再写入 Shared Knowledge；正式晋升到组织图谱属于 Beta。

4. **团队 Mesh 协作（Beta）**
   团队在同一 Workspace 内创建多个专业 Agent，围绕某个 Project 的共享知识空间协作完成复杂分析和交付。

5. **外部 Agent 协作（Beta）**
   通过 A2A 将特定子任务委托给外部 Agent 系统，平台保留受托身份、授权范围、预算窗口、回执和审计。

6. **跨端继续（Web / Mobile Beta）**
   用户在桌面端启动 Run，在其他 Client 表面处理审批与追问，并查看 Trace、Artifact、Knowledge Lineage 和预算消耗。

### 2.4 成功标准

对用户而言，Octopus 的成功应体现为：

- 可以在统一平台中理解并管理 Agent、Run、Knowledge、Grant、Artifact，而不是在多个子系统间切换。
- 多 Agent / Mesh 协作相比单 Agent 有明确收益，并且收益可解释、可审计。
- 常驻代理、自动化和事件驱动运行具有可控自治，不会越过授权边界静默执行。
- 私有记忆、共享知识和组织知识图谱可以减少重复输入并提升协作质量。
- 桌面、本地、远程、Web、移动端围绕同一 Hub 保持统一语义与一致认知。

### 2.5 发版切片

为保证首版可交付，Octopus 必须区分目标态能力与当前发版切片：

| 切片 | 正式范围 |
| --- | --- |
| `GA` | `Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP` |
| `Beta` | `A2A`、`Org Knowledge Graph` 晋升、`Mobile`、高阶 `Mesh` 协作、`DiscussionSession`、`ResidentAgentSession` |
| `Later` | 更广泛事件源生态、跨系统深度治理联动、更丰富多端一致能力 |

首版 GA 的进一步约束：

- 正式执行主线以 `run_type=task`、`run_type=automation` 和审批驱动的 `run_type=review` 为主。
- 事件触发仅开放 `cron`、`webhook`、`manual event`、`MCP event`。
- `Shared Knowledge` 进入 GA；`Org Knowledge Graph` 的正式晋升留在 Beta。
- `A2A` 与高阶 Mesh 协作保留架构接口，但不作为首版默认交付承诺。

---

## 3. 统一领域模型与术语

### 3.1 产品表面与目标态部署形态

| 表面 / 形态 | 说明 | 执行能力 |
| --- | --- | --- |
| Desktop 本地模式 | Client 内嵌本地 Hub，默认可离线运行 | 完整执行能力 |
| Desktop 连接远程 Hub | 桌面端连接企业内网或私有云 Hub | 远程 Hub 执行 |
| Web 连接远程 Hub | 浏览器访问远程 Hub | 远程 Hub 执行 |
| Mobile 连接远程 Hub | 移动端用于审批、查看、通知、轻量介入 | 远程 Hub 执行 |

### 3.2 统一术语表

| 术语 | 定义 |
| --- | --- |
| **Hub** | Octopus 的执行与治理中心，是 Agent、Run、Knowledge、Grant、Artifact、Audit 的事实源 |
| **Client** | 用户交互终端，负责展示、输入、连接、缓存和凭证存储，不作为远程业务事实源 |
| **HubConnection** | Client 保存的一条 Hub 连接配置，包含端点、认证、同步和最近状态信息 |
| **Workspace** | 核心协作边界，承载 Agent、Team、Project、共享知识、Artifact、Run 和成员授权 |
| **Project** | Workspace 下的业务上下文边界，承载某一主题下的 Run、Artifact，并关联一个或多个 KnowledgeSpace 视图 |
| **Tenant** | 远程 Hub 中的组织级隔离边界，拥有独立用户、模型策略、审计和配额 |
| **Agent** | 拥有 Identity、Capability、Knowledge Scope 的数字化协作者 |
| **Team** | 一组 Agent 的协作单元，支持 `leadered`、`mesh`、`hybrid` 三种 `coordination_mode` |
| **Run** | 统一正式运行对象，是所有执行、讨论、自动化、监控、委托和审阅的权威执行外壳 |
| **Task** | 以交付结果为目标的业务对象，对应一个或多个 `run_type=task` 的 Run |
| **DiscussionSession** | 以观点碰撞、争议收敛和结论沉淀为目标的讨论对象，对应 `run_type=discussion` |
| **Automation** | 可重复执行的自动化定义，支持计划触发、事件触发和策略约束 |
| **Trigger** | 启动或续接 Run 的正式触发器；首版 GA 正式支持 `cron`、`webhook`、`manual event`、`MCP event` 四类来源 |
| **ResidentAgentSession** | 常驻代理会话，表示 Agent 在某范围内持续观察、分析和发起后续运行的长时对象 |
| **KnowledgeSpace** | Shared Knowledge 的权威容器与权限边界，可作用于 Workspace 或 Project，并必须指定负责人 |
| **KnowledgeAsset** | 可检索、可追溯、可治理的正式知识条目，支持来源、权限、晋升和删除传播 |
| **KnowledgeCandidate** | 尚未晋升为正式共享知识或组织知识的候选知识记录，必须带来源、信任级别和待处理状态 |
| **Agent Private Memory** | Agent 的私有经验集合，仅该 Agent 在授权范围内可直接读取和治理 |
| **ConversationRecallRef** | 对历史会话、讨论或运行片段的结构化引用，用于 episodic recall，不直接等同于长期共享知识 |
| **Org Knowledge Graph** | 组织级知识图谱，承载实体、关系、结论、约束和长期共享知识 |
| **Artifact** | 正式结果对象，可预览、审批、版本化、导出和追溯 |
| **ArtifactSessionState** | Artifact 交互期内的短期会话状态，只用于 UI/runtime 过程，不作为 Hub 长期事实源 |
| **Attachment** | 用户提供的输入材料或外部引用文件，不等同于 Artifact 或 KnowledgeAsset |
| **CapabilityDescriptor** | CapabilityCatalog 中的正式能力描述项，记录 schema、来源、平台、风险级别、fallback 和观测要求 |
| **CapabilityBinding** | 将某个 CapabilityDescriptor 绑定到 Agent、Team、Workspace、Project、Run 或 Tenant 范围内的正式对象 |
| **ToolSearch** | Hub 内部的元能力，用于发现当前主体在当前上下文中可见的 deferred 或 connector-backed capabilities |
| **CapabilityGrant** | 某主体在特定作用域内可使用哪些能力的正式授权对象 |
| **BudgetPolicy** | 对模型、工具、动作次数、成本、时间窗和升级条件的预算约束对象 |
| **DelegationGrant** | 某次内部或外部委托的受控授权对象，定义 authority、scope、budget、expiry |
| **ApprovalRequest** | 控制 Run 是否继续、是否越界、是否允许组织级写入的正式审批对象 |
| **EnvironmentLease** | 对执行环境的受控租约，定义环境类型、租期、心跳、撤销和恢复入口 |
| **A2APeer** | 已登记、可治理的外部 Agent 对端 |
| **ExternalAgentIdentity** | 外部 Agent 的具体身份声明，绑定到某个 A2APeer，用于表示“哪个外部主体正在代表谁行动” |
| **InboxItem** | 面向用户或 Agent 的待处理项，承载审批、澄清、委托、异常、知识晋升等任务，具备归属人、来源对象和状态 |
| **Notification** | 事件驱动的用户提醒，支持站内、桌面和移动端渠道，具备去重键、投递状态和渠道策略 |
| **InteractionPrompt** | 面向 Chat / Inbox / Approval 的结构化提问或确认对象，支持单选、多选、排序等形式 |
| **MessageDraft** | 由 Agent 生成、待人工审阅或发送的正式消息草稿对象，不等同于直接发送动作 |
| **AgentTemplate** | Agent 的可复用模板定义，声明默认角色、能力、知识边界与流程偏好 |
| **ExecutionProfile** | 运行模板，定义模型档案、能力选择、预算缺省、记忆/检索策略与交互行为 |
| **SkillPack** | 在运行阶段即时注入的规则包，用于规划、生成、验证与安全约束，但不能绕过平台治理 |

### 3.3 协作拓扑

Octopus 必须正式支持以下协作拓扑：

| 拓扑 | 说明 |
| --- | --- |
| **Single Agent** | 单 Agent 在授权边界内独立完成 Run |
| **Leadered Team** | 一个 Leader 统一规划、分派和汇总，适合强中心控制场景 |
| **Mesh Team** | 无固定 Leader，Agent 基于委托和共享知识进行受控协作 |
| **Hybrid Team** | Leader 负责设定目标与边界，执行层通过 Mesh 协作完成 |
| **Cross-Team Mesh** | 多个 Team 基于共享 Project 或 DelegationGrant 协作 |
| **External A2A Collaboration** | 内部 Agent 或 Team 与外部 A2A Peer 在受托和预算限制下协作 |

### 3.4 领域不变量

1. **Run 是权威执行对象**：Task、DiscussionSession、Automation 触发和 ResidentAgentSession 行为都必须投影到 Run 体系中。
2. **Team 的 `coordination_mode` 决定拓扑约束**：`leadered` 与 `hybrid` 必须具备 `leader_id`；`mesh` 不要求固定 Leader。
3. **Mesh 约束必须显式建模**：每个 Team 都必须定义 `authority_scope`、`knowledge_scope` 和 `delegation_edges`，不能只靠提示词约定协作边界。
4. **私有记忆不可跨 Agent 直接编辑**：其他 Agent 不能直接写入或修改他人的私有记忆。
5. **共享知识必须经 KnowledgeSpace 授权访问**：共享知识不等于所有参与者自动可见。
6. **组织知识图谱写入必须可追溯**：任何组织级知识都必须保留来源、信任级别、晋升路径和责任人。
7. **知识必须先成为候选条目再晋升**：来自 Run、MCP、A2A 或文件解析的结果不得直接写入长期共享知识或组织图谱。
8. **KnowledgeAsset 以 KnowledgeSpace 为主属**：Project 可以引用共享知识，但不直接成为 Shared Knowledge 的权威所有者。
9. **真实动作必须经过 CapabilityGrant 与 BudgetPolicy**：没有授权窗口或越界升级处理的动作不得执行。
10. **外部 Agent 协作必须有身份与委托对象**：A2A 不允许匿名对端直接参与生产运行。
11. **Attachment、Artifact、KnowledgeAsset 三者不可混用**：输入、结果和知识必须有独立生命周期。
12. **EnvironmentLease 是执行恢复的基础**：需要环境的动作必须绑定可跟踪租约。
13. **Client 可以缓存，不是事实源**：远程模式下 Hub 始终是正式数据和运行状态事实源。
14. **Hub 间默认隔离**：不跨 Hub 自动同步 Agent、Run、Knowledge、Artifact、Grant 或 Audit。
15. **撤销必须优先于自治**：当 CapabilityGrant、BudgetPolicy 或 DelegationGrant 被撤销时，相关运行必须停止或降级。
16. **能力可见性由正式上下文共同决定**：`platform`、connector 状态、`Workspace / Project` policy、`CapabilityGrant` 与 `BudgetPolicy` 共同决定能力是否可见或可执行。
17. **ToolSearch 只发现，不授予权限**：搜索结果只能暴露当前主体可见的能力 schema、治理标签和 fallback，不能自动完成授权。
18. **结构化交互必须进入正式流程**：`InteractionPrompt` 与 `MessageDraft` 必须进入 `Chat / Inbox / ApprovalRequest` 语义，不得只存在于前端私有控件层。
19. **ArtifactSessionState 不是长期事实**：Artifact 的会话态不得被当作长期 memory、共享知识或 Run 恢复快照。
20. **消费级工具只作为 adapter 或 connector**：消费级设备能力、provider-specific connector 和内容型工具不得直接升级为首版核心领域对象。

---

## 4. 核心能力需求

### 4.1 Agent Runtime

#### 业务目标

让 Agent、Team、Run、Automation、Trigger 和 ResidentAgentSession 构成统一运行体系，覆盖人工发起、周期执行、事件响应和长期驻留。

首版说明：

- 本节描述目标态运行模型；首版 GA 强制交付 `task`、`automation`、`review` 和受控 `watch`。
- `discussion` 与 `ResidentAgentSession` 保留对象模型和架构接口，但进入 Beta。

#### 关键需求

- 用户可创建、编辑、停用、归档和删除 Agent。
- Agent 配置必须按 `Identity`、`Capability`、`Knowledge Scope` 三维组织。
- Team 必须支持 `coordination_mode = leadered | mesh | hybrid`。
- 平台必须提供统一 Run 模型，正式支持以下 `run_type`：
  - `task`
  - `discussion`
  - `automation`
  - `watch`
  - `delegation`
  - `review`
- 平台必须提供 `CapabilityCatalog` 与 `CapabilityResolver`，使 Agent、Team、Workspace、Project 与 Run 通过 `CapabilityDescriptor / CapabilityBinding` 引用能力，而不是直接绑定具体第三方工具名。
- 平台必须支持 `ToolSearch`，用于发现当前上下文中允许暴露的 deferred 或 connector-backed capabilities。
- 平台必须支持 `AgentTemplate`、`ExecutionProfile` 与 `SkillPack`，用于声明默认能力、运行偏好和即时规则注入。
- Run 必须支持创建、计划、执行、暂停、等待输入、等待审批、恢复、终止、失败恢复和回放。
- Automation 必须支持计划任务、事件触发、启停、暂停、预算窗口和最近执行记录。
- ResidentAgentSession 必须支持启动、常驻、降级、暂停、停止和恢复。
- 用户可在运行前查看计划，在运行中查看当前阶段，在运行后查看结果、Trace、成本和知识写回。

#### 业务规则

- Task 与 DiscussionSession 是业务对象，Run 是它们的正式执行外壳。
- DiscussionSession 继续保留，但被定义为“正式讨论型 Run 的业务对象”，不再承担所有协作语义。
- `task`、`discussion` 是用户可直接理解和发起的主要业务型 Run。
- `automation`、`watch` 是由 Trigger 或 Resident 语义驱动的运行型 Run，不单独替代业务对象。
- `delegation`、`review` 是治理和协作辅助型 Run，用于承载受控委托与审阅回路，必须在 Runtime 和审计中有独立适配路径。
- `CapabilityCatalog` 必须显式区分 `core`、`deferred`、`connector-backed` 与 `platform-native` 四类能力。
- `CapabilityResolver` 必须综合 `platform`、connector 状态、`Workspace / Project` policy、`CapabilityGrant` 与 `BudgetPolicy` 决定能力是否可见、可搜索和可执行。
- `ToolSearch` 只能返回当前主体可见的 capability descriptor、schema、风险标签和 fallback，不得直接越过审批或预算窗口。
- Mesh 协作不等于无规则协作，任何委托都必须产生可追溯的 DelegationGrant 或等价受控记录。
- ResidentAgentSession 不能直接越过策略创建高风险动作；必须在授权窗口内行动或生成审批。
- 被运行中 Run 或 ResidentAgentSession 引用的 Agent 不能直接删除，只能停用或归档。
- 如果模型档案、Skill、MCP 绑定或策略窗口失效，Agent 进入“运行受限”状态，不能发起新的自治运行。
- `SkillPack` 是执行时约束和增强，不得凭空创造平台未注册能力，也不得绕过 CapabilityGrant、BudgetPolicy 或 ApprovalRequest。

#### 异常与边界

- 计划生成失败时，系统必须允许用户手动编辑计划、重试规划或改用人工指定协作路径。
- ResidentAgentSession 所依赖的 Trigger 或事件源不可用时，系统需进入降级状态并发出通知。
- 当执行环境不足或 EnvironmentLease 获取失败时，Run 可以等待、改派、降级或终止，不得静默跳过。
- 同一 Trigger 的重复投递必须通过幂等键去重，不得生成重复业务结果。

#### 验收标准

- 用户可以在统一入口中理解 Agent、Team、Run、Automation 和 ResidentAgentSession 的关系。
- 同一业务场景可从人工 Task 平滑演进为 Automation 或 Resident 触发，而无需改写领域语义。
- Hub 重启后，未完成 Run 与常驻会话可以恢复且不会重复执行已确认动作。
- Mesh Team 在无固定 Leader 的前提下仍可在规则内完成协作。

### 4.2 Knowledge System

#### 业务目标

让 Agent 和记忆、团队共享知识以及组织知识图谱构成统一、可治理、可追溯的知识系统。

首版说明：

- 首版 GA 强制交付 `Agent Private Memory` 与 `Shared Knowledge`。
- `Org Knowledge Graph` 的正式晋升链路保留对象模型，但进入 Beta。

#### 关键需求

- 平台必须正式支持三层知识体系：
  - `Agent Private Memory`
  - `Conversation Recall`
  - `KnowledgeCandidate -> Shared Knowledge -> Org Knowledge Graph`
- 平台必须支持 `ConversationRecallRef`，用于按 `Workspace / Project` 边界引用历史会话、讨论和运行结论。
- Run 执行前必须基于授权范围自动检索相关知识。
- Task、Discussion、Automation 和 Resident 观察结果可异步提取为候选知识。
- 候选知识必须至少区分 `candidate`、`verified_shared`、`promoted_org`、`revoked_or_tombstoned` 四类治理状态。
- 用户和管理员可查看知识来源、权限、信任等级、最近使用情况和晋升历史。
- 平台必须支持手动录入、自动提取、人工确认晋升和撤销删除。
- 知识条目必须支持实体、关系、结论、事实、操作经验和约束规则等类型。
- 每个 KnowledgeSpace 必须指定至少一个负责人，用于组织级知识晋升、冲突裁决和删除确认。

#### 业务规则

- 私有记忆只归属于单个 Agent，不因 Team 成员身份自动共享。
- `ConversationRecallRef` 只表示对历史会话片段的结构化引用，不等同于共享知识事实，也不能绕过候选知识晋升路径。
- Shared Knowledge 必须通过 KnowledgeSpace 管理，并按 Workspace、Project 或显式授权决定可见范围。
- Project 对共享知识只形成引用和视图，不直接拥有 Shared Knowledge。
- Org Knowledge Graph 只接收经过写回门控、来源校验和必要审批的正式知识。
- 候选知识先进入待验证队列；只有通过校验和必要审批后，才能晋升为共享知识或组织图谱事实。
- 从共享知识晋升到组织级图谱时，必须保留晋升责任人、依据 Artifact、引用关系和删除传播策略。
- 组织知识图谱事实默认继承来源 KnowledgeSpace 的负责人作为事实 owner，除非审批时显式改写。
- 来自 MCP、外部 HTTP、文件解析或 A2A 对端的结果默认不可信，不能直接写入长期知识。
- 删除知识后，该知识不得继续参与后续检索，但平台保留最小化的墓碑和 lineage 记录以支持审计。
- 组织级知识删除或降级时，必须处理向量索引、图谱关系和共享视图的一致性传播。

#### 异常与边界

- 向量索引不可用时，不得阻塞主流程结果，但必须记录失败并触发重试。
- 知识冲突时，系统必须保留冲突视图和来源差异，不得无提示覆盖。
- 大模型提取质量不达标时，管理员可关闭自动晋升，只保留候选条目或人工确认路径。

#### 验收标准

- Agent 在第二次处理相似任务时能够引用相关私有记忆或共享知识。
- 用户可以看到某条知识从 Artifact 到候选条目，再到共享知识或组织图谱的完整链路。
- 撤销或删除某条知识后，它不会继续被后续检索直接命中。
- 组织知识图谱的条目具有明确来源、责任归属和可信度标记。

### 4.3 Autonomy Governance

#### 业务目标

在支持高自治运行的同时，将权限、预算、审批、撤销和责任追溯做成正式系统能力。

#### 关键需求

- 平台必须支持 `CapabilityGrant`，定义主体、工具集、协议能力、数据范围、环境范围和授权时间窗。
- 平台必须支持 `BudgetPolicy`，覆盖成本、token、动作次数、重试次数、运行时长和升级阈值。
- 平台必须支持 `ApprovalRequest`，用于高风险动作、越界预算、知识晋升、外部委托和关键结论确认。
- 平台必须支持撤销、过期、暂停、重新授权和升级审批。
- 用户必须能看到当前 Run 的授权窗口、预算消耗和即将触发的升级条件。
- 管理员必须能配置默认策略、强制审批规则和不同环境下的沙箱等级。
- 平台必须至少内建四类默认审批：`execution`、`knowledge_promotion`、`external_delegation`、`export_sharing`。
- BudgetPolicy 必须支持 `workspace -> project -> run` 的层级预算，其中 Workspace 为默认主预算。

#### 业务规则

- 没有 CapabilityGrant 的能力不得在运行中隐式出现。
- 高风险动作、跨边界写操作、外部委托和组织级知识写入必须被视为可治理动作。
- `auto_approve` 只能作用于被允许自动化的能力范围，不能覆盖高风险动作和组织级写入。
- 预算超限、权限越界、信任级别不足或环境不满足时，Run 必须暂停、降级或升级审批。
- 撤销优先生效：当授权窗口或预算被撤销时，平台必须尽快收敛相关运行。
- 授权求值必须遵循统一优先级：`Tenant Hard Policy -> Workspace Policy -> Role / Permission -> CapabilityGrant -> BudgetPolicy -> ApprovalRequest`。
- 任一层显式 `deny` 都优先于下游 `allow`；`BudgetPolicy` 只能约束已存在能力，不能凭空授予新能力。
- `ApprovalRequest` 只能作为可升级动作的越界通道，不能覆盖 Tenant 明确标记为不可覆盖的硬禁止策略。
- 默认审批路由不得全部归于 `reviewer`：执行类走 `reviewer`，知识晋升类走 `KnowledgeSpace` 负责人，外部委托类走 `tenant_admin`，导出/共享类走 `workspace_admin`，并允许被策略覆盖。
- Workspace 是默认主预算 owner；Project 和 Run 只派生子预算与消耗视图，不独立成为全局主预算池。

#### 异常与边界

- 多端重复处理同一审批时，只允许一次结果生效，其他请求需返回已处理结果。
- 授权窗口到期时，Run 不得静默延长执行；需终止、等待续授权或回退。
- 管理员策略变更影响正在运行的自治任务时，系统需显式标记并决定立即生效或在安全点生效。

#### 验收标准

- 用户可以通过“预授权 + 越界审批”的方式让 Agent 在既定边界内连续工作。
- 高风险动作一定会触发可见审批或被策略阻断。
- 预算耗尽、策略撤销或超时后，Run 会正确停留在待恢复或终止状态。
- 审计记录可以清楚区分“预先授权了什么”和“真正执行了什么”。

### 4.4 Collaboration Mesh

#### 业务目标

让 Octopus 在支持单 Agent、Leadered Team、Mesh Team、Cross-Team Mesh 和 External A2A Collaboration 的同时，保持协作语义统一、委托边界清晰。

首版说明：

- 本节描述目标态协作模型；首版 GA 不默认交付高阶 Mesh、Cross-Team Mesh 或外部 A2A 协作。

#### 关键需求

- Team 必须支持 `leadered`、`mesh`、`hybrid` 三类协作模式。
- 平台必须支持内部 Agent 之间的受控委托、跨 Team 委托和 A2A 外部委托。
- 平台必须支持 `authority_scope`、`knowledge_scope`、`delegation_edges` 等正式协作约束。
- DiscussionSession 必须支持作为讨论型 Run 参与 Mesh 协作，并输出结论 Artifact。
- 平台必须支持 Agent 和用户的 Inbox / WorkQueue，用于待处理审批、澄清、委托、异常和知识晋升。
- 用户可在同一 Project 下查看多 Agent 协作路径、委托图和结果汇总。

#### 业务规则

- Mesh 协作中的每次委托都必须有明确的责任边界，不能形成不可解释的隐式转包。
- Team 必须把 `authority_scope`、`knowledge_scope`、`delegation_edges` 作为可存储、可校验、可审计的正式配置，而不是会话级约定。
- Team 是协作拓扑单元，不是 Shared Knowledge 的主属边界；`knowledge_scope` 只引用可访问的 KnowledgeSpace。
- Cross-Team Mesh 必须经过 Workspace 或 Project 层级的共享知识与权限约束。
- 外部 A2A 协作必须通过 A2APeer 和 ExternalAgentIdentity 建模，不允许绕过平台治理。
- `A2APeer` 表示对端系统注册；`ExternalAgentIdentity` 表示当前实际执行的外部主体身份，委托与回执都必须绑定到具体身份声明。
- Discussion 默认不启用高风险工具；开启“工具增强讨论”时，只允许授权范围内的低风险或受控中风险工具。
- 讨论参与者集合在会话启动后保持固定；续会通过恢复同一 DiscussionSession 或创建引用前序结论的新会话实现。

#### 异常与边界

- 某内部 Agent 或外部 A2A 对端不可用时，系统必须允许替换、降级、等待或终止，不得静默假定成功。
- Mesh 形成循环委托或委托深度超限时，系统必须阻断并给出解释。
- 外部 A2A 回执不完整、超时或信任级别下降时，平台必须将结果标记为受限可信。

#### 验收标准

- 无固定 Leader 的 Team 可以在共享知识与授权边界内完成协作。
- 用户可以在 Trace 或 Delegation Graph 中看清谁把什么工作委托给了谁。
- 外部 A2A Agent 只能在受托身份和明确预算内行动。
- 讨论结果能够作为 Artifact 和知识候选进入后续协作链路。

### 4.5 Artifacts & Workspaces

#### 业务目标

让 Workspace、Project、Artifact、Attachment、Inbox 和 Notification 构成统一的协作面和正式结果面。

#### 关键需求

- Workspace 必须成为共享协作、共享知识和共享治理的基本边界。
- Project 必须承载某一业务上下文下的 Run、Artifact、附着的 KnowledgeSpace 视图和成员协作视图。
- Artifact 必须支持版本化、预览、审批、导出、引用、Lineage 和当前有效版本概念。
- Attachment 必须支持上传、预览、解析状态、来源引用和替换。
- InboxItem 必须覆盖审批、澄清、异常、委托、知识晋升、自动化告警和恢复待办。
- Chat 与 Inbox 必须支持 `InteractionPrompt` 与 `MessageDraft` 的结构化交互。
- Artifact runtime 可以维护 `ArtifactSessionState`，用于短期 UI 状态、临时草稿或渲染过程状态。
- Notification 在目标态必须支持站内、桌面、移动端推送，并按去重键聚合；首版 GA 至少支持站内与桌面。

#### 业务规则

- Workspace 管理共享边界，不等于 Tenant；Tenant 管理组织治理和资源隔离。
- Shared Knowledge 的主属边界是 KnowledgeSpace；Project 通过附着的 KnowledgeSpace 呈现共享知识，而不是直接拥有 Shared Knowledge。
- Artifact 是正式结果对象；Attachment 是输入对象；KnowledgeAsset 是正式知识对象，三者生命周期独立。
- `InteractionPrompt` 必须具备归属对象、schema、发起主体和可审计结果，不能只是前端控件行为。
- `MessageDraft` 是待审阅或待确认的正式草稿对象，不等同于直接发送动作。
- 用户在审批阶段修改 Artifact 时，必须生成新版本，旧版本仍可追溯。
- InboxItem 必须具备可归因目标对象，不允许无来源待办。
- InboxItem 必须具备 `owner/assignee`、`state`、`priority` 和 `target object`，才能作为正式待处理事实存在。
- Notification 的语义不能替代 Inbox；Notification 是提醒，Inbox 是待处理事实。
- Notification 必须具备去重键、投递状态和抑制策略，不能因同一事件短时间重复轰炸。
- `ArtifactSessionState` 只允许存在于 Artifact 会话生命周期内，默认在会话结束时清空，且不得写入长期 Knowledge 或 Run 恢复状态。

#### 异常与边界

- Attachment 解析失败时，必须保留原文件并标记状态，但不得中断整个 Project 的可见性。
- 导出失败时，不得影响 Artifact 本身的有效版本。
- WorkQueue 堆积时，系统需支持优先级、分派和催办。

#### 验收标准

- 用户能清楚区分输入材料、正式结果和正式知识。
- 同一 Project 下的 Run、Artifact、InboxItem 和 Knowledge 可以形成完整链路。
- 审批、知识晋升、异常恢复都可以通过 Inbox 定位到对应对象。
- Notification 不会短时间重复轰炸同一用户同一事件。

### 4.6 Protocol Interop

#### 业务目标

让 MCP 和 A2A 成为平台的一等互操作能力，而不是零散扩展点。

首版说明：

- 本节描述目标态互操作层；首版 GA 只强制交付 `MCP`，`A2A` 进入 Beta。

#### 关键需求

- 平台必须正式支持 MCP，用于工具、数据源、工作流和应用接入。
- 平台必须正式支持 A2A，用于外部 Agent 的发现、身份、委托、回执和治理。
- 平台必须维护统一的 `CapabilityCatalog`，覆盖原生能力、MCP、A2A、artifact runtime 与 `SkillPack` 注入入口。
- 平台必须支持 `CapabilityBinding`，以便在 `Agent / Team / Workspace / Project / Run / Tenant` 范围内绑定能力。
- MCP 和 A2A 都必须进入统一的能力目录、策略校验、信任分级、Trace 和 Audit 体系。
- 用户和管理员必须能看到对端身份、可用能力、最近健康状态、信任级别和授权范围。
- 外部结果必须支持 provenance、可信度标记和写回门控。

#### 业务规则

- MCP 负责“连接外部工具和数据”。
- A2A 负责“连接外部 Agent 和协作系统”。
- `alarm`、`reminder`、`recipes`、`weather`、`places` 与 provider-specific connectors 默认只能以 adapter 或 MCP connector 形态接入，不进入首版核心领域对象。
- `CapabilityCatalog` 的可见性求值不得复制 Claude “Project 不影响工具行为”的假设；`Workspace / Project` policy 必须能影响能力暴露与搜索结果。
- 任一协议接入都不能绕过 CapabilityGrant、BudgetPolicy、ApprovalRequest 和审计。
- 协议输出默认不等于系统事实；必须由平台决定能否进入 Artifact、Knowledge 或 Org Knowledge Graph。
- 外部 Agent 不能假冒内部 Agent 身份；平台必须区分本地主体、租户主体和外部主体。
- 外部身份轮换、吊销和信任降级必须作用于 `ExternalAgentIdentity`，而不只是作用于粗粒度的 `A2APeer` 记录。

#### 异常与边界

- 协议对端离线、鉴权失败或能力声明变化时，平台必须显式标记降级风险。
- 外部 Agent 提供的结果若超出原委托范围，平台必须拒收或升级审批。
- MCP 或 A2A 对端发生安全事件时，管理员必须可以快速吊销。

#### 验收标准

- 用户在 Agent 配置页能看到 MCP 与 A2A 的有效能力与治理边界。
- 用户或管理员可以解释为什么某个 capability 在当前 `platform + connector + policy + grant` 条件下可见、不可见或只可搜索不可执行。
- 外部 Agent 协作不会绕过审计和授权。
- 协议输出不会未经门控直接进入组织级长期知识。
- 协议对端健康变化可在运行态和治理界面中被感知。

---

## 5. 关键业务流程与交互面

### 5.1 核心交互面

| 交互面 | 面向对象 | 核心用途 |
| --- | --- | --- |
| **Chat** | 用户、Agent、Run | 发起任务、讨论介入、处理 `InteractionPrompt`、查看流式输出和结构化澄清 |
| **Board** | Run、Automation、Resident | 查看运行状态、阶段、预算、阻塞项和负责人 |
| **Trace** | Run、Action、Delegation、Policy | 回放执行路径、工具调用、审批、预算命中和异常 |
| **Inbox** | 用户、Reviewer、Manager | 处理审批、越界、异常、知识晋升、消息草稿审阅和委托待办 |
| **Knowledge** | KnowledgeSpace、KnowledgeAsset、Graph | 查看知识条目、图谱关系、来源、使用情况和治理状态 |
| **Workspace / Project** | 团队、项目、Artifact | 管理协作边界、共享资产和结果沉淀 |
| **Hub Connections** | Client、Hub | 管理本地 Hub、远程 Hub、认证与缓存状态 |

### 5.2 首次使用：桌面端本地模式

1. 用户安装并启动桌面端。
2. 系统初始化本地 Hub、本地默认 Workspace 和本地知识存储。
3. Onboarding 引导用户选择场景、默认模型档案和模板。
4. 用户创建首个 Agent 或使用模板生成 Team。
5. 用户在 Chat 中发起首个 Run，或启用首个 Automation。

异常要求：

- 初始化失败时，必须给出修复建议、日志入口和本地数据位置说明。
- 本地模型不可用时，系统需提示切换远程 Provider 或连接远程 Hub。

### 5.3 自动化与常驻代理（GA + Beta）

1. 用户创建 Automation；`ResidentAgentSession` 为 Beta 能力。
2. 系统配置 Trigger、CapabilityGrant、BudgetPolicy、Knowledge Scope 和通知策略。
3. 事件或计划触发后，系统创建 Run。
4. Run 在预算窗口内执行，必要时请求审批或发出澄清。
5. 结果被写入 Artifact、Inbox、Knowledge 候选或通知中心。

异常要求：

- 事件重复投递必须去重。
- 首版 GA 仅支持 `cron`、`webhook`、`manual event`、`MCP event` 四类 Trigger；邮件、本地文件监听和第三方业务系统事件留在后续阶段。
- 预算超限、权限撤销、对端离线或环境不可用时必须进入可见阻塞状态。

### 5.4 Mesh 协作与共享知识（Beta）

1. 用户在某个 Workspace / Project 下发起 Task。
2. 系统为参与 Agent 和 Team 载入私有记忆、共享知识和相关组织知识。
3. Mesh 或 Hybrid 协作开始，Agent 按权限进行受控委托。
4. 中间结果形成 Artifact，争议点或越界行为进入 Inbox / Approval。
5. 最终结果沉淀为 Artifact，并按策略进入知识写回与图谱晋升。

异常要求：

- 委托链超过限制、形成循环或出现信任不足时必须阻断并解释。

### 5.5 外部 A2A 协作（Beta）

1. `tenant_admin` 登记外部 A2APeer。
2. 系统完成身份、能力、信任和策略校验。
3. 内部 Run 在需要时创建 DelegationGrant 并委托给外部对端。
4. 外部对端回传结果、状态和回执。
5. 平台对回执做范围校验、信任评估和写回门控，再决定是否纳入 Artifact 或 Knowledge。

异常要求：

- 外部对端超时、身份失效或行为越界时，必须能中止、撤销和追溯。

### 5.6 多 Hub 与跨端继续

1. Client 支持保存多个 HubConnection。
2. 用户切换 Hub 时，系统刷新 Workspace、Project、Run、Knowledge 和权限上下文。
3. 用户可在不同 Client 表面继续同一 Run、处理同一 InboxItem 或查看同一 Trace。

异常要求：

- 远程离线时，Client 只能展示缓存快照并阻止远程写操作。
- Token 过期与 Hub 离线必须被区分展示。

---

## 6. 企业治理、安全与平台策略

### 6.1 Tenant、Workspace、Role 与 Permission 模型

#### 业务目标

在不破坏统一平台心智的前提下，支持组织级隔离、团队级边界和细粒度权限治理。

#### 关键需求

- Tenant 负责组织级隔离、模型策略、协议治理和审计。
- Workspace 负责协作边界、共享知识、项目和成员可见性。
- Role 与 Permission 必须分离建模。
- 平台必须支持预置角色和自定义角色。

#### 预置角色

| 角色 | 典型职责 |
| --- | --- |
| `hub_admin` | 管理 Hub 基础设施、租户、系统级策略和协议接入 |
| `tenant_admin` | 管理本租户用户、模型策略、预算基线、审计、配额和 A2A Peer 登记 |
| `workspace_admin` | 管理 Workspace 成员、KnowledgeSpace 负责人、Team、Project、导出共享与授权窗口 |
| `reviewer` | 处理执行类审批、结果审阅和异常恢复 |
| `operator` | 创建 Agent、发起 Run、配置 Automation、查看结果 |
| `viewer` | 只读查看结果、Trace、Knowledge 和状态 |

补充规则：

- `KnowledgeSpace` 负责人不是全局系统角色，而是由 `workspace_admin` 在具体 KnowledgeSpace 上指定的责任人。
- `KnowledgeSpace` 负责人自动获得该空间内的派生权限：`knowledge.promote` 与 `approval.knowledge.review`。
- `external_delegation` 默认不授予 `reviewer` 或 `operator` 直接审批权。
- 外部委托请求必须具备显式权限；默认由 `workspace_admin` 持有 `delegation.external.request`，`operator` 需要额外授权。

#### Permission 范围

- `agent.create` / `agent.update` / `agent.archive`
- `team.create` / `team.manage`
- `run.submit` / `run.resume` / `run.terminate`
- `automation.manage` / `resident.manage`
- `knowledge.read` / `knowledge.write` / `knowledge.promote`
- `grant.issue` / `grant.revoke`
- `delegation.external.request`
- `approval.execution.review`
- `approval.knowledge.review`
- `approval.delegation.review`
- `approval.export.review`
- `artifact.export`
- `mcp.manage` / `a2a.manage`
- `workspace.manage` / `tenant.manage`

#### 授权求值顺序

正式动作执行前，平台必须按照以下顺序做统一判定：

1. `Tenant Hard Policy`
2. `Workspace Policy`
3. `Role / Permission`
4. `CapabilityGrant`
5. `BudgetPolicy`
6. `ApprovalRequest`

判定规则：

- 上游硬禁止优先于下游允许。
- Workspace 只能在 Tenant 允许范围内收紧或细化策略，不能突破组织级硬边界。
- `CapabilityGrant` 只为已被角色和策略允许的能力开窗口，不替代 RBAC。
- `BudgetPolicy` 决定“能做多久、做多少、花多少”，不决定“是否拥有该能力”。
- `ApprovalRequest` 仅用于越界升级与恢复，不得覆盖不可审批的硬禁止策略。

### 6.2 模型中心与预算中心

平台必须提供四层模型治理对象：

1. `ModelProvider`
2. `ModelCatalogItem`
3. `ModelProfile`
4. `TenantModelPolicy`

产品规则：

- Agent 选择的是 `ModelProfile`，不是裸模型字符串或私钥。
- Summary、Graph Extraction、Embedding 等系统级能力使用租户或工作区默认档案。
- BudgetPolicy 应覆盖模型调用成本、动作次数和总体运行时间。
- 不同协作模式和风险等级可以绑定不同默认模型和预算基线。
- 默认主预算挂载在 `Workspace`；`Project` 与 `Run` 只继承并细分子预算，不直接成为跨上下文共享预算池。
- Automation 的子预算必须定义 `budget_period` 或等效 reset 规则；默认按 Automation 所属调度窗口重置，并受 Workspace 主预算约束。

### 6.3 安全与隐私要求

#### 风险分级

| 风险等级 | 示例 | 默认状态 |
| --- | --- | --- |
| Low | 搜索、读取、解析、Markdown 渲染、知识检索 | 可在授权范围内默认允许 |
| Medium | 文件写入、受控 HTTP 读取、沙箱代码执行、共享知识写入 | 需显式授权，可按策略升级审批 |
| High | 系统命令、删除、外部写操作、组织级变更、外部高权限委托 | 必须显式授权并强制治理 |

#### 安全规则

- 远程凭证必须保存在操作系统安全存储或 Hub Secret Store 中。
- 审计与 Trace 必须脱敏展示敏感字段。
- 平台必须定义最少四类执行环境等级：
  - `local_trusted`
  - `tenant_sandboxed`
  - `ephemeral_restricted`
  - `external_delegated`
- 平台必须具备 prompt injection quarantine、信任分级、结果门控和身份冒用防护。

### 6.4 审计、配额与治理策略

- 审计至少覆盖高风险动作、授权发放、授权撤销、审批结果、委托、知识晋升、导出和角色变更。
- 配额至少覆盖 Agent 数、Run 并发数、存储容量、向量索引容量、MCP 接入数、A2A 对端数和预算上限。
- 治理策略至少可控制默认模型、默认预算、默认审批、知识写回开关、导出权限和协议白名单。

### 6.5 评测与上线门禁

Octopus 必须把评测与治理验证视为正式平台能力，而不是后补流程。

要求：

- 关键 Agent 模板、协作拓扑、常驻代理策略、MCP 接入和 A2A 接入必须具备评测用例。
- 平台必须能区分规划失败、工具失败、策略命中、预算超限、恢复失败、知识污染等失败类型。
- 评测必须覆盖不同 `run_type`、知识晋升路径、委托路径、策略命中和恢复路径，而不是只覆盖单轮问答质量。
- 平台必须保存评测样例、失败分类、回归基线和上线门禁结果，使其成为可追溯的正式治理资产。
- 高风险能力默认启用前必须经过安全评测、对抗测试和人工 review。

---

## 7. 非功能性需求

### 7.1 性能与交互指标

| 指标 | 目标值 |
| --- | --- |
| 桌面端冷启动时间 | `< 3s` |
| Chat 首次可交互时间 | `< 2s` |
| 流式输出首 Token 延迟 | `< 500ms`（不含模型网络时延） |
| Board / Inbox 首屏加载时间 | `< 1s` |
| 私有记忆 / 共享知识检索延迟 | `< 500ms` |
| Discussion 首轮发言启动时间 | `< 3s` |
| 组织图谱实体查询时间 | `< 1s` |
| 审批通知投递到达时间 | `< 5s` |

### 7.2 可靠性与运营指标

| 指标 | 目标或要求 |
| --- | --- |
| 任务成功率 | 作为正式运营指标持续追踪 |
| 代理自治成功率 | 作为 Automation / Resident 质量指标持续追踪 |
| 审批时延 | 作为治理效率指标持续追踪 |
| 恢复成功率 | Hub 重启、断连和环境租约恢复必须可衡量 |
| A2A 委托成功率 | 外部协作必须可观测 |
| 知识召回有效率 | 必须能评估知识是否真正改善结果 |
| 每 Run 成本预算命中率 | 预算治理必须可统计 |
| 异常中止率 | 需要区分策略中止、失败中止和人工终止 |
| 人工介入率 | 用于衡量自治程度与治理负担 |
| 策略命中率 | 用于衡量授权窗口和审批规则设计效果 |

### 7.3 可靠性要求

- Hub 重启后，未完成 Run、Automation 和 ResidentAgentSession 必须支持恢复。
- 高风险审批不得因客户端离线被静默跳过。
- Trigger 投递、审批处理、委托回执和知识晋升必须具备幂等机制。
- 记忆写回、知识提取和图谱晋升失败不影响主结果完成，但必须留下失败事件和重试入口。

### 7.4 可扩展性要求

- 新模型 Provider、Skill、MCP Server、A2A Peer 类型可扩展接入。
- 本地模式与远程模式共享同一对象模型、状态机和治理语义。
- 协议扩展不应破坏核心主线中的授权、审计、恢复和知识边界。

### 7.5 目标态平台支持

| 平台 | 说明 |
| --- | --- |
| macOS | 本地模式与远程模式完整支持 |
| Windows | 本地模式与远程模式完整支持 |
| Linux | 本地模式与远程模式完整支持 |
| Web | 远程 Hub 访问、治理、回放和协作 |
| iOS / Android | 审批、通知、查看、轻量介入与跨端继续 |

---

## 8. 约束、假设与文档边界

### 8.1 明确约束与默认假设

- 采用统一平台设计，不按版本拆分个人、团队、企业产品线。
- 首版 GA 以 `Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP` 为主交付。
- 目标态正式支持长期驻留代理；首版 GA 不默认交付。
- 目标态正式支持私有记忆、共享知识库和组织知识图谱三层知识体系；其中 `Org Knowledge Graph` 正式晋升留在 Beta。
- 目标态正式支持去中心化 Agent 网格；`Leader` 不再是所有 Team 的全局硬约束，高阶 Mesh 协作留在 Beta。
- 正式采用“策略 / 预算授权 + 越界审批”的治理模型。
- 正式把 `MCP + A2A` 纳入核心架构，而不是扩展备注；首版 GA 默认只开放 `MCP`。
- 不支持跨 Hub 自动共享正式业务对象。

### 8.2 本文档不锁定的实现细节

以下事项留给实现阶段和 SAD / 详细设计处理，不改变本 PRD 的产品语义：

- 组织知识图谱的物理存储实现
- A2A 的具体传输映射与网关实现
- 不同执行环境的容器化或虚拟化实现细节
- 具体 API 路由、数据库表结构和事件 Envelope 字段

### 8.3 结论

Octopus 的目标不是再做一个“会调用工具的聊天助手”，而是做一个统一的 Agent Runtime Platform。平台必须同时成立三件事：

1. 把 Agent、Run、Knowledge、Grant、Artifact、Interop 做成正式系统对象。
2. 把自治、协作、审批、恢复和审计收敛到统一运行模型中。
3. 让个人、团队和企业在同一平台内按不同治理深度使用同一套核心语义。
