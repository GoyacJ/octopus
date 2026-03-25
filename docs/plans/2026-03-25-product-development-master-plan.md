# 架构依赖驱动产品开发总计划

- Status: `In Progress`
- Last Updated: `2026-03-25`
- Current Focus: `M0 · 文档真相与治理修复`
- Goal: `以“主计划导航 + 里程碑实施计划”管理 octopus 的实现顺序、契约冻结、验证入口与执行证据。`

## 计划模型

- `docs/plans/2026-03-25-product-development-master-plan.md` 是唯一正式导航入口，只负责里程碑顺序、依赖关系、当前焦点和退出条件。
- `docs/plans/2026-03-25-m<nn>-<topic>.md` 是 AI 可直接执行的里程碑实施计划，负责输入文档、待冻结合同、交付件、验证和文档同步。
- `docs/changes/` 只记录已经开始执行的里程碑或主题的变更结果、风险与验证证据，不承担路线图排序职责。
- `PRD / SAD / ARCHITECTURE / DOMAIN / DATA_MODEL / ENGINEERING_STANDARD / API / VISUAL_FRAMEWORK / VIBECODING` 继续表达源文档真相，不承担交付排序。

## Overall Exit Criteria

- `M0-M10` 每个正式里程碑都拥有一份实施计划，并且都能映射到至少一个源文档输入，而不是只映射 PRD 能力域。
- 不再把 `A-K` 作为正式执行编号；旧编号仅保留在本文件附录用于历史追踪。
- 当前仓库事实、目标态蓝图、实施计划和变更记录之间不再相互矛盾。
- 所有验证声明仅基于仓库当前真实存在的文档、脚手架、manifests 和工具，不虚构 `pnpm`、`cargo`、OpenAPI lint、Playwright 或运行时结论。

## Verification Baseline

- required-doc existence checks
- stale-reference searches for旧的 `A-K` 正式执行入口、失效文档名和错误 required-doc 集合
- focused diff review for touched plans, changes, README and guardrails
- targeted consistency grep for `doc-first` 真相、里程碑编号、related plan / related change 引用和旧命名
- 仅在对应 manifests、sources、tools 真实存在后，才把 `pnpm`、`turbo`、`cargo`、OpenAPI lint、Playwright 等写入里程碑验证

## 里程碑总览

| Milestone | Topic | Status | Depends On | Related Implementation Plan | Related Change |
| --- | --- | --- | --- | --- | --- |
| M0 | Doc Truth & Governance Repair | `In Progress` | None | `docs/plans/2026-03-25-m00-doc-truth-and-governance-repair.md` | `docs/changes/2026-03-25-contract-and-repo-baseline.md` |
| M1 | Architecture & Runtime Contract Freeze | `Not Started` | `M0` | `docs/plans/2026-03-25-m01-architecture-and-runtime-contract-freeze.md` | `docs/changes/<date>-architecture-and-runtime-contract-freeze.md` |
| M2 | Identity, Workspace, Models & Secrets Foundation | `Not Started` | `M0`, `M1` | `docs/plans/2026-03-25-m02-identity-workspace-model-and-secret-foundation.md` | `docs/changes/<date>-identity-workspace-model-and-secret-foundation.md` |
| M3 | Execution Plane & Capability Governance Foundation | `Not Started` | `M0`, `M1` | `docs/plans/2026-03-25-m03-execution-plane-and-capability-governance-foundation.md` | `docs/changes/<date>-execution-plane-and-capability-governance-foundation.md` |
| M4 | Agent, Context & Memory | `Not Started` | `M1`, `M2`, `M3` | `docs/plans/2026-03-25-m04-agent-context-and-memory.md` | `docs/changes/<date>-agent-context-and-memory.md` |
| M5 | Team, Task, Approval & Recovery | `Not Started` | `M1`, `M2`, `M3`, `M4` | `docs/plans/2026-03-25-m05-team-task-approval-and-recovery.md` | `docs/changes/<date>-team-task-approval-and-recovery.md` |
| M6 | Discussion Engine | `Not Started` | `M1`, `M3`, `M4`, `M5` | `docs/plans/2026-03-25-m06-discussion-engine.md` | `docs/changes/<date>-discussion-engine.md` |
| M7 | Surface Shells & Governance UI | `Not Started` | `M0`, `M1`, `M2`, `M3`, `M4`, `M5`, `M6` | `docs/plans/2026-03-25-m07-surface-shells-and-governance-ui.md` | `docs/changes/<date>-surface-shells-and-governance-ui.md` |
| M8 | Blueprint, Import/Export, Templates & Console | `Not Started` | `M1`, `M2`, `M3`, `M4`, `M5`, `M7` | `docs/plans/2026-03-25-m08-blueprint-import-export-templates-and-console.md` | `docs/changes/<date>-blueprint-import-export-templates-and-console.md` |
| M9 | Security, Observability & Operations | `Not Started` | `M1`, `M2`, `M3`, `M4`, `M5`, `M6`, `M7`, `M8` | `docs/plans/2026-03-25-m09-security-observability-and-operations.md` | `docs/changes/<date>-security-observability-and-operations.md` |
| M10 | Advanced Backlog | `Not Started` | `M0-M9` | `docs/plans/2026-03-25-m10-advanced-backlog.md` | `docs/changes/<date>-advanced-backlog.md` |

