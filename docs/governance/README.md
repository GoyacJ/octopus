# Governance Docs

This directory owns the repository's execution rules.

Use these documents as owner docs. Other files should summarize or reference them instead of re-declaring the same rules in parallel.

## Owner Docs

| Document | Owns | Update when |
| --- | --- | --- |
| [ai-engineering-guidelines.md](ai-engineering-guidelines.md) | General engineering constraints for AI-authored work | The general engineering standard changes |
| [ai-phase-gates.md](ai-phase-gates.md) | Phase-based execution control | The required work phases or gates change |
| [ai-delivery-templates.md](ai-delivery-templates.md) | Standard task and delivery templates | Template structure changes |
| [repo-structure-guidelines.md](repo-structure-guidelines.md) | Directory responsibilities and placement rules | Repo boundary ownership changes |
| [schema-first-guidelines.md](schema-first-guidelines.md) | Schema-first contract discipline | Cross-language contract process changes |
| [git-quality-guidelines.md](git-quality-guidelines.md) | Git change hygiene, scope control, verification traceability, and rollback expectations | Change-set quality rules change |
| [code-review-checklist.md](code-review-checklist.md) | Review checklist | Review criteria change |
| [change-delivery-guidelines.md](change-delivery-guidelines.md) | Delivery note expectations | Delivery reporting standards change |
| [pull_request_template.md](pull_request_template.md) | Pull request review scaffold | PR review form changes |

## Duplication Control

- `AGENTS.md` may summarize governance expectations, but the detailed rule text should live here.
- Prefer extending an existing owner doc; only add a new owner doc when the rule has a distinct review surface that would otherwise be duplicated across multiple files.
- Product and architecture docs must not become alternate governance owners.
- ADRs may explain why a rule exists, but should not replace the owner doc for ongoing process guidance.
- Task packages may refine a local task, but should not silently redefine repository-wide policy.
