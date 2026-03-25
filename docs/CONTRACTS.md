# Octopus · 契约冻结说明（CONTRACTS）

**版本**: v1.0 | **状态**: Phase 0 冻结版 | **日期**: 2026-03-25
**对应文档**: PRD v2.0 · SAD v2.0

---

## 1. 文档目标

本文档把 PRD 与 SAD 中已经确定的核心对象、共享枚举和事件骨架收敛为一套正式契约基线，供后续仓库骨架、Hub 实现、Client 模型和评测用例共同引用。

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

规则：

- 机器可读契约源使用英文标识。
- `docs/CONTRACTS.md` 负责中文解释与维护规则。
- 若对象语义、枚举值或事件骨架变化，必须同步更新机器可读契约源、本文档以及相关 ADR。

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

约束：

- 字段命名、枚举值和事件名必须使用英文。
- 新对象若进入正式运行路径，不得只写在聊天记录中，必须先补入契约源。
- `Project` 只能附着 `KnowledgeSpace` 视图，不能替代 `KnowledgeSpace` 成为 Shared Knowledge 主属边界。
- `KnowledgeCandidate` 是共享知识写回的强制前置对象；Run、Automation、MCP 或其他外部结果不得直接绕过候选路径写入长期共享知识。

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

本阶段正式冻结的事件骨架：

- `RunStateChanged`
- `PolicyDecisionRecorded`
- `ApprovalRequested`
- `ApprovalResolved`
- `KnowledgeCandidateCreated`
- `KnowledgeCandidatePromoted`
- `TriggerDelivered`
- `DelegationIssued`

约束：

- 事件名表达“已发生事实”，禁止使用命令式命名。
- 事件最小字段集合不得在实现中静默缺失。
- 任何 transport 映射都不得改变正式事件名本身。

---

## 5. 实施与校验规则

- TypeScript 共享契约包和 Rust Hub 契约模块必须对齐 `contracts/v1/`。
- 契约校验至少覆盖：对象名、枚举值、事件名及其最小字段集合。
- 若仓库引入新的可执行验证链路，CI 必须将契约校验纳入默认检查。
- 目标态 transport、数据库表结构和 protobuf/OpenAPI 细节不在本阶段锁定，但不得违反这里冻结的对象与枚举语义。
