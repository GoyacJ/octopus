# Phase 2 GA Core Contracts Task Slice Card

## 1. 基本信息

- `task_name`: `phase2-ga-core-contracts`
- `request_type`: `contract`
- `owner`: `Codex`
- `date`: `2026-03-26`

## 2. 目标

- 本次要解决什么问题：为 Octopus 首轮 GA 重建固定第一批会被 Slice A 直接依赖的正式对象 contract，避免后续在实现阶段继续依赖聊天上下文解释对象语义。
- 为什么现在做：[`docs/plans/2026-03-26-ga-rebuild-project-development-plan.md`](./2026-03-26-ga-rebuild-project-development-plan.md) 已将 `Run`、`ApprovalRequest`、`InboxItem`、`TraceEvent`、`CapabilityVisibility / ToolSearch` 设为 Phase 2 第一批 contract，且 Phase 1 已批准最小文档拓扑，可进入 `docs/contracts/`。

## 3. 作用范围

- `release_slice`: `GA`
- `planes`: `Runtime`, `Governance`, `Interaction`, `Observation`
- `surfaces`: `Chat`, `Trace`, `Inbox`, `Desktop + Remote Hub`
- `affected_objects`: `Run`, `ApprovalRequest`, `InboxItem`, `TraceEvent`, `CapabilityVisibilityResult`, `ToolSearchResultItem`

## 4. 边界检查

- 是否会改变产品范围：`no`
- 是否会改变架构主决策：`no`
- 是否会新增平台表面：`no`
- 是否涉及高风险能力或安全姿态变化：`no`
- 是否需要人工确认：`yes`

若任一项为 `yes`，写明原因：

- 这些 contract 会成为后续 Slice A 和 Phase 4 最小代码骨架的直接引用基线；若人类认为某个字段、状态或治理边界超出当前接受范围，应在进入实现前修正。

## 5. 前置产物判断

- 是否需要 `ADR`：`no`
- 是否需要 `contract`：`yes`
- 是否需要 `implementation plan`：`no`
- 是否需要视觉/IA 设计：`no`
- 是否需要骨架设计说明：`no`，已由 Phase 1 文档覆盖

## 6. 验收条件

- 成功条件 1：`docs/contracts/` 下存在首批五份 contract 文档，并按 `runtime / governance / interaction` 进入最小目录。
- 成功条件 2：每份 contract 都明确状态、actor、scope、治理链路、异常路径、验收条件，并引用 `PRD/SAD` 对应章节。
- 成功条件 3：文档严格停留在 contract 层，不提前定义实现代码目录、API 路由、数据库表结构或测试结果。

## 7. 验证方式

- 当前仓库可实际执行的验证：确认新增文档存在；搜索相关 stale references；审阅 focused diff；运行 `git diff --check`
- 不能声称执行的验证：`pnpm` / `cargo` / app runtime / automated tests 通过

## 8. 风险与停机点

- 主要风险：为了“补全 contract”而发明 PRD/SAD 中未定义的新对象语义；把 UI 需求、实现字段或存储表结构误写成正式 contract；把 Beta 能力带入 GA contract 集合。
- 发现以下情况时必须停下：需要改变 `Run`、`ApprovalRequest` 或 `ToolSearch` 的核心语义；需要把 `A2A`、`DiscussionSession`、`ResidentAgentSession`、`Mobile` 带入首批 contract；需要把 contract 写成 wire API 或数据库 schema 承诺。

## 9. 输出物

- 预计新增或修改的文档/文件：
  - `docs/plans/2026-03-26-phase2-ga-core-contracts-task-slice-card.md`
  - `docs/contracts/README.md`
  - `docs/contracts/runtime/run.md`
  - `docs/contracts/runtime/trace-event.md`
  - `docs/contracts/runtime/capability-visibility-tool-search.md`
  - `docs/contracts/governance/approval-request.md`
  - `docs/contracts/interaction/inbox-item.md`
  - `README.md`
- 最终汇报需要说明的重点：首批 contract 为什么只覆盖这五类对象；哪些内容被明确延后到实现或后续切片； Phase 3 可以直接复用哪些 contract
