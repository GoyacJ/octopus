# Octopus · 开发变更文档（DEVELOPMENT_CHANGELOG）

**版本**: v0.1.0 | **状态**: 累计记录基线 | **日期**: 2026-03-26
**配套文档**: [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md)

---

## 1. 文档目标

本文件用于记录 `octopus` 在 AI 辅助开发过程中的**已确认执行性变化**。

本文件回答三个问题：

1. 当前正在推进什么任务。
2. 它属于哪个阶段、依据哪些正式条款。
3. 它是否完成、如何验证、是否偏离既定范围。

本文件不替代：

- [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) 的阶段定义与执行顺序
- [PRD.md](./PRD.md) / [SAD.md](./SAD.md) / [CONTRACTS.md](./CONTRACTS.md) 的正式语义
- ADR 的架构例外记录职责

---

## 2. 使用规则

- 每个实际开发任务开始前，必须先在本文件建立一条记录。
- 每个任务完成后，必须回填验证方式、风险说明与最终状态。
- 本文件只记录**已确认的执行性变化**，不记录聊天式讨论、头脑风暴或未批准想法。
- 普通阶段推进不写 ADR。
- 仅当任务引入架构例外、目录边界变化、主技术栈变化或正式契约重大调整时，才同步进入 `docs/adr/`。

### 状态枚举

| 状态 | 含义 |
| --- | --- |
| `planned` | 已确认进入执行队列，但尚未开始 |
| `in_progress` | 正在实施中 |
| `completed` | 已完成并回填验证 |
| `blocked` | 因前置依赖或冲突被阻塞 |
| `dropped` | 已决定取消或移出当前范围 |

### 记录模板

后续新增记录必须至少包含以下字段：

- 变更编号
- 日期
- 所属阶段
- 当前状态
- 变更摘要
- 关联 PRD / SAD / CONTRACTS 条款
- 影响范围
- 是否涉及契约变更
- 是否涉及架构例外
- 是否偏离已批准范围
- 验证方式
- 风险与回滚说明
- 完成说明

---

## 3. 变更记录

## 0001 | 建立开发计划文档体系

- 日期: `2026-03-26`
- 所属阶段: `Phase 0`
- 当前状态: `completed`
- 变更摘要:
  - 建立仓库级 AI 开发执行主文档 `docs/DEVELOPMENT_PLAN.md`
  - 建立累计式变更记录文档 `docs/DEVELOPMENT_CHANGELOG.md`
  - 同步更新正式文档入口、工程规范、VibeCoding 基线、guardrails 与 PR 模板
- 关联条款:
  - [PRD.md](./PRD.md): `2.5`、`8.1`
  - [SAD.md](./SAD.md): `1.4`
  - [CONTRACTS.md](./CONTRACTS.md): `6`
  - [ENGINEERING_STANDARD.md](./ENGINEERING_STANDARD.md): `8.3`、`8.5`、`8.6`
  - [VIBECODING.md](./VIBECODING.md): `3`、`4.1`、`7`
- 影响范围:
  - `docs/DEVELOPMENT_PLAN.md`
  - `docs/DEVELOPMENT_CHANGELOG.md`
  - `README.md`
  - `AGENTS.md`
  - `docs/ENGINEERING_STANDARD.md`
  - `docs/VIBECODING.md`
  - `.github/workflows/guardrails.yml`
  - `.github/pull_request_template.md`
- 是否涉及契约变更: `no`
- 是否涉及架构例外: `no`
- 是否偏离已批准范围: `no`
- 验证方式:
  - required docs 与 contract-source existence checks
  - contract JSON 语法校验
  - stale-reference 搜索
  - focused diff review
  - `git diff --check`
- 风险与回滚说明:
  - 风险较低，主要风险是正式文档列表与 guardrails 不一致
  - 如需回滚，必须一起回滚两份新文档及所有入口同步变更，避免形成悬空引用
- 完成说明:
  - 已创建 `docs/DEVELOPMENT_PLAN.md` 与 `docs/DEVELOPMENT_CHANGELOG.md`
  - 已同步更新 `README.md`、`AGENTS.md`、`docs/ENGINEERING_STANDARD.md`、`docs/VIBECODING.md`、`.github/workflows/guardrails.yml`、`.github/pull_request_template.md`
  - 不涉及契约变更，也不触发新增 ADR 条件
