# Architecture Decisions

This directory stores durable architecture decisions.

Use a short, one-decision-per-file ADR format with numbered filenames.

## When To Write An ADR

Write or update an ADR when a task produces a durable conclusion about:

- module or boundary ownership
- source-of-truth ownership
- schema ownership or compatibility rules
- runtime, governance, knowledge, or interop boundaries
- repo structure rules
- implementation assumptions that should become lasting team guidance

Do **not** use an ADR for temporary task notes, local implementation details, or delivery summaries. Those belong in `docs/tasks/`.

## Naming

- Store ADRs as `NNNN-short-kebab-title.md`
- Keep numbering monotonic
- Prefer a stable title that reflects the decision rather than the triggering task

## Current ADRs

- [0001-monorepo-and-boundary-rules.md](0001-monorepo-and-boundary-rules.md)
- [0002-json-schema-source-of-truth-and-generation-boundary.md](0002-json-schema-source-of-truth-and-generation-boundary.md)
- [0003-automation-delivery-projects-into-derived-tasks.md](0003-automation-delivery-projects-into-derived-tasks.md)
- [0004-knowledge-crate-and-project-shared-knowledge-loop.md](0004-knowledge-crate-and-project-shared-knowledge-loop.md)
- [0005-centralized-capability-invocation-and-gated-mcp-interop.md](0005-centralized-capability-invocation-and-gated-mcp-interop.md)

## Minimal Structure

- Status
- Date
- Context
- Decision
- Consequences
- Rejected Alternatives
- Follow-up

Use [adr-template.md](adr-template.md) as the repository template.
