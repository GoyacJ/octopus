# GA Acceptance Matrix

## Scope Alignment

| PRD GA Requirement | Tracked Implementation Evidence | Status | Residual Risk / Note |
| --- | --- | --- | --- |
| `Desktop + Remote Hub` minimum GA surface | Desktop Tauri local host, route-split workbench, remote profile/session handling, secure restore, degraded-state convergence, and remote-hub authenticated shell are tracked in `apps/desktop`, `apps/desktop/src-tauri`, and `apps/remote-hub`. | Satisfied | Full tenant admin / RBAC admin / IdP remain out of scope for this GA baseline. |
| `Task` execution as a formal run path | Local governed runtime and desktop task workbench prove `Task -> Run -> Artifact / Audit` across tracked crates and desktop views. | Satisfied | Collaboration-heavy run types remain Beta or later. |
| `Automation` with GA trigger set | Tracked implementation covers `manual_event`, `cron`, `webhook`, and `mcp_event` into `TriggerDelivery -> Task -> Run`, plus the minimum automation-management surface. | Satisfied | Broader trigger ecosystem remains later work. |
| `Approval` governance loop | Approval detail, inbox resolution, and approval-driven knowledge-promotion request / resolve flows are implemented and verified. | Satisfied | Richer admin governance surfaces remain later work. |
| `Shared Knowledge` minimum GA loop | Knowledge candidate gate, Shared Knowledge recall, project-scoped knowledge index, and lineage / retry records are implemented. | Satisfied | `Org Knowledge Graph` promotion remains Beta by PRD. |
| `MCP` as a first-class GA interop path | Real credentialed MCP transport, server/invocation persistence, and low-trust output gating are implemented. | Satisfied | `A2A` remains Beta by PRD. |
| Read-only / degraded desktop remote behavior stays understandable | Slice 19 secure restore plus Slice 20 shell-level degraded-state convergence now explain auth-required, token-expired, restored-disconnected, and memory-only conditions across the workbench. | Satisfied | Recovery is refresh-on-route-entry rather than push-based. |

## GA Conclusion

- The tracked first-GA baseline is sufficient to satisfy the PRD's scoped GA delivery of `Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`.
- The remaining notable gaps are either explicitly Beta (`A2A`, `Org Knowledge Graph`, higher-order Mesh / discussion / resident-agent flows) or post-GA hardening items (`refresh token`, `token rotation`, `remote admin`, `tenant / IdP` surfaces).
- The required full verification gates are green, so the repo should treat the post-Slice-20 GA core as closed and freeze backlog expansion by default.

## Frozen Post-GA Backlog

- Desktop remote UX deepening beyond the current degraded-state convergence.
- Refresh token / token rotation.
- Remote admin / tenant / IdP surfaces.

## Backlog Rule

- The frozen backlog does not auto-start.
- If a tracked GA gap is discovered, only one gap-filling slice may open next.
- Any non-gap follow-on must start with a new task package and explicit owner-doc updates before implementation.
