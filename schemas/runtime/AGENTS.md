# AGENTS.md

## Purpose

These instructions apply to work under `schemas/runtime/`.

## Local Rules

- Keep only runtime entry, delivery, lease, and run-lifecycle contracts in this directory.
- Preserve Run as the authority execution shell.
- Keep strong runtime state enums separate from object schemas when the state machine is explicitly constrained.
- Do not move approval, artifact, or knowledge contracts into this group.

## Current Files

- `task.schema.json`
- `run.schema.json`
- `automation.schema.json`
- `trigger.schema.json`
- `trigger-delivery.schema.json`
- `environment-lease.schema.json`
- `run-status.schema.json`
- `trigger-delivery-status.schema.json`
- `environment-lease-status.schema.json`
