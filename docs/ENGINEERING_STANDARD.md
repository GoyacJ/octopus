# Octopus · 工程开发规范

**状态**: 基线建立版 | **日期**: 2026-03-26
**适用范围**: 人类工程师与 Coding Agent

---

## 1. 文档目的

本规范定义 Octopus 在实现阶段的统一工程约束，用于把 [`PRD.md`](./PRD.md) 与 [`SAD.md`](./SAD.md) 转换为可执行、可验证、可审计的开发规则。

本规范解决的是“怎么开发”，不改变产品范围、GA/Beta 切片或架构主约束。

当前仓库仍处于 `doc-first rebuild` 阶段；在真实源码、manifests、测试树重新进入 tracked tree 之前，本规范首先约束文档开发、设计收敛、接口建模和 truthful verification。

若任务已经进入“如何写代码、如何做分层、如何命名与 review”的层面，应继续阅读 [`CODING_STANDARD.md`](./CODING_STANDARD.md)。

## 2. 约束优先级

实现和文档变更必须按以下优先级解释：

1. 显式用户指令
2. [`AGENTS.md`](../AGENTS.md)
3. [`PRD.md`](./PRD.md) 中的产品范围、GA/Beta/Later 切片、验收意图
4. [`SAD.md`](./SAD.md) 中的架构边界、运行时约束、技术决策
5. 本文档中的工程实现规则
6. [`CODING_STANDARD.md`](./CODING_STANDARD.md) 中的实现级代码风格与分层模式
7. [`AI_ENGINEERING_PLAYBOOK.md`](./AI_ENGINEERING_PLAYBOOK.md) 中的 AI 执行手册
8. [`AI_DEVELOPMENT_PROTOCOL.md`](./AI_DEVELOPMENT_PROTOCOL.md) 中的固定开发协议与模板入口
9. [`VISUAL_FRAMEWORK.md`](./VISUAL_FRAMEWORK.md) 中的 GA 交互与视觉规则
10. [`DELIVERY_GOVERNANCE.md`](./DELIVERY_GOVERNANCE.md) 中的交付与文档治理流程

解释规则：

- 下层文档只能细化上层，不得改写上层边界。
- 若某实现需求会改变 `PRD` 的产品范围、`SAD` 的架构主决策或 `AGENTS` 的 truthfulness 规则，必须先由人类确认。

## 3. 默认研发模式

Octopus 当前默认采用 `AI-first 小团队` 研发模式：

- 目标是用最少的人类协调成本完成最小可验证垂直切片。
- AI 可以承担文档更新、方案收敛、实现、测试与自查，但不能自行扩 scope。
- 人类负责范围定义、边界变更、验收签字、架构例外、安全姿态变化和高风险依赖选择。

工程上必须优先追求：

1. 小切片
2. 真值一致
3. 审计可追溯
4. 恢复与治理内建
5. 文档与实现同步

## 4. 变更单元与切片规则

每个非平凡变更都必须先收敛为一个最小有用切片。一个切片至少要回答：

- 目标对象是什么，例如 `Run`、`ApprovalRequest`、`KnowledgeAsset`
- 所在平面是什么，例如 Runtime、Governance、Knowledge、Interaction
- 属于 `GA`、`Beta` 还是 `Later`
- 成功条件是什么
- 风险点是什么
- 当前仓库能验证到什么程度

切片规则：

- 一次只交付一个能独立验证的行为闭环。
- 不得把“领域对象调整 + UI 大改 + 技术栈迁移”混成单一任务。
- 若实现需要新增目录、服务、协议边界或新平台表面，必须证明这是当前最小切片，而不是提前铺路。

## 5. 仓库与目录规则

当前仓库的正式事实源是根文档与 `docs/`。

在未来实现树重新进入 tracked tree 前，默认规则如下：

- 不假设 `apps/`、`packages/`、`crates/`、workspace manifests、CI、测试树已经存在。
- 不为了“看起来完整”而恢复旧骨架。
- 非经明确任务要求，不创建与当前切片无关的目录树。

当未来需要新增实现目录时，必须遵守：

- 只创建完成当前切片所需的最小目录。
- 就近新增子树级 `AGENTS.md`，承接局部构建、测试、样式规则。
- 目录命名与分层要能映射到 `PRD/SAD` 中的正式平面与对象边界，而不是按临时实现习惯堆砌。

## 6. 命名与建模规范

### 6.1 术语一致性

实现、契约、计划和评审记录必须优先使用 `PRD/SAD` 已定义的正式术语：

- 使用 `Run`，不要在正式对象层改叫 `Job`
- 使用 `ApprovalRequest`，不要混用 `ReviewTicket`
- 使用 `KnowledgeAsset`、`KnowledgeCandidate`、`KnowledgeSpace`，不要临时发明同义词
- 使用 `CapabilityGrant`、`BudgetPolicy`、`ToolSearch` 等正式对象名，而不是 prompt 里的模糊别名

