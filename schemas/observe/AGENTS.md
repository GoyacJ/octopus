# AGENTS.md

## Purpose

These instructions apply to work under `schemas/observe/`.

## Local Rules

- Keep artifact, audit, trace, inbox, notification, and knowledge-provenance contracts in this directory.
- Preserve observation and lineage objects as shared records, not UI-local display models.
- Keep strong inbox and notification state enums separate from object schemas.
- Do not place approval, capability, or core runtime contracts here.

## Current Files

- `artifact.schema.json`
- `audit-record.schema.json`
- `trace-record.schema.json`
- `inbox-item.schema.json`
- `notification.schema.json`
- `knowledge-candidate.schema.json`
- `knowledge-asset.schema.json`
- `knowledge-lineage-record.schema.json`
- `inbox-item-status.schema.json`
- `notification-status.schema.json`
