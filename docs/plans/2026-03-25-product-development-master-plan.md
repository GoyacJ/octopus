# 产品开发总计划

- Status: `In Progress`
- Last Updated: `2026-03-25`
- Current Focus: `里程碑 B · 契约与仓库基线`
- Goal: `以统一主计划管理 Octopus 全量产品开发顺序、退出条件与执行跟踪。`

## 里程碑总览

| Milestone | Topic | Status | Exit Criteria | Related Change |
| --- | --- | --- | --- | --- |
| A | Planning Governance Unification | `Done` | 仓库只保留一个主计划入口，源文档不再承载交付批次话术 | `docs/changes/2026-03-25-planning-governance-unification.md` |
| B | Contract & Repo Baseline | `Not Started` | 核心文档术语、对象关系与仓库真实状态一致 | `docs/changes/<date>-contract-and-repo-baseline.md` |
| C | Identity, Auth, Hub & Models Foundation | `Not Started` | 身份、认证、多 Hub 和模型中心形成可实现基线 | `docs/changes/<date>-identity-auth-hub-model-foundation.md` |
| D | Client Shell & Governance Surfaces | `Not Started` | Web/Desktop/Mobile 控制面壳与治理型表面边界稳定 | `docs/changes/<date>-client-shell-and-governance-surfaces.md` |
| E | Agent Lifecycle & Memory | `Not Started` | Agent 配置、测试对话、记忆管理与来源追溯可闭环 | `docs/changes/<date>-agent-lifecycle-and-memory.md` |
| F | Team & Task Execution | `Not Started` | Team、Leader、执行计划、审批、恢复、审计链路可闭环 | `docs/changes/<date>-team-and-task-execution.md` |
| G | Discussion Engine | `Not Started` | 讨论创建、发言调度、结论、历史与记忆写回可闭环 | `docs/changes/<date>-discussion-engine.md` |
| H | Extensions & Capability Governance | `Not Started` | Skills、Tools、MCP 与风险约束形成统一治理模型 | `docs/changes/<date>-extensions-and-capability-governance.md` |
| I | Templates, Onboarding & Console | `Not Started` | 模板、引导与管理控制台形成完整治理入口 | `docs/changes/<date>-templates-onboarding-and-console.md` |
| J | Security, Reliability & Operations | `Not Started` | 安全、恢复、观测性和运维能力达到可验证基线 | `docs/changes/<date>-security-reliability-and-operations.md` |
| K | Advanced Capabilities Backlog | `Not Started` | 高级能力进入统一 backlog 并与核心链路解耦管理 | `docs/changes/<date>-advanced-capabilities-backlog.md` |

## 能力域覆盖

| Capability Domain | Covered By |
| --- | --- |
| Agent / Identity / Prompt / Test Chat | E |
| Memory / Recall / Growth / Source Trace | E, G, J |
| Team / Leader / Routing Rules | F |
| Task / Subtask DAG / Approval / Resume / Trace | F, J |
| Discussion / Roundtable / Brainstorm / Debate | G |
| Templates / Onboarding | I |
| Hub Management / Tenants / Users / Quotas | C, I |
| Model Center / Provider / Catalog / Profile / Policy | C |
| Auth / RBAC / Multi-Hub | C, D |
| Skills / Tools / MCP | H |
| Audit / Security / Reliability / NFR | J |
| TeamGroup / 2FA / Reactive Turn Strategy / Cluster | K |

## Overall Exit Criteria

- 全量 PRD 能力域均已映射到主计划中的里程碑，不再散落在源文档中表达交付批次。
- `docs/plans/` 管交付顺序，`docs/changes/` 管执行证据，二者状态保持同步。
- 源文档继续表达产品、架构、领域、数据和 API 真相，不承担“先做什么/后做什么”的职责。
- 所有验证声明仅基于仓库当前真实存在的 manifests、源码和工具，不虚构 `pnpm`、`cargo` 或运行时结论。

## Verification Baseline

- required-doc existence checks
- stale-reference searches for removed roadmap/change files or renamed docs
- focused diff review for touched documents
- targeted consistency grep for renamed fields, contract names, milestone names, or removed delivery wording
- PRD capability-to-plan coverage review
- 仅在对应 manifests、sources、tools 真实存在后，才把 `pnpm`、`turbo`、`cargo`、OpenAPI lint、Playwright 等写入里程碑验证

## Related Changes

- 当前已建立的治理记录：`docs/changes/2026-03-25-planning-governance-unification.md`
- 后续变更记录统一命名：`docs/changes/YYYY-MM-DD-<topic>.md`
- 任务级执行证据保留在本计划中；里程碑级结果、风险、验证和文档同步保留在对应 change 文档中

## Update Rules

