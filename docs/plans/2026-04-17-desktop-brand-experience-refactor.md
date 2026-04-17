# Octopus Desktop Brand Experience Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild the desktop product into a commercially credible AI workbench with a stronger Octopus brand identity, calmer information hierarchy, and consistent cross-surface interaction quality without splitting the product into separate personal/team/enterprise visual modes.

**Architecture:** Keep the existing `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2` desktop architecture, adapter boundaries, and schema contracts intact. Perform the redesign as a system-first refactor: update canonical design governance, replace shared tokens and primitives in `@octopus/ui`, then rebuild shell chrome and page archetypes so business surfaces inherit the new language instead of inventing page-local styles.

**Tech Stack:** `apps/desktop`, `packages/ui`, Tailwind CSS with design tokens, Lucide icons, Reka-backed shared primitives, Vitest, existing frontend governance checks, browser-host desktop preview via `pnpm dev:web`, and Playwright CLI for verification captures.

---

## Read Before Starting

- `AGENTS.md`
- `docs/AGENTS.md`
- `docs/design/DESIGN.md`
- `packages/ui/src/tokens.css`
- `scripts/check-frontend-governance.mjs`
- `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- `apps/desktop/src/views/project/ConversationView.vue`

## Locked Product Decisions

- **Design direction:** `Calm Intelligence`.
  Keep the structural discipline of Notion, then add Octopus brand memory and Apple-grade finish through palette control, rhythm, and interaction quality rather than ornamental effects.
- **Audience model:** one product language for personal, professional, and enterprise use.
  Complexity expands progressively through information density and permissions, not through separate visual themes or enterprise-only chrome.
- **Brand palette:** replace the current blue-led accent system with an Octopus coral system derived from the logo.
  - Light theme primary accent: `#ff6a2a`
  - Light hover accent: `#e65a1f`
  - Light accent soft: `#fff1e8`
  - Light warm support tint: `#fff5e7`
  - Dark theme primary accent: `#ff8a57`
  - Dark hover accent: `#ff9b6e`
  - Dark accent soft: `rgba(255, 138, 87, 0.18)`
  - Dark warm support tint: `rgba(255, 208, 170, 0.12)`
- **Neutral base:** keep warm neutral canvases and matte dark mode. Do not move toward glossy black, cold slate, or saturated AI gradients.
- **Typography:** keep a single sans UI stack (`Inter` / `SF Pro Text` / `SF Pro Display` / system sans). Do not add serif or mono to chrome-level UI; keep mono only inside code/editor surfaces.
- **Mascot policy:** the octopus character remains part of the brand, but it appears only in onboarding, empty states, assistant moments, personal center, and pet-related surfaces. Dense workbench pages use palette and iconography, not mascot illustrations, as their primary identity system.
- **Motion:** 120-180ms defaults, opacity plus small translate, no bounce, no glow, no novelty reveal motion. Honor `prefers-reduced-motion` without introducing a new persisted preference.
- **Enterprise constraint:** no second theme, no enterprise-only shell. Enterprise-heavy pages must use the same shell, surfaces, and components as personal and team pages.

## Public Surface Decisions

- **No OpenAPI, backend, persistence, or adapter contract changes.**
  This refactor is frontend and design-system only.
- **No new persisted shell preference keys in v1.**
  Keep `theme`, `locale`, `fontSize`, and sidebar collapse state only.
- **`@octopus/ui` changes are additive and backward-compatible.**
  Existing exports remain stable; any new props introduced for shared primitives must be optional and default-preserving.
- **`docs/design/DESIGN.md` becomes the updated single source of truth before UI edits land.**
  Frontend governance checks must enforce the new brand and visual rules.

## Task 1: Rewrite the canonical desktop design standard and governance rules

**Files:**
- Modify: `docs/design/DESIGN.md`
- Modify: `scripts/check-frontend-governance.mjs`
- Modify: `apps/desktop/test/repo-governance.test.ts`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

**Steps:**
1. Replace the current "single blue accent" standard in `docs/design/DESIGN.md` with the locked coral-led Octopus brand system, while preserving warm neutrals, restrained density, and matte dark mode.
2. Add explicit guidance for:
   - mascot usage boundaries
   - AI-specific emphasis states
   - document, list/detail, conversation, and settings page archetypes
   - motion and empty-state usage
   - progressive-complexity behavior for personal/team/enterprise scenarios
