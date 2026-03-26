# Phase 1 Project Skeleton Task Slice Card

## 1. 基本信息

- `task_name`: `phase1-project-skeleton-design`
- `request_type`: `skeleton`
- `secondary_type`: `design`
- `owner`: `Codex`
- `date`: `2026-03-26`

## 2. 目标

- 本次要解决什么问题：在 `doc-first rebuild` 阶段，为 Octopus 定义一个可批准的最小 repo topology、workspace 边界、manifests 引入策略和局部 `AGENTS.md` 放置规则，避免后续 contract 与实现阶段重新回到“先把全骨架搭起来”的旧路径。
- 为什么现在做：[`docs/plans/2026-03-26-ga-rebuild-project-development-plan.md`](./2026-03-26-ga-rebuild-project-development-plan.md) 已把 “项目骨架设计” 定义为 Phase 1，也是 Phase 2 contract 与 Phase 4 最小代码骨架建立的前置条件。

## 3. 作用范围

- `release_slice`: `GA`
- `planes`: `cross-plane engineering baseline`, `Runtime`, `Governance`, `Interaction`, `Interop`, `Execution`
- `surfaces`: `Desktop`, `Remote Hub` 的未来代码承载拓扑；当前仅落文档，不落实现
- `affected_objects`: `repository topology`, `workspace/manifests introduction policy`, `local AGENTS placement rules`

## 4. 边界检查

- 是否会改变产品范围：`no`
- 是否会改变架构主决策：`no`
- 是否会新增平台表面：`no`
- 是否涉及高风险能力或安全姿态变化：`no`
- 是否需要人工确认：`yes`

若任一项为 `yes`，写明原因：

- Phase 1 的退出条件要求人类确认最小化骨架边界后，才能进入 Phase 2 contract 定义。

## 5. 前置产物判断

- 是否需要 `ADR`：`yes`，因为 repo topology 至少存在 “最小 app+crates” 与 “提前恢复完整 monorepo” 两类长期可行方案，且该决策会长期影响后续实现结构。
- 是否需要 `contract`：`no`
- 是否需要 `implementation plan`：`no`
- 是否需要视觉/IA 设计：`no`
- 是否需要骨架设计说明：`yes`

## 6. 验收条件

- 成功条件 1：骨架设计文档明确回答未来哪些顶层目录会存在、哪些目录在当前阶段明确不创建、哪些目录属于后续切片再引入。
- 成功条件 2：文档明确禁止恢复历史全量 `apps/ packages/ crates/` 树，并把 manifests 与局部 `AGENTS.md` 的引入时机写清楚。
- 成功条件 3：人类可直接基于该文档判断是否批准进入 Phase 2 contract 定义，而无需再通过聊天补全目录边界。

## 7. 验证方式

- 当前仓库可实际执行的验证：确认新增文档存在；搜索与当前 tracked tree 不一致的 stale references；审阅 focused diff；运行 `git diff --check`
- 不能声称执行的验证：`pnpm` / `cargo` / app runtime / test suite 通过

## 8. 风险与停机点

- 主要风险：把目标态技术选型误写成当前已存在仓库现实；为“完整感”提前恢复历史目录；在骨架设计阶段过早引入 contract、事件、状态机细节。
- 发现以下情况时必须停下：需要改变 `GA/Beta/Later` 边界；需要新增 `Web`、`Mobile`、`A2A`、高阶 `Mesh` 等非当前主线表面；需要一次性恢复大规模代码树；无法保持 `Run` 权威执行壳与 `Hub` 事实源边界。

## 9. 输出物

- 预计新增或修改的文档/文件：
  - `docs/plans/2026-03-26-phase1-project-skeleton-task-slice-card.md`
  - `docs/plans/2026-03-26-phase1-project-skeleton-design.md`
  - `docs/adr/20260326-phase1-minimal-repo-topology.md`
  - `docs/SAD.md`
- 最终汇报需要说明的重点：为何需要 ADR；Phase 4 最小代码骨架只包含什么；哪些目录与 manifests 被明确延后；当前文档阶段完成了什么、仍未实现什么
