# Octopus · 开发计划主文档（DEVELOPMENT_PLAN）

**版本**: v0.1.0 | **状态**: 正式执行基线 | **日期**: 2026-03-26
**权威输入**: PRD v2.1 · SAD v2.1 · CONTRACTS v1.1 · ENGINEERING_STANDARD v0.2.2 · VIBECODING 2026-03-26
**配套文档**: [DEVELOPMENT_CHANGELOG.md](./DEVELOPMENT_CHANGELOG.md)

---

## 1. 文档目标与适用范围

本文件用于回答一个问题：**在 `octopus` 当前 `doc-first rebuild` 状态下，AI 和人工应如何按阶段推进开发而不偏离 PRD 与 SAD**。

本文件是仓库级开发执行主文档，负责定义：

- 分阶段开发顺序与阶段门禁
- 每阶段的范围、前置依赖、产出物与验收标准
- AI 执行时的防跑偏规则
- 跨阶段公共约束与最小验证要求

本文件不替代以下文档：

- 产品语义、范围、非目标与发版切片：见 [PRD.md](./PRD.md)
- 架构平面、状态机、恢复与治理模型：见 [SAD.md](./SAD.md)
- 正式对象、枚举、事件与 capability 基线：见 [CONTRACTS.md](./CONTRACTS.md)
- 具体工程实现规范、评审与完成定义：见 [ENGINEERING_STANDARD.md](./ENGINEERING_STANDARD.md)
- AI 执行边界、增量纵切片原则与仓库事实基线：见 [VIBECODING.md](./VIBECODING.md)

---

## 2. 权威输入文档列表

后续任何实现、重构、契约扩展或门禁变化前，必须先对齐以下文档：

1. [README.md](../README.md)
2. [AGENTS.md](../AGENTS.md)
3. [PRD.md](./PRD.md)
4. [SAD.md](./SAD.md)
5. [CONTRACTS.md](./CONTRACTS.md)
6. [contracts/README.md](../contracts/README.md)
7. [ENGINEERING_STANDARD.md](./ENGINEERING_STANDARD.md)
8. [VIBECODING.md](./VIBECODING.md)
9. [adr/README.md](./adr/README.md)
10. [DEVELOPMENT_CHANGELOG.md](./DEVELOPMENT_CHANGELOG.md)

若上述文档之间存在冲突，以以下顺序处理：

1. 先暂停实现
2. 明确冲突点属于产品、架构、契约、工程规范还是执行边界
3. 需要变更正式语义时，优先更新 PRD / SAD / CONTRACTS / ADR，而不是靠代码或聊天记录绕过

---

## 3. 当前仓库事实边界

截至 2026-03-26，当前仓库仍处于 `doc-first rebuild` 状态，必须遵守以下事实边界：

- 当前 tracked tree 的正式事实源是仓库根目录文档、`docs/`、`contracts/` 与 `.github/`。
- 当前仓库不包含可执行的 `apps/`、`packages/`、`crates/` 实现骨架，也不包含根级 `package.json`、`pnpm-workspace.yaml`、`Cargo.toml` 等 workspace manifests。
- 任何运行时、API、UI、测试或构建能力，都必须以后续重新引入的 tracked manifests、源码与验证结果为准。
- 现阶段允许真实声明的最小验证集合是：required docs、contract source existence、旧引用扫描、聚焦 diff 复核、`git diff --check`。
- 旧的计划目录与旧的变更目录已被当前 guardrails 视为废弃目录模式，不得恢复使用。

当前正式重建焦点仍以以下能力为优先：

- `CapabilityCatalog`
- `CapabilityResolver`
- `ToolSearch`
- 结构化交互
- 分层记忆
- `ArtifactSessionState`
- `SkillPack`

---

## 4. 开发总原则与防跑偏规则

### 4.1 必守不变量

- `Run` 是唯一权威执行外壳；`Task`、`Automation`、`DiscussionSession`、`ResidentAgentSession` 等业务对象都必须投影到 `Run` 体系中。
- `Project` 不拥有 `Shared Knowledge`；它只挂载 `KnowledgeSpace` 视图。
- `ToolSearch` 只负责发现 capability，不授予 capability。
- `ArtifactSessionState` 只能是 session-scoped 短期状态，不能进入长期事实层、恢复快照或共享知识。
- 外部协议结果默认低信任，必须经过 provenance、trust、write gate、grant、budget、approval 才能进入长期事实。
- `CapabilityDescriptor / CapabilityBinding` 是能力目录唯一入口，顶层领域对象不得直接绑定第三方工具名。

