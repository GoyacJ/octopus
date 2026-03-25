# M01 架构基础与运行时契约冻结实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-architecture-and-runtime-contract-freeze.md`
- Objective: `在进入身份、能力治理、Agent 与任务实现前，先冻结系统级职责边界、运行时对象、状态机和传输契约。`

## Inputs

- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/ENGINEERING_STANDARD.md`
- `docs/API/README.md`

## Contracts To Freeze

- `Control Plane / Runtime Kernel / Execution Plane / Surface` 的职责边界与“不承担职责”。
- 统一运行时对象：`Run`、`TriggerExecution`、`AskUserRequest`、`ApprovalRequest`。
- 统一事件模型：`EventEnvelope`、状态投影、恢复语义、`resume_token`、`idempotency_key`。
- 对外 API 传输语义与对内 `mTLS gRPC` / 插件 RPC 边界。
- 最小审计与时间线语义：`ReasoningSummary`、`ToolCallTrace`、`ExecutionTimeline`。

## Repo Reality

- 当前阶段只能冻结文档合同，不能假设已有 Rust runtime、node 协议实现或事件总线代码。
- `ARCHITECTURE.md` 中的目录与 crate 结构只能作为目标态蓝图引用，不能视作当前仓库事实。

## Deliverables

- 一份可执行的系统合同矩阵，明确每个子系统的职责、边界、输入/输出与禁止职责。
- 一份运行时对象与事件模型清单，明确创建、流转、恢复和审计字段。
- 一份传输契约清单，明确外部控制面、流式输出、恢复回调、节点协议和插件宿主协议的正式边界。

## Verification

- 检查上述合同在 `SAD / ARCHITECTURE / API/README` 三处命名一致。
- 检查每个系统合同都能映射到至少一个后续里程碑，不存在孤立设计项。
- 对 `resume_token`、`idempotency_key`、`EventEnvelope`、`ExecutionTimeline` 做 targeted consistency grep。

## Docs Sync

- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/API/README.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-architecture-and-runtime-contract-freeze.md`

## Open Risks

- `SAD` 与 `ARCHITECTURE` 的抽象层级不同，若不先定义“正式冻结字段”容易在实现阶段重复解释。

## Out Of Scope

- 身份、模型、工具、Agent 或 UI 具体实现。
- 任何技术选型以外的代码目录初始化。
