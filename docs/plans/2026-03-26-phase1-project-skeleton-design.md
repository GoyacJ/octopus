# Octopus Phase 1 Project Skeleton Design

**状态**: `approved`
**日期**: `2026-03-26`
**owner**: `Codex`
**release_slice**: `GA`

## 1. 目标

本设计文档只回答 Phase 1 的问题：在不恢复历史全量仓库骨架、也不提前进入 contract / 实现细节的前提下，Octopus 后续 GA 重建应该采用什么最小 repo topology。

本设计不做以下事情：

- 不创建任何代码目录或 manifests
- 不定义正式对象 schema、事件或状态机
- 不进入 Slice A 的 IA、页面布局或 implementation plan
- 不把 `Web`、`Mobile`、`A2A`、`DiscussionSession`、`ResidentAgentSession`、高阶 `Mesh` 带入当前骨架主线

## 2. 输入依据

- [`README.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/README.md)
- [`AGENTS.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/AGENTS.md)
- [`docs/PRD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md)
- [`docs/SAD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md)
- [`docs/ENGINEERING_STANDARD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/ENGINEERING_STANDARD.md)
- [`docs/CODING_STANDARD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/CODING_STANDARD.md)
- [`docs/AI_DEVELOPMENT_PROTOCOL.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/AI_DEVELOPMENT_PROTOCOL.md)
- [`docs/DELIVERY_GOVERNANCE.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/DELIVERY_GOVERNANCE.md)
- [`docs/plans/2026-03-26-ga-rebuild-project-development-plan.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/plans/2026-03-26-ga-rebuild-project-development-plan.md)
- [`docs/adr/20260326-phase1-minimal-repo-topology.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/adr/20260326-phase1-minimal-repo-topology.md)

## 3. 当前仓库现实

截至 `2026-03-26`，当前 tracked tree 只证明以下事实：

- 根目录文档存在：`README.md`、`AGENTS.md`
- `docs/` 目录存在，并包含 PRD、SAD、工程规范、AI 规范、视觉框架、交付治理、模板和总计划文档
- 运行时代码树、workspace manifests、测试树、CI 工作流都不在当前 tracked tree 中

因此，本阶段只能定义“未来如何最小化地恢复实现骨架”，不能把任何目录或构建链路写成已实现现实。

## 4. 设计约束

本骨架设计必须同时满足以下约束：

1. 只服务 `GA` 主线：`Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`
2. Phase 4 最小代码骨架只允许支持 Slice A，不为全部目标态能力铺结构
3. `Run` 继续作为权威执行壳，`Hub` 继续作为事实源
4. `ToolSearch` 只发现能力，不自动授权
5. 目录与 manifests 的引入时机必须按切片推进，而不是按“最终形态”一次性恢复

## 5. 拓扑决策

基于 ADR，本设计采用以下最小 repo topology 策略：

### 5.1 当前阶段保持 doc-first

Phase 1 结束时，仓库仍保持以根目录文档与 `docs/` 为主的结构，不新增任何实现目录。

### 5.2 Phase 2 只新增 contract 文档目录

进入 GA 核心 contract 阶段时，仅新增：

- `docs/contracts/runtime/`
- `docs/contracts/governance/`
- `docs/contracts/interaction/`

如后续切片确有需要，再扩到 `knowledge/` 与 `interop/` 子目录。此阶段仍不创建实现目录。

### 5.3 Phase 4 最小代码骨架

当进入 “最小代码骨架建立” 阶段时，只引入支撑 Slice A 所需的最小目录：

- `apps/octopus-desktop/`
  - 作为唯一已跟踪 Client 表面
  - 承载 Vue 3 + TypeScript 前端与 Tauri 2 桌面壳
  - 负责 `Interaction Plane` 的页面壳、组件、状态投影和与 Hub 的本地/远程连接
- `crates/octopus-hub/`
  - 作为唯一已跟踪 Rust 核心 crate
  - 承载 `Run`、审批、Inbox、Trace 所需的最小 Hub 核心
  - 内部分层遵循 `domain / application / adapter / infra`

Phase 4 明确不创建：

- `crates/octopus-server/`
- `packages/`
- `apps/octopus-web/`
- 任何与 Slice A 无关的 worker、connector、evaluation 或 infra 目录

### 5.4 Phase 6 才引入远程 Hub 入口

`crates/octopus-server/` 作为远程 Hub HTTP/SSE 入口，只在 Phase 6 “远程与上下文主线” 启动时引入。理由：

- 当前总体计划把 Remote Hub 的主线收敛放在 Slice B，而不是 Slice A
- 在此之前提前加入 server 入口，会把骨架设计变成半成品恢复

### 5.5 `packages/` 默认延后

`packages/` 目录不是 Phase 4 最小骨架的一部分。只有当出现至少一个明确的第二个 TypeScript 消费者，且共享代码无法保持在 `apps/octopus-desktop/` 内部时，才可以通过单独切片引入 `packages/`。

这意味着以下目录当前都不应预建：

