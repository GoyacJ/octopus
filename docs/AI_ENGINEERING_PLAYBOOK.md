# Octopus · AI 工程执行手册

**状态**: 基线建立版 | **日期**: 2026-03-26
**适用对象**: Codex、Claude Code、兼容 coding agent

---

## 1. 文档目标

本手册专门约束 AI 在 Octopus 仓库中的工作方式，避免 AI 在 `AI-first 小团队` 场景下发生以下偏差：

- 无依据扩 scope
- 把目标态写成已实现现实
- 先写 UI 再补领域与治理
- 用 prompt 技巧替代正式运行时对象
- 在没有验证证据时宣称完成

本手册是实现阶段行为约束，不改变 [`PRD.md`](./PRD.md) 与 [`SAD.md`](./SAD.md) 的正式语义。

补充说明：

- 原则与禁区以本手册为准。
- 实现级代码风格、分层模式和 review 基线由 [`CODING_STANDARD.md`](./CODING_STANDARD.md) 约束。
- 每次实际执行时的固定步骤与模板入口由 [`AI_DEVELOPMENT_PROTOCOL.md`](./AI_DEVELOPMENT_PROTOCOL.md) 定义。
- 面向人类的常用提示词示例由 [`AI_PROMPT_HANDBOOK.md`](./AI_PROMPT_HANDBOOK.md) 提供。

## 2. AI 开工前的强制流程

AI 处理任何非平凡任务前，必须完成以下顺序：

1. 阅读 [`README.md`](../README.md)、[`AGENTS.md`](../AGENTS.md)、[`PRD.md`](./PRD.md)、[`SAD.md`](./SAD.md)
2. 判断请求属于哪类：
   - 文档补齐
   - 设计收敛
   - 实现准备
   - 运行时实现
   - 治理/知识/协议实现
   - 交互与视觉实现
   - 代码/文档评审
3. 判断是否仍在 `GA` 边界内
4. 写出最小切片理解：
   - 目标
   - 影响对象
   - 影响平面
   - 影响表面
   - 验收条件
   - 可执行验证
5. 再开始实施

如果第 3 或第 4 步无法明确，AI 必须先停下来，回到设计或人工确认，而不是直接实现。

任务切片卡与标准步骤模板见：

- [`docs/templates/task-slice-card.md`](./templates/task-slice-card.md)
- [`AI_DEVELOPMENT_PROTOCOL.md`](./AI_DEVELOPMENT_PROTOCOL.md)

## 3. AI 的默认工作模式

AI 在 Octopus 中默认采用以下行为准则：

- 先收敛边界，再动手
- 先确认正式对象，再讨论实现形式
- 先完成最小垂直切片，再考虑扩展
- 先确保治理链路成立，再做体验优化
- 先做 truthful verification，再输出完成结论

AI 不是自由创作助手；它是受 `PRD/SAD/AGENTS` 约束的实现代理。

## 4. 请求分类与对应动作

### 4.1 文档或设计类请求

若请求目标是补文档、补规范、收敛方案、评审边界：

- 优先输出文档、设计、约束和验收
- 不得借机扩展到未批准的实现范围
- 若需要新规范，优先放入对应治理文档，而不是污染 `PRD/SAD`

### 4.2 实现类请求

若请求目标是开始实现：

- 非 trivial 任务必须先形成明确实施方案或切片说明
- 必须能指出影响的正式对象和状态语义
- 必须说明验证路径

### 4.3 评审类请求

若请求是 review：

- 先找风险、回归、越界和缺测点
- 不先做风格建议
- 没有问题时也要说明剩余风险和验证盲区

## 5. 何时必须停下来请求人工确认

出现以下任一情况，AI 必须暂停并请求人工确认：

- 请求改变 `GA/Beta/Later` 边界
- 请求改变核心对象语义，例如 `Run`、`KnowledgeAsset`、`CapabilityGrant`
- 请求引入新的平台表面或高风险能力
- 请求引入新的高风险依赖、外部协议模式或安全姿态变化
- 现有文档不足以定义 acceptance condition
- 需要删除、重命名或替换重要主文档
- 需要创建大规模目录骨架，但当前 repo 并无 tracked 证明

## 6. AI 任务切片卡

AI 在执行前应能口头或书面给出如下“任务切片卡”：

- 请求目标
- 正式对象
- 所属平面
- 所属交互面
- 影响的治理链路
- 影响的知识链路
- 验收条件
- 验证方法
- 未决风险

