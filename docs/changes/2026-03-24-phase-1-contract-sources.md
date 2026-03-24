# Stage Change Record: Phase 1 Contract Sources

- Stage: `phase-1`
- Status: `Done`
- Last Updated: `2026-03-24`
- Related Plan: `docs/plans/2026-03-24-v1-development-roadmap.md`

## Summary

- Added the first external control plane OpenAPI skeleton covering `workspaces`, `agents`, `runs`, `inbox`, `audit`, and `resume`.
- Added the first internal gRPC contract skeleton for node registration, run dispatch, execution event reporting, and resume handoff.
- Added the first `PluginManifest` JSON Schema plus `proto/` directory governance rules, versioning guidance, and lint/generation expectations.

## Scope

- In scope:
  - `proto/openapi/control-plane.v1.yaml`
  - `proto/grpc/octopus/runtime/v1/node_runtime.proto`
  - `proto/schemas/plugin-manifest.schema.json`
  - `proto/README.md` and per-subdirectory README files
  - `proto/buf.yaml`
- Out of scope:
  - Generated SDKs or transport implementations
  - Full OpenAPI lint pipeline wiring
  - Runtime business logic behind these contracts

## Risks

- Main risk:
  - These contracts are intentionally skeletal and will need careful evolution once the Phase 3 MVP slice fixes exact wire semantics.
- Rollback or mitigation:
  - Keep the files versioned and additive, and introduce breaking changes through new versions or ADR-backed contract updates.

## Verification

- Commands run:
  - `cargo metadata --no-deps --format-version 1`
  - `command -v buf`
  - `command -v spectral`
- Manual checks:
  - Confirmed `buf` and `spectral` are not installed in the current environment, so contract lint is documented but not yet executable.
  - Reviewed the new contract files against the roadmap and `proto/README.md` governance rules.

## Docs Sync

- [ ] `docs/PRD.md`
- [ ] `docs/SAD.md`
- [ ] `docs/DEVELOPMENT_STANDARDS.md`
- [ ] `docs/VIBECODING.md`
- [ ] `docs/VISUAL_FRAMEWORK.md`
- [ ] `docs/adr/`
- [x] `docs/plans/`
- [x] `docs/changes/`
- [ ] No doc update needed

## UI Evidence

- [x] Not applicable
- [ ] Light theme screenshot attached
- [ ] Dark theme screenshot attached
- [ ] zh-CN screenshot attached
- [ ] en-US screenshot attached

## Review Notes

- ADR or architecture impact:
  - None. This phase stays inside the approved contract-source baseline and does not change the architecture boundary.
- Security or policy impact:
  - None.
- Contract or schema impact:
  - Introduces first skeleton contract sources for external HTTP, internal gRPC, and plugin manifest schema.
- Blocking reason:
  - None.
- Next action:
  - Use these contracts to back the Web shell and Rust transport/runtime skeletons in Phase 2 and Phase 3.
