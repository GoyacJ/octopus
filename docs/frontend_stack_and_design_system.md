# Octopus Desktop — Frontend Stack And Design System Governance

> **Purpose:** Define the tracked frontend baseline for the desktop app, the ownership boundary between `apps/desktop` and `packages/ui`, and the mandatory implementation rules for future UI work.
> **Scope:** Governance and documentation only. This document defines the steady-state frontend rules and verification bar after the migration program completed.
> **Last verified against repo state:** 2026-04-03

---

## 1. Governance Summary

The desktop application is standardized on:

- `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2`
- `@octopus/ui` as the only approved shared UI entrypoint for app code
- `Tailwind CSS + design tokens` as the only approved styling foundation
- **`Minimalist Refined Foundation`** (Notion-inspired) as the mandatory visual aesthetic:
  - **Whisper-quiet borders**: `border-border/40` or `dark:border-white/[0.08]`
  - **Apple-style easing**: `cubic-bezier(0.32, 0.72, 0, 1)`
  - **Hierarchical Typography**: Bold titles, tight tracking, uppercase tracked-out eyebrows
  - **Refined Surfaces**: Subtle shadows and background gradients via `UiSurface`

Future frontend work must treat this document and the root `AGENTS.md` as binding constraints:

- business pages consume `@octopus/ui`, not ad-hoc UI libraries
- missing shared components are added to `packages/ui` before page usage
- repeated hero, panel, metric, ranking, timeline, toolbar, and nav-card visuals move into `packages/ui` before reuse
- accessibility-sensitive overlays and composite widgets standardize on `Reka UI`
- data entry standardizes on `vee-validate + zod`
- structured data display standardizes on `@tanstack/vue-table` and `@tanstack/vue-virtual`
- editor-like text surfaces default to `CodeMirror`
- desktop behaviors prefer Tauri native capabilities
- page-level hero, action, and info cards should prefer shared `UiPageHero`, `UiActionCard`, and `UiInfoCard` before adding new page-local variants

---

## 2. Current Repo Audit

### 2.1 App Entry And Frontend Baseline

The tracked desktop app entry is `apps/desktop`.

Current repo evidence:

- `apps/desktop/package.json` declares `vue`, `vite`, `pinia`, `vue-router`, `vue-i18n`, and `@tauri-apps/api`
- `apps/desktop/src/main.ts` bootstraps `createPinia()`, `i18n`, `router`, and imports `@octopus/ui/main.css`
- `apps/desktop/vite.config.ts` configures the Vue plugin and aliases `@octopus/schema` and `@octopus/ui`
- `apps/desktop/src-tauri/tauri.conf.json` and `apps/desktop/src-tauri/src/lib.rs` confirm the desktop shell is running on Tauri 2

### 2.2 Shared UI Package State

`packages/ui` is now the enforced shared design-system package and the only approved UI entrypoint for app code.

Current repo evidence:

- `packages/ui/package.json` exports `.`, `./main.css`, and `./tokens.css` and declares the runtime dependencies used by shared primitives, including `reka-ui`, `@tanstack/vue-table`, `@tanstack/vue-virtual`, `@iconify/vue`, `@rive-app/canvas`, and `@lottiefiles/dotlottie-vue`
- `packages/ui/src/index.ts` exposes the shared surface used across the desktop app, including:
  - base controls: `UiButton`, `UiInput`, `UiTextarea`, `UiCheckbox`, `UiSwitch`, `UiSelect`, `UiRadioGroup`, `UiSectionHeading`
  - overlay and navigation: `UiDialog`, `UiPopover`, `UiDropdownMenu`, `UiCombobox`, `UiTabs`, `UiAccordion`, `UiContextMenu`, `UiSelectionMenu`
  - page and shell abstractions: `UiPageHero`, `UiPanelFrame`, `UiToolbarRow`, `UiNavCardList`, `UiSurface`
  - data display: `UiBadge`, `UiEmptyState`, `UiMetricCard`, `UiRankingList`, `UiTimelineList`, `UiRecordCard`, `UiListRow`, `UiStatTile`, `UiPagination`, `UiDataTable`, `UiVirtualList`
  - context-specific blocks: `UiArtifactBlock`, `UiTraceBlock`, `UiInboxBlock`
  - media and editor: `UiCodeEditor`, `UiIcon`, `UiDotLottie`, `UiRiveCanvas`

