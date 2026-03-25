# M04 Agent、上下文与记忆实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-agent-context-and-memory.md`
- Objective: `冻结 Agent 资产、上下文工程和记忆系统的正式对象与验收边界。`

## Inputs

- `docs/PRD.md`
- `docs/SAD.md`
- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/API/AGENTS.md`

## Contracts To Freeze

- `AgentProfile`、身份、Prompt、Prompt 版本历史和测试对话边界。
- `Context Engine`、`WorkingMemory`、`ContextSnapshot`、compaction 与外部引用策略。
- `MemoryEntry`、来源追溯、手动维护与自动写回边界。
- Agent 与模型、工具、Skill、MCP 的绑定方式。

## Repo Reality

- Agent API 仍可能引用旧的 `model_config` 结构，实施本里程碑前必须先承接 `M2` 的冻结结果。
- 记忆存储和上下文压缩当前只有文档设计，没有实现级数据流。

## Deliverables

- Agent 正式对象清单。
- Context / Memory 数据流与来源追溯图。
- Prompt 与测试对话的验收矩阵。

## Verification

- 对 `AgentService`、`MemoryEntry`、`WorkingMemory`、`ContextSnapshot`、Prompt 版本字段和来源追溯字段做一致性 grep。
- 检查 `DOMAIN / DATA_MODEL / API/AGENTS` 三处的字段和不变量一致。
- 检查 `M5 / M6 / M7` 只消费冻结后的 Agent 合同。

## Docs Sync

- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/API/AGENTS.md`
- `docs/SAD.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-agent-context-and-memory.md`

## Open Risks

- Agent、Task、Discussion 都会消费上下文和记忆；若本里程碑未先冻结，后续三条链路会各自发散。

## Out Of Scope

- Team 与 Task 的执行编排。
- Discussion 模式和主持人调度。
