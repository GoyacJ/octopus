# AGENTS.md

## AI Planning And Execution Protocol

- Codex reads `AGENTS.md` from the repository root down to the current working directory. Keep repo-wide rules here and put narrower overrides in nested `AGENTS.md` files close to the affected subtree.

## Frontend Governance

- This root file defines repo-wide defaults. If a more specific `AGENTS.md` exists deeper in the tree, the nearest file wins for that subtree.
- All git worktrees must be created from `main`. Do not create worktrees from feature branches or other non-`main` branch heads.
- Before changing HTTP/API/OpenAPI-related code or docs, read `docs/api-openapi-governance.md`.
- Before changing files under `docs/**`, follow `docs/AGENTS.md`.
- Before changing files under `contracts/openapi/**`, follow `contracts/openapi/AGENTS.md`.
- Desktop frontend baseline: `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2`.
- UI/UX source of truth for product surfaces is `docs/design/DESIGN.md`. Any AI agent generating or modifying UI must read and follow it before changing layouts, components, motion, copy hierarchy, or visual styling.
- `docs/design/DESIGN.md` is the canonical visual and interaction standard for desktop UI and design-system evolution. Treat preview files in `docs/design/*` as references only.
- Desktop frontend delivery uses real workspace and host APIs through the shared adapter layer by default.
- Real Tauri and browser-host integration must stay behind the existing adapter layer so pages, stores, and view models consume one contract surface.
- Shared schemas in `packages/schema` must be defined in feature-based files under `packages/schema/src/*`. `packages/schema/src/index.ts` is the public export surface only and must not keep accumulating schema definitions.
- Any local fixtures, tests, or seeded development data must reuse `@octopus/schema` contracts so non-production helpers stay aligned with real API payloads.
- Shared UI must go through `@octopus/ui`. Business pages must not introduce ad-hoc third-party UI styles or bypass the shared design system.
- Frontend implementation order:
  1. Follow `docs/design/DESIGN.md`.
  2. Reuse existing shared components and tokens first.
  3. If missing, extract or extend reusable shared components in `@octopus/ui` or shared tokens instead of hardcoding business-page-specific UI.
- Component selection order:
  1. Reuse `@octopus/ui`.
  2. If missing, reference `shadcn-vue` interaction and structure patterns, but implement the component inside `@octopus/ui`.
  3. `Dialog`, `Popover`, `DropdownMenu`, `Combobox`, `Tabs`, `Accordion`, and `ContextMenu` must be built on `Reka UI` primitives.
- Shared UI Component Catalog:
- Base: `UiButton`, `UiInput`, `UiTextarea`, `UiCheckbox`, `UiSwitch`, `UiSelect`, `UiRadioGroup`, `UiKbd`, `UiSectionHeading`.
  - Layout: `UiSurface`, `UiPanelFrame`, `UiToolbarRow`, `UiPageHero`, `UiNavCardList`.
  - Data Display: `UiBadge`, `UiEmptyState`, `UiMetricCard`, `UiRankingList`, `UiTimelineList`, `UiRecordCard`, `UiListRow`, `UiStatTile`, `UiPagination`.
  - Context Blocks: `UiArtifactBlock`, `UiTraceBlock`, `UiInboxBlock`.
  - Composite: `UiDialog`, `UiPopover`, `UiDropdownMenu`, `UiCombobox`, `UiTabs`, `UiAccordion`, `UiContextMenu`, `UiSelectionMenu`, `UiDataTable`, `UiVirtualList`.
  - Media/Editor: `UiCodeEditor`, `UiIcon`, `UiDotLottie`, `UiRiveCanvas`.
- Styling must use `Tailwind CSS + design tokens` only. Do not mix multiple styling systems in the same surface.
- Forms default to `vee-validate + zod`.
- Tables default to `@tanstack/vue-table`.
- Long lists default to `@tanstack/vue-virtual`.
- Config and prompt editors default to `CodeMirror`. Use `Monaco` only for explicit IDE-class scenarios.
- Desktop capabilities should prefer Tauri native APIs for file picking, drag/drop import, tray, and window-level behavior.
- Icons default to `lucide-vue-next`. Brand or rare icons use `Iconify + unplugin-icons`.
- Motion policy:
  - system motion: `motion-v`
  - natural list, form, and settings transitions: `AutoAnimate`
  - welcome, highlight, and choreography surfaces: `GSAP`
  - character and state-machine animation: `Rive`
  - success, failure, loading, and empty-state assets: `dotLottie`
  - illustration: `unDraw`
- Forbidden patterns:
  - do not bypass `docs/design/DESIGN.md` with ad-hoc UI decisions
  - do not hardcode business-page-specific colors, radii, spacing, shadows, or animation values when a shared token or reusable component should exist
  - do not introduce large all-in-one UI frameworks
  - do not import unapproved UI libraries directly in business pages
  - do not deep import from `packages/ui/src/components/*`; consume the public `@octopus/ui` export surface only

## Request Contract Governance

