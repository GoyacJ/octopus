# ADR Trigger Note

- ADR Required Because:
  - This slice introduces new core governance objects and fixes canonical terminology for future Model Center work.
  - It also decides the document status of the March 27 supplement and the bounded order of follow-on slices.
- Proposed ADR:
  - `0007-model-center-governance-boundary-and-terminology.md`
- Resolution:
  - ADR 0007 is created in this slice and accepts the governance-first foundation approach.
- Deferred Decision Surface:
  - `ProviderAdapter` SPI
  - provider built-in tool descriptors / policies / usage traces
  - `ModelFeatureSet`
  - `ProviderEndpointProfile`
  - `ModelRoutingPolicy`
  - runtime `CapabilityResolver` / `ToolSearch` refactors
