# Release Notes Governance

Release notes are governed as structured source material, not as ad-hoc Markdown dumps.

## Fragment Rules

- keep one fragment per releasable change
- every fragment filename must start with a category prefix
- fragment content must stay user-facing and describe behavior, impact, or migration requirements
- do not paste commit history, internal file paths, or implementation-only details into user-facing fragments

Supported prefixes:

- `summary-*`: required for formal releases; provides the manually reviewed version overview
- `feature-*`: user-visible capability updates
- `fix-*`: user-visible fixes
- `breaking-*`: breaking behavior changes that must appear in upgrade guidance
- `migration-*`: upgrade or configuration migration instructions
- `docs-*`: user-visible documentation or usability guidance worth surfacing in notes
- `internal-*`: internal changes; preview-only unless referenced from the appendix
- `governance-*`: release/process/governance changes; preview-only unless referenced from the appendix

Examples:

- `summary-2026-04-08-initial-desktop-release.md`
- `feature-2026-04-08-desktop-installers.md`
- `fix-2026-04-08-runtime-session-timeout.md`
- `breaking-2026-04-08-config-scope-layout.md`

## Formal Release Rules

- formal releases must include at least one `summary-*` fragment
- formal notes may auto-aggregate commits and PR subjects, but that auto-generated text cannot replace the manual summary
- `internal-*` and `governance-*` fragments do not belong in the formal user-facing body

## Preview Release Rules

- preview releases may be generated entirely from fragments plus Git history
- preview notes must clearly state that they come from `main` and are not stable guarantees
- preview notes default to `latest formal tag -> current preview tag` as the change range unless `--since-ref` is provided
- `internal-*` and `governance-*` fragments may appear in preview notes

## Lifecycle

- add fragments as releasable changes land
- remove or archive consumed fragments after a formal tagged release
- the generated Markdown is a render target; the source of truth is the fragment set plus Git metadata
