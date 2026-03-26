# AGENTS.md

## Purpose

These instructions apply to work under `crates/`.

## Local Rules

- `crates/` owns Rust-side shared implementation once concrete members are introduced.
- Do not place cross-language contract truth here; shared contracts still belong in `schemas/`.
- Keep crate boundaries aligned with the approved ownership groups: `runtime/`, `domain-context/`, `governance/`, `observe-artifact/`, `interop-mcp/`, and `execution/`.
- If a new Rust subtree does not fit one of those groups, update the local design or ADR before creating it.

## Current Phase Constraint

- The tracked tree freezes ownership groups only.
- Do not describe any directory under `crates/` as a real crate unless tracked crate manifests, source files, and verification evidence exist.
