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

Current tracked task packages now cover the GA foundation and slice sequence through Slice 20 `desktop degraded-state convergence`, plus the implemented post-GA desktop `Dashboard + Conversation first` redesign.

The completed post-GA closeout slices are [2026-03-30-post-ga-session-token-lifecycle](2026/2026-03-30-post-ga-session-token-lifecycle/README.md), [2026-03-30-post-ga-model-center-foundation](2026/2026-03-30-post-ga-model-center-foundation/README.md), and [2026-03-30-post-ga-model-governance-persistence](2026/2026-03-30-post-ga-model-governance-persistence/README.md).

The queued next design-only candidate is [2026-03-30-post-ga-model-governance-read-transport](2026/2026-03-30-post-ga-model-governance-read-transport/README.md).

The implemented and verified post-GA desktop redesign package is [2026-03-30-post-ga-desktop-dashboard-conversation-redesign](2026/2026-03-30-post-ga-desktop-dashboard-conversation-redesign/README.md). It records the bounded `Dashboard + Conversation first` desktop target-state implementation without adding new shared conversation contracts.

Post-Slice-20 follow-on work must start by creating a new task package before implementation. The current queue is persistence complete, read-only transport design-only, and any later transport/UI implementation must explicitly promote that design package first.

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
- [2026-03-27-slice-9-real-mcp-transport-credentials](2026/2026-03-27-slice-9-real-mcp-transport-credentials/README.md)
- [2026-03-27-slice-10-remote-hub-persistence-auth](2026/2026-03-27-slice-10-remote-hub-persistence-auth/README.md)
- [2026-03-27-ga-minimum-automation-surface](2026/2026-03-27-ga-minimum-automation-surface/README.md)
- [2026-03-28-slice-11-ga-governance-interaction-surface](2026/2026-03-28-slice-11-ga-governance-interaction-surface/README.md)
- [2026-03-28-slice-12-ga-governance-explainability](2026/2026-03-28-slice-12-ga-governance-explainability/README.md)
- [2026-03-28-slice-13-desktop-local-host-foundation](2026/2026-03-28-slice-13-desktop-local-host-foundation/README.md)
- [2026-03-28-slice-14-desktop-task-workbench](2026/2026-03-28-slice-14-desktop-task-workbench/README.md)
- [2026-03-29-slice-15-project-knowledge-index](2026/2026-03-29-slice-15-project-knowledge-index/README.md)
- [2026-03-29-slice-16-desktop-remote-connection-surface](2026/2026-03-29-slice-16-desktop-remote-connection-surface/README.md)
- [2026-03-29-slice-17-desktop-project-scope-entry](2026/2026-03-29-slice-17-desktop-project-scope-entry/README.md)
- [2026-03-29-slice-18-run-control-surface](2026/2026-03-29-slice-18-run-control-surface/README.md)
- [2026-03-29-slice-19-session-hardening](2026/2026-03-29-slice-19-session-hardening/README.md)
- [2026-03-29-slice-20-desktop-degraded-state-convergence](2026/2026-03-29-slice-20-desktop-degraded-state-convergence/README.md)
- [2026-03-30-post-ga-session-token-lifecycle](2026/2026-03-30-post-ga-session-token-lifecycle/README.md)
- [2026-03-30-post-ga-model-center-foundation](2026/2026-03-30-post-ga-model-center-foundation/README.md)
- [2026-03-30-post-ga-desktop-dashboard-conversation-redesign](2026/2026-03-30-post-ga-desktop-dashboard-conversation-redesign/README.md)
- [2026-03-30-post-ga-model-governance-consumers](2026/2026-03-30-post-ga-model-governance-consumers/README.md)
- [2026-03-30-post-ga-model-governance-persistence](2026/2026-03-30-post-ga-model-governance-persistence/README.md)
- [2026-03-30-post-ga-model-governance-read-transport](2026/2026-03-30-post-ga-model-governance-read-transport/README.md)
- [2026-03-30-desktop-launch-entry-fix](2026/2026-03-30-desktop-launch-entry-fix/README.md)
- [2026-03-30-desktop-dev-workflow](2026/2026-03-30-desktop-dev-workflow/README.md)

## Relationship To ADRs

- Use a task package for local, slice-specific work.
- Use an ADR only when the conclusion becomes durable repository guidance.
- If a task triggers an ADR, keep the local design details in the task package and record only the durable conclusion in `docs/decisions/`.
