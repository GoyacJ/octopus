# docs/AGENTS.md

## Purpose

These instructions apply to work that primarily touches repository documentation.

## Documentation Rules

- Treat [README.md](../README.md) and [README.md](README.md) as the entry points for humans and agents.
- Keep links relative. Do not use local absolute filesystem links inside repository docs.
- Update `docs/README.md` and the relevant category README whenever you add, move, rename, or delete a normative document.
- Keep one owner document per rule. Summary docs should point to owner docs instead of restating large policy sections.
- Product semantics belong in `product/PRD.md`, architecture boundaries in `architecture/SAD.md`, GA sequencing in `architecture/ga-implementation-blueprint.md`, and execution rules in `governance/`.
- Durable architecture decisions belong in `decisions/`.
- Task-local design and delivery artifacts belong in `tasks/`.
- References under `references/` are non-normative and must not silently override product, architecture, or governance docs.

## Documentation Quality Bar

- State current tracked reality, not target-state assumptions.
- Separate current state from target state explicitly.
- Prefer short owner docs plus indexed navigation over large duplicated summary text.
- If you move a document, fix every inbound reference before considering the refactor complete.
