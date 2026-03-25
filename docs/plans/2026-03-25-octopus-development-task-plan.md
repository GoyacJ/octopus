# Octopus 项目开发任务计划 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 产出一份以当前 tracked tree 为事实基线的路线型开发任务计划，指导 Octopus 从 Phase 1 skeleton 收敛到首版 GA，并为 Beta/Later 能力保留清晰入口与治理边界。

**Architecture:** 本计划按“阶段路线 + 近细远粗”组织，先巩固治理与契约基线，再围绕 `Run -> approval -> resume -> artifact -> trace/audit` 纵切片逐步扩展到 Automation、Trigger、Shared Knowledge 和 MCP。文档严格以 `README.md`、`docs/PRD.md`、`docs/SAD.md`、`docs/CONTRACTS.md`、`docs/ENGINEERING_STANDARD.md`、`docs/VIBECODING.md`、`docs/adr/README.md` 和 ADR 0001 为设计输入，不把目标态能力误写为当前已实现事实。

**Tech Stack:** Markdown、contracts/v1 JSON、pnpm workspace、Vue 3 + TypeScript + Pinia、Rust workspace、GitHub guardrails

---

## 1. 项目现状与事实边界

### 1.1 当前仓库事实基线

截至 2026-03-25，`octopus` 当前仍是 `doc-first` 治理仓库，但已进入 `Phase 1 skeleton` 阶段。当前 tracked tree 已经明确存在以下可依赖输入：

- 根目录治理文档与正式入口：`README.md`、`AGENTS.md`
- 正式设计文档：`docs/PRD.md`、`docs/SAD.md`、`docs/CONTRACTS.md`、`docs/ENGINEERING_STANDARD.md`、`docs/VIBECODING.md`
- 架构决策入口：`docs/adr/README.md`、`docs/adr/0001-contract-source-and-phase1-skeleton.md`
- 机器可读契约源：`contracts/v1/core-objects.json`、`contracts/v1/enums.json`、`contracts/v1/events.json`
- TypeScript 契约镜像：`packages/contracts`
- Rust workspace skeleton：`crates/octopus-hub`、`crates/octopus-server`、`crates/octopus-tauri`
- Web 控制面壳：`apps/octopus-client`
- 当前真实验证链路：`pnpm run typecheck:web`、`pnpm run test:web`、`pnpm run build:web`、`cargo test --workspace`、`cargo build --workspace`

### 1.2 当前已成立能力

本计划以以下“已存在且可验证”的起点推进，不把它们误写成完整平台：

- `packages/contracts` 已镜像核心对象、共享枚举和事件骨架，并有最小测试校验 JSON 契约名录一致性。
- `crates/octopus-hub` 已提供最小契约加载能力与 `InMemoryRuntimeService`，可演示 `task -> waiting_approval -> approval resolve -> resume -> completed` 主线。
- `crates/octopus-server` 已提供最小 HTTP 入口：`/healthz`、`/api/v1/contracts`、`/api/v1/runs/task`、`/api/v1/runs/{run_id}`、`/api/v1/runs/{run_id}/resume`、`/api/v1/approvals/{approval_id}/resolve`。
- `apps/octopus-client` 已提供 Vue 3 控制面壳、路由、i18n、主题切换和 skeleton 展示。
- `crates/octopus-tauri` 仅提供桌面桥接 bootstrap 结构，不代表完整桌面集成已经成立。

### 1.3 当前不能宣称已成立的能力

除非后续 manifest、源码和验证链证明其成立，当前不得把以下内容描述为“已经实现”：

- 数据库事实层、持久化 Repository、SQLite/PostgreSQL 双实现
- 正式 SSE 推送链、完整远程同步语义、OpenAPI、protobuf 或 `buf`
- 完整桌面集成、移动端、跨端继续
- `A2A`、正式 `Org Knowledge Graph` 晋升、高阶 `Mesh`、`DiscussionSession`、`ResidentAgentSession`
- 完整 Chat/Board 工作台、端到端 UI 自动化验证