## 里程碑详情

### M0 文档真相与治理修复

- Depends On: `None`
- Source Docs: `AGENTS.md`, `README.md`, `docs/plans/README.md`, `docs/changes/README.md`, `docs/SAD.md`, `docs/ARCHITECTURE.md`, `docs/DOMAIN.md`, `docs/DATA_MODEL.md`, `docs/ENGINEERING_STANDARD.md`, `docs/API/README.md`, `.github/workflows/guardrails.yml`, `.github/pull_request_template.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m00-doc-truth-and-governance-repair.md`
- Related Change: `docs/changes/2026-03-25-contract-and-repo-baseline.md`
- Exit Criteria: 正式执行入口切换到两层计划体系；README / guardrails / PR 模板 / 主计划 / 当前 change 记录同步；旧命名、失效链接和“目标态冒充现状”的表达不再误导实现。

### M1 架构基础与运行时契约冻结

- Depends On: `M0`
- Source Docs: `docs/SAD.md`, `docs/ARCHITECTURE.md`, `docs/ENGINEERING_STANDARD.md`, `docs/API/README.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m01-architecture-and-runtime-contract-freeze.md`
- Related Change: `docs/changes/<date>-architecture-and-runtime-contract-freeze.md`
- Exit Criteria: 控制面 / 运行时 / 执行面 / 表面层职责边界、统一运行时对象、事件模型、恢复协议、外部/内部传输协议和最小审计语义全部冻结为实现前合同。

### M2 身份、工作区、模型与密钥基础

- Depends On: `M0`, `M1`
- Source Docs: `docs/DOMAIN.md`, `docs/DATA_MODEL.md`, `docs/API/README.md`, `docs/API/AUTH.md`, `docs/PRD.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m02-identity-workspace-model-and-secret-foundation.md`
- Related Change: `docs/changes/<date>-identity-workspace-model-and-secret-foundation.md`
- Exit Criteria: `Tenant / User / RBAC / Multi-Hub / Model Registry / SecretBinding` 的对象关系、API 范围、数据约束与默认差异冻结完成，后续里程碑不再临时改写这些基线。

### M3 执行面与能力治理基础

- Depends On: `M0`, `M1`
- Source Docs: `docs/SAD.md`, `docs/ARCHITECTURE.md`, `docs/DOMAIN.md`, `docs/API/MCP.md`, `docs/API/SKILLS_TOOLS.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m03-execution-plane-and-capability-governance-foundation.md`
- Related Change: `docs/changes/<date>-execution-plane-and-capability-governance-foundation.md`
- Exit Criteria: `Built-in Tools / MCP / Plugin Host / Node Runtime / PlatformToolProfile / fallback / capability grant / approval gate` 形成统一实现前合同，后续 agent、task、discussion 复用该基线。

### M4 Agent、上下文与记忆

- Depends On: `M1`, `M2`, `M3`
- Source Docs: `docs/PRD.md`, `docs/SAD.md`, `docs/DOMAIN.md`, `docs/DATA_MODEL.md`, `docs/API/AGENTS.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m04-agent-context-and-memory.md`
- Related Change: `docs/changes/<date>-agent-context-and-memory.md`
- Exit Criteria: `AgentProfile`、Prompt、Context Engine、WorkingMemory、ContextSnapshot、Memory recall/writeback 和来源追溯具备冻结后的对象、接口和验收边界。

### M5 Team、Task、Approval 与 Recovery

- Depends On: `M1`, `M2`, `M3`, `M4`
- Source Docs: `docs/PRD.md`, `docs/SAD.md`, `docs/DOMAIN.md`, `docs/DATA_MODEL.md`, `docs/API/TEAM.md`, `docs/API/EVENTS.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m05-team-task-approval-and-recovery.md`
- Related Change: `docs/changes/<date>-team-task-approval-and-recovery.md`
- Exit Criteria: `LeaderPlanningService`、Task DAG、审批、等待恢复、Decision 队列和时间线汇总建立闭环，且明确依赖 M1-M4 的哪些冻结合同。

