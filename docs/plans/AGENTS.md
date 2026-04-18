# Plans AGENTS

- Files under `docs/plans/**` are implementation control documents. They must make execution state recoverable without re-reading the whole chat.
- New plans should follow `docs/plans/PLAN_TEMPLATE.md` unless the local subtree already has a canonical plan format with the same control fields.
- Each task must be atomic and include exact files, preconditions, `Done when`, verification commands, and `Stop if` conditions.
- Each task must expose a status marker: `pending`, `in_progress`, `blocked`, or `done`.
- Update execution state in the plan itself after each batch. Do not keep the real current step only in chat messages.
- Use `docs/plans/EXECUTION_TEMPLATE.md` or the checkpoint format in `docs/plans/PLAN_TEMPLATE.md` when resuming work or handing execution to another agent.
- Avoid vague task wording such as "implement", "wire", or "finish" without file boundaries and acceptance criteria.
- If a plan uncovers a new repository policy, move that rule to the canonical governance doc or a higher-level `AGENTS.md`, then keep the plan focused on execution.
