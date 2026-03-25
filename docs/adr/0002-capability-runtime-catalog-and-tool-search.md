# ADR 0002：Capability Runtime Catalog And Tool Search

- 状态：Accepted
- 日期：2026-03-26
- 关联文档：`docs/PRD.md` v2.1、`docs/SAD.md` v2.1、`docs/CONTRACTS.md` v1.1、`contracts/README.md`

## 背景

`octopus` 当前已回到 `doc-first rebuild` 状态，历史 `apps/`、`packages/`、`crates/` 与 workspace manifests 不再构成现状事实。与此同时，`docs/references/Claude_Hidden_Toolkit.md` 展示了一个成熟 Agent 产品在工具分层、结构化交互、记忆分层、artifact 执行层与 skill 注入上的可借鉴模式。

问题不在于是否“复刻 Claude 全部工具”，而在于如何吸收其运行时设计思想，同时保持 `octopus` 作为统一 Agent Runtime Platform 的定位与治理边界。

## 决策

1. 引入统一 `CapabilityCatalog`，并以 `CapabilityDescriptor` 与 `CapabilityBinding` 作为正式能力入口。
2. 引入 `CapabilityResolver`，统一基于 `platform`、connector 状态、`Workspace / Project` policy、`CapabilityGrant` 与 `BudgetPolicy` 求值能力可见性。
3. 引入 `ToolSearch` 作为 Hub 内部元能力，只允许发现当前主体可见的 deferred 或 connector-backed capabilities。搜索结果返回 schema、治理标签与 fallback，不等同于自动授权。
4. 把结构化提问和消息草稿正式建模为 `InteractionPrompt` 与 `MessageDraft`，并纳入 `Chat / Inbox / Approval` 语义。
5. 采用三层记忆/召回设计：`Agent Private Memory`、`ConversationRecallRef`、`KnowledgeCandidate -> KnowledgeAsset`。`ConversationRecallRef` 是 episodic recall，不等同于长期共享知识。
6. 保留 artifact execution layer 思路，但将 `ArtifactSessionState` 明确定义为 session-scoped 短期状态，默认不进入长期 Knowledge、Run checkpoint 或跨会话事实层。
7. 引入 `AgentTemplate`、`ExecutionProfile` 与 `SkillPack`，用于运行时即时注入规划、生成、验证与安全规则；`SkillPack` 不能绕过能力目录、授权、预算或审批。
8. `alarm`、`reminder`、`recipes`、`weather`、`places` 与 provider-specific connectors 不进入首版核心领域对象；若未来需要，只能以 adapter 或 connector-backed capability 形式接入。

## 结果

- `octopus` 不再把“工具是否存在”建模为 prompt 层偶然行为，而是建模为正式 capability runtime。
- `Project` 与 `Workspace` 可以影响能力可见性与搜索结果，不复制 Claude “Project 不影响工具行为”的假设。
- 结构化交互、artifact 会话态和记忆分层都获得正式契约位，可在后续实现中被测试、审计和治理。

## 代价与约束

- 契约层需要新增 `contracts/v1/capabilities.json`、capability card 模板以及相关对象/事件/枚举。
- 后续重建实现时，首条纵切片必须优先打通 `CapabilityCatalog -> Resolver -> ToolSearch -> InteractionPrompt/MessageDraft -> Audit`，而不是先堆具体消费级工具。
- 若未来引入新的 adapter 或 connector family，必须先更新 ADR、contracts 与正式文档，再进入实现阶段。