- `docs/api-openapi-governance.md` is the canonical policy for frontend/backend HTTP contract work.
- Shared request/response contracts must be defined in `packages/schema/src/*` and consumed from `@octopus/schema`. Do not invent view-local API shapes for host or backend requests.
- Frontend integration remains adapter-first, store-first, and real-API-first.
- `apps/desktop` business code must not call bare `fetch` for host or workspace business APIs. Use the existing adapter boundary:
  - shell and host requests through `apps/desktop/src/tauri/shell.ts`
  - workspace and runtime requests through `apps/desktop/src/tauri/workspace-client.ts`
  - shared header, session, and error helpers through `apps/desktop/src/tauri/shared.ts`
- Browser host and Tauri host must expose the same public adapter contract shape.
- For `/api/v1/*` HTTP changes, the required order is: update `contracts/openapi/src/**`, run `pnpm openapi:bundle`, run `pnpm schema:generate`, update adapter/store/server/tests, then run the relevant checks.
- Never hand-edit `contracts/openapi/octopus.openapi.yaml` or `packages/schema/src/generated.ts`.

## Persistence Governance

- Octopus uses layered local-first persistence, not a single storage technology.
- Storage responsibilities are fixed:
  - `config/*` and runtime layered config files are the canonical source for declarative configuration and startup parameters
  - `data/main.db` is the only structured local database for queryable state and projections
  - `runtime/events/*.jsonl` and audit/trace logs are append-only event/audit streams
  - `data/blobs`, `data/artifacts`, `data/knowledge`, and `data/inbox` store file content and large objects
  - the database stores metadata, indexes, hashes, paths, and projections, not large file bodies as the primary copy
- Do not introduce additional frontend truth sources such as IndexedDB mirrors, Pinia persistence layers, or ad-hoc JSON caches for business state unless explicitly approved.
- New persistent data must declare its responsibility up front:
  - config/state that humans may edit or inspect directly belongs in files when appropriate
  - query-heavy structured state belongs in SQLite
  - immutable append-only operational history belongs in JSONL/audit logs
  - binary or large textual artifacts belong on disk with metadata in SQLite
- File layout under the workspace root is intentional and must be preserved:
  - `config/` for workspace and application registry files
  - `data/` for database plus artifact/blob/knowledge/inbox storage
  - `runtime/` for runtime session/debug/event/trace/approval/cache files
  - `logs/` for audit and server logs
  - `tmp/` for transient files only
- Blob/artifact persistence rules:
  - store content on disk
  - store `storage_path`, `content_hash`, `byte_size`, `content_type`, timestamps, and related IDs in SQLite/schema contracts
  - avoid duplicating blob content across DB rows and ad-hoc export files
- JSON session files under `runtime/sessions/*` are debug/export artifacts, not the sole source of truth. Runtime projections must be recoverable from SQLite plus append-only event logs.
- Any new persistence path must have a corresponding initialization step in workspace layout/bootstrap. Do not rely on lazy directory creation scattered across unrelated modules.

## Runtime Config And Runtime Persistence Rules

- Runtime config remains file-first. `main.db` must not become the canonical source of runtime settings.
- The canonical runtime config model for Octopus is ownership-driven:
  - `workspace` scope stored at `config/runtime/workspace.json`
  - `project` scope stored at `config/runtime/projects/<project-id>.json`
  - `user` scope stored at `config/runtime/users/<user-id>.json`
  - merge precedence is `user < workspace < project`
  - deep-merge, validation, and patch behavior continue to reuse `crates/runtime`, but `.claw` path discovery is not the Octopus runtime source model
- Desktop settings only edit workspace runtime config. User runtime config belongs to the user center, and project runtime config belongs to the project workspace surface.
- Runtime config saves must use partial patch semantics:
  - preserve unknown keys
  - preserve hand-maintained content outside the edited patch
  - avoid whole-file overwrite strategies unless a migration explicitly requires them
- Sensitive config values must not be written back to normal config files as plaintext when they belong in secure storage. Files should hold references or non-sensitive settings; APIs should return redacted values when needed.
- Runtime config APIs should support:
  - reading effective config
  - listing config sources and override order
  - validating a scoped patch before save
  - saving a scoped patch
  - exposing public source metadata via `scope`, `ownerId`, `displayPath`, and `sourceKey` without leaking absolute filesystem paths
  - reporting secret reference presence/missing state without leaking plaintext
- Session/run persistence rules:
  - runtime sessions, runs, and approvals keep their current projection in SQLite
  - append-only runtime events stay in JSONL for audit, replay, and debugging
  - active session detail should be reconstructable from SQLite projection plus event log
- Config snapshot rules:
  - every session/run start records a `config_snapshot`
  - session/run records reference `config_snapshot_id`, `effective_config_hash`, and `started_from_scope_set`
  - snapshots capture `sourceRefs` and hashes, not absolute source paths or an uncontrolled duplicate config body everywhere
- Live session behavior:
  - a running session is bound to the effective config captured at start
  - config edits affect only new sessions unless explicit hot-reload support is designed and documented
  - do not silently mutate active runtime sessions because a config file changed on disk
- Host consistency rule:
  - Tauri host and browser host must expose the same runtime config and runtime session behavior through shared adapter contracts
  - host-specific transport code may differ, but public runtime contracts and adapter return shapes must stay identical