3. Extend `scripts/check-frontend-governance.mjs` to reject:
   - legacy page-private blue accent usage that bypasses token aliases
   - direct orange/yellow tint classes in business surfaces
   - new one-off shadow, blur, or gradient patterns that break the brand system
4. Update governance tests so the new design rules are discoverable and enforced by CI.

## Task 2: Replace the shared token system with the Octopus brand foundation

**Files:**
- Modify: `packages/ui/src/tokens.css`
- Modify: `packages/ui/src/main.css`
- Modify: `packages/ui/src/components/UiSurface.vue`
- Modify: `packages/ui/src/components/UiButton.vue`
- Modify: `packages/ui/src/components/UiBadge.vue`
- Modify: `packages/ui/src/components/UiPageHeader.vue`
- Modify: `packages/ui/src/components/UiPanelFrame.vue`
- Modify: `packages/ui/src/components/UiRecordCard.vue`
- Modify: `packages/ui/src/components/UiMetricCard.vue`
- Modify: `packages/ui/src/components/UiEmptyState.vue`
- Modify: `packages/ui/src/components/UiNotificationBadge.vue`
- Modify: `packages/ui/src/components/UiDialog.vue`
- Modify: `packages/ui/src/components/UiPopover.vue`
- Modify: `packages/ui/src/components/UiToolbarRow.vue`

**Steps:**
1. Rebuild the light and dark token maps around the new brand palette while keeping semantic success, warning, and danger distinct from the brand accent.
2. Tighten the shadow, border, and ring system so the product feels more premium and less flat without introducing glossy elevation.
3. Standardize selected, hover, and focus states across shared primitives:
   - selection uses brand-soft fill plus a stronger whisper border
   - hover remains mostly neutral
   - focus keeps one consistent outer ring treatment
4. Update foundational layout primitives so document pages, list/detail shells, cards, metric tiles, and dialogs share the same spacing rhythm and radii.
5. Extend shared components only where system gaps exist:
   - optional illustration/media slot for `UiEmptyState`
   - optional compact header mode for `UiPageHeader`
   - optional density/emphasis variants for cards and panels when required by list/detail surfaces

## Task 3: Rebuild the workbench shell chrome and global overlays