- 任一 checklist 项完成时：勾选主计划、更新 `Last Updated`、补一行简短 evidence/note。
- 任一里程碑或工作流进入 `In Progress / Blocked / Done` 时：同一次提交内同步更新对应 change 文档。
- `Blocked` 必须写清：阻塞原因、影响范围、下一步动作。
- `Done` 必须补齐：`Verification`、`Docs Sync`、`Risks`，以及必要时的 `UI Evidence`。

## 里程碑 A：规划系统统一

- Status: `Done`
- Related Change: `docs/changes/2026-03-25-planning-governance-unification.md`
- Exit Criteria: 仓库只保留一个主计划入口，源文档不再承载交付批次话术。

### Checklist

- [x] 清理 `PRD / ARCHITECTURE / VIBECODING / VISUAL_FRAMEWORK / API` 中的交付批次表述。 — Evidence: 源文档改为只表达产品、架构和界面真相，不再表达交付批次。
- [x] 建立 `docs/plans/2026-03-25-product-development-master-plan.md` 作为唯一主计划入口。 — Evidence: 本文件已创建并覆盖全量能力域与里程碑。
- [x] 建立中性的规划治理变更记录并同步入口文档、目录规则、模板和 guardrails。 — Evidence: `README.md`、`AGENTS.md`、`docs/plans/README.md`、`docs/changes/README.md`、`docs/changes/TEMPLATE.md`、`.github/workflows/guardrails.yml` 已改为统一治理表述。
- [x] 移除旧的分阶段计划与变更文件引用，并删除旧文件。 — Evidence: 旧计划与旧治理记录已由新入口替代。

## 里程碑 B：契约与仓库基线

- Status: `Not Started`
- Related Change: `docs/changes/<date>-contract-and-repo-baseline.md`
- Exit Criteria: 核心文档术语、对象关系与仓库真实状态一致。

### Checklist

- [ ] 对齐 `PRD / SAD / ARCHITECTURE / DOMAIN / DATA_MODEL / API / VIBECODING / VISUAL_FRAMEWORK` 的术语、对象关系和状态描述。 — Evidence: Pending.
- [ ] 收敛仓库真实存在的目录、manifest、脚手架与验证入口，移除与现状不一致的目标态表述。 — Evidence: Pending.
- [ ] 仅在新增架构决策或例外时补充 ADR，并同步相关文档。 — Evidence: Pending.
- [ ] 建立并维护对应的契约与仓库基线变更记录。 — Evidence: Pending.

## 里程碑 C：身份、认证、Hub 连接与模型中心

- Status: `Not Started`
- Related Change: `docs/changes/<date>-identity-auth-hub-model-foundation.md`
- Exit Criteria: 身份、认证、多 Hub 和模型中心形成可实现基线。

### Checklist

- [ ] 落地 Tenant、User、RBAC、权限守卫和租户隔离规则。 — Evidence: Pending.
- [ ] 打通远程 Hub 握手、Token 生命周期、多 Hub 注册与切换语义。 — Evidence: Pending.
- [ ] 落地 `ModelProvider / ModelCatalogItem / ModelProfile / TenantModelPolicy` 四层模型中心。 — Evidence: Pending.
- [ ] 明确本地 Hub 与远程 Hub 在认证、模型管理和租户视图上的差异。 — Evidence: Pending.

## 里程碑 D：客户端壳与治理型表面

- Status: `Not Started`
- Related Change: `docs/changes/<date>-client-shell-and-governance-surfaces.md`
- Exit Criteria: Web/Desktop/Mobile 控制面壳与治理型表面边界稳定。

### Checklist

- [ ] 建立 Web/Desktop 的全局壳、一级导航、Hub/租户切换、主题与 i18n 基线。 — Evidence: Pending.
- [ ] 定义 Mobile 轻控制面的审批、查看、通知与轻量接管入口。 — Evidence: Pending.
- [ ] 落地治理型页面布局语法与关键页面边界。 — Evidence: Pending.
- [ ] 保证 `zh-CN` / `en-US` 文案路径可持续维护。 — Evidence: Pending.

## 里程碑 E：Agent 全生命周期

- Status: `Not Started`
- Related Change: `docs/changes/<date>-agent-lifecycle-and-memory.md`
- Exit Criteria: Agent 配置、测试对话、记忆管理与来源追溯可闭环。

### Checklist

- [ ] 完成 Agent CRUD、Identity / Persona / System Prompt、头像与角色配置。 — Evidence: Pending.
- [ ] 完成 Prompt 版本历史、diff、回滚和测试对话。 — Evidence: Pending.
- [ ] 完成 ModelProfile 绑定、工具白名单、Skill 绑定、MCP 绑定与有效能力预览。 — Evidence: Pending.
- [ ] 完成记忆列表、搜索、手动添加、删除、来源任务追溯与成长信息展示。 — Evidence: Pending.

