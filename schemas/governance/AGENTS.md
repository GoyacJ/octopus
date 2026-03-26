# AGENTS.md

## Purpose

These instructions apply to work under `schemas/governance/`.

## Local Rules

- Keep only capability, grant, budget, and approval shared contracts in this directory.
- Preserve approval and policy semantics as governed runtime boundaries, not UI-local state.
- Keep approval status as a separate strong enum file.
- Do not place transport-specific payloads or observation records here.

## Current Files

- `capability-descriptor.schema.json`
- `capability-binding.schema.json`
- `capability-grant.schema.json`
- `budget-policy.schema.json`
- `approval-request.schema.json`
- `approval-request-status.schema.json`
