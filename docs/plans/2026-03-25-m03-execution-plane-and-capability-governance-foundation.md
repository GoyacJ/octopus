# M03 执行面与能力治理基础实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-execution-plane-and-capability-governance-foundation.md`
- Objective: `把执行面和能力治理从产品功能里程碑中抽离为基础层合同，供 Agent、Task、Discussion 和 UI 统一复用。`

## Inputs

- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/DOMAIN.md`
- `docs/API/MCP.md`
- `docs/API/SKILLS_TOOLS.md`

## Contracts To Freeze

- `Built-in Tools` 分类、发现、加载、风险等级和 fallback 语义。
- `MCP Manager` 的注册、授权、allowlist/denylist、输出不可信默认值。
- `Plugin Host` 的受控 RPC、清单字段、capability grant 和隔离约束。
- `Node Runtime`、`PlatformToolProfile`、审批门和表面差异语义。

## Repo Reality

- 当前只有文档合同，没有插件宿主、MCP 客户端或节点协议实现。
- 能力治理项必须先冻结名称、字段和风险语义，再允许进入 Agent 和 UI 配置流。

## Deliverables

- 一份统一能力治理字段表。
- 一份执行面对象与授权边界图。
- 一份 fallback 与审批联动规则清单。

## Verification

- 检查 `SAD / ARCHITECTURE / API/SKILLS_TOOLS / API/MCP` 对工具、MCP 和插件的命名一致。
- 检查高风险能力都映射到审批、审计或表面限制，不存在“能执行但不可治理”的入口。
- 检查 `M4 / M5 / M6 / M7` 只引用冻结后的治理字段。

## Docs Sync

- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/DOMAIN.md`
- `docs/API/MCP.md`
- `docs/API/SKILLS_TOOLS.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-execution-plane-and-capability-governance-foundation.md`

## Open Risks

- 若 capability governance 继续散落在多个里程碑中，后续审批、UI 和审计都将各自定义一套语义。

## Out Of Scope

- 具体工具实现。
- 具体 MCP server、plugin 或 node runtime 的代码接入。