Conclusion:

- `@octopus/ui` is stable as the package boundary for shared frontend UI
- design tokens, shared shell abstractions, and shared interaction primitives are no longer migration-era gaps
- remaining frontend debt is now about steady-state correctness, accessibility, and contract drift rather than export-surface expansion

### 2.3 Accessibility-Sensitive Components Are Standardized On `Reka UI`

The desktop app now routes shared overlays and composite interactions through `Reka UI`-backed primitives in `packages/ui`.

Current repo evidence:

- `UiDialog`, `UiPopover`, `UiDropdownMenu`, `UiCombobox`, `UiTabs`, `UiAccordion`, and `UiContextMenu` are all implemented in `packages/ui/src/components/*`
- business pages consume those wrappers through `@octopus/ui` instead of importing `reka-ui` directly
- current closure work focuses on keeping those wrappers type-safe and a11y-clean, not replacing hand-rolled modal/menu systems page by page

Governance decision:

- no new accessibility-sensitive composite widget may be hand-rolled in business code
- any new overlay or navigation primitive must land in `packages/ui` and satisfy the same shared accessibility contract

### 2.4 Business Pages Now Consume Shared UI Abstractions By Default

The broad migration phase is complete. Business views now compose `@octopus/ui` primitives and page abstractions instead of maintaining page-local visual systems.

Current repo evidence:

- the five migration batches retired the previous hero, panel, metric, ranking, timeline, toolbar, nav-card, modal-card, record-card, and tag-chip variants from business pages
- `scripts/check-frontend-governance.mjs` now enforces the steady-state rules for business surfaces:
  - no deep imports from `packages/ui/src/components/*`
  - no unapproved UI libraries in app code
  - no oversized scoped-style debt outside an explicit allowlist
  - no reusable visual-pattern class names in business surfaces
  - no native form controls in non-allowlisted business surfaces
- `legacyAllowlist` is empty, so new work starts from zero migration debt by default

Governance decision:

- page-level features use `Ui*` abstractions backed by shared tokens and Tailwind utilities
- if the shared abstraction is missing, it must be created in `packages/ui` first

### 2.5 Current Verification State

Observed on 2026-04-03 from local verification:

- `pnpm check:frontend-governance` passes
- `pnpm -C apps/desktop test` passes across the tracked desktop suite
- `pnpm -C apps/desktop typecheck` and `pnpm -C apps/desktop build` are part of the steady-state frontend closure gate

Current governance implication:

- broad UI migration is no longer blocked
- the remaining bar for frontend quality is now quiet-green verification: governance, typecheck, tests, and build should all pass without accessibility warnings or schema drift

---

## 3. Adoption Matrix

The following matrix records the governance target against tracked repo state as of 2026-04-03.