### 1.4 当前开发的正式边界

本计划固定采用以下权威边界，不额外发明新的事实源：

- 机器可读契约源仅认 `contracts/v1/core-objects.json`、`contracts/v1/enums.json`、`contracts/v1/events.json`
- TypeScript 契约镜像仅认 `packages/contracts`
- Rust 领域与运行时骨架仅认 `crates/octopus-hub`
- HTTP 适配层起点仅认 `crates/octopus-server`
- 桌面壳层仅认 `crates/octopus-tauri`，并继续声明为桥接 skeleton
- Web 控制面仅认 `apps/octopus-client` 当前 Vue 壳层

首版 GA 只承诺以下能力进入正式交付范围：

- `task`
- `automation`
- 审批驱动的 `review`
- 受控 Trigger
- `Shared Knowledge`
- `MCP`

以下能力在当前阶段只保留对象、状态、接口或任务入口，不列为 GA 已交付：

- `A2A`
- `Org Knowledge Graph` 正式晋升
- `DiscussionSession`
- `ResidentAgentSession`
- 高阶 `Mesh / Cross-Team Mesh`
- `Mobile`

### 1.5 计划使用规则

- 本文档是执行计划，不替代正式治理文档。
- 任何新增核心对象、共享枚举、事件骨架、公开 transport 语义，必须同步更新契约源、中文说明文档和 ADR。
- 任何阶段性结论都不得超出当前 tracked tree 与验证结果能证明的事实边界。
- 本计划默认采用“近细远粗”：只把 `Phase 1 / 首版 GA` 细拆为可执行任务，`Beta / Later` 保持能力包级别。

## 2. 阶段路线总览

### 2.1 路线设计原则

- 先收敛治理与契约，再扩展代码实现。
- 每次只推进一条最小可验证纵切片。
- Beta/Later 能力先保留接口位，再按验证结果解锁。
- 任何架构例外先进入 ADR，再进入实施。

### 2.2 阶段里程碑表

| Stage | 阶段目标 | 核心输出 | 前置依赖 | 完成标志 |
| --- | --- | --- | --- | --- |
| `Stage 0: 治理与契约基线固化` | 确保正式文档入口、契约源、guardrails、PR 模板和 skeleton workspace 一致 | 文档入口一致、契约同步规则、真实验证链说明 | 当前正式文档入口已存在 | 文档入口、契约源、guardrails、PR 模板与仓库现状一致，验证链声明不超出事实 |
| `Stage 1: Phase 1 skeleton 完整化` | 把现有 TS/Rust/Client/Server/Tauri skeleton 收敛成稳定起跑线 | contracts 镜像稳定、in-memory runtime 可演示、HTTP smoke 稳定、Client 壳层稳定 | Stage 0 | skeleton 能稳定通过现有验证链，且所有对外叙述都准确描述当前范围 |
| `Stage 2: GA 纵切片一` | 打通 `task -> approval -> resume -> artifact -> trace/audit` | 首条完整 Run 主线 | Stage 1 | 手动任务、审批等待、审批决议、恢复完成、结果与审计链一致 |
| `Stage 3: GA 纵切片二` | 打通 `automation / trigger / controlled watch` | 计划任务与受控事件触发最小闭环 | Stage 2 | Automation 能发起或续接 Run，Trigger 去重与失败分类成立 |
| `Stage 4: GA 纵切片三` | 打通 `candidate knowledge -> verified shared` | Shared Knowledge GA 最小闭环 | Stage 2、Stage 3 | Run/Automation 结果能受控进入候选知识并晋升到共享知识 |
| `Stage 5: GA 交付硬化` | 补齐 MCP、Connections、Inbox、Trace、Knowledge 表面与远程 Hub 语义一致性 | 首版 GA 发布基线 | Stage 2、Stage 3、Stage 4 | MCP、控制面壳和最小治理链路形成首版 GA 可说明范围 |
| `Stage 6: Beta 能力入口` | 为 `DiscussionSession`、`ResidentAgentSession`、高阶 Mesh、`A2A`、`Org Knowledge Graph`、`Mobile` 建立进入条件 | 保留接口、边界说明、后续任务包 | Stage 5 | 每个 Beta 能力都具备明确依赖、对象边界和 ADR/契约前置条件 |
| `Stage 7: Later` | 扩展企业治理、多端连续性、生态互操作深度 | 远期路线与能力包 | Stage 6 | 只保留路线，不形成当前执行承诺 |