若给不出这张切片卡，说明任务理解仍不合格。

推荐直接使用模板：

- [`docs/templates/task-slice-card.md`](./templates/task-slice-card.md)

## 7. 常见变更的专用检查单

### 7.1 MCP / Capability 相关变更

AI 必须同时检查：

- `CapabilityCatalog`
- `CapabilityBinding`
- `CapabilityResolver`
- `ToolSearch`
- `CapabilityGrant`
- `BudgetPolicy`
- `ApprovalRequest`
- `Audit / Trace`
- `Knowledge Write Gate`

禁止只做“工具接入”而忽略可见性、授权、审计和知识写回门控。

### 7.2 Knowledge 相关变更

AI 必须同时检查：

- 是否仍经过候选知识路径
- 是否保留 source、trust、owner、lineage
- 是否处理删除、降级、墓碑和传播
- 是否把外部结果错误当作系统事实

### 7.3 Governance 相关变更

AI 必须同时检查：

- `Role / Permission`
- `CapabilityGrant`
- `BudgetPolicy`
- `ApprovalRequest`
- `Policy Decision Log`
- 撤销、过期、恢复与阻断路径

### 7.4 UI / Interaction 相关变更

AI 必须同时检查：

- 是否仍符合 `Interaction Plane` 只负责呈现与连续性的边界
- 是否符合 [`VISUAL_FRAMEWORK.md`](./VISUAL_FRAMEWORK.md)
- 是否正确呈现 `approval-needed`、`blocked`、`degraded`、`offline-cache`、`policy-hit`
- 是否把 Notification 与 InboxItem 混成同一种事实

## 8. VibeCoding 常见偏差与防偏规则

### 8.1 偏差：先做界面，再补对象

纠偏：

- 先确认页面对应的正式对象、状态机和治理链路
- 再定义组件和视觉表现

### 8.2 偏差：凭经验补全 repo skeleton

纠偏：

- 只有 tracked tree 证明存在的目录与构建链路，才能被当作事实
- 不为了“完整感”补出未批准的基础设施

### 8.3 偏差：把 prompt 技巧当平台能力

纠偏：

- 任何 capability、interaction、message draft、skill 注入都必须能映射到正式对象
- 不能靠 system prompt 约定代替正式 runtime semantics

### 8.4 偏差：看似完成，实则无验收

纠偏：

- 输出前必须再次检查 acceptance condition
- 验证必须与当前 tracked repo 能力相匹配

### 8.5 偏差：静默带入 Beta 能力

纠偏：

- 一旦涉及 `A2A`、`DiscussionSession`、`ResidentAgentSession`、高阶 `Mesh`、`Org Knowledge Graph`、`Mobile`
- 默认视为越界风险，先回到 `PRD` 对照

## 9. AI 的输出要求

AI 交付结果至少应包含以下四项中的相关部分：

- 做了什么
- 为什么这样做
- 验证了什么
- 还有什么风险或未覆盖项

对于文档或实现说明，AI 应优先引用正式文档与修改文件，而不是泛泛解释。

## 10. 自检与事实核查

在声称完成前，AI 必须自检：

1. 有没有把目标态当已实现
2. 有没有超出用户批准范围
3. 有没有遗漏必须同步的文档
4. 有没有跳过治理对象或异常路径
5. 有没有使用当前仓库无法支撑的验证说法

当前仓库的 truthful minimum verification 仍以 [`AGENTS.md`](../AGENTS.md) 为准。

## 11. 推荐的交付记录模板

对于非 trivial 变更，建议 AI 用以下最小结构记录结果：

### 11.1 变更摘要

- 本次切片目标
- 影响对象 / 平面 / 表面

### 11.2 文档或实现更新

- 更新了哪些正式文档或文件
- 哪些约束被新增或澄清

### 11.3 验证记录

- 实际运行了什么检查
- 检查结果是什么

### 11.4 风险与后续

- 当前未覆盖项
- 需要人工决策的点

对于需要先计划再实施的任务，推荐使用：

- [`docs/templates/implementation-plan-template.md`](./templates/implementation-plan-template.md)

## 12. 结论

AI 在 Octopus 中的职责不是“尽快写出看起来合理的东西”，而是：

- 在正式对象边界内工作
- 在受控范围内推进最小切片
- 在可验证证据基础上完成交付

只有这样，`AI-first 小团队` 才不会退化成不可治理的 VibeCoding。