| Capability | Status | Current repo state |
|---|---|---|
| `Vue 3` | Already present | Declared in `apps/desktop/package.json` and used in `apps/desktop/src/main.ts`. |
| `Vite` | Already present | Declared in `apps/desktop/package.json` and configured in `apps/desktop/vite.config.ts`. |
| `Pinia` | Already present | Declared and wired through `createPinia()`. |
| `Vue Router` | Already present | Declared and wired through `router`. |
| `Vue I18n` | Already present | Declared and wired through `plugins/i18n.ts`. |
| `Tauri 2` | Already present | Declared through `@tauri-apps/api` and `src-tauri` config/code. |
| `@octopus/ui` | Already present | Shared package boundary, public export surface, and package dependency ownership are now in place for app-facing `Ui*` primitives. |
| `Tailwind CSS + tokens` | Already present | Root and desktop Tailwind configs exist; `@octopus/ui` ships `main.css` and `tokens.css`. |
| `Reka UI` for composite widgets | Already present | Shared dialog, popover, dropdown, combobox, tabs, accordion, and context menu primitives are standardized through `packages/ui`. |
| `vee-validate + zod` | Not yet adopted | Not declared in tracked app or UI package manifests, and page forms still use raw controls. |
| `@tanstack/vue-table` | Already present | `UiDataTable.vue` uses it and `packages/ui/package.json` declares the dependency. |
| `@tanstack/vue-virtual` | Already present | `UiVirtualList.vue` uses it and `packages/ui/package.json` declares the dependency. |
| `CodeMirror` | Partially present | `UiCodeEditor.vue` exists, but it is currently a textarea wrapper rather than a CodeMirror-based editor. |
| `Monaco` only for IDE-class use | Not yet adopted | No tracked Monaco integration is currently in use. |
| `lucide-vue-next` | Already present | Declared in `apps/desktop/package.json` and used in app views/components. |
| `Iconify + unplugin-icons` | Partially present | `UiIcon.vue` and package ownership are in place; `unplugin-icons` remains optional until a concrete app-side need appears. |
| `motion-v` | Not yet adopted | No tracked usage in app or UI source. |
| `AutoAnimate` | Not yet adopted | No tracked usage in app or UI source. |
| `GSAP` | Not yet adopted | No tracked usage in app or UI source. |
| `Rive` | Already present | `UiRiveCanvas.vue` exists and `packages/ui/package.json` owns the dependency. |
| `dotLottie` | Already present | `UiDotLottie.vue` exists and `packages/ui/package.json` owns the dependency. |
| `unDraw` | Not yet adopted | No tracked illustration integration in app or UI source. |

Status definitions:

- **Already present:** tracked manifests and source align with the governance target
- **Partially present:** some source or package boundary exists, but exports, ownership, or standardization are incomplete
- **Not yet adopted:** the capability is not yet established in tracked source/manifests

---

## 4. Dependency Ownership Matrix

Use this ownership rule whenever adding or relocating frontend dependencies.

| Dependency class | Owner | Examples | Rule |
|---|---|---|---|
| App shell and runtime framework | `apps/desktop` | `vue`, `vite`, `pinia`, `vue-router`, `vue-i18n`, `@tauri-apps/api`, `lucide-vue-next` | Anything tied to app bootstrap, routing, state, or host/runtime integration belongs in the app package. |
| Shared UI primitives and composite interactions | `packages/ui` | `reka-ui`, `@tanstack/vue-table`, `@tanstack/vue-virtual`, `vee-validate`, `zod`, `@iconify/vue`, `@codemirror/*` | If a dependency powers reusable `Ui*` components, it belongs in the shared UI package. |
| Shared motion and media adapters used by `Ui*` components | `packages/ui` | `motion-v`, `@formkit/auto-animate`, `gsap`, `@rive-app/canvas`, `@lottiefiles/dotlottie-vue` | Animation/media packages belong in `packages/ui` when wrapped as shared components or directives. |
| Page-only feature code | `apps/desktop` | view models, view-specific data helpers, route-specific composition utilities | Business logic can stay in the app package, but should compose shared UI rather than define a competing UI system. |
| Shared schema contracts | `packages/schema` | workbench, runtime, shell, RBAC, catalog, knowledge contracts | Shared contracts must be organized by feature file under `packages/schema/src/*`; `src/index.ts` stays a barrel export surface. |

Rule:

- `shadcn-vue` is a reference for structure and interaction semantics only
- business pages must not import `shadcn-vue` components directly
- if a `shadcn-vue` pattern is adopted, it is reimplemented or wrapped inside `@octopus/ui`

---

## 5. `@octopus/ui` Public Surface Rules

`@octopus/ui` is the only approved shared UI entrypoint for application code.

Mandatory rules:

