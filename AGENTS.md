# AGENTS.md

## Frontend Governance

- Desktop frontend baseline: `Vue 3 + Vite + Pinia + Vue Router + Vue I18n + Tauri 2`.
- **Design System Aesthetic**: `Minimalist Refined Foundation` (Notion-inspired). Focus on:
  - **Whisper-quiet borders**: `border-border/40` or `dark:border-white/[0.08]`.
  - **Apple-style easing**: `cubic-bezier(0.32, 0.72, 0, 1)`.
  - **Hierarchical Typography**: Bold titles, tight tracking, small uppercase eyebrows with wide tracking.
  - **Refined Surfaces**: `UiSurface` with subtle shadows and background gradients.
- Frontend-first delivery uses mock data by default. Pages, stores, and view models must be able to complete their primary flows without requiring a live backend or Tauri host response.
- Real Tauri or backend integration may remain behind the existing adapter layer, but it must not become the default path for new frontend feature development in the current phase.
- Shared schemas in `packages/schema` must be defined in feature-based files under `packages/schema/src/*`. `packages/schema/src/index.ts` is the public export surface only and must not keep accumulating schema definitions.
- Frontend mock data must reuse `@octopus/schema` contracts so mock flows and later real integrations stay aligned.
- Shared UI must go through `@octopus/ui`. Business pages must not introduce ad-hoc third-party UI styles or bypass the shared design system.
- Component selection order:
  1. Reuse `@octopus/ui`.
  2. If missing, reference `shadcn-vue` interaction and structure patterns, but implement the component inside `@octopus/ui`.
  3. `Dialog`, `Popover`, `DropdownMenu`, `Combobox`, `Tabs`, `Accordion`, and `ContextMenu` must be built on `Reka UI` primitives.
- Shared UI Component Catalog:
  - Base: `UiButton`, `UiInput`, `UiTextarea`, `UiCheckbox`, `UiSwitch`, `UiSelect`, `UiRadioGroup`, `UiSectionHeading`.
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
  - do not introduce large all-in-one UI frameworks
  - do not import unapproved UI libraries directly in business pages
  - do not deep import from `packages/ui/src/components/*`; consume the public `@octopus/ui` export surface only

## Request Contract Governance

- Shared request/response contracts must be defined in `packages/schema/src/*` and consumed from `@octopus/schema` by both frontend and backend. Do not invent view-local API shapes for host, backend, or mock data.
- `apps/desktop` business code must not call `fetch` directly for workspace business APIs. Use the existing adapter boundary:
  - shell/host requests through `apps/desktop/src/tauri/shell.ts`
  - workspace/domain requests through `apps/desktop/src/tauri/workspace-client.ts`
  - shared header/session helpers through `apps/desktop/src/tauri/shared.ts`
- Pages should talk to Pinia stores or view-model actions. Stores talk to the adapter client. Views must not embed backend request assembly, auth header logic, or idempotency logic.
- Browser host and Tauri host must expose the same contract shape through the same adapter surface. New capabilities should extend the adapter and schema first, then be consumed by stores/pages.
- Frontend-first and mock-first remain mandatory:
  - every new primary flow must have a mock-capable path
  - workspace fixtures and mocks must return the same `@octopus/schema` payloads as real clients
  - frontend stores must not depend on a live backend as their default execution path
- Request headers and transport conventions are standardized:
  - every backend request carries `X-Request-Id`
  - workspace-scoped requests carry `X-Workspace-Id`
  - authenticated workspace requests use bearer session tokens
  - mutation endpoints that can be retried must support `Idempotency-Key`
  - SSE resume uses `Last-Event-ID`
- Route design conventions:
  - public/system bootstrap routes are explicit and minimal
  - workspace-scoped APIs live under `/api/v1/*`
  - runtime APIs live under `/api/v1/runtime/*`
  - request validation happens at the server boundary before touching services
- Service boundary conventions:
  - `crates/octopus-server` owns HTTP transport, auth checks, request-id/idempotency/cors mechanics, and response shaping
  - `crates/octopus-platform` owns trait contracts between transport and implementations
  - adapters/services return typed domain results, not ad-hoc JSON maps
- When adding a new endpoint or command:
  - update `@octopus/schema`
  - update the client adapter
  - update mock fixtures
  - add or update store usage
  - add tests for client/store/server as appropriate

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
- The canonical runtime config model is the layered loader in `crates/runtime`:
  - user scope
  - project/shared scope
  - local machine scope
  - precedence and merge behavior must remain compatible with `ConfigLoader`
- Desktop settings may provide full runtime config editing, but writes must go back to the correct scope file instead of replacing the layered model.
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
  - reporting secret reference presence/missing state without leaking plaintext
- Session/run persistence rules:
  - runtime sessions, runs, and approvals keep their current projection in SQLite
  - append-only runtime events stay in JSONL for audit, replay, and debugging
  - active session detail should be reconstructable from SQLite projection plus event log
- Config snapshot rules:
  - every session/run start records a `config_snapshot`
  - session/run records reference `config_snapshot_id`, `effective_config_hash`, and `started_from_scope_set`
  - snapshots capture source files and hashes, not an uncontrolled duplicate config body everywhere
- Live session behavior:
  - a running session is bound to the effective config captured at start
  - config edits affect only new sessions unless explicit hot-reload support is designed and documented
  - do not silently mutate active runtime sessions because a config file changed on disk
- Host consistency rule:
  - Tauri host and browser host must expose the same runtime config and runtime session behavior through shared adapter contracts
  - mocks should mirror those contracts closely enough for stores and settings flows to work without a real backend