### 4.2 范围控制原则

- 首版默认交付严格限定为 PRD 与 SAD 明确承诺的 `GA` 子集：`Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`。
- `Beta` 与 `Later` 能力只能在 `GA` 闭环稳定、评测记录齐备后解锁。
- 不得把 `DiscussionSession`、`ResidentAgentSession`、高阶 `Mesh`、`A2A`、`Org Knowledge Graph` 正式晋升、`Mobile` 混入首版 GA 默认任务。
- 不得把 consumer-only 工具或 provider-specific connector 升格为核心领域对象；如需引入，只能以 adapter 或 connector-backed capability 进入。

### 4.3 文档同步原则

若变更影响正式文档入口、工程规则、验证门禁或执行边界，必须同步更新：

- [README.md](../README.md)
- [AGENTS.md](../AGENTS.md)
- [ENGINEERING_STANDARD.md](./ENGINEERING_STANDARD.md)
- [VIBECODING.md](./VIBECODING.md)
- [.github/workflows/guardrails.yml](../.github/workflows/guardrails.yml)
- [.github/pull_request_template.md](../.github/pull_request_template.md)

若变更影响契约语义，还必须同步更新：

- [CONTRACTS.md](./CONTRACTS.md)
- [contracts/README.md](../contracts/README.md)
- `contracts/v1/`
- 相关 ADR

### 4.4 变更登记原则

- 每个实际开发任务开始前，先在 [DEVELOPMENT_CHANGELOG.md](./DEVELOPMENT_CHANGELOG.md) 建立记录。
- 每个任务完成后，必须回填结果、验证方式、风险与状态。
- 普通阶段进展不写 ADR。
- 仅当出现架构例外、目录边界调整、主技术栈变化、协议模型变化或契约语义重大变化时，才进入 `docs/adr/`。

---

## 5. 阶段总览

| Phase | 名称 | 目标 | 退出门槛 | 当前状态 |
| --- | --- | --- | --- | --- |
| 0 | 文档治理与执行基线 | 建立正式开发计划、变更记录与入口门禁 | `DEVELOPMENT_PLAN` / `DEVELOPMENT_CHANGELOG` 落地并完成入口同步 | completed |
| 1 | 契约与状态模型先行 | 把正式对象、枚举、事件、状态机和 capability seed 收敛为实现基线 | 共享契约基线与 GA/Beta 边界可直接引用 | next |
| 2 | 架构骨架与仓库重建顺序 | 明确七层架构骨架与仓库重建顺序 | 代码骨架重建路径、职责边界与 ADR 前置条件明确 | planned |
| 3 | 首版 GA 核心闭环 | 按最小纵切片完成首版 GA 默认能力 | `task/review`、`automation/watch`、`knowledge/shared`、`MCP` 四条闭环成立 | planned |
| 4 | GA 收口与门禁 | 完成评测、恢复、安全、前端门禁与默认审批收口 | 评测与治理门禁成为正式上线前提 | planned |
| 5 | Beta 能力解锁 | 在 GA 稳定后按顺序解锁 Beta 能力 | 每个 Beta 能力都具备额外治理与评测 checklist | planned |
| 6 | Later 与生态扩展 | 保留路线位并受控扩展生态能力 | Later 只在正式升格后进入执行范围 | planned |

当前默认执行起点：

- 新任务默认从 `Phase 1` 开始，除非它本身是对文档治理、入口门禁或执行规则的再次调整。

---

## 6. 分阶段执行细则

### Phase 0: 文档治理与执行基线

**阶段目标**

建立仓库级 AI 开发执行主文档与累计变更文档，并把它们纳入正式入口与 guardrails。

**In scope**

- `DEVELOPMENT_PLAN` 与 `DEVELOPMENT_CHANGELOG`
- 正式文档入口同步
- guardrails 与 PR 模板同步
- 变更登记流程与阶段切换流程

**Out of scope**

- 新业务对象
- 代码骨架重建
- 契约扩展

**前置依赖**

- [README.md](../README.md)
- [AGENTS.md](../AGENTS.md)
- [PRD.md](./PRD.md)
- [SAD.md](./SAD.md)
- [ENGINEERING_STANDARD.md](./ENGINEERING_STANDARD.md)
- [VIBECODING.md](./VIBECODING.md)

**必做任务 checklist**

