# Octopus · AI 开发协议

**状态**: 基线建立版 | **日期**: 2026-03-26
**适用对象**: 所有进入本仓库执行文档、设计、契约、实现或评审任务的 AI Agent

---

## 1. 文档目标

本协议把 [`AI_ENGINEERING_PLAYBOOK.md`](./AI_ENGINEERING_PLAYBOOK.md) 中的原则性要求，固化为每次开发都应遵循的标准作业程序。

它回答的不是“为什么要这样开发”，而是“AI 每次进入 Octopus 仓库时，具体要怎么做”。

本协议不改变 [`AGENTS.md`](../AGENTS.md)、[`PRD.md`](./PRD.md)、[`SAD.md`](./SAD.md) 的优先级，只负责把已有约束落成固定开发流程。

补充说明：

- 若需要面向人类的可复制提示词示例，见 [`AI_PROMPT_HANDBOOK.md`](./AI_PROMPT_HANDBOOK.md)。
- 提示词手册是示例库，不是真相源，也不替代本协议。

## 2. 适用范围

以下任务默认适用本协议：

- 文档补齐
- 设计收敛
- contract / ADR / plan / review 编写
- 项目骨架设计与实现准备
- Runtime / Governance / Knowledge / Interop / Interaction 相关实现
- 代码评审与风险审查

对于 trivial 文本修订，可适度简化，但不得跳过真相源阅读与 truthful verification。

## 3. 协议总览

每次开发默认走以下九步：

1. 读取真相源
2. 分类请求
3. 填写任务切片卡
4. 判断前置产物
5. 获得必要批准
6. 实施最小切片
7. 执行可支撑验证
8. 同步相关文档
9. 按固定格式汇报

若其中任一步无法成立，AI 不得继续假设推进。

## 4. Step 1：读取真相源

开始任何非 trivial 工作前，至少读取：

1. [`README.md`](../README.md)
2. [`AGENTS.md`](../AGENTS.md)
3. [`PRD.md`](./PRD.md)
4. [`SAD.md`](./SAD.md)
5. [`ENGINEERING_STANDARD.md`](./ENGINEERING_STANDARD.md)
6. 若任务进入实现级规则：[`CODING_STANDARD.md`](./CODING_STANDARD.md)
7. [`AI_ENGINEERING_PLAYBOOK.md`](./AI_ENGINEERING_PLAYBOOK.md)
8. 根据任务类型再读取：
   - 交互/视觉任务：[`VISUAL_FRAMEWORK.md`](./VISUAL_FRAMEWORK.md)
   - 交付/评审/计划任务：[`DELIVERY_GOVERNANCE.md`](./DELIVERY_GOVERNANCE.md)

规则：

- 没读完真相源，不开始设计和实现。
- 已知结论必须建立在当前 tracked tree 上，而不是历史记忆或外部参考上。

## 5. Step 2：分类请求

AI 必须先把请求归入以下一种主类型：

- `doc`
- `design`
- `contract`
- `skeleton`
- `implementation`
- `review`

若一个请求跨多个类型，必须明确主类型和次类型。

随后，AI 必须把自然语言请求路由到对应模板或前置产物。默认规则如下：

| 自然语言请求特征 | 必选模板或前置产物 |
| --- | --- |
| 任何非 trivial 请求 | `task slice card` |
| 定义或修改正式对象、接口、事件、schema、状态机、跨平面协议 | `contract` |
| 涉及多步骤、多模块、多文档、多阶段实施 | `implementation plan` |
| 收敛 GA 页面、布局、信息架构、状态表达 | 交互/IA 设计说明，必要时补充视觉文档 |
| 设计 repo topology、workspace、最小目录骨架 | 骨架设计文档 |
| 纯措辞调整、错字修复、无语义变化的小修订 | 可跳过模板 |

规则补充：

- 人类不需要先手工填写模板；AI 应根据请求自行选择。
- 若同时命中多条规则，按 `task slice card -> contract -> implementation plan` 的顺序组合使用。
- 若 AI 无法判断该走哪条路由，应先停在分析阶段并说明缺的边界信息。

## 6. Step 3：填写任务切片卡

AI 在实施前必须先完成任务切片卡。

统一使用模板：

- [`docs/templates/task-slice-card.md`](./templates/task-slice-card.md)

任务切片卡至少应明确：

- 请求目标
- 影响的正式对象
- 所属平面
- 所属交互面
- `GA/Beta/Later` 归属
- 验收条件
- 可行验证
- 风险和停机点

若切片卡无法完整填写，说明任务定义仍不够清晰，应回到设计或人工确认。

默认要求：

- 即使用户只用自然语言描述目标，AI 也要先在内部或输出中形成切片卡结构。
- 若任务后续还要进入 contract 或 implementation plan，切片卡仍然是第一入口，而不是可跳过步骤。