- all shared components must be exported through `packages/ui/src/index.ts`
- if a stable subpath export is needed, it must be declared explicitly in `packages/ui/package.json`
- app code consumes `@octopus/ui`, not `packages/ui/src/components/*`
- package dependencies imported by `Ui*` components must be declared in `packages/ui/package.json`
- tests for shared UI should import through the same public surface used by the app
- shared visual shells and repeated surface patterns belong in `@octopus/ui`, including:
  - `UiPanelFrame`
  - `UiMetricCard`
  - `UiRankingList`
  - `UiTimelineList`
  - `UiToolbarRow`
  - `UiNavCardList`

Why this matters:

- public surface drift turns design-system reuse into a regression risk, so stable exports remain mandatory even after migration closure
- public surface discipline is required before design-system reuse can scale safely

---

## 5.1 Page Styling Rules

Business pages may keep `<style scoped>` blocks, but only for:

- page layout and responsive grid orchestration
- view-local spacing adjustments
- small compatibility patches that do not define a reusable visual language
- chart, illustration, or visualization-specific drawing details that are local to a single surface

Business pages must not define reusable visual component styles for:

- hero or overview shells
- metric, stat, action, or info cards
- ranking, timeline, toolbar, nav-card, and modal-card patterns
- shared modal, menu, dropdown, popover, tab, or accordion appearances
- generic primary or secondary button looks

Required page-layer defaults:

- prefer `@octopus/ui` primitives and page abstractions first
- prefer Tailwind utilities plus tokens over large named style clusters
- if a visual block repeats across more than one screen, move it into `packages/ui`
- if a new interaction primitive is needed, add it to `@octopus/ui` before app usage
- feature-local components may exist, but they may only compose shared `Ui*` primitives and must not become a second visual system
- new shared visual abstractions must land in `packages/ui` before page adoption

First batch migrated toward this rule set:

- `apps/desktop/src/components/layout/ConversationTabsBar.vue`
- `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- `apps/desktop/src/views/project/ProjectDashboardView.vue`
- `apps/desktop/src/views/workspace/WorkspaceOverviewView.vue`
- `apps/desktop/src/views/project/ProjectKnowledgeView.vue`
- `apps/desktop/src/views/workspace/UserCenterView.vue`

Second batch migrated the conversation workbench core path into the same shared skeleton:

- `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- `apps/desktop/src/components/layout/ConversationContextPane.vue`
- `apps/desktop/src/views/project/ConversationView.vue`
- `apps/desktop/src/views/project/ProjectResourcesView.vue`

Third batch migrated the management-panel family into the same shared skeleton:

- `apps/desktop/src/views/workspace/user/UserCenterPermissionsView.vue`
- `apps/desktop/src/views/workspace/user/UserCenterRolesView.vue`
- `apps/desktop/src/views/workspace/user/UserCenterUsersView.vue`
- `apps/desktop/src/views/workspace/user/UserCenterMenusView.vue`
- `apps/desktop/src/views/workspace/user/UserCenterProfileView.vue`
- `apps/desktop/src/views/workspace/ToolsView.vue`
- `apps/desktop/src/views/app/SettingsView.vue`
- `apps/desktop/src/views/app/ConnectionsView.vue`
- `apps/desktop/src/views/workspace/AutomationsView.vue`
- `apps/desktop/src/views/workspace/TeamsView.vue`

This batch standardized management screens on shared `UiMetricCard`, `UiToolbarRow`, `UiTabs`, `UiRecordCard`, `UiListRow`, `UiTimelineList`, and shared `UiField` form primitives. Local `metric-card`, `toolbar`, `card`, `editor-shell`, `binding-panel`, and `timeline` systems were removed instead of being preserved page-by-page.

Fourth batch migrated the Agents family into the same shared skeleton:

- `apps/desktop/src/views/workspace/WorkspaceAgentsView.vue`
- `apps/desktop/src/views/project/ProjectAgentsView.vue`
- `apps/desktop/src/views/agents/AgentsFilterBar.vue`
- `apps/desktop/src/views/agents/AgentEmployeeCard.vue`
- `apps/desktop/src/views/agents/TeamUnitCard.vue`
- `apps/desktop/src/views/agents/AgentsRecommendations.vue`
- `apps/desktop/src/views/agents/AgentsHeroSection.vue`
- `apps/desktop/src/views/agents/AgentsEmptyState.vue`

