# Slice 9 Real MCP Transport Credentials

This task package records the GA slice that replaces the current fake MCP gateway execution path with one governed, credential-aware HTTP JSON-RPC transport boundary.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum real MCP transport and credential-resolution path while preserving the existing governed `Run -> Policy/Budget/Approval -> Artifact -> Knowledge gate` shell.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Extend `crates/interop-mcp` with a transport adapter boundary that supports both the existing simulated executor and a new HTTP JSON-RPC transport.
  - Add runtime registry APIs for MCP server metadata, credential references, invocation inspection, lease inspection, and health observation needed by Slice 9 tests and later remote-hub assembly.
  - Add Slice 9 runtime integration tests for success, normalized failure mapping, retry/reopen behavior, approval wait/resume, and low-trust knowledge gating.
- Out of Scope:
  - Remote-hub user auth, sessions, JWT, or richer multi-tenant persistence.
  - Desktop connector-management UI or automation-management surface work.
  - `packages/hub-client` surface expansion.
  - Additional transports such as stdio, SSE, or provider-specific SDK adapters.
- Acceptance Criteria:
  - Connector-backed execution can call a real HTTP JSON-RPC MCP endpoint through `RunOrchestrator` without bypassing runtime governance.
  - MCP credential references resolve at execution time and inject auth headers without exposing plaintext secrets in query DTOs.
  - Missing credential, wrong credential, timeout, unreachable endpoint, non-2xx, and invalid JSON-RPC responses normalize into auditable invocation failures with correct retryability.
  - Artifact provenance, trust level, lease lifecycle, approval wait/resume, retry/reopen, and knowledge gating semantics remain intact.
- Non-functional Constraints:
  - Keep one execution shell.
  - Keep the slice SQLite-backed and Rust-first.
  - Avoid inventing TS/client contracts unless a tracked shared consumer actually needs them.
- MVP Boundary:
  - One outbound HTTP JSON-RPC transport plus simulated transport for regression tests.
  - One credential-reference model with runtime resolution and header injection.
  - No richer remote-hub persistence or UI assembly.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update runtime migrations and Rust interop/runtime boundaries.
  - Add an ADR only if a durable credential/transport boundary conclusion emerges during implementation.
- Affected Modules:
  - `crates/interop-mcp`
  - `crates/runtime`
  - `docs/tasks`
- Affected Layers:
  - Rust runtime/orchestration layer
  - Interop transport and persistence layer
  - SQLite migration layer
- Risks:
  - Letting real MCP calls escape the governed runtime shell.
  - Leaking credential material into public DTOs or logs.
  - Misclassifying retryable vs non-retryable transport failures.
  - Regressing existing Slice 5 and Slice 8 fake-path behavior.
- Validation:
  - Add failing Slice 9 integration tests first.
  - Re-run targeted Slice 5 and Slice 8 regressions.
  - Run `cargo test --workspace` before claiming completion.
