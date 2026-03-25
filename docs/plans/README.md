# 实施计划目录说明

`docs/plans/` 用于保存可执行的实施计划，而不是产品愿景或架构原则。

## 目录结构

当前目录采用“两层结构”：

1. 主计划：一个正式导航入口，负责里程碑顺序、依赖关系、当前焦点和退出条件。
2. 里程碑实施计划：一里程碑一文件，负责 AI 可直接执行的输入、合同冻结项、交付件、验证和文档同步。

## 适用范围

以下内容应进入本目录：

1. 跨多步、跨文档的产品开发总计划。
2. 以里程碑或关键主题为单位的实施计划。
3. 需要明确输入文档、合同冻结项、验证步骤和交付顺序的任务计划。

## 不适用范围

以下内容不应放入本目录：

1. PRD
2. SAD
3. 开发规范
4. ADR
5. 临时聊天记录

## 命名规则

文件名统一为：

- 主计划：`YYYY-MM-DD-product-development-master-plan.md`
- 里程碑实施计划：`YYYY-MM-DD-m<nn>-<topic>.md`

## 必备字段

主计划中的每个正式里程碑至少应包含：

1. `Depends On`
2. `Source Docs`
3. `Related Implementation Plan`
4. `Related Change`
5. `Exit Criteria`

里程碑实施计划至少应包含：

1. `Inputs`
2. `Contracts To Freeze`
3. `Repo Reality`
4. `Deliverables`
5. `Verification`
6. `Docs Sync`
7. `Open Risks`
8. `Out Of Scope`

## 状态与证据

计划文档建议显式包含以下字段：

1. 总体状态：`Not Started / In Progress / Blocked / Done`
2. `Last Updated`
3. `Current Focus` 或 `Objective`
4. 退出条件（Exit Criteria）
5. 验证基线（Verification Baseline）或具体验证步骤
6. 对应变更记录（`docs/changes/`）

## 更新约定

1. 里程碑顺序、依赖和当前焦点变化时，先更新主计划。
2. 里程碑内部任务、合同冻结项和交付件变化时，更新对应实施计划。
3. 里程碑或工作流进入 `In Progress / Blocked / Done` 时，同一次变更中同步更新对应 `docs/changes/` 文档。
4. 不为每个小任务单独创建 `docs/changes/` 文档，除非该任务本身就是独立里程碑或关键主题。
