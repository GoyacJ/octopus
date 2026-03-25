# Capability Card Template

> Capability ID: `capability_id`
> Kind: `core | deferred | connector_backed | platform_native`
> Source: `native | mcp | a2a | adapter | artifact_runtime | skill_pack`
> Delivery Phase: `ga | beta | later`
> Platform: `desktop | web | mobile`
> Risk Level: `low | medium | high | restricted`

## Overview

[One paragraph describing what the capability is for and where it belongs in Octopus.]

## Descriptor

| Field | Value |
| --- | --- |
| `schema_ref` | |
| `default_visibility` | |
| `search_exposure` | |
| `fallback` | |
| `observation_requirements` | |

## Resolver Rules

- Which `platform` states affect visibility?
- Which connector states affect visibility?
- Which `Workspace / Project` policies affect visibility?
- Which `CapabilityGrant / BudgetPolicy / ApprovalRequest` gates apply?

## Interaction Surface

- Which surface uses it: `Chat`, `Inbox`, `Board`, `Artifact`, `Knowledge`, `Trace`
- Is it searchable by `ToolSearch`?
- Does it create `InteractionPrompt`, `MessageDraft`, `ArtifactSessionState`, or other formal objects?

## Boundaries

- Why it is or is not part of the core domain model
- Whether it can only exist as an adapter or connector-backed capability
- Whether it is session-scoped, run-scoped, or long-lived

## Risks And Gotchas

- Security / governance risks
- Knowledge pollution risks
- Recovery or session-state constraints
- Platform-specific limitations

## Contract Sync

- `docs/PRD.md`
- `docs/SAD.md`
- `docs/CONTRACTS.md`
- `contracts/v1/capabilities.json`
- Related ADR
