# Task Packages

Task packages store the local design, contract, verification, and delivery artifacts for a meaningful task.

## When To Create A Task Package

Create a task package before coding a meaningful slice or module, especially when the work is:

- multi-step
- architecture-sensitive
- schema-affecting
- cross-language
- boundary-changing
- difficult to review from code alone

## Location Convention

Store task packages under:

`docs/tasks/YYYY/YYYY-MM-DD-short-topic/`

Example:

`docs/tasks/2026/2026-03-26-document-governance-refactor/`

## Recommended Package Structure

- `README.md` — task summary, scope, and links to package files
- `design-note.md` — required when boundary, flow, or structure needs explicit design
- `contract-change.md` — required when contracts, schemas, DTOs, events, or shared interfaces change
- `implementation-summary.md` — required once implementation starts
- `verification.md` — required before claiming completion
- `delivery-note.md` — required for substantial delivery
- `adr-trigger-note.md` — optional when the task may require a new ADR

Use the templates in [../governance/ai-delivery-templates.md](../governance/ai-delivery-templates.md).

## Current Task Packages

- [2026-03-26-ga-foundation-repo-skeleton](2026/2026-03-26-ga-foundation-repo-skeleton/README.md)
- [2026-03-26-slice-1-task-run-artifact-audit](2026/2026-03-26-slice-1-task-run-artifact-audit/README.md)
- [2026-03-26-slice-2-approval-inbox-notification](2026/2026-03-26-slice-2-approval-inbox-notification/README.md)
- [2026-03-26-slice-3-automation-manual-event](2026/2026-03-26-slice-3-automation-manual-event/README.md)
- [2026-03-27-slice-4-shared-knowledge](2026/2026-03-27-slice-4-shared-knowledge/README.md)
- [2026-03-27-slice-5-mcp-gateway-environment-lease](2026/2026-03-27-slice-5-mcp-gateway-environment-lease/README.md)
- [2026-03-27-ga-minimal-surface-hub-foundation](2026/2026-03-27-ga-minimal-surface-hub-foundation/README.md)
- [2026-03-27-ga-trigger-expansion-foundation](2026/2026-03-27-ga-trigger-expansion-foundation/README.md)
- [2026-03-27-slice-6-cron-trigger](2026/2026-03-27-slice-6-cron-trigger/README.md)
- [2026-03-27-slice-7-webhook-trigger](2026/2026-03-27-slice-7-webhook-trigger/README.md)
- [2026-03-27-slice-8-mcp-event-trigger](2026/2026-03-27-slice-8-mcp-event-trigger/README.md)

## Relationship To ADRs

- Use a task package for local, slice-specific work.
- Use an ADR only when the conclusion becomes durable repository guidance.
- If a task triggers an ADR, keep the local design details in the task package and record only the durable conclusion in `docs/decisions/`.
