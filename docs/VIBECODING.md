# octopus VibeCoding 执行基线

更新时间：2026-03-26
文档状态：Draft
文档定位：仓库级研发流程基线
关联文档：`README.md`、`AGENTS.md`、`docs/PRD.md`、`docs/SAD.md`、`docs/CONTRACTS.md`、`contracts/README.md`、`docs/DEVELOPMENT_PLAN.md`、`docs/DEVELOPMENT_CHANGELOG.md`、`docs/ENGINEERING_STANDARD.md`、`docs/adr/README.md`

---

## 1. 文档目的

本文档定义 `octopus` 项目在 AI 主导实现模式下的执行边界，目标是防止项目在“AI 能快速产出代码”的错觉中失去范围控制、验收标准和安全底线。

本文件不替代产品、架构或工程规范文档，而是约束“如何开发”。

---

## 2. 基本原则

`octopus` 采用 VibeCoding 开发模式，但不采用“放任 AI 连续生成整仓代码”的开发方式。

项目执行遵守以下总原则：

1. 人主导边界、验收、例外审批和风险签收。
2. AI 负责在既定范围内推进细节实现、重复劳动和文档同步。
3. 任何超出 PRD、SAD、开发规范、ADR 或仓库级协作约束的扩张，都必须先回到人工决策。
4. 安全底线与功能迭代并行推进，不允许用“先跑起来再说”替代治理。

---

## 3. 开发前九大预备步骤

任何核心功能、目录级初始化或架构级调整前，必须完成以下准备：

1. 需求梳理：明确问题、用户、场景、非目标和约束。
2. 结构化 PRD 与验收标准：写清范围、场景、核心对象和可验证验收。
3. 交互与表面规划：明确表面职责、导航、关键页面边界、主题和语言边界。
4. 边界与非功能需求界定：明确性能、恢复、安全、审计、兼容性要求。
5. 可验证技术栈选定：选型必须可安装、可构建、可测试、可运维。
6. 轻量化架构草案：先锁模块职责、数据流和信任边界，不先铺细节实现。
7. 根目录文档固化：将规则写入仓库，而不是停留在口头约定或聊天记录。
8. 开发规范制定：统一命名、目录、测试、契约、文档同步和发布规则。
9. Git 质量管控：建立分支策略、提交规范、PR 模板、hooks、CI 和最小门禁。

---

## 4. 开发中五大关键原则

### 4.1 增量纵切片迭代

1. 每轮只交付一条最小可验证纵切片。
2. 在一个闭环通过之前，不扩展第二批能力。
3. 所有“未来能力”必须先留接口位或目录位，再按优先级解锁。
4. 每轮开始前先对齐 `docs/DEVELOPMENT_PLAN.md` 当前阶段，并在 `docs/DEVELOPMENT_CHANGELOG.md` 建立或更新记录。

### 4.2 人工介入防代码混沌

以下事项必须由人工确认：

1. 范围定义与验收签收。
2. 架构级例外与 ADR。
3. 安全策略变化。
4. 高风险依赖准入。
5. 关键产品交互取舍。

### 4.3 AI 行为强制限域

AI 默认只允许：

1. 在已批准目录中增量实现。
2. 依据现有契约和文档补代码、补测试、补脚手架。
3. 做局部重构与文档同步。
4. 在前端页面优化或界面生成任务中，默认遵循 `docs/ENGINEERING_STANDARD.md` 中的 AI 前端设计提示模板，并以现有 design tokens、主题、i18n 和组件约束为准。

AI 默认不允许：

1. 自行扩范围。
2. 改主技术栈。
3. 绕过 ADR、审批或文档同步。
4. 把目标态设计说成现状能力。
5. 在未验证时声称“已完成”。
6. 把外部参考资料中的消费级工具全集直接映射为 Octopus 的核心领域对象或默认能力面。

### 4.4 安全底线刚性执行

以下能力必须从第一条纵切片开始纳入硬约束：

1. 命令执行审批。
2. 网络访问控制。
3. 敏感资源保护。
4. 审计与时间线留痕。
5. 恢复流程的 freshness / policy / budget 重验证。

### 4.5 报错科学处置三步法

出现错误、失败或偏航时，必须按以下顺序处理：

1. 先分类：归入 `planning`、`context`、`tool schema`、`policy`、`resume/idempotency`、`human coordination` 等失败类型。
2. 再定界：明确是需求问题、架构问题、契约问题、实现问题、环境问题还是测试问题，并补充最小复现与证据。
3. 后修复：只做最小必要改动，随后立即回归验证与文档同步。

禁止一边猜原因、一边并行堆补丁。

---

## 5. 人与 AI 的职责分工

| 事项 | 人工负责 | AI 负责 |
| --- | --- | --- |
| 产品边界 | 定义与裁剪 | 不得越权扩张 |
| 验收标准 | 建立与签收 | 对照实现与回归 |
| 架构边界 | 确认与例外批准 | 按边界内执行 |
| 代码实现 | 把控关键方案 | 大量细节落地 |
| 安全与依赖 | 准入与升级签收 | 执行检查与补文档 |
| 文档同步 | 决定是否需要补充 | 实际补写与更新 |

---

## 6. 当前仓库事实基线

截至 2026-03-26，`octopus` 当前处于 `doc-first rebuild` 状态：根目录文档、`docs/`、`contracts/` 与 `.github/` 共同构成事实基线。历史 `apps/`、`packages/`、`crates/` 与 workspace manifests 已不在当前 tracked tree 中。

已具备：

1. [README.md](../README.md)
2. [AGENTS.md](../AGENTS.md)
3. [PRD.md](PRD.md)
4. [SAD.md](SAD.md)
5. [CONTRACTS.md](CONTRACTS.md)
6. [../contracts/README.md](../contracts/README.md)
7. [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md)
8. [DEVELOPMENT_CHANGELOG.md](DEVELOPMENT_CHANGELOG.md)
9. [ENGINEERING_STANDARD.md](ENGINEERING_STANDARD.md)
10. [adr/README.md](adr/README.md)

在这一状态下，执行时必须遵守以下事实约束：

1. 优先把产品、架构、数据、契约和治理规则对齐，再推进实现。
2. 不把目标态目录、脚手架或运行时行为误写成“当前已存在能力”。
3. 在缺少 manifest、源码或工具时，不虚构构建、测试或运行结论；仅当对应 manifests 和源码重新进入 tracked tree 时，才使用 `pnpm` / `cargo` 验证链路。
4. 不引用已删除的旧文档树作为正式输入，例如旧 API 文档目录、旧计划目录、旧变更目录，以及已经下线的架构/领域/数据模型/视觉框架文档。
5. 不把参考资料中的 `alarm`、`reminder`、`weather`、`places` 等消费级工具默认纳入首版核心范围；若未来需要，只能先经契约和 ADR 进入 adapter / connector 设计。

---

## 7. 默认工程依赖顺序

后续所有实施默认按以下依赖顺序推进：

1. 治理与文档基线。
2. `docs/DEVELOPMENT_PLAN.md` 与 `docs/DEVELOPMENT_CHANGELOG.md`。
3. 契约源目录。
4. `CapabilityCatalog / CapabilityResolver / ToolSearch / SkillPack` 等能力运行时骨架。
5. 结构化交互、分层记忆与 Artifact 会话态的共享契约。
6. 如需重建实现，再引入仓库和工具链骨架。
7. 任一条 `run -> interaction/approval -> resume -> timeline/audit` 的可验证纵切片。
8. 后续能力按验证结果逐条解锁。
