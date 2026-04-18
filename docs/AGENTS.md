# Documentation AGENTS

- Files under `docs/**` are long-lived product, governance, audit, or design documents. Do not treat them as scratch notes.
- Keep documents concise, normative, and actionable. Prefer rules and clear ownership statements over historical narration.
- `docs/plans/**` are execution documents for humans and AI. The plan-specific operating rules live in `docs/plans/AGENTS.md`.
- If a plan changes repository policy, move the normative rule into the canonical governance doc or `AGENTS.md` first, then keep the plan focused on execution.
- `docs/api-openapi-governance.md` is the canonical policy for frontend/backend HTTP contract work, OpenAPI source management, and adapter/server transport rules.
- `docs/openapi-audit.md` is a status and coverage audit. Update it when coverage, allowlists, or migration state changes, but do not introduce new canonical rules there first.
- `docs/release-governance.md` is for release and artifact policy. Keep it focused on release mechanics rather than generic API-development rules.
- `docs/runtime_config_api.md` is the runtime-config-specific contract companion. Keep runtime-specific behavior there, and keep general transport policy in `docs/api-openapi-governance.md`.
- When changing policy, update the canonical policy document first and then adjust audit, release, or companion documents to match.
- Avoid duplicating the same full rule set across multiple docs. Cross-reference the canonical policy instead.
