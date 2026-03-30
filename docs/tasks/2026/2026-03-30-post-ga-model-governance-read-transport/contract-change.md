# Contract Change

- Contract Decision:
  - No contract change is approved in this design-only package.
- Existing Contracts:
  - `ModelProvider`
  - `ModelCatalogItem`
  - `ModelProfile`
  - `TenantModelPolicy`
  - `ModelSelectionDecision`
- Sufficiency Statement:
  - The default assumption is that the existing five model-governance contracts remain sufficient for the first read-only transport slice.
- Hard Stop:
  - If implementation proves that summary/detail DTOs, transport-only projections, or route-specific query envelopes are required, stop and create a bounded follow-on contract-change before coding those objects.
- Out Of Scope:
  - New commands, events, write DTOs, or provider-connectivity contracts.
