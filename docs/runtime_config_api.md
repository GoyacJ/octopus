# Runtime Config API And Host Contract

This document describes the current Octopus runtime config model and the public API and adapter contracts that power it.

## Ownership Model

Runtime config is no longer discovered from `.claw/settings*.json` as the canonical Octopus model.

The public ownership model is:

- `workspace`
- `project`
- `user`

The merge order is:

1. `user`
2. `workspace`
3. `project`

The canonical file layout under a workspace root is:

- `config/runtime/workspace.json`
- `config/runtime/projects/<project-id>.json`
- `config/runtime/users/<user-id>.json`

Legacy `.claw` files are migration input only. They are not public API and they are not runtime discover sources after migration.

## Page Responsibility

Runtime config editing is split by business ownership:

- `设置 -> Runtime` edits workspace runtime config only
- `用户中心 -> Profile -> Runtime 配置` edits the current workspace user's runtime config
- `项目 -> Runtime 配置` edits the active project's runtime config

The settings page is not a combined multi-scope editor anymore. The old `local` scope is removed.

## HTTP API

Workspace runtime config routes:

- `GET /api/v1/runtime/config`
- `POST /api/v1/runtime/config/validate`
- `PATCH /api/v1/runtime/config/scopes/workspace`

These public workspace routes are `workspace`-only. They do not resolve current-user runtime config because they do not carry user identity.

Current-user runtime config routes:

- `GET /api/v1/workspace/user-center/profile/runtime-config`
- `POST /api/v1/workspace/user-center/profile/runtime-config/validate`
- `PATCH /api/v1/workspace/user-center/profile/runtime-config`

Project runtime config routes:

- `GET /api/v1/projects/:project_id/runtime-config`
- `POST /api/v1/projects/:project_id/runtime-config/validate`
- `PATCH /api/v1/projects/:project_id/runtime-config`

Authenticated project routes resolve effective config with `user -> workspace -> project` precedence using the current session user.

Runtime session routes remain under `/api/v1/runtime/*`, but session snapshots now reference runtime config sources by identity instead of filesystem path.

## Public Contract Shape

`RuntimeConfigSource` exposes public source metadata only:

- `scope`: `workspace | project | user`
- `ownerId?`
- `displayPath`
- `sourceKey`
- `exists`
- `loaded`
- `contentHash`
- `document`

The public contract must not expose:

- absolute `path`
- `sourcePaths`
- `local` runtime scope

Examples:

- workspace source: `displayPath = "config/runtime/workspace.json"`, `sourceKey = "workspace"`
- project source: `displayPath = "config/runtime/projects/proj-1.json"`, `sourceKey = "project:proj-1"`
- user source: `displayPath = "config/runtime/users/user-1.json"`, `sourceKey = "user:user-1"`

## Snapshot Persistence

Runtime session startup still records a config snapshot, but snapshot metadata is path-free:

- keep `effective_config_hash`
- keep `started_from_scope_set`
- store `sourceRefs` in effective precedence order, such as `["user:user-1", "workspace", "project:proj-1"]`

SQLite projections, debug exports, and session snapshots must not persist absolute runtime config file paths.

## Desktop Adapter Contract

Desktop pages consume runtime config through `apps/desktop/src/tauri/workspace-client.ts`.

The adapter methods are:

- workspace: `runtime.getConfig`, `runtime.validateConfig`, `runtime.saveConfig`
- project: `runtime.getProjectConfig`, `runtime.validateProjectConfig`, `runtime.saveProjectConfig`
- user: `runtime.getUserConfig`, `runtime.validateUserConfig`, `runtime.saveUserConfig`

Pages should call stores, and stores should call these adapter methods. Views should not assemble runtime API requests directly.

## Host Consistency

Browser host and Tauri host use different transport implementations, but they must expose the same schema-backed runtime contract:

- shell bootstrap and host runtime selection live in `apps/desktop/src/tauri/shell.ts`
- shared request headers and error decoding live in `apps/desktop/src/tauri/shared.ts`
- workspace and runtime domain requests live in `apps/desktop/src/tauri/workspace-client.ts`

Transport differences must not change runtime config payload shape, source metadata shape, or runtime session snapshot semantics.
