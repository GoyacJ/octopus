# Octopus · 交付治理规范

**状态**: 基线建立版 | **日期**: 2026-03-26
**适用范围**: 文档交付、实现交付、评审与上线门禁

---

## 1. 文档目标

本规范定义 Octopus 在 `doc-first rebuild` 与后续实现阶段的交付治理规则，确保：

- 不同类型的信息进入正确文档
- 非平凡变更有明确的决策记录与验收路径
- AI 与人类工程师都能遵循相同的 Ready / Done 标准

## 2. 文档类型矩阵

| 文档类型 | 作用 | 何时更新 | 不应承载什么 |
| --- | --- | --- | --- |
| `README.md` | 仓库状态、入口导航 | 入口变化、文档结构变化 | 产品细节、实现细则 |
| `AGENTS.md` | 仓库级 agent 约束 | truthfulness、协作规则变化 | 详细产品/架构设计 |
| `docs/PRD.md` | 产品范围、切片、验收意图 | 产品范围或 GA/Beta 边界变化 | 函数级实现细节 |
| `docs/SAD.md` | 架构边界、运行时约束、技术主决策 | 核心对象语义、平面职责、技术主决策变化 | 日常实现步骤 |
| `docs/ENGINEERING_STANDARD.md` | 工程实现规范 | 开发规则、命名、验证基线变化 | 产品范围决策 |
| `docs/CODING_STANDARD.md` | 实现级代码规范 | 代码风格、分层模式、错误处理、review 基线变化 | 产品边界或架构主语义 |
| `docs/AI_ENGINEERING_PLAYBOOK.md` | AI 行为手册 | AI 工作流、停机条件、自检方式变化 | 架构主决策 |
| `docs/AI_DEVELOPMENT_PROTOCOL.md` | AI 开发协议 | AI 固定 SOP、任务切片步骤、模板入口变化 | 产品或架构主边界 |
| `docs/VISUAL_FRAMEWORK.md` | GA 交互与视觉规则 | 页面语法、组件规则、状态表达变化 | 超出 GA 的产品扩张 |
| `docs/DELIVERY_GOVERNANCE.md` | 交付流程与门禁 | 文档流、ADR/contract/plan/review 流程变化 | 产品对象语义 |
| `docs/templates/*.md` | 模板资产 | 协议模板、contract 模板、plan 模板变化 | 真相源与正式决策 |
| `docs/adr/*.md` | 决策记录 | 存在重要替代方案或边界变化时 | 日常任务清单 |
| `docs/contracts/**/*.md` | 接口、对象、事件契约 | 新增/变更正式契约时 | 大段产品背景 |
| `docs/plans/*.md` | 实施计划 | 开始非 trivial 实现前 | 最终真相源 |
| `docs/review/*.md` | 评审记录与门禁结论 | 需要沉淀正式评审结论时 | 主规范文档 |

## 3. 变更路由规则

### 3.1 何时必须改 PRD

出现以下任一情况，必须先更新 `PRD` 并获得人工确认：

- 改变 `GA/Beta/Later` 切片
- 改变正式能力边界
- 增删核心用户价值主张
- 新增平台表面或交互主线

### 3.2 何时必须改 SAD

出现以下任一情况，必须更新 `SAD`：

- 改变核心对象语义
- 改变平面职责
- 改变 source of truth、恢复机制、治理链路
- 改变技术主选型或主要协议接入模式

### 3.3 何时必须改补充规范

- 改实现规则、命名、验证方式：更新 `ENGINEERING_STANDARD`
- 改代码风格、实现分层、语言/框架写法、错误处理与 review checklist：更新 `CODING_STANDARD`
- 改 AI 行为边界、停机条件、自检：更新 `AI_ENGINEERING_PLAYBOOK`
- 改 AI 固定 SOP、任务切片步骤或模板入口：更新 `AI_DEVELOPMENT_PROTOCOL`
- 改 GA 页面语法、组件层级、视觉状态语义：更新 `VISUAL_FRAMEWORK`
- 改文档流程、ADR/contract/plan/review 机制：更新本文档
- 改通用模板结构：同步更新 `docs/templates/`

## 4. ADR 规则

