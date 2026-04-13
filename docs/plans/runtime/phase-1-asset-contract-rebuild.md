# Phase 1 Asset Contract Rebuild

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild the Octopus asset plane so agents, teams, skills, MCP servers, plugins, and workflow templates become runtime-relevant native assets with versioned bundle contracts, trust metadata, dependency metadata, and translation-aware import or export behavior.

**Architecture:** Phase 1 is asset-first and runtime-preparatory. It does not implement the unified turn engine yet. It upgrades the asset schemas, bundle envelope, import or export translator, and native Rust asset types so Phase 2 can compile `ActorManifest` and `TeamManifest` from real product objects instead of prompt-centric rows.

**Tech Stack:** OpenAPI source under `contracts/openapi/src/**`, handwritten schema files under `packages/schema/src/*`, Rust types in `crates/octopus-core`, asset translation and persistence in `crates/octopus-infra`, bundle seed and examples under `scripts/` and `.octopus/manifest.json`.

---

## Fixed Decisions

- `AssetBundleManifest v2` becomes the only forward path for agent bundle import or export.
- bundle import is translation-based, not passthrough execution
- agent and team assets must become runtime-facing policy objects, not prompt-plus-tool lists
- `crates/octopus-core/src/lib.rs` and `crates/octopus-infra/src/agent_import.rs` must stop growing as monoliths during this phase
- no compatibility layer is added to preserve the current thin asset model as the authoring truth

## Scope

This phase covers:

- asset contract redesign
- bundle manifest redesign
- import or export translation redesign
- native asset type extraction and modularization
- asset-plane persistence and diagnostics

This phase does not cover:

- multi-turn runtime loop cutover
- session or run execution logic
- team worker execution
- durable memory runtime

## Task 1: Split Asset Contracts Into Feature Files

**Files:**
- Modify: `contracts/openapi/src/components/schemas/catalog.yaml`
- Modify: `contracts/openapi/src/components/schemas/projects.yaml`
- Modify: `contracts/openapi/src/paths/catalog.yaml`
- Modify: `contracts/openapi/src/paths/projects.yaml`
- Create: `packages/schema/src/asset-bundle.ts`
- Modify: `packages/schema/src/workspace-plane.ts`
- Modify: `packages/schema/src/agent-import.ts`
- Modify: `packages/schema/src/catalog.ts`
- Modify: `packages/schema/src/index.ts`

**Implement:**
- add first-class asset bundle transport types instead of burying bundle semantics inside preview/result payloads only
- add runtime-facing policy fields to agent and team transport shapes
- add explicit import translation and trust diagnostics to bundle import or export contracts
- keep schema ownership split by feature; do not add new handwritten asset types into `runtime.ts` or `knowledge.ts`

**Required fields for agent assets:**
- `taskDomains`
- `defaultModelStrategy`
- `capabilityPolicy`
- `permissionEnvelope`
- `memoryPolicy`
- `delegationPolicy`
- `approvalPreference`
- `outputContract`
- `sharedCapabilityPolicy`

**Required fields for team assets:**
- `leaderRef`
- `memberRefs`
- `teamTopology`
- `sharedMemoryPolicy`
- `mailboxPolicy`
- `artifactHandoffPolicy`
- `workflowAffordance`
- `workerConcurrencyLimit`

**Required fields for bundle contracts:**
- `version`
- `dependencies`
- `trustMetadata`
- `compatibilityMapping`
- `policyDefaults`
- `translationReport`
- `unsupportedFeatures`
- `trustWarnings`
- `dependencyResolution`

