# ADR 0008: Desktop Dashboard Plus Conversation First Interaction Model

- Status: Accepted
- Date: 2026-03-30
- Deciders: Repository maintainers
- Consulted: PRD, SAD, GA blueprint, Visual Framework, Slice 14 through Slice 20 desktop task packages, and the post-GA desktop dashboard/conversation redesign task package
- Informed: Future desktop, remote-hub, interaction-model, and cross-device continuity slices

---

## Context

The tracked desktop shell currently reflects the GA validation stance:

- `Tasks` is the default active-project landing route.
- The primary IA is `Tasks / Runs / Knowledge / Inbox / Notifications / Connections`.
- Users begin by manipulating formal system objects directly.

That stance was valid for proving the GA runtime loop, but it exposes system structure too early for a mature product interaction model. The approved post-GA redesign instead follows the product principle that complexity should stay inside the system while users operate a smaller set of stable mental models.

At the same time, the repository must preserve architectural truth:

- `Task` and `Run` remain formal execution truth.
- `Inbox` remains the authoritative action surface.
- `Knowledge` remains a read/provenance surface.
- `Connections` and `Models` remain environment/governance surfaces.
- No new cross-language conversation truth is approved in this slice.

The repository needs one durable answer for how desktop initiation should work after GA without silently redesigning backend contracts.

## Decision

### 1. The desktop primary interaction model becomes `Dashboard + Conversation first`

For active projects, the default desktop landing route is `Dashboard`, not `Tasks`.

The primary initiation path becomes:

`Dashboard -> Conversation -> explicit proposal confirmation -> formal Task / Run`

### 2. Conversation is an app-local governed drafting surface, not formal runtime truth

Conversation draft/proposal state is allowed only as desktop app-local state in this slice.

It may:

- collect clarifications
- assemble an execution proposal
- gate formal execution behind explicit confirmation

It may not:

- become a new cross-language shared contract
- replace `Task` or `Run` as formal execution truth
- silently auto-create backend runs during drafting

### 3. Authority surfaces remain explicit and formal

The redesign does not collapse or replace the existing authority pages:

- `Runs` remains the project execution follow-up surface
- `Run Detail` remains the authoritative state/result/governance/diagnosis page
- `Inbox` remains the action surface
- `Knowledge` remains the provenance/read surface
- `Connections` and `Models` remain dedicated secondary governance/environment surfaces

### 4. `Tasks` becomes an expert direct-execution path

`Tasks` remains available for explicit direct execution and automation setup, but it no longer defines the default product entry path or first-class primary navigation.

### 5. `Notifications` becomes reminder context, not a primary work surface

Notifications remain distinct from inbox decisions, but they move out of first-class primary navigation and into lighter reminder surfaces in the shell and dashboard context.

## Consequences

### Positive

- The desktop product model better matches users who begin with evolving intent instead of fully specified tasks.
- Formal backend truth remains stable because run creation still happens through existing execution APIs.
- The shell can surface connection, governance, and reminder state without making every user start in a systems-operator page.
- Internationalization and theme preferences fit naturally into a more productized shell.

### Negative

- Desktop now owns more app-local interaction state and copy complexity.
- The first conversation experience is necessarily guided and local because no shared conversation backend exists in this slice.
- Existing `Tasks-first` route assumptions and tests must be rewritten.

### Trade-off

The repository accepts a more opinionated desktop shell and more app-local state in exchange for a more mature user entry model, while deliberately refusing to widen into new shared conversation contracts.

## Rejected Alternatives

### 1. Keep `Tasks-first` and only restyle the existing shell

Rejected because the core problem is interaction model exposure, not visual polish alone.

### 2. Replace `Task` / `Run` with a new shared conversation object immediately

Rejected because it would widen scope into schema-first, hub-client, and remote-hub redesign before the product value of conversation continuity is proven.

### 3. Collapse `Inbox` and `Notifications` into one mixed reminder/action surface

Rejected because approval action authority and reminder visibility have materially different semantics.

### 4. Make `Conversation` a generic consumer chat surface

Rejected because the product still needs governed operational framing, proposal visibility, and explicit execution confirmation.

## Follow-up

- Implement the bounded post-GA desktop redesign through [2026-03-30-post-ga-desktop-dashboard-conversation-redesign](../tasks/2026/2026-03-30-post-ga-desktop-dashboard-conversation-redesign/README.md).
- Keep conversation draft/proposal truth app-local unless a later task package explicitly promotes cross-device continuity into a schema-first slice.
- Update owner docs and desktop tests so the repository no longer treats `Tasks-first` as the default desktop truth.
