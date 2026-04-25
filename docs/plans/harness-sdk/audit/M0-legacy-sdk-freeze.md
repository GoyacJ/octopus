# M0 Legacy SDK Freeze

> Status: Active from M0 through M7.
> Owner: Harness SDK refactor.

## Frozen Crates

The following legacy SDK crates remain in the workspace during M0-M7:

- `octopus-sdk`
- `octopus-sdk-contracts`
- `octopus-sdk-core`
- `octopus-sdk-model`
- `octopus-sdk-tools`
- `octopus-sdk-permissions`
- `octopus-sdk-sandbox`
- `octopus-sdk-hooks`
- `octopus-sdk-context`
- `octopus-sdk-session`
- `octopus-sdk-subagent`
- `octopus-sdk-observability`
- `octopus-sdk-mcp`
- `octopus-sdk-plugin`

## Freeze Rules

Allowed changes:

- compilation fixes
- security fixes
- CI compatibility fixes
- minimal compatibility fixes required by the M8 business cutover

Forbidden changes:

- new public APIs
- new business capabilities
- new persistence paths
- new event types
- dependencies from any `octopus-harness-*` crate to any `octopus-sdk*` crate

## Removal Timing

The legacy SDK crates are removed only after the M8 business cutover gate passes.
Until then, server, desktop, and CLI business paths may continue using the legacy SDK.

## Boundary

M0 creates the new harness crate surfaces without copying legacy SDK implementation code.
Boundary enforcement lives in `scripts/harness-legacy-boundary.sh`.