This batch standardized the agent center on shared `UiPageHero`, `UiToolbarRow`, `UiFilterChipGroup`, `UiRecordCard`, `UiListRow`, and `UiDialog` shells. Local hero, toolbar, tag-chip, record-card, recommendation-panel, table-shell, and dialog-shell systems were removed instead of being preserved as feature-local styling debt.

Final tail-cleanup batch retired the last dashboard-family allowlist debt:

- `apps/desktop/src/views/project/ProjectDashboardView.vue`
- `apps/desktop/src/views/workspace/WorkspaceOverviewView.vue`
- `apps/desktop/src/views/project/ProjectKnowledgeView.vue`

This batch removed the remaining page-local progress, trend, toolbar-search, and panel-heading visual systems from the dashboard family. Those surfaces now compose shared `UiPageHero`, `UiPanelFrame`, `UiToolbarRow`, `UiFilterChipGroup`, `UiMetricCard`, `UiRankingList`, `UiTimelineList`, `UiNavCardList`, and `UiSurface` primitives with layout-only page orchestration.

Current governance status after the five migration batches:

- `legacyAllowlist` is empty
- dashboard / overview / knowledge no longer rely on scoped-style escape hatches
- frontend style governance has moved from migration mode to steady-state enforcement for new work

---

## 5.2 Governance Checks

Use the root commands below as the steady-state frontend gate:

- `pnpm check:frontend-governance`
- `pnpm check:frontend`

Current automated checks cover:

- forbid deep imports from `packages/ui/src/components/*`
- forbid business code from importing disallowed UI libraries directly
- hard-fail new business views and layout panes that keep oversized scoped-style debt without an explicit allowlist entry
- freeze scoped-style debt for any temporary allowlisted legacy files so style-line counts cannot grow
- forbid reusable visual-pattern class names in non-allowlisted business surfaces
- forbid native form controls in non-allowlisted business surfaces
- keep the allowlist as a temporary debt register that should remain empty by default and only be used for explicitly approved migrations

`pnpm check:frontend-governance` is the style-and-boundary backstop. `pnpm check:frontend` is the steady-state verification gate that combines governance, typecheck, and tests. The allowlist remains a temporary migration ledger, not a permanent escape hatch.

---

## 6. Component Taxonomy

Shared UI work in `packages/ui` should be grouped by capability rather than by page.

### 6.1 Base Controls

Examples:

- `UiButton`
- `UiInput`
- `UiTextarea`
- `UiCheckbox`
- `UiSwitch`
- `UiSelect`
- `UiRadioGroup`
- `UiSectionHeading`

Rules:

- expose consistent variants, sizes, disabled/loading states, and token usage
- do not duplicate button/input styling in business pages
- `UiSectionHeading` should be used for internal surface grouping

### 6.2 Form Wrappers And Validation Adapters

Examples:

- `UiField`
- `UiFormField`
- `UiFormMessage`
- `UiFormActions`

Future target:

- `vee-validate + zod` adapters should live here
- form pages should compose shared field, message, and action components rather than raw labels plus native controls

### 6.3 Overlay And Navigation Primitives

Examples:

- `UiDialog`
- `UiPopover`
- `UiDropdownMenu`
- `UiCombobox`
- `UiTabs`
- `UiAccordion`
- `UiContextMenu`
- `UiSelectionMenu`

Rules:

- these components are already standardized through `Reka UI` wrappers in `packages/ui`
- keep shared overlay contracts quiet-green: no accessibility warnings in tests, no page-local dialog/menu systems, and no direct business-page imports from `reka-ui`

### 6.4 Data Display Primitives

Examples:

- `UiSurface`
- `UiBadge`
- `UiEmptyState`
- `UiListRow`
- `UiPagination`
- `UiDataTable`
- `UiVirtualList`
- `UiStatTile`
- `UiTraceBlock`

Rules:

- table and large-list experiences should converge on the TanStack wrappers owned by `packages/ui`
- page-level list shells should compose these primitives instead of redefining row/table states locally
- `UiSurface` is the foundation for all "raised" or "panel" visual containers

