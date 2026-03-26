# AGENTS.md

## Purpose

These instructions apply to work under `packages/`.

## Local Rules

- `packages/` owns shared TypeScript and frontend-side consumer layers.
- Consume shared contracts from `schemas/`; do not redefine shared DTO or enum truth here.
- Keep the current ownership split focused on `schema-ts/` and `hub-client/`.
- Defer UI kit and feature package work until a later slice explicitly requires them.

## Current Phase Constraint

- The tracked tree freezes package ownership only.
- Do not describe any directory under `packages/` as a real package unless tracked manifests, source files, and verification results are present.