### 2.3 分阶段推进说明

`Stage 0` 与 `Stage 1` 不视为纯绿地工作，而是对现有 skeleton 的收敛与对齐。  
`Stage 2`、`Stage 3`、`Stage 4` 是首版 GA 的三条核心纵切片，必须依次形成闭环。  
`Stage 5` 负责把已验证主线与控制面和 MCP 边界对齐，形成首版 GA 对外叙述基线。  
`Stage 6` 与 `Stage 7` 只保留能力入口和治理前置条件，不在当前阶段承诺完整交付。

## 3. Phase 1 / 首版 GA 详细任务拆解

### 3.1 近端任务包组织规则

以下工作包必须统一写明：

- 目标
- 涉及目录
- 依赖
- 执行动作
- 完成条件
- 验证命令

每个工作包只解决一个主问题；若实施中出现架构边界变化、契约源变化或新的正式对象，必须先补 ADR 与文档同步。

### WP1 Governance & Contract Sync

**目标**

确保 JSON 契约源、TypeScript 契约镜像、Rust 契约镜像以及正式中文说明文档长期保持一致，建立“改契约即改文档/ADR/测试”的硬约束。

**涉及目录**

- `contracts/v1`
- `contracts/README.md`
- `packages/contracts`
- `crates/octopus-hub/src/contracts.rs`
- `docs/CONTRACTS.md`
- `docs/adr`

**依赖**

- 当前 `contracts/v1/*` 已存在
- `packages/contracts` 与 `crates/octopus-hub` 已有最小镜像实现

**执行动作**

- 盘点 `core-objects`、`enums`、`events` 的 JSON / TS / Rust 三处镜像是否完全一致
- 补齐镜像校验范围，从“对象名/事件名/枚举名”扩展到最小字段集合与枚举值集合
- 明确契约变更的同步规则：契约源、中文说明文档、ADR、测试必须同批变更
- 在后续公共接口扩张前，先把契约变更门禁写清楚，避免实现先行、文档补票

**完成条件**

- TS / Rust / JSON 三处镜像一致
- 最小字段集合、枚举值、事件最小 payload 均有自动化校验
- 契约变更同步规则被固定为仓库共识

**验证命令**

- `pnpm --filter @octopus/contracts typecheck`
- `pnpm --filter @octopus/contracts test`
- `cargo test -p octopus-hub --test contract_catalog`
- `cargo test --workspace`

### WP2 Hub Runtime Backbone

**目标**

把 `InMemoryRuntimeService` 从演示级骨架收敛成首条 GA 纵切片的稳定 runtime backbone，覆盖 Run、Approval、Artifact、Inbox、Trace、Audit 的最小领域闭环。

**涉及目录**

- `crates/octopus-hub/src/runtime.rs`
- `crates/octopus-hub/src/lib.rs`
- `crates/octopus-hub/tests`

**依赖**

- WP1 契约镜像稳定
- `RunType`、`RunStatus`、`ApprovalType` 等基础枚举保持稳定

**执行动作**

- 梳理 `RunRecord`、`ApprovalRequestRecord`、`ArtifactRecord`、`InboxItemRecord`、`TraceEvent`、`AuditEntry` 的最小职责与字段边界
- 明确 `submit_task`、`resolve_approval`、`resume_run` 的状态推进、错误语义与幂等预期
- 保持 `InMemoryRuntimeService` 作为首个可验证实现，不提前引入数据库事实层
- 为主线补足非法状态、非法恢复、缺失对象等失败路径测试

**完成条件**

