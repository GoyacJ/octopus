# Change Record: Contract And Repo Baseline

> Use this record for tracked milestone outcomes only. Task-level completion remains tracked in `docs/plans/2026-03-25-product-development-master-plan.md`.

- Change: `contract-and-repo-baseline`
- Status: `In Progress`
- Last Updated: `2026-03-25`
- Related Plan: `docs/plans/2026-03-25-m00-doc-truth-and-governance-repair.md`

## Summary

- 把旧的 `A-K` 主计划重构为 `M0-M10` 的“两层计划体系”，为每个正式里程碑新增可直接执行的实施计划文件。
- 同步 README、guardrails、PR 模板和当前 change 记录的治理入口，撤回“里程碑 B 已完成”的失真状态。
- 开始修复会误导后续实现的文档真相问题，包括旧命名、失效链接、错误 required-doc 依赖，以及“目标态蓝图冒充当前仓库事实”的表述。

## Scope

- In scope:
  - 主计划与 `M0-M10` 实施计划
  - `README.md`、`docs/plans/README.md`、`docs/changes/README.md`
  - `.github/workflows/guardrails.yml`、`.github/pull_request_template.md`
  - `docs/API/README.md`
  - `docs/ARCHITECTURE.md`、`docs/ENGINEERING_STANDARD.md`、`docs/DATA_MODEL.md`、`docs/DOMAIN.md`
- Out of scope:
  - 代码脚手架恢复、manifest 回归或运行时实现
  - 模型中心正式公共契约冻结
  - `pnpm`、`cargo`、Playwright 等实现级验证

## Risks

- Main risk:
  - 若未同步所有正式入口，AI 仍可能从旧里程碑、旧 required-doc 集合或旧命名继续推导错误实现顺序。
- Rollback or mitigation:
  - 以新的 `M0-M10` 主计划和 `M00` 实施计划为准，并通过 guardrails 的 required-doc 与 stale-reference 检查收敛正式入口。

## Verification

- Commands run:
  - `test -e` for `AGENTS.md`, `README.md`, current required docs, `docs/plans/2026-03-25-product-development-master-plan.md`, `docs/plans/2026-03-25-m00` to `m10`, `docs/changes/2026-03-25-contract-and-repo-baseline.md`, `.github/pull_request_template.md`
  - `grep -Rni "docs/DEVELOPMENT_STANDARDS.md" README.md AGENTS.md docs/plans/README.md docs/changes/README.md docs/plans/2026-03-25-product-development-master-plan.md docs/API/README.md .github/pull_request_template.md`
  - `grep -RniE "2026-03-24-v1-development-roadmap|2026-03-24-phase-(0-planning-and-tracking|1-contract-sources|2-monorepo-scaffolding|3-mvp-vertical-slice)|2026-03-25-phase-1-mvp-roadmap|2026-03-25-phase-0-planning-and-tracking" README.md AGENTS.md docs/plans/README.md docs/changes/README.md docs/plans/2026-03-25-product-development-master-plan.md docs/API/README.md .github/pull_request_template.md`
  - `grep -nE '^## 里程碑 [A-K]：' docs/plans/2026-03-25-product-development-master-plan.md`
  - `rg -n "shadcn-vue|Tailwind CSS|\\./PRD/PRD.md|Novai" README.md docs/API/README.md docs/ARCHITECTURE.md docs/ENGINEERING_STANDARD.md docs/DATA_MODEL.md docs/DOMAIN.md docs/API/AGENTS.md docs/API/MCP.md`
  - `for file in docs/plans/2026-03-25-m*.md; do grep -q '^## Inputs$' \"$file\"; grep -q '^## Contracts To Freeze$' \"$file\"; grep -q '^## Repo Reality$' \"$file\"; grep -q '^## Deliverables$' \"$file\"; grep -q '^## Verification$' \"$file\"; grep -q '^## Docs Sync$' \"$file\"; grep -q '^## Open Risks$' \"$file\"; grep -q '^## Out Of Scope$' \"$file\"; done`
  - `git diff --name-only -- README.md .github/pull_request_template.md .github/workflows/guardrails.yml docs/plans docs/changes docs/API/README.md docs/API/AGENTS.md docs/API/MCP.md docs/ARCHITECTURE.md docs/ENGINEERING_STANDARD.md docs/DATA_MODEL.md docs/DOMAIN.md`
- Manual checks:
  - 当前 change 记录已从错误的 `Done` 回收到 `In Progress`。
  - 当前主计划已切换到 `M0-M10`，旧 `A-K` 仅保留为历史映射附录。
  - `docs/API/MODELS.md` 不再被 README、API 导航或 guardrails 当作当前正式 required-doc。

## Docs Sync

- [x] `README.md`
- [ ] `AGENTS.md`
- [x] `.github/workflows/guardrails.yml`
- [ ] `docs/PRD.md`
- [ ] `docs/SAD.md`
- [x] `docs/ARCHITECTURE.md`
- [x] `docs/DOMAIN.md`
- [x] `docs/DATA_MODEL.md`
- [x] `docs/ENGINEERING_STANDARD.md`
- [x] `docs/API/`
- [ ] `docs/VIBECODING.md`
- [ ] `docs/VISUAL_FRAMEWORK.md`
- [ ] `docs/adr/`
- [x] `docs/plans/`
- [x] `docs/changes/`
- [ ] No doc update needed

## UI Evidence

- [x] Not applicable
- [ ] Light theme screenshot attached
- [ ] Dark theme screenshot attached
- [ ] zh-CN screenshot attached
- [ ] en-US screenshot attached

## Review Notes

- ADR or architecture impact:
  - 本轮不新增 ADR；优先收敛正式入口、依赖顺序和文档真相，再进入系统合同冻结。
- Security or policy impact:
  - 无新增安全能力；仅修正治理入口与验证边界。
- Contract or schema impact:
  - 模型中心、Agent API 和数据模型的正式冻结顺序已改为在 `M02` 中处理，当前不再把草案写成既成事实。
- Blocking reason:
  - None.
- Next action:
  - 完成 `M00` 剩余的文档真相收敛，再启动 `M01` 的系统合同冻结。
