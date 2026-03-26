# AGENTS.md

## Purpose

These instructions apply to work under `apps/`.

## Local Rules

- Treat `apps/` as surface-specific assembly only.
- Do not place shared runtime truth, shared contracts, or cross-surface logic here.
- Current planned surface placeholders are `desktop/` and `remote-hub/`.
- Defer `web`, `mobile`, `admin`, and `cli` until a task package explicitly brings them into scope.

## Current Phase Constraint

- The tracked tree establishes surface boundaries only.
- Do not describe any app under `apps/` as runnable unless tracked manifests, source files, and verification results are actually present.