- 手动任务主线稳定
- 审批等待、审批决议、恢复完成路径明确
- 结果对象、追踪事件和审计链语义一致
- 非法状态切换返回可判定错误，而不是静默放过

**验证命令**

- `cargo test -p octopus-hub`
- `cargo test --workspace`
- `cargo build --workspace`

### WP3 Server Transport Baseline

**目标**

以现有最小 HTTP 接口为种子，建立“按纵切片增量扩 API”的 transport 基线，不预铺全量目标态接口。

**涉及目录**

- `crates/octopus-server/src/lib.rs`
- `crates/octopus-server/src/main.rs`
- `crates/octopus-server/tests`

**依赖**

- WP2 runtime backbone 稳定
- 错误语义与状态推进已在 hub 层稳定

**执行动作**

- 固化现有 `/healthz`、`/api/v1/contracts`、`/api/v1/runs/task`、`/api/v1/runs/{run_id}`、`/api/v1/runs/{run_id}/resume`、`/api/v1/approvals/{approval_id}/resolve`
- 明确 transport 层只负责 HTTP 映射、状态码和错误转换，不复制业务规则
- 后续接口仅沿已验证的纵切片补充，例如 Automation、Trigger、Knowledge、MCP 相关入口
- 补齐 HTTP 层 smoke 和冲突场景覆盖

**完成条件**

- HTTP 返回码、错误信息、状态推进与 runtime 一致
- transport 层不越权承载领域规则
- 新增接口只围绕已批准纵切片扩张

**验证命令**

- `cargo test -p octopus-server --test http_smoke`
- `cargo test --workspace`
- `cargo build --workspace`

### WP4 Client Control Plane Shell

**目标**

基于现有 Vue 壳层，逐步补齐首版 GA 所需的最小控制面表面，使其能展示 Connections、Workspace、Run Detail、Inbox、Trace、Knowledge 等正式对象与状态。

**涉及目录**

- `apps/octopus-client/src/components`
- `apps/octopus-client/src/views`
- `apps/octopus-client/src/router`
- `apps/octopus-client/src/stores`
- `apps/octopus-client/src/i18n`
- `apps/octopus-client/src/styles`

**依赖**

- WP2 runtime backbone 稳定
- WP3 transport 基线稳定
- 工程规范中的 i18n、主题和分层规则已被遵守

**执行动作**

- 在现有 shell 基础上扩展 `Connections / Workspace / Run Detail / Inbox / Trace / Knowledge` 最小表面
- `Chat / Board` 首版只保留壳层或占位，不强行承诺完整工作台
- 保持页面、Store、Transport、组件职责清晰，不让页面直接承担通信细节
- 为路由、i18n、主题切换和主线对象展示补最小 smoke 验证

**完成条件**

- 本地壳层能稳定展示当前 GA 主线对象与状态
- 路由、i18n、主题切换与主要展示组件保持可验证状态
- 前端未越权复制 Hub 业务规则

**验证命令**

- `pnpm --filter @octopus/client typecheck`
- `pnpm --filter @octopus/client test`
- `pnpm run build:web`

### WP5 Automation & Trigger

**目标**

实现首版 GA 允许的四类 Trigger：`cron`、`webhook`、`manual event`、`MCP event`，打通 Automation 发起或续接 Run 的最小闭环。

**涉及目录**

- `contracts/v1/enums.json`
- `contracts/v1/events.json`
- `packages/contracts`
- `crates/octopus-hub`
- `crates/octopus-server`
- `apps/octopus-client`

**依赖**

- WP1 契约同步规则稳定
- WP2、WP3 已建立稳定 Run 主线

**执行动作**

- 明确 `Automation`、`Trigger`、`watch` 的最小字段、状态和事件流
- 建立 Trigger 投递幂等键、重复投递去重、失败分类与等待/终止语义
- 让 Automation 能显式发起或续接 Run，而不是绕过 Run 体系
- 在控制面上补最小查看与调试入口，不要求一次做成完整调度中心

**完成条件**