**Verification:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
```

**Done when:**
- schema generation passes
- asset, team, and bundle contracts are feature-scoped and runtime-relevant

## Task 2: Extract Native Asset Types Out Of `octopus-core`

**Files:**
- Create: `crates/octopus-core/src/asset_bundle.rs`
- Create: `crates/octopus-core/src/actor_assets.rs`
- Create: `crates/octopus-core/src/runtime_policy.rs`
- Modify: `crates/octopus-core/src/lib.rs`

**Implement:**
- move bundle import or export types out of `crates/octopus-core/src/lib.rs`
- define native Rust types for:
  - asset bundle manifest v2
  - actor asset policy payloads
  - team topology and shared policy payloads
  - translation diagnostics
  - trust and dependency metadata
- keep `lib.rs` as export surface only

**Required native groups:**
- `AssetBundleManifestV2`
- `AssetDependency`
- `AssetTrustMetadata`
- `AssetTranslationReport`
- `AgentAssetPolicy`
- `TeamAssetPolicy`
- `OutputContract`
- `WorkflowAffordance`

**Verification:**
```bash
cargo test -p octopus-core
```

**Done when:**
- `lib.rs` no longer accumulates new asset-plane struct definitions
- all bundle and asset policy types live in dedicated modules

## Task 3: Rebuild Import And Export Translation As A Real Translator

**Files:**
- Create: `crates/octopus-infra/src/agent_bundle/mod.rs`
- Create: `crates/octopus-infra/src/agent_bundle/manifest_v2.rs`
- Create: `crates/octopus-infra/src/agent_bundle/translation.rs`
- Create: `crates/octopus-infra/src/agent_bundle/import.rs`
- Create: `crates/octopus-infra/src/agent_bundle/export.rs`
- Modify: `crates/octopus-infra/src/lib.rs`
- Modify: `crates/octopus-infra/src/agent_assets.rs`
- Modify: `crates/octopus-infra/src/agent_import.rs`

**Implement:**
- break the current `agent_import.rs` monolith into manifest loading, translation planning, import execution, and export execution modules
- translate foreign bundle content into native assets
- emit explicit downgrade, reject, and trust-warning diagnostics
- keep preview and actual import using the same translation planner so preview and execution cannot drift

**Translator responsibilities:**
- parse manifest v2
- validate dependencies
- validate trust metadata
- map foreign capability references into native Octopus asset forms
- produce explicit `translationReport`
- refuse unsupported runtime semantics instead of silently accepting them

**Verification:**
```bash
cargo test -p octopus-infra agent_import
cargo test -p octopus-infra split_module
```

**Done when:**
- preview and import use the same translation plan
- unsupported features produce diagnostics instead of implicit loss
- `agent_import.rs` is reduced to a thin facade or removed

## Task 4: Upgrade Workspace Asset Persistence And Projections

**Files:**
- Modify: `crates/octopus-infra/src/agent_assets.rs`
- Modify: `crates/octopus-infra/src/projects_teams.rs`
- Modify: `crates/octopus-infra/src/infra_state.rs`
- Modify: `packages/schema/src/workspace-plane.ts`
- Modify: `packages/schema/src/catalog.ts`

**Implement:**
- persist new agent and team policy fields
- persist trust, dependency, and translation metadata for imported assets
- project richer asset summaries back to the UI and runtime-facing callers
- keep workspace and project assignment behavior consistent with the richer asset model

**Required projection changes:**
- asset revision or manifest revision metadata
- import origin and translation status
- trust state
- dependency health state
- team topology summary
- task-domain summary

**Verification:**
```bash
cargo test -p octopus-infra
pnpm schema:check
```

**Done when:**
- imported assets can be reloaded with full policy and trust shape intact
- workspace and project projections expose enough data for manifest compilation in Phase 2

## Task 5: Update Bundle Seeds, Examples, And Preparation Scripts

**Files:**
- Modify: `scripts/prepare-agent-bundle-seed.mjs`
- Modify: `example/agent/.octopus/manifest.json`
- Modify: `crates/octopus-infra/seed/builtin-assets/bundle/.octopus/manifest.json`

**Implement:**
- update generated and checked-in bundle manifests to v2
- add policy defaults, dependency metadata, and trust metadata to sample bundles
- keep examples representative of non-coding and team-aware assets, not only prompt definitions

**Verification:**
```bash
node scripts/prepare-agent-bundle-seed.mjs
cargo test -p octopus-infra agent_seed
```

**Done when:**
- bundle seeds and examples validate against v2
- examples demonstrate agent, team, skill, and MCP composition with policy metadata

## Task 6: Phase-Level Validation And Deletion Fence

**Files:**
- Verify only

**Run:**
```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
cargo test -p octopus-core
cargo test -p octopus-infra
git diff --stat -- \
  contracts/openapi/src/components/schemas/catalog.yaml \
  contracts/openapi/src/components/schemas/projects.yaml \
  contracts/openapi/src/paths/catalog.yaml \
  contracts/openapi/src/paths/projects.yaml \
  packages/schema/src \
  crates/octopus-core/src \
  crates/octopus-infra/src \
  scripts/prepare-agent-bundle-seed.mjs \
  example/agent/.octopus/manifest.json \
  crates/octopus-infra/seed/builtin-assets/bundle/.octopus/manifest.json
```

**Acceptance criteria:**
- bundle v2 is the canonical contract
- agent and team assets are runtime-facing policy objects
- import or export preview and execution share one translator
- `octopus-core` and `octopus-infra` asset code is split into dedicated modules
- no new work remains blocked on the old thin asset model

## Handoff To Phase 2

Phase 1 is complete only when Phase 2 can assume all of the following:

- agent and team assets compile from native rich contracts
- bundle imports produce native assets plus diagnostics
- trust and dependency metadata are persisted
- runtime no longer needs to infer execution behavior from prompt strings and ad hoc tool lists

The next implementation document is:

- `docs/plans/runtime/phase-2-unified-runtime-core.md`
