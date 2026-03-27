# Slice 8 MCP Event Trigger

This task package records the final GA trigger expansion slice in this program: add the minimum governed MCP-event ingress path on top of the shared automation delivery projection model.

## Package Files

- [design-note.md](design-note.md)
- [contract-change.md](contract-change.md)
- [implementation-summary.md](implementation-summary.md)
- [verification.md](verification.md)
- [delivery-note.md](delivery-note.md)

## Task Definition

- Goal:
  - Implement the minimum governed `mcp_event` trigger so registered MCP servers can project matching events into deduped automation deliveries and governed runs.
- Scope:
  - Create this task package and keep local design, contract, verification, and delivery notes here.
  - Add runtime support for MCP-event trigger metadata and validated dispatch against registered `McpServer` records.
  - Add tests for known-server happy path, unknown-server rejection, event mismatch rejection, duplicate dedupe, and low-trust knowledge-gate preservation.
- Out of Scope:
  - Real credentialed MCP transport.
  - Desktop automation UI.
  - Broad connector management or provider-specific event ecosystems.
- Acceptance Criteria:
  - An `mcp_event` trigger can be persisted with its server selector and event match metadata.
  - Runtime only accepts MCP-event dispatch from registered servers with matching event metadata.
  - Duplicate event deliveries reuse the same delivery/run projection.
  - Low-trust output gate behavior remains unchanged.
- Non-functional Constraints:
  - Keep the feature on the existing fake/test-double interop path; do not imply real MCP transport exists.
  - Preserve delivery, approval, retry, and knowledge-gate semantics.
- MVP Boundary:
  - Runtime ingress only.
  - No production connector transport or UI management surface.
- Human Approval Points:
  - None.
- Source Of Truth Updates:
  - Update this task package.
  - Update shared trigger contracts if MCP-event metadata is refined.
  - Update current-state docs if tracked state changes materially.
- Affected Modules:
  - `schemas/runtime`
  - `crates/runtime`
  - `crates/interop-mcp`
  - `docs/tasks`
- Affected Layers:
  - Cross-language contracts
  - Rust runtime/orchestration layer
  - Interop-runtime integration layer
- Risks:
  - Treating MCP-event ingress as proof of real transport support.
  - Accepting events from unknown servers or mismatched event selectors.
  - Breaking low-trust output gate behavior.
- Validation:
  - Runtime integration tests for happy, unknown-server, mismatch, duplicate, and low-trust gate scenarios.
