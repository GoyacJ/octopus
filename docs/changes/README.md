# 阶段变更记录目录说明

`docs/changes/` 用于记录每个阶段、里程碑或跨模块交付的实际变更结果，强调“做了什么、验证了什么、风险是什么、文档有没有同步”。

## 适用范围

以下内容应进入本目录：

1. 阶段完成后的变更摘要与证据。
2. 重要骨架初始化、契约落地、MVP 纵切片和发布硬化记录。
3. 需要和主计划状态联动更新的执行结果文档。

## 不适用范围

以下内容不应放入本目录：

1. PRD
2. SAD
3. 长期实施计划
4. ADR
5. 临时调试笔记

## 命名规则

文件名统一为：

`YYYY-MM-DD-<phase-or-topic>.md`

推荐示例：

1. `2026-03-24-phase-0-planning-and-tracking.md`
2. `2026-03-26-phase-1-contract-sources.md`
3. `2026-04-02-phase-3-mvp-vertical-slice.md`

## 更新时机

1. 阶段开始时，可以先创建文档并标记为 `In Progress`。
2. 阶段阻塞时，更新阻塞原因、下一步动作和影响范围。
3. 阶段完成时，必须更新为 `Done`，并补齐验证证据与文档同步情况。

## 必备结构

阶段变更记录默认对齐仓库 PR 模板，至少应包含：

1. `Summary`
2. `Scope`
3. `Risks`
4. `Verification`
5. `Docs Sync`
6. `UI Evidence`
7. `Review Notes`

此外，建议在文档头部增加：

1. `Stage`
2. `Status`
3. `Last Updated`
4. `Related Plan`

## 与主计划的关系

1. `docs/plans/` 管路线图、阶段目标、退出条件和默认顺序。
2. `docs/changes/` 管已经发生的阶段变更、证据和风险回顾。
3. 阶段状态变更时，主计划与对应变更记录必须同步更新。
