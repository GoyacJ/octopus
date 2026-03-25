# Change Record: Planning Governance Unification

> Use this record for tracked change outcomes only. Task-level completion remains tracked in `docs/plans/2026-03-25-product-development-master-plan.md`.

- Change: `planning-governance-unification`
- Status: `Done`
- Last Updated: `2026-03-25`
- Related Plan: `docs/plans/2026-03-25-product-development-master-plan.md`

## Summary

- 建立了 `docs/plans/2026-03-25-product-development-master-plan.md`，将 Octopus 的完整开发顺序、里程碑、退出条件和能力域映射收敛到单一入口。
- 建立了 `docs/changes/2026-03-25-planning-governance-unification.md`，用于承接本次规划治理统一、验证证据和文档同步结果。
- 清理了源文档与入口文档中的交付批次话术，并移除了对旧分阶段计划文件的正式引用。

## Scope

- In scope:
  - 替换旧 roadmap 为统一主计划
  - 替换旧 phase-specific change 记录为中性规划治理记录
  - 同步 `README.md`、`AGENTS.md`、目录规则、模板与 guardrails
  - 清理 `PRD`、`ARCHITECTURE`、`VIBECODING`、`VISUAL_FRAMEWORK`、`API/DISCUSSIONS` 中的交付批次表述
- Out of scope:
  - 契约与仓库基线的进一步收敛
  - 仓库骨架、Client、Hub 或运行时能力实现
  - 新增任何代码级功能或运行时验证栈

## Risks

- Main risk:
  - 若后续又把交付批次写回源文档，会破坏“源文档讲真相、主计划讲顺序”的治理边界。
- Rollback or mitigation:
  - 统一以 `docs/plans/2026-03-25-product-development-master-plan.md` 作为交付顺序入口，并通过 guardrails 和 stale-reference 检索阻止旧文件名回流。

## Verification

- Commands run:
  - `test -f docs/plans/2026-03-25-product-development-master-plan.md && test -f docs/changes/2026-03-25-planning-governance-unification.md && test ! -e docs/plans/2026-03-25-phase-1-mvp-roadmap.md && test ! -e docs/changes/2026-03-25-phase-0-planning-and-tracking.md`
  - `! grep -RniE '2026-03-25-phase-1-mvp-roadmap|2026-03-25-phase-0-planning-and-tracking' README.md AGENTS.md docs/PRD.md docs/ARCHITECTURE.md docs/VIBECODING.md docs/VISUAL_FRAMEWORK.md docs/API/DISCUSSIONS.md docs/plans/README.md docs/changes/README.md docs/changes/TEMPLATE.md`
  - `! grep -RniE 'Phase 1|Phase 2|MVP|Beta|首批|首版|P1 功能|\(P1\)' README.md AGENTS.md docs/PRD.md docs/ARCHITECTURE.md docs/VIBECODING.md docs/VISUAL_FRAMEWORK.md docs/API/DISCUSSIONS.md docs/plans/README.md docs/changes/README.md docs/changes/TEMPLATE.md`
  - `grep -nE 'Agent / Identity|Memory / Recall|Team / Leader|Task / Subtask|Discussion / Roundtable|Templates / Onboarding|Hub Management|Model Center|Auth / RBAC|Skills / Tools / MCP|Audit / Security|TeamGroup / 2FA' docs/plans/2026-03-25-product-development-master-plan.md`
  - `git diff --stat -- README.md AGENTS.md docs/PRD.md docs/ARCHITECTURE.md docs/VIBECODING.md docs/VISUAL_FRAMEWORK.md docs/API/DISCUSSIONS.md docs/plans/README.md docs/changes/README.md docs/changes/TEMPLATE.md .github/workflows/guardrails.yml docs/plans/2026-03-25-product-development-master-plan.md docs/changes/2026-03-25-planning-governance-unification.md docs/plans/2026-03-25-phase-1-mvp-roadmap.md docs/changes/2026-03-25-phase-0-planning-and-tracking.md`
- Manual checks:
  - 主计划覆盖 Agent、Memory、Team、Task、Discussion、Templates、Hub Management、Models、Auth/RBAC、Multi-Hub、Skills/Tools/MCP、Audit/Security、Advanced Capabilities 等能力域。
  - 入口文档统一指向新的主计划与治理变更记录，旧 phase-specific 文件不再作为正式入口出现。
  - 源文档继续表达产品、架构、界面和 API 真相，不再承担交付批次描述。

## Docs Sync

- [x] `README.md`
- [x] `AGENTS.md`
- [x] `.github/workflows/guardrails.yml`
- [x] `docs/PRD.md`
- [ ] `docs/SAD.md`
- [x] `docs/ARCHITECTURE.md`
- [ ] `docs/DOMAIN.md`
- [ ] `docs/DATA_MODEL.md`
- [ ] `docs/ENGINEERING_STANDARD.md`
- [x] `docs/API/`
- [x] `docs/VIBECODING.md`
- [x] `docs/VISUAL_FRAMEWORK.md`
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
  - 无新增架构决策；仅移除交付批次话术并统一规划治理入口。
- Security or policy impact:
  - 无新增安全或权限模型变更。
- Contract or schema impact:
  - 无接口字段或数据结构变更；仅调整文档表达方式和计划治理机制。
- Blocking reason:
  - None.
- Next action:
  - 按主计划推进 `里程碑 B · 契约与仓库基线`。