- `packages/ui/`
- `packages/contracts/`
- `packages/shared-ts/`

## 6. Manifests 与入口规则

### 6.1 Rust manifests

只有在 Phase 4 真正创建 Rust 实现目录时，才引入 root `Cargo.toml` workspace。原因：

- `crates/octopus-hub/` 需要成为可复用核心
- 未来 `crates/octopus-server/` 会复用同一 Hub 核心
- 在当前 doc-only 阶段提前恢复 Rust workspace 会制造“已可构建”的误导

### 6.2 JavaScript manifests

Phase 4 只在 `apps/octopus-desktop/` 内引入局部 `package.json`。以下文件明确延后：

- root `package.json`
- `pnpm-workspace.yaml`
- 任何共享 TS package 的 workspace 清单

延后原因是：在只有一个 JS 消费者时，根工作区清单并不是最小必需资产。

### 6.3 代码入口规则

- 本地模式入口：`apps/octopus-desktop/` 通过 Tauri 壳调用 `crates/octopus-hub/`
- 远程模式入口：延后到 `crates/octopus-server/` 引入时，再建立 HTTP/SSE 远程入口
- 正式对象与执行语义仍以 `crates/octopus-hub/` 为中心，不允许把 `Run`、审批或 Trace 逻辑散落在前端壳层

## 7. 局部 AGENTS 放置规则

当实现目录首次进入 tracked tree 时，必须同步加入更近的 `AGENTS.md`：

- `apps/octopus-desktop/AGENTS.md`
  - 约束 Vue / TypeScript / Tauri 壳层的构建、测试和样式规则
- `crates/AGENTS.md`
  - 约束 Rust workspace 的通用构建、测试、命名和分层规则

若后续某个 crate 拥有显著不同的本地命令或约束，再单独添加更近的 crate 级 `AGENTS.md`。在没有这种差异之前，不提前创建多层重复规则文件。

## 8. 阶段化目录表

| 资产 | 引入阶段 | 结论 |
| --- | --- | --- |
| `docs/contracts/**` | Phase 2 | 允许创建，先文档化正式对象 contract |
| `docs/plans/**` | 已存在 / 持续使用 | 继续承载切片卡、设计和实施计划 |
| `docs/adr/**` | Phase 1 | 允许创建，用于记录 repo topology 取舍 |
| `apps/octopus-desktop/` | Phase 4 | 允许创建，作为首个也是唯一客户端表面 |
| `crates/octopus-hub/` | Phase 4 | 允许创建，作为唯一 Hub 核心实现 crate |
| root `Cargo.toml` | Phase 4 | 允许创建，但仅在 Rust 目录实际出现时创建 |
| `crates/octopus-server/` | Phase 6 | 延后创建，服务 Remote Hub 主线 |
| root `package.json` / `pnpm-workspace.yaml` | 延后 | 只有在第二个 JS 消费者出现时才考虑 |
| `packages/**` | 延后 | 不是 Slice A 最小骨架的一部分 |
| `apps/octopus-web/` | 非当前主线 | 不进入当前 GA 重建阶段 |

## 9. 明确不创建的目录

在 Phase 4 之前，以及未获后续切片批准之前，以下目录都不应被“顺手”创建：

- `apps/octopus-web/`
- `apps/octopus-mobile/`
- `packages/ui/`
- `packages/contracts/`
- `packages/shared-ts/`
- `crates/octopus-server/`
- `crates/octopus-a2a/`
- `crates/octopus-mesh/`
- `contracts/`
- `.github/workflows/`

说明：

- `contracts/` 的正式位置改为 `docs/contracts/`，不再恢复历史根目录 `contracts/`
- `.github/workflows/` 不是当前 truthful minimum verification 的必需前置条件，不应在骨架阶段提前恢复

## 10. 验收条件

本设计被视为达到 Phase 1 退出条件，需要同时满足：

1. 人类可以明确看出未来最小目录拓扑，而不是只看到“以后再说”
2. 文档已经明确写清什么现在不创建、什么在 Phase 4 创建、什么延后到后续切片
3. 后续 Phase 2 contract 与 Phase 4 最小骨架建立都能直接引用本设计，不再重新争论目录边界

## 11. 风险与后续

### 11.1 当前仍未完成的事项

- 尚未创建任何实现目录、manifests、测试树或构建链路
- 尚未建立 Phase 4 最小代码骨架
- 尚未进入 Slice A 的真实代码实现与验证

### 11.2 已完成的衔接

- `docs/contracts/` 首批 contract 已形成并作为 Phase 2 基线进入仓库
- Slice A 的 task slice card、IA 设计与 implementation plan 已进入 `docs/plans/`
- 因此后续 Phase 4 应直接按这些文档进入最小代码骨架，而不是重新争论目录边界

### 11.3 停机点

如果后续有人要求在 Phase 2 或 Phase 3 提前恢复：

- `apps/octopus-web/`
- `packages/`
- `crates/octopus-server/`
- 大规模 CI / workflow / infra 目录

则应先停止执行，并回到边界确认，而不是继续默认推进。
