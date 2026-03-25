# 变更记录目录说明

`docs/changes/` 用于记录每个里程碑、主题或跨模块交付的实际变更结果，强调“做了什么、验证了什么、风险是什么、文档有没有同步”。

## 适用范围

以下内容应进入本目录：

1. 里程碑或关键主题完成后的变更摘要与证据。
2. 重要骨架初始化、契约落地、能力治理和发布硬化记录。
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

`YYYY-MM-DD-<topic>.md`

推荐示例：

1. `2026-03-25-planning-governance-unification.md`
2. `2026-03-26-contract-and-repo-baseline.md`
3. `2026-03-30-agent-and-team-runtime-foundation.md`

## 更新时机

1. 记录对象开始时，可以先创建文档并标记为 `In Progress`。
2. 记录对象阻塞时，更新阻塞原因、下一步动作和影响范围。
3. 记录对象完成时，必须更新为 `Done`，并补齐验证证据与文档同步情况。

## 必备结构

变更记录默认对齐仓库 PR 模板，至少应包含：

1. `Summary`
2. `Scope`
3. `Risks`
4. `Verification`
5. `Docs Sync`
6. `UI Evidence`
7. `Review Notes`

此外，建议在文档头部增加：

1. `Change`
2. `Status`
3. `Last Updated`
4. `Related Plan`

## 与计划体系的关系

1. `docs/plans/` 管正式顺序、里程碑依赖与实施计划。
2. 主计划只负责导航；里程碑实施计划负责可执行任务、验证与文档同步项。
3. `docs/changes/` 管已经发生的变更、证据和风险回顾。
4. 被跟踪对象状态变更时，主计划、实施计划与对应变更记录必须同步更新。
