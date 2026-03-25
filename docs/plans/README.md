# 实施计划目录说明

`docs/plans/` 用于保存可执行的实施计划，而不是产品愿景或架构原则。

## 适用范围

以下内容应进入本目录：

1. 跨多步的仓库初始化计划。
2. 跨能力域的产品开发总计划。
3. 跨模块改造计划。
4. 需要明确文件、验证步骤和交付顺序的任务计划。

## 不适用范围

以下内容不应放入本目录：

1. PRD
2. SAD
3. 开发规范
4. ADR
5. 临时聊天记录

## 命名规则

文件名统一为：

`YYYY-MM-DD-<topic>.md`

## 状态与证据

实施计划建议显式包含以下字段：

1. 总体状态：`Not Started / In Progress / Blocked / Done`
2. `Last Updated`
3. `Current Focus`
4. 退出条件（Exit Criteria）
5. 验证基线（Verification Baseline）
6. 对应变更记录（`docs/changes/`）

## 更新约定

1. 任务级 checklist 完成时，优先更新主计划中的勾选状态、`Last Updated` 与简短 evidence/note。
2. 里程碑、工作流或其他被跟踪对象进入 `In Progress / Blocked / Done` 时，同一次变更中同步更新对应 `docs/changes/` 文档。
3. 不为每个小任务单独创建 `docs/changes/` 文档，除非该任务本身就是独立里程碑或关键主题。