- [x] 创建 [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md)
- [x] 创建 [DEVELOPMENT_CHANGELOG.md](./DEVELOPMENT_CHANGELOG.md)
- [x] 将新文档纳入 `README.md`
- [x] 将新文档纳入 `AGENTS.md`
- [x] 将新文档纳入 `ENGINEERING_STANDARD.md`
- [x] 将新文档纳入 `VIBECODING.md`
- [x] 将新文档纳入 `.github/workflows/guardrails.yml`
- [x] 将新文档纳入 `.github/pull_request_template.md`
- [x] 明确普通阶段进展不要求 ADR

**产出物**

- 正式开发计划主文档
- 正式累计变更文档
- 更新后的入口文档与最小门禁

**验收门槛**

- 新文档被 guardrails 视为 required docs
- 正式文档列表完成同步
- 无已废弃旧计划目录或旧变更目录依赖

**不允许越过的边界**

- 不得把这一步写成“已完成产品开发”
- 不得新增与 PRD / SAD 冲突的执行边界

### Phase 1: 契约与状态模型先行

**阶段目标**

以 `contracts/v1/` 为唯一机器可读契约源，明确实现阶段必须对齐的对象、枚举、事件骨架、状态机和 capability seed。

**In scope**

- `contracts/v1/` 对齐
- 共享对象、共享枚举、事件骨架、状态机映射
- `GA / Beta / Later` 边界明确化
- 契约变更同步规则

**Out of scope**

- transport 细节
- 数据库表结构
- API 路由样板

**前置依赖**

- [CONTRACTS.md](./CONTRACTS.md)
- [contracts/README.md](../contracts/README.md)
- [SAD.md](./SAD.md) 中状态机与能力平面定义

**必做任务 checklist**

- [ ] 以 `contracts/v1/core-objects.json` 收敛首批正式对象
- [ ] 以 `contracts/v1/enums.json` 收敛共享枚举与 `GA/Beta` 语义
- [ ] 以 `contracts/v1/events.json` 收敛正式事件骨架
- [ ] 以 `contracts/v1/capabilities.json` 收敛 capability schema 与 seed
- [ ] 把 `Run`、`Automation`、`TriggerDelivery`、`EnvironmentLease`、`ApprovalRequest`、`KnowledgeAsset`、`InboxItem`、`Notification` 的状态机映射到实现计划
- [ ] 明确 `A2A`、`Org Knowledge Graph` 晋升、高阶 `Mesh`、`ResidentAgentSession` 只保留 Beta 能力位
- [ ] 定义契约变化时的同步要求：`contracts/`、`docs/CONTRACTS.md`、ADR、guardrails、PR 模板必须同改

**产出物**

- 可实现的共享契约基线
- 状态机与 capability 基线映射清单

**验收门槛**

- `GA` 与 `Beta` 边界在契约层可明确追踪
- 任何实现者无需再从零判断哪些对象是正式基线

**不允许越过的边界**

- 不得扩展未冻结的对象语义
- 不得以 transport mirror 名义引入新的 Beta 语义

### Phase 2: 架构骨架与仓库重建顺序

**阶段目标**

按 SAD 七层明确仓库骨架重建顺序、组件职责和需要 ADR 的前置条件。

**In scope**

- 七层架构骨架顺序
- Rust Hub、Vue Client、shared contracts、存储抽象职责边界
- monorepo / 目录边界的 ADR 前置条件

**Out of scope**

- 具体业务逻辑
- 完整 UI 实现
- 物理存储细节锁定

**前置依赖**

- [SAD.md](./SAD.md) `4.2-4.8`
- [SAD.md](./SAD.md) `9.4`
- [adr/README.md](./adr/README.md)

**必做任务 checklist**

- [ ] 以 `Interaction / Runtime / Knowledge / Governance / Interop / Execution / Observation` 建立重建顺序
- [ ] 先定义共享契约层，再定义 Hub 核心骨架，再定义 Client 壳层
- [ ] 为本地 / 远程双模式保留统一对象语义与 transport 抽象
- [ ] 为 SQLite / PostgreSQL、LanceDB / Qdrant 留出双实现位
- [ ] 若要重建 monorepo 边界、crate 边界或 workspace manifests，先补 ADR

**产出物**

- 可执行的仓库重建顺序
- 清晰的边界职责表

**验收门槛**

- 下一步实现不再依赖口头约定决定模块边界
- 目录级调整是否需要 ADR 有明确标准

**不允许越过的边界**

- 不得先写实现再补目录与边界解释
- 不得把单层“万能模块”替代七层职责分解

