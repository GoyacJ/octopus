# Octopus Desktop — Frontend Stack And Design System Governance

> **Purpose:** Define the tracked frontend baseline for the desktop app, the ownership boundary between `apps/desktop` and `packages/ui`, and the mandatory implementation rules for future UI work.
> **Scope:** Governance and documentation only. This document does not migrate pages, install dependencies, or repair current runtime drift.
> **Last verified against repo state:** 2026-04-02

---

## 1. Governance Summary

The desktop application is standardized on:

- `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2`
- `@octopus/ui` as the only approved shared UI entrypoint for app code
- `Tailwind CSS + design tokens` as the only approved styling foundation

Future frontend work must treat this document and the root `AGENTS.md` as binding constraints:

- business pages consume `@octopus/ui`, not ad-hoc UI libraries
- missing shared components are added to `packages/ui` before page usage
- accessibility-sensitive overlays and composite widgets standardize on `Reka UI`
- data entry standardizes on `vee-validate + zod`
- structured data display standardizes on `@tanstack/vue-table` and `@tanstack/vue-virtual`
- editor-like text surfaces default to `CodeMirror`
- desktop behaviors prefer Tauri native capabilities

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

`packages/ui` already exists and is the intended shared design-system package.

Current repo evidence:

- `packages/ui/package.json` exposes only `.`, `./main.css`, and `./tokens.css`
- `packages/ui/src/index.ts` exports only a narrow subset of the available `Ui*` components
- `packages/ui/src/components/` already contains many more components than the public export surface exposes, including:
  - base controls: `UiButton`, `UiInput`, `UiTextarea`, `UiCheckbox`, `UiSwitch`, `UiSelect`, `UiRadioGroup`
  - overlay and navigation: `UiDialog`, `UiPopover`, `UiDropdownMenu`, `UiCombobox`, `UiTabs`, `UiAccordion`, `UiContextMenu`
  - data display: `UiDataTable`, `UiVirtualList`, `UiBadge`, `UiSurface`, `UiEmptyState`
  - media and editor: `UiCodeEditor`, `UiIcon`, `UiDotLottie`, `UiRiveCanvas`

Conclusion:

- `@octopus/ui` is already the correct package boundary
- its public export surface is incomplete
- package-level dependency ownership is also incomplete because `packages/ui/package.json` currently declares no runtime dependencies for the libraries imported by several `Ui*` components

### 2.3 Accessibility-Sensitive Components Are Still Hand-Rolled

Several complex components exist today, but they are not yet consistently standardized on `Reka UI` primitives.

Observed examples:

- `packages/ui/src/components/UiDialog.vue` manually manages `Escape` handling, `body.style.overflow`, overlay clicks, and `Teleport`
- `packages/ui/src/components/UiPopover.vue` manually registers and removes document click listeners
- `packages/ui/src/components/UiDropdownMenu.vue` manually handles open state, outside click, and item selection
- `packages/ui/src/components/UiContextMenu.vue` layers custom positioning over `UiDropdownMenu`
- `packages/ui/src/components/UiTabs.vue` and `UiAccordion.vue` currently implement controlled selection/toggle behavior directly

Governance decision:

- `Dialog`, `Popover`, `DropdownMenu`, `Combobox`, `Tabs`, `Accordion`, and `ContextMenu` must move to `Reka UI` primitives before broad UI migration work begins

### 2.4 Business Pages Still Bypass The Intended UI Abstractions

Business views are not yet consistently routed through shared form and interaction primitives.

Observed examples:

- `apps/desktop/src/views/user-center/UserCenterUsersView.vue` uses raw `<input>`, `<select>`, `<option>`, checkbox, and radio controls inside a page-local form
- `apps/desktop/src/views/SettingsView.vue` uses raw `<select>`, `<input type="checkbox">`, and a page-local primary action button
- page-local `<style scoped>` blocks still define business-surface presentation details directly in several views

Governance decision:

- page-level features should use `Ui*` abstractions backed by shared tokens and Tailwind utilities
- if the shared abstraction is missing, it should be created in `packages/ui` first

### 2.5 Current Build And Test Drift

Observed on 2026-04-02 from local verification:

#### `pnpm -C apps/desktop build`

Current build fails in `apps/desktop/src/stores/runtime.ts` because the file references runtime types and client APIs that are not currently exported by the tracked schema and Tauri client layer.

Observed failure classes:

- missing exports from `@octopus/schema`, including `ProviderConfig`, `RuntimeApprovalRequest`, `RuntimeEventEnvelope`, `RuntimeMessage`, `RuntimeRunSnapshot`, `RuntimeSessionDetail`, `RuntimeSessionSummary`, and `RuntimeTraceItem`
- missing exports from `@/tauri/client`, including `bootstrapRuntime`, `createRuntimeSession`, `loadRuntimeSession`, `pollRuntimeEvents`, `resolveRuntimeApproval`, and `submitRuntimeUserTurn`
- object-shape drift such as `rollbackEnabled` not existing on the current `Message` type