## 里程碑 F：Team 与任务执行

- Status: `Not Started`
- Related Change: `docs/changes/<date>-team-and-task-execution.md`
- Exit Criteria: Team、Leader、执行计划、审批、恢复、审计链路可闭环。

### Checklist

- [ ] 完成 Team / Leader 创建、成员管理和路由规则配置。 — Evidence: Pending.
- [ ] 完成执行计划预览、确认、修改和手动重规划机制。 — Evidence: Pending.
- [ ] 完成 Subtask DAG、审批模式、Decision 队列、批量处理与高风险强制审批。 — Evidence: Pending.
- [ ] 完成终止、恢复、结果汇总、时间线、Trace/Audit 与导出。 — Evidence: Pending.

## 里程碑 G：Discussion Engine

- Status: `Not Started`
- Related Change: `docs/changes/<date>-discussion-engine.md`
- Exit Criteria: 讨论创建、发言调度、结论、历史与记忆写回可闭环。

### Checklist

- [ ] 完成 Roundtable / Brainstorm / Debate 三种讨论模式与参与者配置。 — Evidence: Pending.
- [ ] 完成主持人、发言策略、用户插话、暂停、继续和结束控制。 — Evidence: Pending.
- [ ] 完成结论生成、结论编辑、历史检索、续会和导出。 — Evidence: Pending.
- [ ] 完成讨论后对参与 Agent 的记忆提取与写回。 — Evidence: Pending.

## 里程碑 H：Extensions 与能力治理

- Status: `Not Started`
- Related Change: `docs/changes/<date>-extensions-and-capability-governance.md`
- Exit Criteria: Skills、Tools、MCP 与风险约束形成统一治理模型。

### Checklist

- [ ] 完成 Skills 库、Prompt 片段合并、授权工具集合与运行时能力合并。 — Evidence: Pending.
- [ ] 完成 Tool Registry、Tool Search、风险等级与权限校验。 — Evidence: Pending.
- [ ] 完成 MCP Server 注册、测试、管理与 Agent 侧绑定能力。 — Evidence: Pending.
- [ ] 完成高风险工具审批联动、讨论模式工具限制与审计约束。 — Evidence: Pending.

## 里程碑 I：模板、引导与控制台

- Status: `Not Started`
- Related Change: `docs/changes/<date>-templates-onboarding-and-console.md`
- Exit Criteria: 模板、引导与管理控制台形成完整治理入口。

### Checklist

- [ ] 完成内置模板、私有模板、JSON 导入导出与模板治理。 — Evidence: Pending.
- [ ] 完成首次使用引导、场景推荐和默认团队生成流程。 — Evidence: Pending.
- [ ] 完成 Hub 控制台总览、租户管理、用户管理、配额管理与连接状态可视化。 — Evidence: Pending.
- [ ] 完成通知、审批入口和结果摘要的控制面承载。 — Evidence: Pending.

## 里程碑 J：安全、可靠性与运维

- Status: `Not Started`
- Related Change: `docs/changes/<date>-security-reliability-and-operations.md`
- Exit Criteria: 安全、恢复、观测性和运维能力达到可验证基线。

### Checklist

- [ ] 完成数据库与向量存储加密、敏感信息脱敏和安全存储策略。 — Evidence: Pending.
- [ ] 完成断点续跑、离线历史查看、失败恢复和并发执行边界。 — Evidence: Pending.
- [ ] 完成 SSE 事件、日志、指标、追踪与故障恢复策略。 — Evidence: Pending.
- [ ] 完成性能、可靠性和运维验证基线。 — Evidence: Pending.

## 里程碑 K：高级能力与扩展

- Status: `Not Started`
- Related Change: `docs/changes/<date>-advanced-capabilities-backlog.md`
- Exit Criteria: 高级能力进入统一 backlog 并与核心链路解耦管理。

### Checklist

- [ ] 把 `TeamGroup`、模板社区、2FA、Reactive 发言策略、Hub 集群化等能力纳入统一 backlog。 — Evidence: Pending.
- [ ] 明确高级能力与核心链路之间的依赖、前置条件和风险。 — Evidence: Pending.
- [ ] 确保高级能力只在主计划中排序，不在源文档中标注交付阶段。 — Evidence: Pending.

## Assumptions

- 源文档继续表达完整目标态需求与契约，不表达交付批次；交付顺序只在本计划表达。
- `P0 / P1 / P2` 这类用户层级表述保留，因为它们是用户分层，不是交付阶段。
- 旧的 phase-specific 计划与变更文件不再作为正式入口保留；若需要追溯，依赖 Git 历史。
- 主计划采用“里程碑 + 能力域 + 退出条件”结构，而不是旧的分阶段结构。