以下情况必须产出 ADR，而不是只改 `SAD`：

- 存在 2 个以上合理方案且需要取舍
- 决策会长期影响实现结构或团队工作方式
- 决策改变以下任一核心方向：
  - 运行时 authority 模型
  - 知识分层与写回路径
  - 治理链路
  - 互操作模式
  - 技术栈主决策
  - GA 交互壳层语法

建议命名：

- `docs/adr/YYYYMMDD-<slug>.md`

每份 ADR 至少包含：

- 背景
- 备选方案
- 决策
- 影响
- 风险

## 5. Contract 规则

以下情况必须补充或更新 contract 文档：

- 新增公共 API
- 新增或修改正式对象 schema
- 新增事件 envelope
- 新增状态机或恢复契约
- 新增跨平面交互协议

建议路径：

- `docs/contracts/runtime/`
- `docs/contracts/governance/`
- `docs/contracts/knowledge/`
- `docs/contracts/interaction/`
- `docs/contracts/interop/`

推荐从模板开始：

- [`docs/templates/contract-template.md`](./templates/contract-template.md)

当前目录尚未建立时，可先在计划中声明预期路径，再在实际需要时创建最小目录。

## 6. Plan 规则

以下情况建议先产出计划文档，再执行实现：

- 涉及多个文档或模块
- 影响多个正式对象
- 需要跨平面协调
- 需要多轮验证

建议命名：

- `docs/plans/YYYY-MM-DD-<topic>.md`

推荐从模板开始：

- [`docs/templates/implementation-plan-template.md`](./templates/implementation-plan-template.md)

计划必须至少包含：

- 目标
- 影响对象
- 关键步骤
- 验收
- 验证方式

## 7. Review 规则

以下情况应沉淀正式 review 记录：

- 重要文档评审
- 架构或治理变更
- 高风险能力启用前检查
- 大型交互面收敛

建议命名：

- `docs/review/YYYY-MM-DD-<topic>.md`

## 8. Definition of Ready

任务进入实施前，至少满足以下条件：

- 已完成任务切片卡
- 范围处于已批准边界内
- GA/Beta/Later 归属明确
- 目标对象明确
- 验收条件明确
- 所需上层真相文档已识别
- 当前仓库能支撑的验证方式明确
- 若涉及边界变化，已有人类确认

## 9. Definition of Done

任务声称完成前，至少满足以下条件：

- 变更已落在正确文档或实现位置
- 必要的主文档与补充规范已同步
- 若需要 ADR/contract/plan/review，已补齐或明确记录未执行原因
- 已完成当前仓库可支撑的验证
- 已审阅 focused diff
- 未留下与主文档冲突的表述
- 已说明剩余风险、未覆盖项或后续动作

## 10. 评审模板

任何非 trivial 交付都建议用以下清单评审：

- 这次变更影响了哪些正式对象
- 是否改变了 GA/Beta 边界
- 是否引入新的治理旁路
- 是否引入新的知识污染路径
- 是否改变了 Interaction Plane 的职责边界
- 是否需要 ADR 或 contract
- 当前验证是否与 tracked repo 能力匹配
- 是否存在 stale reference 或夸大实现状态

## 11. 上线与回归门禁

当前仓库处于文档优先阶段，因此门禁以文档与 truthful verification 为主：

- required docs 存在
- stale references 已检查
- focused diff 已审阅
- `git diff --check` 通过

未来当真实源码、manifests、tests 出现后，再按 `PRD/SAD` 的评测与门禁要求扩展为：

- contract tests
- state machine tests
- policy regression
- recovery regression
- UI state regression
- high-risk capability evaluation records

## 12. 角色职责

默认职责分工如下：

- 人类：范围定义、边界确认、架构例外、安全例外、最终验收
- AI：在已批准范围内实施、同步文档、执行可行验证、诚实报告剩余风险

若两者冲突，以人类批准边界为准。

## 13. 结论

Octopus 的交付治理目标不是增加流程表演，而是确保每次变更都能回答三件事：

1. 这件事属于哪个真相层
2. 这件事是否完成了正确的同步与记录
3. 这件事是否真的被当前仓库条件验证过