- Automation 能创建或续接 Run
- 同一 Trigger 的重复投递不会产生重复业务结果
- Trigger 与 Run 的关联、失败分类和审计语义成立

**验证命令**

- `pnpm run typecheck:web`
- `pnpm run test:web`
- `cargo test --workspace`
- `cargo build --workspace`

### WP6 Shared Knowledge GA

**目标**

围绕 `candidate -> verified_shared` 建立 Shared Knowledge GA 最小闭环，同时把 `promoted_org` 明确保留为 Beta 接口。

**涉及目录**

- `contracts/v1/core-objects.json`
- `contracts/v1/enums.json`
- `contracts/v1/events.json`
- `packages/contracts`
- `crates/octopus-hub`
- `crates/octopus-server`
- `apps/octopus-client`
- `docs/CONTRACTS.md`

**依赖**

- WP2 Run 主线稳定
- WP5 Automation / Trigger 闭环稳定

**执行动作**

- 明确 `KnowledgeSpace` 是 Shared Knowledge 主属边界，`Project` 只附着视图
- 建立候选知识、共享知识、来源、负责人、审批、删除墓碑的最小治理模型
- 将来自 Run、Automation、MCP 或文件解析的结果先落为候选知识，再决定是否晋升
- 为控制面补最小知识列表、来源链与状态展示入口

**完成条件**

- Run / Automation 结果可进入候选知识
- 候选知识可以受控晋升为共享知识
- `promoted_org` 只保留状态与接口，不被写成 GA 已交付
- 知识主属、来源、负责人和删除语义明确

**验证命令**

- `pnpm run typecheck:web`
- `pnpm run test:web`
- `cargo test --workspace`
- `cargo build --workspace`

### WP7 MCP & Governance Guardrails

**目标**

把 MCP 纳入正式互操作边界，同时保持治理与知识写回门控，确保外部结果默认不可信。

**涉及目录**

- `contracts/v1`
- `packages/contracts`
- `crates/octopus-hub`
- `crates/octopus-server`
- `apps/octopus-client`
- `docs/SAD.md`
- `docs/VIBECODING.md`

**依赖**

- WP2 runtime backbone 稳定
- WP5 Trigger / Automation 闭环稳定
- WP6 Shared Knowledge 规则稳定

**执行动作**

- 建立 MCP 调用在 Runtime、Governance、Observation、Knowledge 写回之间的最小责任边界
- 明确 MCP 输出默认视为不可信，不允许直接写入长期知识
- 为 MCP 结果引入审批、审计、候选知识写回与失败分类语义
- 在控制面上补最小 MCP 状态查看和风险提示入口

**完成条件**

- MCP 调用、审计、审批、知识写回门控一致
- 外部结果默认不可信的规则在实现与文档中保持一致
- MCP 不成为绕过 Runtime / Governance / Knowledge 的旁路

**验证命令**

- `pnpm run typecheck:web`
- `pnpm run test:web`
- `cargo test --workspace`
- `cargo build --workspace`

### WP8 Release Hardening

**目标**

把首版 GA 的验证链、文档同步规则、PR 模板、ADR 触发条件和对外叙述边界全部收敛，确保后续可持续演进。

**涉及目录**

- `README.md`
- `AGENTS.md`
- `docs/CONTRACTS.md`
- `docs/ENGINEERING_STANDARD.md`
- `docs/VIBECODING.md`
- `docs/adr`
- `.github/workflows/guardrails.yml`
- `.github/pull_request_template.md`

**依赖**

- WP1 到 WP7 已形成稳定结果

**执行动作**

- 校准仓库文档入口、PR 模板、guardrails 与当前已成立能力
- 明确“可以宣称的能力”和“只保留接口的能力”
- 收敛首版 GA 对外叙述，不提前承诺 Beta/Later 能力
- 固化后续变更的文档同步、验证命令和 ADR 触发规则

**完成条件**

- 仓库能以当前真实链路支撑首版 GA 说明与持续开发
- 文档、PR 模板与 CI 门禁口径一致
- 后续团队不会把目标态能力误写成当前现状