### Phase 3: 首版 GA 核心闭环

**阶段目标**

按 PRD / SAD 明确承诺的首版 `GA` 子集，完成最小可验证纵切片闭环。

**In scope**

- `Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`
- `task`、`automation`、`review`、受控 `watch`
- `Shared Knowledge`
- `MCP`

**Out of scope**

- `A2A`
- 高阶 `Mesh`
- `DiscussionSession`
- `ResidentAgentSession`
- `Org Knowledge Graph` 正式晋升
- `Mobile`

**前置依赖**

- `Phase 1`
- `Phase 2`

**必做任务 checklist**

- [ ] 完成纵切片 1：`run(task) -> policy/grant/budget -> approval -> resume -> artifact -> trace/audit`
- [ ] 完成纵切片 2：`automation/trigger -> run(automation/watch) -> idempotency -> recovery -> inbox/notification`
- [ ] 完成纵切片 3：`knowledge candidate -> shared knowledge -> retrieval -> lineage -> revocation/tombstone`
- [ ] 完成纵切片 4：`MCP Gateway + CapabilityCatalog + CapabilityResolver + ToolSearch`
- [ ] 为每条纵切片单独建立 checklist、验证方式与 change log 记录
- [ ] 确保 `Workspace` 是默认主预算 owner，`Project / Run` 只是子预算

**产出物**

- 首版 GA 四条最小闭环
- 与审批、恢复、审计、知识写回一致的最小运行模型

**验收门槛**

- 用户可走通首版 GA 正式闭环
- `Run`、审批、恢复、Artifact、Knowledge、MCP 不再是孤立点状能力

**不允许越过的边界**

- 不得把 Beta 能力混入 GA 闭环
- 不得跳过 approval / recovery / audit 就宣称闭环成立

### Phase 4: GA 收口与门禁

**阶段目标**

完成评测、恢复、安全、前端门禁和默认审批的收口，使 GA 拥有正式上线前提。

**In scope**

- `Evaluation Harness`
- checkpoint / lease / idempotency / freshness / resume token
- 四类默认审批
- i18n、主题、a11y、通知去重、Inbox/Notification 分离

**Out of scope**

- Beta 功能扩张
- 高风险外部委托默认启用

**前置依赖**

- `Phase 3`

**必做任务 checklist**

- [ ] 建立 `Evaluation Harness` 最小版
- [ ] 覆盖 failure taxonomy：规划失败、工具失败、策略命中、预算超限、恢复失败、知识污染
- [ ] 完成 `execution`、`knowledge_promotion`、`external_delegation`、`export_sharing` 四类默认审批模型
- [ ] 完成 checkpoint、lease、idempotency、freshness、resume token 恢复链路
- [ ] 完成 i18n、主题、可访问性、通知去重与 Inbox/Notification 语义门禁

**产出物**

- GA 上线前治理门禁
- 统一恢复链路
- 前端系统级质量门禁

**验收门槛**

- 评测记录进入正式观测资产，而不是只留在 CI 日志
- 恢复与审批默认策略具备真实约束力

**不允许越过的边界**

- GA 未全部闭环前，不得进入 Beta 默认实现
- 不得以手工口头检查替代评测与治理门禁

### Phase 5: Beta 能力解锁

**阶段目标**

在 GA 稳定、评测记录齐备后，按顺序解锁 `Beta` 能力。

**In scope**

- `DiscussionSession`
- `ResidentAgentSession`
- 高阶 `Mesh`
- `A2A`
- `Org Knowledge Graph` 正式晋升
- `Mobile`

**Out of scope**

- 无评测记录的外部委托
- 未经治理证明的自主常驻代理

**前置依赖**

- `Phase 4`

**必做任务 checklist**

- [ ] 为每个 Beta 能力先补主文档阶段任务
- [ ] 为每个 Beta 能力先在 change log 建立记录
- [ ] 为每个 Beta 能力补额外治理 checklist
- [ ] 为每个 Beta 能力补额外评测 checklist
- [ ] 对 `A2A`、`ResidentAgentSession`、`Org Knowledge Graph` 晋升单独建立高风险启用门槛

**产出物**

- 受控解锁的 Beta 能力计划与实施闭环

**验收门槛**

- 每个 Beta 能力都有单独门禁，不依赖 GA 规则直接复用

**不允许越过的边界**

- 不得把 Beta 说成首版默认能力
- 不得在缺少评测记录时启用高风险 Beta 能力