### M6 Discussion Engine

- Depends On: `M1`, `M3`, `M4`, `M5`
- Source Docs: `docs/PRD.md`, `docs/SAD.md`, `docs/ARCHITECTURE.md`, `docs/DOMAIN.md`, `docs/API/DISCUSSIONS.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m06-discussion-engine.md`
- Related Change: `docs/changes/<date>-discussion-engine.md`
- Exit Criteria: 讨论模式、调度、用户插话、结论、记忆写回和与 TaskEngine 的边界全部冻结，不再在实现期临时决策。

### M7 Surface Shells 与治理型 UI

- Depends On: `M0`, `M1`, `M2`, `M3`, `M4`, `M5`, `M6`
- Source Docs: `docs/VISUAL_FRAMEWORK.md`, `docs/ENGINEERING_STANDARD.md`, `docs/SAD.md`, `docs/API/README.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m07-surface-shells-and-governance-ui.md`
- Related Change: `docs/changes/<date>-surface-shells-and-governance-ui.md`
- Exit Criteria: Web / Desktop / Mobile 壳、导航、i18n、主题和治理型页面边界在不承载底层运行时决策的前提下冻结。

### M8 Blueprint / Import-Export / Templates / Console

- Depends On: `M1`, `M2`, `M3`, `M4`, `M5`, `M7`
- Source Docs: `docs/SAD.md`, `docs/PRD.md`, `docs/VISUAL_FRAMEWORK.md`, `docs/API/README.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m08-blueprint-import-export-templates-and-console.md`
- Related Change: `docs/changes/<date>-blueprint-import-export-templates-and-console.md`
- Exit Criteria: Blueprint、模板、导入导出、首次引导和管理控制台被整合为统一迁移与管理面，而不是分散在多个产品里程碑中。

### M9 安全、观测与运维

- Depends On: `M1`, `M2`, `M3`, `M4`, `M5`, `M6`, `M7`, `M8`
- Source Docs: `docs/SAD.md`, `docs/ARCHITECTURE.md`, `docs/ENGINEERING_STANDARD.md`, `docs/API/EVENTS.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m09-security-observability-and-operations.md`
- Related Change: `docs/changes/<date>-security-observability-and-operations.md`
- Exit Criteria: secrets、审计、`ToolCallTrace / ExecutionTimeline`、日志、指标、追踪、恢复和性能基线拥有统一验证入口与实施边界。

### M10 高级能力 Backlog

- Depends On: `M0-M9`
- Source Docs: `docs/PRD.md`, `docs/SAD.md`, `docs/VISUAL_FRAMEWORK.md`
- Related Implementation Plan: `docs/plans/2026-03-25-m10-advanced-backlog.md`
- Related Change: `docs/changes/<date>-advanced-backlog.md`
- Exit Criteria: 高级能力只保留为统一 backlog 和依赖图，不再混入 v1 核心链路或抢占正式执行顺序。

## Update Rules

- 里程碑当前状态、依赖关系和当前焦点统一在本文件更新。
- 里程碑内的可执行任务、合同冻结项、交付件和验证步骤统一在对应实施计划中更新。
- 里程碑进入 `In Progress / Blocked / Done` 时，同一次变更内同步更新对应 `docs/changes/` 文档。
- 源文档发生影响架构、契约、数据模型、页面边界或验证基线的变化时，必须同时检查是否需要更新相关实施计划和 change 记录。

## Historical Mapping

| 旧编号 | 旧主题 | 新归属 |
| --- | --- | --- |
| A | Planning Governance Unification | `M0` 的历史前置工作 |
| B | Contract & Repo Baseline | `M0` |
| C | Identity, Auth, Hub & Models Foundation | `M2` |
| D | Client Shell & Governance Surfaces | `M7` |
| E | Agent Lifecycle & Memory | `M4` |
| F | Team & Task Execution | `M5` |
| G | Discussion Engine | `M6` |
| H | Extensions & Capability Governance | `M3` |
| I | Templates, Onboarding & Console | `M8` |
| J | Security, Reliability & Operations | `M9` |
| K | Advanced Capabilities Backlog | `M10` |

## Assumptions

- 当前仓库仍然是 `doc-first`，本轮只建立可执行计划体系并同步必要文档真相，不假设代码脚手架已存在。
- `docs/API/MODELS.md` 即使在本地工作区存在，也不自动成为当前 required-doc 或正式 API 基线，直到 `M2` 冻结相关合同。
- 未开始的里程碑可以先保留 `Related Change` 的目标文件名；只有在里程碑正式开始时才创建对应 change 记录。
