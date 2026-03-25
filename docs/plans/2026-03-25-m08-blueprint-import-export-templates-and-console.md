# M08 Blueprint / Import-Export / Templates / Console 实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-blueprint-import-export-templates-and-console.md`
- Objective: `把 Blueprint、模板、导入导出、首次引导和管理控制台整合为统一的迁移与管理面里程碑。`

## Inputs

- `docs/SAD.md`
- `docs/PRD.md`
- `docs/VISUAL_FRAMEWORK.md`
- `docs/API/README.md`

## Contracts To Freeze

- `AgentBlueprintBundle` 的导出边界、导入预检与重绑规则。
- 模板体系：内置模板、私有模板、JSON 导入导出与治理边界。
- 首次引导、默认团队生成与控制台入口职责。
- Hub 控制台总览、租户/用户/配额/连接状态的管理面边界。

## Repo Reality

- 当前 Blueprint 和模板仍停留在架构与产品文档层，实施时需要依赖 `M1-M7` 已冻结的对象名与治理字段。

## Deliverables

- Blueprint / Template 合同矩阵。
- 管理控制台信息架构清单。
- 引导和导入导出的验收边界。

## Verification

- 检查 Blueprint 只迁移静态能力与策略，不携带运行历史、敏感数据或 secrets。
- 检查模板、引导和管理控制台没有重新定义 Agent、Model 或 Capability 合同。
- 检查 `VISUAL_FRAMEWORK` 与 `SAD` 对控制台入口职责一致。

## Docs Sync

- `docs/SAD.md`
- `docs/PRD.md`
- `docs/VISUAL_FRAMEWORK.md`
- `docs/API/README.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-blueprint-import-export-templates-and-console.md`

## Open Risks

- 若控制台与模板治理先于基础合同冻结，会把管理面变成新的契约来源，破坏源文档单一真相。

## Out Of Scope

- Marketplace。
- 正式社区模板生态。
