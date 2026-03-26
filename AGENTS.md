# AGENTS.md

Repository-wide instructions for Codex and compatible coding agents.

## Repo Status

- This repository is currently a `doc-first rebuild`.
- The current tracked tree only proves the existence of root docs and `docs/`.
- Do not assume `apps/`, `packages/`, `crates/`, workspace manifests, CI workflows, tests, or runtime code exist unless they are present in the tracked tree.
- Human owns scope, boundaries, acceptance, and exception approval. AI owns in-scope execution, doc updates, and truthful verification.

## Source of Truth

Read these files before making product, architecture, or workflow changes:

1. `README.md`
2. `docs/PRD.md`
3. `docs/SAD.md`

Use them with this precedence:

1. Explicit user instruction
2. `AGENTS.md`
3. `docs/PRD.md` for product scope and release slicing
4. `docs/SAD.md` for architecture and runtime constraints

## Operating Guidance

When the task involves implementation workflow, AI execution behavior, GA interaction design, or delivery governance, also read:

1. `docs/ENGINEERING_STANDARD.md`
2. `docs/CODING_STANDARD.md`
3. `docs/AI_ENGINEERING_PLAYBOOK.md`
4. `docs/AI_DEVELOPMENT_PROTOCOL.md`
5. `docs/VISUAL_FRAMEWORK.md`
6. `docs/DELIVERY_GOVERNANCE.md`
7. `docs/templates/README.md`

These documents refine implementation and delivery behavior. They do not override explicit user instructions, this root `AGENTS.md`, `docs/PRD.md`, or `docs/SAD.md`.

## Template Routing

- Humans may give natural-language requests. AI must decide whether the task is `doc`, `design`, `contract`, `skeleton`, `implementation`, or `review`.
- For any non-trivial request, AI must start by using `docs/templates/task-slice-card.md`, even if the human did not ask for a template explicitly.
- If the request defines or changes a formal object, interface, event, schema, state machine, or cross-plane contract, AI must also use `docs/templates/contract-template.md`.
- If the request spans multiple steps, modules, documents, or implementation stages, AI must also use `docs/templates/implementation-plan-template.md`.
- For trivial wording-only edits, AI may skip templates, but must still follow truthfulness and verification rules.
- AI should state which template route it selected before doing substantial work. Detailed routing rules live in `docs/AI_DEVELOPMENT_PROTOCOL.md` and `docs/templates/README.md`.

## Before You Start

Before any non-trivial change, confirm all of the following:

1. The request stays inside approved scope and does not silently expand product goals.
2. The target behavior has a concrete acceptance condition, not just a broad intention.
3. The request respects the current `GA / Beta / Later` split from `docs/PRD.md`.
4. Non-goals are still explicit, especially when a request sounds adjacent to roadmap items.
5. Required non-functional constraints are clear enough: safety, recovery, auditability, and truthfulness of system state.
6. The current tracked repository actually supports any claimed tech stack, build chain, or validation command.
7. The smallest useful slice is identified before doing broader design or implementation work.
8. Any change to root guidance keeps `README.md`, `AGENTS.md`, `docs/PRD.md`, and `docs/SAD.md` logically aligned.
9. If the request changes boundaries or acceptance criteria, stop and get human confirmation before proceeding.

## Execution Rules

- Build in MVP-sized vertical slices. Finish one small verifiable loop before expanding scope.
- Keep humans in the loop for scope definition, acceptance sign-off, architecture exceptions, security posture changes, and high-risk dependency decisions.
- Do not add new platform surfaces, Beta capabilities, or target-state features just because they appear related.
- Treat safety constraints as day-one requirements, not hardening work for later.
- When something fails, use this sequence: classify the failure, define the boundary of the problem, then apply the smallest fix and re-verify.

## Safety and Truthfulness

- Do not describe target-state architecture as implemented reality.
- Do not claim `pnpm`, `cargo`, app runtime, or test success when the manifests and sources are not present.
- Do not restore deleted repository skeletons unless the task explicitly requires rebuilding them.
- Do not treat external references as repository truth. For this repo, `docs/references/Claude_Hidden_Toolkit.md` is inspiration only, not an authority.
- Respect the documented product boundary:
  - First-release `GA`: `Desktop + Remote Hub + Task/Automation + Approval + Shared Knowledge + MCP`
  - `Beta`: `A2A`, `DiscussionSession`, `ResidentAgentSession`, high-order `Mesh`, `Org Knowledge Graph` promotion, `Mobile`
- Preserve core invariants from `PRD` and `SAD`:
  - `Run` is the authoritative execution shell.
  - Hub is the source of truth; Client is not.
  - `ToolSearch` discovers capabilities but does not grant permission.
  - Long-term knowledge writes must stay governed; external outputs are not trusted by default.

## Verification

- Run only checks that the current tracked repository can actually support.
- The truthful minimum verification set for the current repo is:
  - confirm required docs exist
  - search for stale references when relevant
  - review the focused diff
  - run `git diff --check`
- If future tracked files add real manifests, source trees, or test suites, then extend verification accordingly.
- Never imply completion without fresh verification evidence.

## Nested AGENTS Policy

- Keep this root file short and stable.
- When future tracked subtrees such as `apps/`, `packages/`, or `crates/` appear, add a closer `AGENTS.md` inside that subtree for local rules.
- The nearest `AGENTS.md` should carry subproject-specific build, test, and style instructions; this root file should remain repository-wide.
