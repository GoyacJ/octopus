# Octopus GA Rebuild Project Development Plan

> **For Claude:** REQUIRED SUB-SKILL: Use [`docs/AI_DEVELOPMENT_PROTOCOL.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/AI_DEVELOPMENT_PROTOCOL.md) before executing any phase below. Convert each phase into its own `task slice card`, then produce `contract` and `implementation plan` artifacts as needed.

**Goal:** 在不越过 `GA` 边界、不过度恢复旧仓库骨架的前提下，把 Octopus 从 `doc-first rebuild` 推进到可持续实施的 GA 重建顺序。

**Architecture:** 本计划不是排期表，也不是人力安排，而是面向 `AI-first 小团队` 的执行顺序计划。整体采用“骨架设计 -> contract -> 切片计划 -> 最小实现 -> 验证闭环”的推进方式，优先建立正式对象边界、治理链路和最小垂直切片，再扩展到 `MCP`、`Shared Knowledge` 与 `Automation`。

**Tech Stack:** 以 [`docs/SAD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md) 已确认的 `Tauri 2 + Rust + Vue 3 + TypeScript + Pinia + Tailwind/shadcn-vue` 为目标态技术方向；在对应 manifests 与源码实际进入 tracked tree 之前，不把这些实现形态描述成已存在现实。

---

## 1. 计划定位

本计划用于回答三件事：

1. 先做什么、后做什么
2. 每个阶段的产出物和退出条件是什么
3. 在当前仓库事实下，哪些工作应先停留在文档与 contract 层，哪些工作可以进入骨架和实现

本计划不包含：

- 时间排期
- 人力分配
- 里程碑日期
- 功能点估时

