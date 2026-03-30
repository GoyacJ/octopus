# ADR 0007: Model Center Governance Boundary And Terminology

- Status: Accepted
- Date: 2026-03-30
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, schema-first guidelines, the March 27 model-center supplement, and the post-GA model-center foundation task package
- Informed: Future model-center, runtime, capability, provider-adapter, and surface slices

---

## Context

PRD already fixes a four-layer model-governance vocabulary:

- `ModelProvider`
- `ModelCatalogItem`
- `ModelProfile`
- `TenantModelPolicy`

SAD already reserves `Model Center` inside the Governance Plane, but only at a placeholder level.

The March 27 supplement proposes a richer model-center and capability-boundary design, but it currently:

- lives outside the owner-doc / task-package / ADR chain
- overlaps existing PRD terminology with alternate names such as `ModelCatalogEntry` and `ModelAccessPolicy`
- bundles provider adapters, built-in tools, routing, and capability-runtime refactors into one expansion path that is too large for the current post-GA backlog controls

The tracked repository also has capability-governance contracts, but no tracked model-governance contracts yet.

Because post-GA backlog expansion is frozen by default, the repository needs one durable answer for:

- canonical terminology
- first-slice scope
- boundary ownership between Model Center and capability runtime
- the document status of the supplement

## Decision

### 1. Canonical model-governance terminology follows PRD

The tracked repository uses the following canonical vocabulary:

- `ModelProvider`
- `ModelCatalogItem`
- `ModelProfile`
- `TenantModelPolicy`

This ADR adds `ModelSelectionDecision` as the first post-GA per-run model-choice record.

For overlapping supplement language:

- `ModelCatalogEntry` maps to `ModelCatalogItem`
- `ModelAccessPolicy` maps to `TenantModelPolicy` for first-slice tracked contracts

The following terms remain candidate extensions, not first-slice tracked objects:

- `ModelFeatureSet`
- `ProviderEndpointProfile`
- `ModelRoutingPolicy`

### 2. Model Center remains a Governance Plane subsystem and stays distinct from capability runtime

Model Center owns provider, catalog, profile, and tenant-policy truth.

Runtime may consume a `ModelSelectionDecision`, but that record does not replace or weaken the existing capability-governance chain:

- `CapabilityCatalog`
- `CapabilityResolver`
- `ToolSearch`
- `ExecutionProfile`
- `SkillPack`

`ExecutionProfile` continues to express defaults only.

`SkillPack` continues to constrain behavior only and may not create model or capability truth.

### 3. The first post-GA Model Center slice is doc/schema-only

The first foundation slice is limited to:

- a task package
- this ADR
- minimal shared contracts in `schemas/governance`
- `packages/schema-ts` consumer registration
- owner-doc and index alignment

It explicitly excludes:

- `ProviderAdapter` SPI
- provider built-in tool modeling
- runtime `CapabilityResolver` / `ToolSearch` rewiring
- provider connectivity or multi-provider execution

### 4. The March 27 supplement becomes non-normative reference material

The supplement is preserved for design input, but it does not act as an owner doc or a standalone source of truth.

Durable conclusions belong in:

- PRD
- SAD
- the GA blueprint
- ADRs
- task packages
- `schemas/`

## Consequences

### Positive

- Model Center work now has one canonical vocabulary aligned to PRD.
- Future slices can build on shared contracts without first relitigating names and ownership.
- The repository keeps the post-GA backlog bounded while still preserving the supplement's design value.
- Capability-runtime boundaries remain stable while model-governance work begins.

### Negative

- The first slice does not deliver provider integration or runtime behavior improvements.
- Some richer supplement concepts remain deferred and must be re-opened in later task packages.
- Future model-center consumers must tolerate a deliberately small first contract set.

### Trade-off

The repository accepts a slower governance-first start in exchange for avoiding a single oversized post-GA refactor that would mix terminology cleanup, schema introduction, provider adapters, and capability-runtime redesign.

## Rejected Alternatives

### 1. Implement the full supplement as one post-GA slice

Rejected because it would widen into adapters, runtime behavior, provider-tool modeling, and broader capability refactors before the repository has frozen shared terminology or first-slice contracts.

### 2. Treat the supplement as a normative owner document

Rejected because it sits outside the repository owner-doc chain and would create a parallel source of truth next to PRD, SAD, and the blueprint.

### 3. Keep both PRD terms and supplement terms alive in tracked contracts

Rejected because parallel names for the same governance objects would guarantee drift across docs, schemas, and future consumers.

### 4. Place model-governance contracts outside `schemas/governance`

Rejected because these are cross-language shared contracts and must remain under the existing schema-first source-of-truth boundary.

## Follow-up

- Treat [2026-03-30-post-ga-session-token-lifecycle](../tasks/2026/2026-03-30-post-ga-session-token-lifecycle/README.md) and [2026-03-30-post-ga-model-center-foundation](../tasks/2026/2026-03-30-post-ga-model-center-foundation/README.md) as completed post-GA closeouts; the queued next step is the design-only [2026-03-30-post-ga-model-governance-consumers](../tasks/2026/2026-03-30-post-ga-model-governance-consumers/README.md) package.
- Revisit `ModelFeatureSet`, `ProviderEndpointProfile`, `ModelRoutingPolicy`, `ProviderAdapter`, and provider built-in tools only through later task packages.
- Add runtime/governance persistence, evaluation, and transport consumers for the new model-governance contracts only when a later slice is approved.
