# AGENTS.md

## Purpose

These instructions apply to work under `schemas/`.

## Local Rules

- `schemas/` is the cross-language contract source of truth.
- Use JSON Schema as the authoritative source format.
- Derived Rust or TypeScript artifacts are downstream consumers only.
- Use stable `$id` values in the form `https://octopus.local/schemas/<group>/<name>.schema.json`.
- Keep shared naming conventions stable: `id`, `workspace_id`, `project_id`, `created_at`, `updated_at`, `dedupe_key`, `idempotency_key`, `resume_token`.

## Group Boundaries

- `context/` holds Workspace, Project, and KnowledgeSpace contracts.
- `runtime/` holds Task, Run, Automation, Trigger, TriggerDelivery, EnvironmentLease, and their strong runtime state enums.
- `governance/` holds Capability, BudgetPolicy, ApprovalRequest, and approval state enums.
- `observe/` holds Artifact, Audit, Trace, Inbox, Notification, KnowledgeCandidate, KnowledgeAsset, and lineage contracts.

## Current Phase Constraint

- These schemas are placeholders that freeze naming, grouping, and minimal boundary references only.
- Do not silently expand them into full production contracts without a slice-specific task package.
- Do not introduce Beta-only contracts here unless the source-of-truth docs explicitly bring them into scope.
