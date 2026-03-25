# M07 Surface Shells 与治理型 UI 实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-surface-shells-and-governance-ui.md`
- Objective: `在核心运行时和治理合同冻结后，再定义 Web / Desktop / Mobile 壳与治理型页面边界。`

## Inputs

- `docs/VISUAL_FRAMEWORK.md`
- `docs/ENGINEERING_STANDARD.md`
- `docs/SAD.md`
- `docs/API/README.md`

## Contracts To Freeze

- Web / Desktop / Mobile 的表面职责与能力差异。
- 全局壳、导航、Hub/租户切换、主题、语言和治理型布局语法。
- 页面分层：views / components / stores / composables / transport。
- `zh-CN / en-US`、token、theme 与 approval/exception/timeline 等治理型视觉语义。

## Repo Reality

- 当前没有前端脚手架，实施本里程碑时只能先冻结页面边界与组件/状态分层，不得虚构可运行 UI。
- `AGENTS.md` 是当前正式前端基线的上位约束，优先于旧的 `Tailwind / shadcn` 叙述。

## Deliverables

- 表面职责矩阵。
- 导航与关键页面边界清单。
- 前端分层与 UI 基线合同。

## Verification

- 检查 `VISUAL_FRAMEWORK / ENGINEERING_STANDARD / SAD / API/README` 对页面职责和传输语义没有冲突。
- 检查前端基线与 `AGENTS.md` 完全一致。
- 检查所有用户可见文案与主题要求都在合同中有明确落点。

## Docs Sync

- `docs/VISUAL_FRAMEWORK.md`
- `docs/ENGINEERING_STANDARD.md`
- `docs/API/README.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-surface-shells-and-governance-ui.md`

## Open Risks

- 若在基础合同冻结前先展开页面设计，前端会被迫为未冻结的运行时与治理语义做临时决定。

## Out Of Scope

- 具体组件实现。
- 图形资源、品牌或营销页面设计。