### 6.5 Context-Specific Blocks

Examples:

- `UiArtifactBlock`
- `UiTraceBlock`
- `UiInboxBlock`
- `UiRecordCard`

Rules:

- use these for specific domain concepts (artifacts, logs, notifications, records) to maintain visual consistency across different views
- these blocks must follow the `Minimalist Refined` aesthetic (whisper borders, tight typography)

### 6.6 Editors, Media, And Icons

Examples:

- `UiCodeEditor`
- `UiIcon`
- `UiDotLottie`
- `UiRiveCanvas`

Rules:

- `UiCodeEditor` should converge on `CodeMirror` for config and prompt editing
- `UiIcon` should standardize the default split between `lucide-vue-next` and `Iconify`
- motion and media wrappers belong in shared UI if they are reused across more than one view

---

## 7. Phased Adoption Policy

Adopt this stack in layers. Do not install everything at once without an implementation need.

### Phase 1: Core Baseline

Adopt first:

- `Reka UI`
- `vee-validate`
- `zod`
- `@tanstack/vue-table`
- `@tanstack/vue-virtual`
- `CodeMirror`
- `Iconify + unplugin-icons`

Phase 1 goal:

- make the design-system foundation correct, accessible, and stable
- establish dependency ownership and public export discipline
- remove the need for raw page-level controls in future migration work

### Phase 2: Experiential And Media Layer

Adopt as needed after the core baseline is stable:

- `motion-v`
- `AutoAnimate`
- `GSAP`
- `Rive`
- `dotLottie`
- `unDraw`

Phase 2 goal:

- enrich product storytelling, empty states, and interaction polish without destabilizing the foundational UI layer

---

## 8. Implementation Rules For Future Additions

These rules apply to all future desktop frontend work.

- new shared UI components land in `packages/ui`
- new business-page features use existing tokens, Tailwind utilities, and `Ui*` abstractions
- frontend-first feature delivery must default to mock data so the primary UI path works before live backend integration is ready
- page, store, and view-model logic must not require live backend or Tauri responses as the default development path
- all new shared schema definitions must be added to feature-based files under `packages/schema/src/*`
- `packages/schema/src/index.ts` is the public export surface only; do not keep adding concrete schema definitions there
- mock data and seed factories must reuse `@octopus/schema` contracts so frontend-first flows and later real integrations stay aligned
- if a shared primitive is missing, add it to `packages/ui` before using it in a page
- avoid direct third-party UI imports in business pages unless the dependency is explicitly approved and wrapped
- prefer Tauri native APIs for desktop-specific capabilities before introducing browser-only workarounds
- treat `CodeMirror` as the default editor for prompt, config, and structured text editing surfaces
- treat `Monaco` as an exception that requires an explicit IDE-class justification
- standardize icon usage:
  - system/product icons: `lucide-vue-next`
  - brand or uncommon icon sets: `Iconify + unplugin-icons`

---

## 9. Current Backlog And Steady-State Focus

Broad migration work is complete. The remaining steady-state backlog is narrower:

1. Keep `pnpm check:frontend-governance`, `pnpm -C apps/desktop typecheck`, `pnpm -C apps/desktop test`, and `pnpm -C apps/desktop build` green together.
2. Keep shared overlay primitives, especially `UiDialog`, free of accessibility warnings in test output.
3. Continue converging future form work onto shared validated form primitives backed by `vee-validate + zod`.
4. Continue converging editor-like text surfaces on `CodeMirror` where the current textarea wrapper is no longer sufficient.
5. Treat future schema/runtime drift as ordinary maintenance work, not as justification to bypass shared UI or governance rules.

---

## 10. Non-Goals For This Round

This governance document does not:

- re-open page-family migration work that is already complete
- authorize page-local visual systems, scoped-style escape hatches, or direct third-party UI usage in app code
- require installing every optional frontend capability before a real product need exists
- replace normal engineering verification; governance rules and green commands remain mandatory for code changes