**验证命令**

- `git diff --check`
- `pnpm run typecheck:web`
- `pnpm run test:web`
- `pnpm run build:web`
- `cargo test --workspace`
- `cargo build --workspace`

## 4. Beta / Later 能力包

以下能力在当前阶段只保留能力包，不做细粒度任务拆解。每个能力包必须说明：为什么不进首版 GA、依赖哪些 GA 基线、当前阶段保留哪些对象/状态/接口/目录位、进入实施前必须补哪些 ADR/契约/验证链。

### 4.1 DiscussionSession

- 不进首版 GA 的原因：当前首版交付优先保证 `task`、`automation`、`review` 主线闭环；多角色讨论与结论沉淀属于 Beta 复杂协作能力。
- 依赖的 GA 基线：Run 主线、Approval、Artifact、Trace、Inbox、最小 Knowledge 闭环。
- 当前阶段保留内容：`discussion` 作为正式 `run_type`，`DiscussionSession` 作为对象模型与架构接口。
- 进入实施前必须补齐：Discussion 的状态流、参与者模型、结论 Artifact 语义、必要 ADR 与测试链。

### 4.2 ResidentAgentSession

- 不进首版 GA 的原因：常驻代理涉及长期运行、事件源可靠性、降级恢复、治理边界与通知策略，复杂度高于首版纵切片。
- 依赖的 GA 基线：Trigger、Automation、Approval、EnvironmentLease、Inbox、Trace/Audit。
- 当前阶段保留内容：`ResidentAgentSession` 作为正式对象与 Beta 接口位，`watch` 作为受控运行语义入口。
- 进入实施前必须补齐：长时会话状态机、事件源健康与降级、租约恢复策略、必要 ADR 与验证链。

### 4.3 高阶 Mesh / Cross-Team Mesh

- 不进首版 GA 的原因：高阶协作需要 `authority_scope`、`knowledge_scope`、`delegation_edges`、责任边界与循环委托阻断全部成立。
- 依赖的 GA 基线：Agent、Team、Run、Approval、Inbox、KnowledgeSpace、Delegation 记录语义。
- 当前阶段保留内容：`Team`、`DelegationGrant`、相关枚举和对象边界已冻结。
- 进入实施前必须补齐：协作拓扑与权限约束的正式配置模型、循环委托保护、Cross-Team 边界规则与 ADR。

### 4.4 A2A

- 不进首版 GA 的原因：外部 Agent 协作涉及对端登记、身份声明、授权窗口、预算、信任级别和不完整回执治理。
- 依赖的 GA 基线：Delegation、Approval、BudgetPolicy、Audit、Interop 基线、MCP 治理语义。
- 当前阶段保留内容：`A2APeer`、`ExternalAgentIdentity`、`delegation` 作为正式对象和接口位。
- 进入实施前必须补齐：A2A 协议边界、对端健康模型、身份校验、最小 transport 与验证链、必要 ADR。

### 4.5 Org Knowledge Graph

- 不进首版 GA 的原因：组织级晋升要求正式 lineage、责任归属、冲突裁决、删除传播与索引一致性，不宜在首版一次做全。
- 依赖的 GA 基线：候选知识、共享知识、审批、负责人、删除墓碑、检索与审计闭环。
- 当前阶段保留内容：`promoted_org` 状态、`KnowledgeAsset` 分层语义、相关对象与边界约束。
- 进入实施前必须补齐：晋升规则、组织级所有权、图谱投影策略、删除传播与必要 ADR/验证链。

### 4.6 Mobile

- 不进首版 GA 的原因：移动端首要承载审批、通知、查看和轻量介入，但当前最小可验证骨架仍在 Desktop/Web/Remote Hub。
- 依赖的 GA 基线：Inbox、Notification、Approval、Trace、Artifact、Connection 语义稳定。
- 当前阶段保留内容：SAD 中的跨端继续目标态、PRD 中的 Beta 路线定位。
- 进入实施前必须补齐：移动端表面范围、鉴权与通知语义、离线缓存策略、必要 ADR 与验证链。