#### `pnpm -C apps/desktop test`

Current desktop tests are not green.

Observed failure classes:

- `test/tauri-client.test.ts` fails because desktop runtime transport helpers are missing, including `client.bootstrapRuntime`
- `test/runtime-store.test.ts` and `test/trace-view.test.ts` fail because no active runtime session is established under current runtime wiring
- `test/app-backend-guard.test.ts` fails because the expected desktop backend guard UI is not rendered as asserted
- `test/ui-primitives.test.ts` fails because `UiTabs` is unresolved from `@octopus/ui` and several shared component mounts fail with `Invalid value used as weak map key`

Governance implication:

- do not start broad page migration while runtime/schema drift and `@octopus/ui` export drift are unresolved

---

## 3. Adoption Matrix

The following matrix records the governance target against tracked repo state as of 2026-04-02.

| Capability | Status | Current repo state |
|---|---|---|
| `Vue 3` | Already present | Declared in `apps/desktop/package.json` and used in `apps/desktop/src/main.ts`. |
| `Vite` | Already present | Declared in `apps/desktop/package.json` and configured in `apps/desktop/vite.config.ts`. |
| `Pinia` | Already present | Declared and wired through `createPinia()`. |
| `Vue Router` | Already present | Declared and wired through `router`. |
| `Vue I18n` | Already present | Declared and wired through `plugins/i18n.ts`. |
| `Tauri 2` | Already present | Declared through `@tauri-apps/api` and `src-tauri` config/code. |
| `@octopus/ui` | Partially present | Package exists, but exports and dependency ownership are incomplete. |
| `Tailwind CSS + tokens` | Already present | Root and desktop Tailwind configs exist; `@octopus/ui` ships `main.css` and `tokens.css`. |
| `Reka UI` for composite widgets | Partially present | Target library exists in workspace state, but tracked `Ui*` composites are still hand-rolled. |
| `vee-validate + zod` | Not yet adopted | Not declared in tracked app or UI package manifests, and page forms still use raw controls. |
| `@tanstack/vue-table` | Partially present | `UiDataTable.vue` uses it, but `packages/ui/package.json` does not declare the dependency. |
| `@tanstack/vue-virtual` | Partially present | `UiVirtualList.vue` uses it, but `packages/ui/package.json` does not declare the dependency. |
| `CodeMirror` | Partially present | `UiCodeEditor.vue` exists, but it is currently a textarea wrapper rather than a CodeMirror-based editor. |
| `Monaco` only for IDE-class use | Not yet adopted | No tracked Monaco integration is currently in use. |
| `lucide-vue-next` | Already present | Declared in `apps/desktop/package.json` and used in app views/components. |
| `Iconify + unplugin-icons` | Partially present | `UiIcon.vue` references `@iconify/vue`, but package manifests and public export policy are not complete. |
| `motion-v` | Not yet adopted | No tracked usage in app or UI source. |
| `AutoAnimate` | Not yet adopted | No tracked usage in app or UI source. |
| `GSAP` | Not yet adopted | No tracked usage in app or UI source. |
| `Rive` | Partially present | `UiRiveCanvas.vue` exists, but package dependency ownership/export policy is incomplete. |
| `dotLottie` | Partially present | `UiDotLottie.vue` exists, but package dependency ownership/export policy is incomplete. |
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

Why this matters:

- current app and test failures already show drift between component files and the package export surface
- public surface discipline is required before design-system reuse can scale safely

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

Rules:

- expose consistent variants, sizes, disabled/loading states, and token usage
- do not duplicate button/input styling in business pages

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

Rules:

- these components are the highest-priority candidates for `Reka UI` standardization
- do not add new hand-rolled accessibility-sensitive composite widgets in business code

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

### 6.5 Editors, Media, And Icons

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
- if real Tauri or backend integration is needed, keep it behind adapter boundaries and preserve a mock-first default behavior
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

## 9. Current Backlog And Blockers

The following items are blockers to broad migration work.

1. Complete the `@octopus/ui` public export surface so shared components can be consumed through stable package exports.
2. Declare actual `packages/ui` dependencies instead of relying on incidental workspace availability.
3. Replace hand-rolled accessibility-sensitive components with `Reka UI` implementations.
4. Converge raw page forms onto shared validated form primitives backed by `vee-validate + zod`.
5. Repair current runtime/schema drift so `apps/desktop` build and runtime-related tests match the tracked backend contract.
6. Repair current shared UI test drift so package exports and primitive behavior are aligned before page migration begins.

---

## 10. Non-Goals For This Round

This governance round does not:

- migrate business pages
- add or remove runtime APIs
- install the full target stack immediately
- repair the current `apps/desktop` build/test failures

Those tasks should follow this document, but are intentionally out of scope for this specific change.
