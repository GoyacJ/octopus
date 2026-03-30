# AGENTS.md

## Purpose

These instructions apply to work under `schemas/governance/`.

## Local Rules

- Keep only capability, model-governance, grant, budget, and approval shared contracts in this directory.
- Preserve approval and policy semantics as governed runtime boundaries, not UI-local state.
- Keep approval status as a separate strong enum file.
- Keep provider adapters, provider connectivity details, and transport-specific payloads out of this directory unless a later approved task package explicitly widens scope.
- Do not place transport-specific payloads or observation records here.

## Current Files

- `capability-descriptor.schema.json`
- `capability-binding.schema.json`
- `capability-grant.schema.json`
- `capability-resolution.schema.json`
- `budget-policy.schema.json`
- `approval-request.schema.json`
- `approval-request-status.schema.json`
- `approval-resolve-command.schema.json`
- `model-provider.schema.json`
- `model-catalog-item.schema.json`
- `model-profile.schema.json`
- `tenant-model-policy.schema.json`
- `model-selection-decision.schema.json`