若必须引入新术语，必须说明它属于：

- 现有正式对象的别名
- 现有对象的子类型
- 新对象，并说明为何不能复用已有模型

### 6.2 状态机与事件

所有核心对象的实现设计都应显式建模状态、触发条件和恢复边界。

要求：

- 状态命名使用稳定枚举，不用自由文本
- 触发条件与越界条件要可追溯到 policy、grant、budget 或 external signal
- 幂等键、恢复点、撤销条件和失败分类必须在设计中明确
- 事件名称建议使用过去式语义，例如 `RunSubmitted`、`ApprovalGranted`、`KnowledgePromoted`

### 6.3 契约建模

所有公共接口、事件 envelope、持久化核心对象和跨平面读写关系都必须满足：

- 区分 authority source 与 projection/cache
- 显式声明 actor、scope、time、status、lineage
- 外部输入默认不可信
- 不允许用隐式 prompt 行为替代正式字段和治理链路

## 7. 领域约束检查项

凡涉及以下主题，设计和实现都必须显式检查对应约束：

### 7.1 Runtime

- `Run` 是否仍是权威执行壳
- 运行是否需要 checkpoint、resume、idempotency
- ToolSearch 是否只发现不授权

### 7.2 Governance

- 是否经过 `Role/Permission -> CapabilityGrant -> BudgetPolicy -> ApprovalRequest` 的约束链
- 是否新增了不可审计的越权路径
- 高风险动作是否仍有显式治理入口

### 7.3 Knowledge

- 是否经过 `candidate -> verified_shared -> promoted_org` 或受控子路径
- 是否保留 lineage、trust level、source owner
- 删除、降级和撤销是否有传播规则

### 7.4 Interaction

- Interaction Plane 是否只负责呈现、输入、解释和连续性
- UI 是否错误承担最终权限判断或事实写回
- 高风险状态、审批状态、离线缓存状态是否可见

## 8. 测试与验证规范

### 8.1 当前仓库的 truthful minimum

在当前 `doc-first rebuild` 阶段，最低验证集是：

- 确认所需文档存在
- 搜索并修复相关 stale references
- 审阅 focused diff
- 运行 `git diff --check`

在当前 tracked tree 未出现真实 manifests 与源码前，不得声称：

- `pnpm` / `cargo` / app runtime 成功
- 测试套件通过
- 真实界面或协议运行成功

### 8.2 未来实现阶段的最小验证思路

当实现树进入 tracked tree 后，应按变更类型补齐验证：

- 领域对象变更：状态机与 contract tests
- 治理变更：policy matrix tests、权限边界 tests
- 恢复变更：idempotency、resume、replay tests
- 知识变更：write gate、promotion、deletion propagation tests
- 交互面变更：状态呈现、页面语法和关键任务流 tests

## 9. 文档同步规则

任何非平凡变更都要检查以下文档是否需要同步：

- 改产品范围或 GA/Beta 切片：更新 `PRD`，且先获人工确认
- 改架构边界、平面职责、核心对象语义、技术主决策：更新 `SAD`
- 改开发流程、实现约束、命名、验证基线：更新本文档
- 改代码风格、实现分层、设计模式、错误处理或 code review 基线：更新 `CODING_STANDARD`
- 改 AI 执行方式和停机条件：更新 `AI_ENGINEERING_PLAYBOOK`
- 改 AI 标准作业程序、任务切片步骤或模板入口：更新 `AI_DEVELOPMENT_PROTOCOL`
- 改 GA 表面的页面语法、组件规则或视觉状态：更新 `VISUAL_FRAMEWORK`
- 改 ADR/contract/plan/review 流程：更新 `DELIVERY_GOVERNANCE`

禁止出现“代码已改，但文档以后再补”的默认做法。

## 10. 提交与评审粒度

每次提交或评审应尽量围绕单一切片：

- 一个问题定义
- 一组相关对象
- 一套明确验证

提交说明和评审摘要至少应写明：

- 影响了哪些正式对象
- 属于哪个平面
- 是否影响 GA/Beta 边界
- 做了哪些验证
- 还有哪些未决风险

## 11. 明确禁止事项

以下行为在 Octopus 仓库中默认禁止：

- 未经确认扩展产品范围或提前实现 Beta/Later 能力
- 把目标态架构描述成当前已实现现实
- 为了迎合 AI 生成习惯而发明未建模对象
- 把 ToolSearch、SkillPack、外部协议或 UI 控件当作绕过治理的旁路
- 跳过 acceptance condition 就宣称完成
- 在没有 tracked 事实的情况下补出整套 repo skeleton
- 只做表面 UI 而不补领域状态、治理边界和异常语义

## 12. 结论

Octopus 的工程规范不是为了增加流程负担，而是为了保证：

- 人和 AI 都在同一组对象边界上开发
- 每个切片都能被 truthful verification 支撑
- 视觉、治理、运行时和知识系统不会在实现阶段重新发散