这些内容已被 [`docs/PRD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md) 明确排除在正式产品文档之外。

## 2. 计划约束

本计划必须持续遵守以下约束：

- 范围只覆盖 `GA`：`Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`
- 不提前把 `A2A`、`DiscussionSession`、`ResidentAgentSession`、高阶 `Mesh`、`Org Knowledge Graph`、`Mobile` 带入当前开发主线
- `Run` 必须保持权威执行壳
- `ToolSearch` 只发现能力，不自动授权
- 长期知识写入必须经过候选和治理门控
- 在当前 repo 无真实源码树前，不声称 runtime、测试链路或 app 已可运行

## 3. 当前基线

当前仓库已经具备的基线：

- `README / AGENTS / PRD / SAD` 真相源
- 工程、AI、视觉、交付治理规范
- AI 开发协议与模板
- AI 协作提示词手册

当前仍缺失的关键实施资产：

- 项目骨架设计
- `docs/contracts/` 下的正式契约
- `docs/plans/` 下的切片级实施计划
- 第一批真实实现骨架

## 4. 总体推进顺序

推荐总顺序如下：

1. 项目骨架设计
2. 核心 contract 定义
3. 首个 GA 垂直切片的视觉/IA 与实施计划
4. 最小代码骨架建立
5. 首个垂直切片实现
6. 后续 GA 切片按依赖顺序推进
7. GA 集成验证与门禁闭环

进入下一阶段前，必须满足上一阶段的退出条件。

## 5. Phase 1：项目骨架设计

### 目标

定义 Octopus 的最小可行 repo topology、workspace 边界、实现分层和代码入口规则，但不在此阶段恢复完整旧仓库骨架。

### 必要产出

- 一份项目骨架设计文档
- 如存在多种合理目录/分层方案，补一份 ADR
- 对未来顶层目录的最小决策，例如：
  - 哪些顶层目录会存在
  - 哪些目录现在明确不创建
  - 哪些目录需要就近 `AGENTS.md`

### 退出条件

- 可以明确说明未来实现的最小目录拓扑
- 不再存在“先把全骨架都搭起来”的模糊空间
- 人类确认骨架边界与最小化原则

## 6. Phase 2：GA 核心 contract

### 目标

先把首轮 GA 重建一定会用到的正式对象 contract 固定下来，避免进入实现后再反复改对象语义。

### 第一批 contract 建议

- `Run`
- `ApprovalRequest`
- `InboxItem`
- `TraceEvent`
- `CapabilityVisibility / ToolSearch`

### 必要产出

- `docs/contracts/` 下的第一批 contract 文档
- 每份 contract 均引用 `PRD/SAD` 对应章节
- 明确状态、actor、scope、治理链路、异常路径、验收条件

### 退出条件

- 首个垂直切片的关键对象都已有可引用 contract
- 对象命名和状态语义不再依赖聊天上下文解释

## 7. Phase 3：Slice A 计划与 IA 收敛

### 目标

为首个 GA 垂直切片建立可执行实施前置物。

### Slice A 定义

推荐首个垂直切片为：

`Chat 发起 task -> Run 执行 / 阻塞 -> ApprovalRequest -> Inbox 处理 -> Trace 可回放`

### 为什么先做这条

- 它覆盖 Runtime、Governance、Interaction 三个核心平面
- 它能验证 `Run`、审批、待办、审计是否真的形成闭环
- 它不依赖 `A2A`、高阶 `Mesh`、`Org Graph` 等 Beta 能力

### 必要产出

- Slice A 的 `task slice card`
- 必要的交互/IA 设计说明
- Slice A 的 `implementation plan`

### 退出条件

- 可以明确说出 Slice A 的边界、页面、对象和验收条件
- 没有“边做边想”的模糊实现空间

## 8. Phase 4：最小代码骨架建立

### 目标

基于已批准的骨架设计和 contract，只创建支持 Slice A 的最小 tracked 实现骨架。

### 原则

- 不提前为全部目标态能力铺代码结构
- 不为了完整感恢复旧 `apps/ packages/ crates/` 全量树
- 只创建首个切片必需的 manifests、目录和局部 `AGENTS.md`

### 必要产出

- 已批准的最小代码目录
- 最小构建入口
- 最小验证入口

### 退出条件

- 可以承载 Slice A 的实现与验证
- 没有明显与 `PRD/SAD` 冲突的结构性歧义

## 9. Phase 5：Slice A 实现

### 目标

实现首个 GA 垂直切片，证明：

- `Run` 是权威执行壳
- 审批是正式对象而不是前端临时交互
- `InboxItem` 是待处理事实而不是通知流
- `Trace` 能解释发生了什么

### 必要产出

- Slice A 对应实现
- 文档同步
- 当前仓库可支撑的验证记录

### 退出条件

- 用户能在单条主链路上看见 `Chat -> Run -> Approval -> Inbox -> Trace`
- 核心对象语义未漂移
- 治理链路可解释

## 10. Phase 6：Slice B 远程与上下文主线

### 目标

补齐 `Workspace / Project / HubConnection / Remote Hub` 的最小主线，为 GA 的 `Desktop + Remote Hub` 成立打基础。

### 推荐范围

- `HubConnection`
- `Workspace / Project` 上下文切换
- 本地与远程事实源差异表达
- 只读缓存与离线状态语义

### 退出条件

- 本地/远程模式边界清晰
- Client 与 Hub 的事实源关系可正确表达

## 11. Phase 7：Slice C Capability 与 MCP 基线

### 目标

建立 GA 所需的最小 capability runtime 与 `MCP` 基线，而不是先做大量 connector 集成。

### 推荐范围

- `CapabilityCatalog`
- `CapabilityBinding`
- `CapabilityResolver`
- `ToolSearch`
- 最小 `MCP` 接入基线

### 退出条件

- 可以解释为什么某能力可见、不可见或仅可搜索
- `MCP` 不绕过 grant、budget、approval 和 audit

## 12. Phase 8：Slice D Shared Knowledge 基线

### 目标

建立 `KnowledgeCandidate -> Shared Knowledge` 的 GA 基线，但不提前推进 `Org Knowledge Graph` 正式晋升。

### 推荐范围

- `KnowledgeCandidate`
- `Shared Knowledge`
- `KnowledgeSpace`
- `Knowledge Write Gate`
- `Lineage`

### 退出条件

- 长期知识写入已受控
- 候选、共享、撤销和 lineage 路径清晰

## 13. Phase 9：Slice E Automation 基线

### 目标

建立首版 GA 的 `Automation -> Trigger -> Run` 主线。

### 推荐范围

- `Automation`
- `Trigger`
- `run_type=automation`
- `cron / webhook / manual event / MCP event`
- 幂等与去重

### 退出条件

- 自动化不再是旁路逻辑，而是进入正式 Run 主线
- 重复投递、预算超限、权限撤销、对端离线等异常路径清晰

## 14. Phase 10：GA 集成门禁

### 目标

在多个切片完成后，建立最低 GA 集成判断，而不是只看单点功能可用。

### 必要检查

- 正式对象语义是否一致
- Runtime / Governance / Knowledge / Interaction 是否仍对齐
- 文档、contract、plan、实现是否同步
- 高风险路径是否可解释
- 当前仓库真实支持的验证是否都已执行

### 退出条件

- 可以开始讨论 GA 首轮可交付边界
- 仍未实现或未验证的能力被明确列出，而不是被暗示为已完成

## 15. 每阶段固定动作

每个 phase 都必须按以下顺序推进：

1. 先写 `task slice card`
2. 判断是否需要 `contract / ADR / implementation plan`
3. 由人类确认边界
4. 再进入实施
5. 实施后做 truthful verification
6. 最后同步文档并汇报下一步

## 16. 明确非范围

以下事项不进入本计划的当前开发主线：

- `A2A`
- `DiscussionSession`
- `ResidentAgentSession`
- 高阶 `Mesh`
- `Org Knowledge Graph` 正式晋升
- `Mobile`
- 面向公共 SaaS 控制面的扩展
- 与当前切片无关的大规模基础设施恢复

## 17. 立即下一步

本计划批准后，建议立即启动以下动作：

1. 产出“项目骨架设计”文档
2. 产出第一批 GA 核心 contract
3. 产出 Slice A 的 `task slice card`
4. 产出 Slice A 的 `implementation plan`

## 18. 结论

Octopus 当前最需要的不是一次性恢复完整仓库，而是建立一条可持续的 GA 重建主线。这个主线必须从骨架设计、正式 contract 和首个垂直切片开始，再逐步推进到 `Remote Hub`、`MCP`、`Shared Knowledge` 与 `Automation`。只有这样，AI-first 的推进方式才会稳定，而不是持续返工。