### Phase 6: Later 与生态扩展

**阶段目标**

保留路线位并受控扩展生态能力，不让 Later 项挤占默认开发排期。

**In scope**

- 路线位
- adapter / connector-backed capability 扩展入口

**Out of scope**

- 未经正式升格的默认开发任务
- 破坏核心对象模型的生态接入

**前置依赖**

- `Phase 5`

**必做任务 checklist**

- [ ] Later 项只记录路线位，不默认排入当前开发执行
- [ ] 新能力只能以 adapter 或 connector-backed capability 进入
- [ ] 若 Later 项要升格为正式范围，先更新 PRD / SAD / CONTRACTS
- [ ] 再同步更新本主文档与变更文档

**产出物**

- 受控的 Later 路线位

**验收门槛**

- Later 不再通过“顺手实现”混入当前交付范围

**不允许越过的边界**

- 不得绕过 PRD / SAD / CONTRACTS 直接升格 Later 范围

---

## 7. 跨阶段公共约束

### 7.1 公共技术与治理约束

- 远程模式下，Hub 永远是正式事实源；Client 只能缓存与呈现。
- `Tenant Hard Policy -> Workspace Policy -> Role/Permission -> CapabilityGrant -> BudgetPolicy -> ApprovalRequest` 是唯一授权求值顺序。
- `InboxItem` 是正式待处理事实；`Notification` 只是提醒层。
- `KnowledgeCandidate` 是共享知识写回强制前置对象。
- `ToolSearchExecuted` 记录搜索事实，不等于自动授权。
- `SkillPack` 只能约束和增强运行，不得授予未注册能力。

### 7.2 文档与记录约束

- 实际任务开始前先读本文件的“当前阶段”与“防跑偏规则”。
- 实际任务开始前先登记 [DEVELOPMENT_CHANGELOG.md](./DEVELOPMENT_CHANGELOG.md)。
- 任务结束后回填验证方式、风险与状态。
- 若实现改变了正式对象语义、阶段边界或验证预期，必须同改主文档。

### 7.3 架构例外约束

以下情况必须先补 ADR，再进入实现：

1. 主技术栈调整
2. Monorepo / 目录边界调整
3. 运行时模式调整
4. 数据存储策略调整
5. 组件体系、tokens、契约源重大调整
6. MCP / A2A / CapabilityCatalog / ToolSearch / SkillPack / 结构化交互模型重大变化

---

## 8. AI 执行规则

AI 在本仓库执行开发任务时必须遵守以下规则：

- 每次开发前先确认当前任务属于哪个 `Phase`。
- 每次开发前先检查该 `Phase` 的 `In scope` / `Out of scope` / `不允许越过的边界`。
- 每次开发前先登记 `DEVELOPMENT_CHANGELOG`。
- 不得跳阶段开发；若必须跳过前置依赖，先记录 blocker，并在需要时提交 ADR。
- 不得把 `Beta / Later` 内容混入 `GA` 任务。
- 不得把目标态架构描述为当前已实现事实。
- 不得在缺少 manifests / 源码 / 验证结果时虚构构建、测试或运行结论。
- 不得用聊天记录替代文档同步、变更登记或正式例外记录。

---

## 9. 验证与完成定义

### 9.1 最小验证集合

当前仓库支持的真实最小验证集合为：

- required-doc 与 contract-source existence checks
- stale-reference 搜索
- focused diff review
- `git diff --check`

仅当对应 manifests 与源码实际进入 tracked tree 时，才追加：

- `pnpm run typecheck:web`
- `pnpm run test:web`
- `pnpm run build:web`
- `cargo test --workspace`
- `cargo build --workspace`

### 9.2 任务完成前 checklist

- [ ] 任务已归属到明确阶段
- [ ] `In scope / Out of scope` 已核对
- [ ] `DEVELOPMENT_CHANGELOG` 已登记并回填
- [ ] 文档与门禁同步要求已核对
- [ ] 至少完成本次变更涉及关键路径的真实验证
- [ ] 没有超出当前 tracked tree 可以证明的事实边界

### 9.3 Definition of Done

一个变更只有同时满足以下条件才算完成：

1. 符合 PRD、SAD、CONTRACTS、ENGINEERING_STANDARD、VIBECODING 与本主文档。
2. 对应阶段的 checklist 已满足。
3. 对应阶段的验收门槛已满足。
4. 必要文档、契约、门禁与变更记录已同步。
5. 验证结论真实、可复查、未超出 tracked tree 事实边界。