### 4.7 更广泛事件生态与企业治理扩展

- 不进首版 GA 的原因：更广泛事件源生态、模型中心、多租户深治理与配额联动依赖更成熟的运行时与观测基线。
- 依赖的 GA 基线：Trigger、Approval、BudgetPolicy、Audit、MCP 治理、Shared Knowledge。
- 当前阶段保留内容：PRD/SAD 中的目标态边界、相关对象语义与治理原则。
- 进入实施前必须补齐：企业级存储与策略模型、更多协议适配、容量与成本治理链、必要 ADR 与验证链。

## 5. 验证与文档同步规则

### 5.1 文档级验证

所有文档变更至少执行以下 truthful minimum verification：

- required-doc existence checks
- stale-reference 搜索，确认未重新引入已删除文档树、旧项目名或无效正式输入
- 聚焦 diff 复核，确认没有把目标态能力写成当前现状
- `git diff --check`

### 5.2 workspace 级验证

若变更触达 `contracts/`、`packages/`、`apps/` 或 `crates/`，追加执行以下真实存在的 workspace 验证链：

- `pnpm run typecheck:web`
- `pnpm run test:web`
- `pnpm run build:web`
- `cargo test --workspace`
- `cargo build --workspace`

### 5.3 近端关键场景清单

后续实施首版 GA 时，至少要持续验证以下场景：

- contracts JSON 与 TS/Rust 镜像不漂移
- task 在无审批时直接完成并产出 artifact
- task 在有审批时进入 `waiting_approval`，决议后可 `resume`
- 非法 resume、非法 approval resolve 返回冲突或不存在
- Trigger 幂等去重不生成重复 Run
- 候选知识不能绕过验证直接成为长期共享知识
- MCP 输出默认不可信，不能直接写入长期知识
- Client 壳层至少通过路由、i18n、主题切换和 GA 主线展示 smoke

### 5.4 文档同步规则

以下变化必须同步更新相关正式文档，而不能只停留在代码或聊天记录中：

- 核心对象、共享枚举、事件骨架变化
- 目录边界、主技术栈、运行时模式、数据存储策略变化
- guardrails、PR 模板、验证链说明变化
- MCP、A2A、宿主扩展或组件体系的重大调整

最小同步要求如下：

- 契约变化：同步 `contracts/v1/*`、`packages/contracts`、`crates/octopus-hub`、`docs/CONTRACTS.md`
- 架构边界变化：同步 `docs/SAD.md` 与 `docs/adr`
- 工程规则变化：同步 `docs/ENGINEERING_STANDARD.md`、`docs/VIBECODING.md`
- 正式入口或门禁变化：同步 `README.md`、`AGENTS.md`、`.github/workflows/guardrails.yml`、`.github/pull_request_template.md`

### 5.5 ADR 触发条件

出现以下情况时，必须新增或更新 ADR：

- 主技术栈调整
- Monorepo 目录边界调整
- 运行时模式调整
- 数据存储策略调整
- 契约源调整
- 插件、MCP 或宿主扩展机制的重大变化

### 5.6 默认假设

- 默认不写具体排期日期、负责人和人力估算，只写阶段里程碑。
- 默认以当前 tracked tree 为唯一起点，不补写不存在的数据库、SSE、protobuf、OpenAPI、`turbo`、`buf` 或端到端 UI 能力为“已成立”。
- 默认采用“近细远粗”，只对 `Phase 1 / 首版 GA` 给出可执行任务拆解。
- 默认把 `InMemoryRuntimeService + 最小 HTTP + Vue shell` 视为当前最安全的纵切片起跑线。
- 默认不在当前阶段承诺 `A2A`、`Org Knowledge Graph`、`DiscussionSession`、`ResidentAgentSession`、高阶 `Mesh`、`Mobile` 为首版交付。
- 若后续需要改变阶段切分、技术栈、目录边界、契约源或治理策略，先更新 ADR，再更新本计划。