**Files:**
- Modify: `apps/desktop/src/App.vue`
- Modify: `apps/desktop/src/layouts/WorkbenchLayout.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSidebar.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Modify: `apps/desktop/src/components/layout/ConnectWorkspaceDialog.vue`
- Modify: `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue`
- Modify: `apps/desktop/src/navigation/menuRegistry.ts`
- Modify: `apps/desktop/test/layout-shell.test.ts`
- Modify: `apps/desktop/test/search-overlay.test.ts`
- Modify: `apps/desktop/test/app-runtime-error-boundary.test.ts`

**Steps:**
1. Redesign the sidebar so it feels like a product workbench instead of a generic admin rail:
   - stronger logo lockup
   - quieter group labels
   - better project hierarchy readability
   - cleaner active row language driven by the new brand accent
2. Rebuild the topbar around one consistent cluster system:
   - calmer breadcrumbs
   - stronger search launcher
   - unified menu affordances
   - cleaner account and notification entry points
3. Remove page-local dropdown shells in favor of shared menu/popover styling so theme, spacing, border, and shadow behavior stay consistent.
4. Turn the search overlay into a premium command palette surface with clearer input framing, better result density, and more obvious keyboard focus movement.
5. Align auth, connection, and runtime-error overlays to the same brand system so failure and recovery states feel first-class instead of incidental.

## Task 4: Standardize the page archetypes for core workbench surfaces

**Files:**
- Modify: `packages/ui/src/components/UiListDetailShell.vue`
- Modify: `packages/ui/src/components/UiListDetailWorkspace.vue`
- Modify: `packages/ui/src/components/UiListRow.vue`
- Modify: `packages/ui/src/components/UiInspectorPanel.vue`
- Modify: `apps/desktop/src/views/workspace/WorkspaceOverviewView.vue`
- Modify: `apps/desktop/src/views/project/ProjectDashboardView.vue`
- Modify: `apps/desktop/src/views/app/SettingsView.vue`
- Modify: `apps/desktop/src/views/app/SettingsGeneralPanel.vue`
- Modify: `apps/desktop/src/views/app/SettingsThemePanel.vue`
- Modify: `apps/desktop/src/views/app/SettingsConnectionPanel.vue`
- Modify: `apps/desktop/src/views/app/SettingsVersionPanel.vue`
- Modify: `apps/desktop/src/views/workspace/ProjectsView.vue`
- Modify: `apps/desktop/src/views/workspace/ToolsView.vue`
- Modify: `apps/desktop/src/views/workspace/ModelsView.vue`
- Modify: `apps/desktop/src/views/workspace/WorkspaceKnowledgeView.vue`
- Modify: `apps/desktop/src/views/workspace/WorkspaceResourcesView.vue`
- Modify: `apps/desktop/src/views/project/ProjectDeliverablesView.vue`
- Modify: `apps/desktop/src/views/project/ProjectKnowledgeView.vue`
- Modify: `apps/desktop/src/views/project/ProjectResourcesView.vue`
- Modify: `apps/desktop/src/views/project/ProjectSettingsView.vue`
- Modify: `apps/desktop/test/overview-dashboard.test.ts`
- Modify: `apps/desktop/test/projects-view.test.ts`
- Modify: `apps/desktop/test/tools-view.test.ts`
- Modify: `apps/desktop/test/settings-view.test.ts`

**Steps:**
1. Define and apply the four canonical archetypes:
   - document page
   - list/detail page
   - conversation page
   - settings page
2. Convert overview, dashboard, and settings surfaces to quieter document-style pages with stronger header rhythm and fewer nested card treatments.
3. Convert projects, tools, models, resources, knowledge, and deliverables to a consistent list/detail structure:
   - stable list widths
   - shared row behavior
   - shared inspector treatment
   - consistent toolbar density
4. Remove page-local panel inventions and fold them back into shared shell primitives so every surface reads as one product.
5. Keep high-density information pages readable by reducing saturation and strengthening hierarchy instead of adding more containers.

## Task 5: Make the conversation and AI-native surfaces feel premium and branded

**Files:**
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/views/project/TraceView.vue`
- Modify: `apps/desktop/src/components/layout/ConversationContextPane.vue`
- Modify: `apps/desktop/src/components/layout/ConversationTabsBar.vue`
- Modify: `packages/ui/src/components/UiConversationComposerShell.vue`
- Modify: `packages/ui/src/components/UiArtifactBlock.vue`
- Modify: `packages/ui/src/components/UiTraceBlock.vue`
- Modify: `packages/ui/src/components/UiInboxBlock.vue`
- Modify: `packages/ui/src/components/UiMessageCenter.vue`
- Modify: `packages/ui/src/components/UiNotificationRow.vue`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/trace-view.test.ts`

**Steps:**
1. Rework the conversation stream so it feels AI-native without turning into a neon chat app:
   - neutral message surfaces
   - brand accent reserved for active runs, primary actions, and important AI cues
   - stronger typography and spacing around assistant output
2. Rebuild the composer as a premium command surface with better hierarchy for prompt input, attachments, approvals, and send/run actions.
3. Make tool calls, artifacts, approvals, trace entries, and inbox-style action blocks visually consistent with the rest of the workbench instead of looking like separate micro-products.
4. Keep the right context pane visually subordinate and easier to scan, with the same shell language as other inspectors.
5. Ensure the conversation, trace, and message-center surfaces share one notification/status grammar.

## Task 6: Align secondary, enterprise-heavy, and expressive surfaces to the same system

**Files:**
- Modify: `apps/desktop/src/views/agents/*.vue`
- Modify: `apps/desktop/src/views/workspace/WorkspaceConsoleView.vue`
- Modify: `apps/desktop/src/views/workspace/WorkspaceAgentsView.vue`
- Modify: `apps/desktop/src/views/workspace/TeamsView.vue`
- Modify: `apps/desktop/src/views/workspace/AccessControlView.vue`
- Modify: `apps/desktop/src/views/workspace/access-control/*.vue`
- Modify: `apps/desktop/src/views/workspace/PersonalCenterView.vue`
- Modify: `apps/desktop/src/views/workspace/personal-center/*.vue`
- Modify: `apps/desktop/src/views/auth/*.vue`
- Modify: `apps/desktop/src/startup/diagnostics.ts`
- Modify: `apps/desktop/test/agent-center.test.ts`
- Modify: `apps/desktop/test/access-centers.test.ts`
- Modify: `apps/desktop/test/personal-center-pet-view.test.ts`

**Steps:**
1. Bring agent, team, and workspace-console surfaces onto the same card, list, and inspector language used by the primary workbench pages.
2. Refactor access-control screens so even the densest enterprise workflows still feel like Octopus rather than a separate admin subsystem.
3. Keep personal center and pet surfaces slightly more expressive, but enforce the same token, spacing, and motion rules.
4. Rebuild login, startup, and fatal-diagnostics surfaces to feel intentional and branded, using the same palette and typography system.
5. Remove any remaining raw business-page controls or ad hoc visual recipes discovered during the sweep.

## Task 7: Add a restrained motion, illustration, and state-feedback layer

**Files:**
- Modify: `packages/ui/src/tokens.css`
- Modify: `packages/ui/src/components/UiDotLottie.vue`
- Modify: `packages/ui/src/components/UiRiveCanvas.vue`
- Modify: `packages/ui/src/components/UiStatusCallout.vue`
- Modify: `packages/ui/src/components/UiToastItem.vue`
- Modify: `apps/desktop/src/assets/logo.png` only if a cleaned export is required for better shell rendering
- Add or Modify: brand-safe empty-state and loading assets under `apps/desktop/src/assets/**` if the current set is insufficient

**Steps:**
1. Define the approved motion timings, easing, and reduced-motion behavior in tokens and shared wrappers.
2. Standardize toast, callout, loading, success, and empty-state feedback so the product feels polished under both happy and unhappy paths.
3. Add only a small, reusable set of brand-supporting illustrations or animations for:
   - empty states
   - loading and processing
   - success/failure confirmation
4. Keep illustrations soft and editorial; do not add mascot-heavy decoration to dense operational pages.

## Task 8: Verify the redesign with structural, visual, and release-ready checks

**Files:**
- Modify or Add: any focused desktop tests needed to lock the new shell and shared primitive contracts
- Add: approved capture outputs under `output/playwright/desktop-brand-refactor/` if snapshot documentation is needed

**Steps:**
1. Run the desktop verification suite after each major phase:

```bash
pnpm check:frontend-governance
pnpm -C apps/desktop typecheck
pnpm -C apps/desktop test
```

2. Use the browser-host desktop preview for visual QA:

```bash
pnpm dev:web
```

3. Capture and review current-light and current-dark screenshots for at least:
   - workspace overview
   - project dashboard
   - conversation
   - settings
   - projects
   - tools
   - access control

4. Use Playwright CLI to archive verification captures under `output/playwright/desktop-brand-refactor/` so future redesign work has concrete before/after references.
5. Do not consider the refactor complete until:
   - no business surface relies on page-local accent styling
   - light and dark themes both feel equally premium
   - shell, search, conversation, settings, and list/detail pages read as one product
   - brand orange reads as identity, not warning
   - enterprise-heavy screens still feel calm and usable

## Acceptance Criteria

- The desktop shell is recognizably Octopus within one second of launch, even before entering a specific workflow.
- The product keeps the calm order of a serious workbench while gaining a memorable brand signature.
- High-frequency workflows no longer feel like separate design dialects.
- Visual emphasis comes from hierarchy, copy, and spacing first; color is supportive and restrained.
- The redesign improves perceived quality in light and dark themes without increasing noise.
- No regression is introduced in routing, stores, adapters, or persisted preferences.

## Assumptions And Defaults

- The worktree may already contain unrelated changes; do not revert them.
- This plan does not include backend or OpenAPI work unless a concrete UI issue exposes a real contract gap later.
- The current logo remains the source for palette extraction; no mascot or logo redesign is required in this phase.
- Existing fonts remain in place; premium feel comes from typography tuning, not a new font family.
- Where a shared component can solve the problem, use `@octopus/ui` instead of adding business-page styling.