## 7. Step 4：判断前置产物

AI 必须在动手前判断当前任务是否需要先产出以下产物：

| 场景 | 必要前置产物 |
| --- | --- |
| 需要在多个方案中取舍 | `ADR` |
| 定义公共对象/接口/事件/状态机 | `contract` |
| 涉及多步骤、多模块、多对象实现 | `implementation plan` |
| 新增或收敛 GA 页面语法 | 交互/IA 设计或视觉补充文档 |
| 新增项目骨架或 workspace 拓扑 | 骨架设计文档 |

统一模板：

- contract：[`docs/templates/contract-template.md`](./templates/contract-template.md)
- implementation plan：[`docs/templates/implementation-plan-template.md`](./templates/implementation-plan-template.md)

原则：

- 能先定义 contract，就不要直接跳代码。
- 能先定义 plan，就不要让 AI 边做边想。
- 人类不需要指定“请用哪个模板”；AI 应依据请求特征主动选择并说明原因。

## 8. Step 5：获得必要批准

出现以下情况，AI 必须先停下并等人类确认：

- 改变 `GA/Beta/Later` 边界
- 改变核心对象语义
- 引入新的平台表面
- 引入高风险能力或安全姿态变化
- 创建超出最小切片的大规模骨架
- 现有 acceptance condition 不成立

AI 可以做的，是把分歧、方案和风险讲清楚；不能直接替人类做边界决策。

## 9. Step 6：实施最小切片

一旦进入实施，必须遵守：

- 只做当前切片
- 不顺手扩出 Beta 或目标态能力
- 不把“骨架搭建”演化为“全量铺仓库”
- 不把 UI 美化当作领域完成
- 不把 prompt 技巧当作正式 capability runtime

实施顺序建议：

1. contract / design
2. 最小实现
3. 状态与异常路径补齐
4. 文档同步

## 10. Step 7：执行可支撑验证

AI 只能运行当前仓库真实支持的验证。

对当前 `doc-first rebuild` 仓库，最低验证集仍为：

- 确认新增文档存在
- 搜索相关 stale references
- 审阅 focused diff
- 运行 `git diff --check`

禁止声称：

- app 能运行
- `pnpm` / `cargo` 通过
- 测试通过

除非相关 tracked manifests、源码和测试树已经存在，并且确实运行过。

## 11. Step 8：同步相关文档

AI 在结束前必须检查是否需要同步：

- `README.md`
- `AGENTS.md`
- `PRD.md`
- `SAD.md`
- `ENGINEERING_STANDARD.md`
- `AI_ENGINEERING_PLAYBOOK.md`
- `VISUAL_FRAMEWORK.md`
- `DELIVERY_GOVERNANCE.md`

如果本次变更引入新的固定流程、模板或文档入口，就不能只新增文件而不更新导航文档。

## 12. Step 9：按固定格式汇报

AI 的交付汇报至少应包含：

1. 做了什么
2. 为什么这样做
3. 验证了什么
4. 还有哪些风险或未覆盖项

若使用了切片卡、contract 或 plan，应在汇报中指出其位置。

## 13. 停机条件

以下情况默认停止推进：

- 无法判断请求是否仍在 `GA`
- 无法指出影响对象和验收条件
- 需要新增大骨架但无明确边界
- 验证结果与预期冲突且无法自洽解释
- 发现当前文档之间存在直接冲突

停机后应输出：

- 卡住在哪里
- 为什么不能继续假设
- 需要人类确认什么

## 14. 推荐执行顺序

### 14.1 文档/规范任务

`read truth -> slice card -> draft doc -> sync navigation -> verify -> report`

### 14.2 contract 任务

`read truth -> slice card -> contract template -> review against PRD/SAD -> sync governance docs if needed -> verify -> report`

### 14.3 骨架任务

`read truth -> slice card -> skeleton design -> approval -> minimal skeleton -> verify -> report`

### 14.4 实现任务

`read truth -> slice card -> contract/plan if needed -> implement minimal slice -> verify -> sync docs -> report`

### 14.5 review 任务

`read truth -> classify scope -> inspect diff/artifacts -> findings first -> residual risks -> report`

## 15. 与其他文档的关系

- [`AI_ENGINEERING_PLAYBOOK.md`](./AI_ENGINEERING_PLAYBOOK.md) 定义原则、禁区和自检
- 本协议定义固定执行步骤
- [`DELIVERY_GOVERNANCE.md`](./DELIVERY_GOVERNANCE.md) 定义这些步骤涉及的文档路由与门禁
- 模板目录为协议提供可复用输入格式

## 16. 结论

Octopus 的 AI 开发协议是一套强制 SOP：

- 先定义
- 再切片
- 后实施
- 最后验证和同步

它的目标不是增加仪式感，而是让任何 AI 进入仓库后都按同一方式工作，减少偏航、返工和不一致。
