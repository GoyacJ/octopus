# M0 文档真相与治理修复实施计划

- Status: `In Progress`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/2026-03-25-contract-and-repo-baseline.md`
- Objective: `把正式执行入口切换到两层计划体系，并修复会误导后续 AI 实现的文档真相问题。`

## Inputs

- `AGENTS.md`
- `README.md`
- `docs/plans/README.md`
- `docs/changes/README.md`
- `docs/SAD.md`
- `docs/ARCHITECTURE.md`
- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/ENGINEERING_STANDARD.md`
- `docs/API/README.md`
- `.github/workflows/guardrails.yml`
- `.github/pull_request_template.md`

## Contracts To Freeze

- 正式执行入口采用“主计划导航 + 里程碑实施计划 + change 记录”三层职责分离。
- `doc-first` 当前事实与“目标态蓝图”必须显式区分，不得把未来目录、manifest 或运行时入口写成当前仓库事实。
- 正式 required-doc 集合不得包含尚未冻结的草案文档。
- 前端默认基线以 `AGENTS.md` 为准：`Vue 3 + TypeScript + Vite + Vue Router + Pinia + VueUse + UnoCSS + Vue I18n + self-built UI + shared design tokens + Tauri 2 / Tauri Mobile`。

## Repo Reality

- 当前跟踪的事实来源主要位于 `docs/` 与 `.github/`。
- 当前仓库没有已跟踪的 `Cargo.toml`、`package.json`、`apps/`、`crates/` 等脚手架入口。
- `docs/API/MODELS.md` 可能在本地工作区存在，但在 `M2` 冻结前不作为正式 required-doc。
- 当前工作区存在对 `docs/ARCHITECTURE.md` 的在途引用改动，执行本计划时只能合并式修改，不得覆盖。

## Deliverables

- 重写 `docs/plans/2026-03-25-product-development-master-plan.md` 为 `M0-M10` 体系。
- 为 `M0-M10` 新增独立实施计划文件，并固定模板字段。
- 同步 `README.md`、`docs/plans/README.md`、`.github/workflows/guardrails.yml`、`.github/pull_request_template.md` 和当前 change 记录。
- 修正核心源文档中的旧命名、失效链接、错误 required-doc 依赖和“目标态冒充现状”的表达。

## Verification

- `test -f` 检查主计划、`M0-M10` 对应的实施计划文件、当前 change 记录、README、guardrails 和 PR 模板。
- `! grep -Rni "docs/DEVELOPMENT_STANDARDS.md" README.md AGENTS.md docs .github/pull_request_template.md docs/changes/TEMPLATE.md`
- `! grep -nE "^## 里程碑 [A-K]：" docs/plans/2026-03-25-product-development-master-plan.md`
- `! grep -nE "^\\| [A-K] \\|" docs/plans/2026-03-25-product-development-master-plan.md`
- `git diff --stat -- README.md .github/workflows/guardrails.yml .github/pull_request_template.md docs/plans docs/changes docs/API/README.md docs/ARCHITECTURE.md docs/ENGINEERING_STANDARD.md docs/DATA_MODEL.md docs/DOMAIN.md`

## Docs Sync

- `README.md`
- `.github/workflows/guardrails.yml`
- `.github/pull_request_template.md`
- `docs/plans/README.md`
- `docs/changes/README.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/2026-03-25-contract-and-repo-baseline.md`
- `docs/ARCHITECTURE.md`
- `docs/ENGINEERING_STANDARD.md`
- `docs/DATA_MODEL.md`
- `docs/DOMAIN.md`
- `docs/API/README.md`

## Open Risks

- 深层 API 资源文档仍可能保留旧命名或旧合同，需要在对应里程碑启动时继续收敛。
- 本地未跟踪草案文件可能与正式基线并存，后续需要靠 guardrails 和实施计划边界继续约束。

## Out Of Scope

- 任何代码脚手架、manifest 或运行时实现。
- 模型中心正式公共契约冻结。
- 对所有源文档做一次性完全重写。
