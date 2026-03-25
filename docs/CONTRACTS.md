# Octopus · 契约冻结说明（CONTRACTS）

**版本**: v1.1 | **状态**: Phase 1 冻结版 | **日期**: 2026-03-26
**对应文档**: PRD v2.1 · SAD v2.1

---

## 1. 文档目标

本文档把 PRD 与 SAD 中已经确定的核心对象、共享枚举、事件骨架与能力目录基线收敛为一套正式契约基线，供后续仓库骨架、Hub 实现、Client 模型和评测用例共同引用。

本文档的作用不是替代 PRD 或 SAD，而是回答两个具体问题：

1. 哪些对象已经进入正式契约集合。
2. 这些对象在实现阶段的机器可读来源位于哪里。

---

## 2. 正式契约源

当前正式契约源由以下文件组成：

| 文件 | 作用 |
| --- | --- |
| `contracts/v1/core-objects.json` | 冻结核心对象最小字段集合与所属边界上下文 |
| `contracts/v1/enums.json` | 冻结跨平面共享枚举值 |
| `contracts/v1/events.json` | 冻结事件骨架与最小 payload 字段 |
| `contracts/v1/capabilities.json` | 冻结 CapabilityCatalog schema 与首版 capability seed 集合 |
| `contracts/templates/capability-card.template.md` | 统一 capability card 模板，供中文文档与机器可读契约同步维护 |

规则：

- 机器可读契约源使用英文标识。
- `docs/CONTRACTS.md` 负责中文解释与维护规则。
- 若对象语义、枚举值、事件骨架或 capability card 基线变化，必须同步更新机器可读契约源、本文档以及相关 ADR。

---

## 3. 核心对象冻结范围

本阶段正式冻结以下对象：

- `HubConnection`
- `Workspace`
- `Project`
- `Agent`
- `Team`
- `Run`
- `Task`
- `Automation`
- `Trigger`
- `CapabilityGrant`
- `BudgetPolicy`
- `ApprovalRequest`
- `EnvironmentLease`
- `Artifact`
- `InboxItem`
- `KnowledgeSpace`
- `KnowledgeCandidate`
- `KnowledgeAsset`
- `DelegationGrant`
- `A2APeer`
- `ExternalAgentIdentity`
- `CapabilityDescriptor`
- `CapabilityBinding`
- `CapabilitySearchQuery`
- `CapabilitySearchResult`
- `InteractionPrompt`
- `MessageDraft`
- `ArtifactSessionState`
- `ConversationRecallRef`
- `AgentTemplate`
- `ExecutionProfile`
- `SkillPack`

约束：

- 字段命名、枚举值和事件名必须使用英文。
- 新对象若进入正式运行路径，不得只写在聊天记录中，必须先补入契约源。
- `Project` 只能附着 `KnowledgeSpace` 视图，不能替代 `KnowledgeSpace` 成为 Shared Knowledge 主属边界。
- `KnowledgeCandidate` 是共享知识写回的强制前置对象；Run、Automation、MCP 或其他外部结果不得直接绕过候选路径写入长期共享知识。
- `CapabilityDescriptor` 与 `CapabilityBinding` 是正式能力目录的唯一入口；顶层领域对象不得直接绑定具体第三方工具名。
- `ArtifactSessionState` 是 session-scoped 短期状态，不得被提升为长期 Knowledge、Run 恢复或 Hub 事实。

---

## 4. 共享枚举与事件骨架

本阶段正式冻结的共享枚举：

- `run_type`
- `run_status`
- `approval_type`
- `trigger_source`
- `sandbox_tier`
- `knowledge_status`
- `trust_level`
- `capability_kind`
- `capability_source`
- `capability_risk_level`
- `binding_subject_type`
- `binding_status`
- `interaction_prompt_kind`
- `message_draft_channel`
- `artifact_session_state_scope`
- `skill_pack_source`

本阶段正式冻结的事件骨架：

- `RunStateChanged`
- `PolicyDecisionRecorded`
- `ApprovalRequested`
- `ApprovalResolved`
- `KnowledgeCandidateCreated`
- `KnowledgeCandidatePromoted`
- `TriggerDelivered`
- `DelegationIssued`
- `CapabilityBound`
- `CapabilityBindingRevoked`
- `ToolSearchExecuted`
- `InteractionPromptIssued`
- `MessageDraftPrepared`
- `ConversationRecallRecorded`
- `ArtifactSessionStateCleared`
- `SkillPackResolved`

约束：

- 事件名表达“已发生事实”，禁止使用命令式命名。
- 事件最小字段集合不得在实现中静默缺失。
- 任何 transport 映射都不得改变正式事件名本身。
- `ToolSearchExecuted` 只记录搜索请求、可见结果与 policy snapshot，不得被解释为自动授权完成。

---

## 5. Capability Catalog 与模板要求

- `contracts/v1/capabilities.json` 中的 descriptor schema 至少要覆盖：`capability_id`、`kind`、`source`、`schema_ref`、`risk_level`、`platforms`、`default_visibility`、`fallback`、`observation_requirements`。
- 首版 capability seed 至少覆盖：
  - `octopus.capability.catalog`
  - `octopus.capability.tool_search`
  - `octopus.interaction.prompt`
  - `octopus.interaction.message_draft`
  - `octopus.memory.conversation_recall`
  - `octopus.artifact.session_state`
  - `octopus.skill.skill_pack`
  - `octopus.connector.mcp`
  - `octopus.connector.a2a`
- capability card 模板必须同时表达：用途、平台、风险级别、fallback、搜索暴露规则、观测要求、契约同步要求与不纳入核心领域对象的边界。
- `alarm`、`reminder`、`recipes`、`weather`、`places` 与 provider-specific connectors 默认不得进入首版 capability seed；若未来引入，只能以 adapter 或 connector-backed descriptor 形式登记。

---

## 6. 实施与校验规则

- 当 TypeScript 共享契约包和 Rust Hub 契约模块重新引入后，它们必须对齐 `contracts/v1/`。
- 未来若重新引入 `packages/contracts` 与 `crates/octopus-hub`，允许在冻结对象之上提供最小 transport mirror，例如 `RunDetailResponse`、`InboxListResponse`、`RuntimeEventEnvelope` 与 capability search transport envelope，但这些包裹不得扩展未冻结 Beta 对象或改变冻结对象语义。
- 契约校验至少覆盖：对象名、枚举值、事件名及其最小字段集合。
- 契约校验还必须覆盖：capability descriptor schema、首版 capability seed 唯一性、template 文件存在性与 JSON 语法有效性。
- 若仓库引入新的可执行验证链路，CI 必须将契约校验纳入默认检查。
- 目标态 transport、数据库表结构和 protobuf/OpenAPI 细节不在本阶段锁定，但不得违反这里冻结的对象、枚举、事件与 capability 语义。
