# Release Notes Governance

Release notes are governed as structured source material, not as ad-hoc Markdown dumps.

## Source Of Truth

- user-facing release note正文 comes from fragments under `docs/release-notes/fragments/`
- Git history is used only to determine the release range and emit diagnostics when releasable changes landed without matching fragments
- the repository does not maintain a hand-edited root `CHANGELOG.md`; versioned release note archives are the historical record

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
- formal notes do not use commit subjects as a default user-facing正文 source
- `internal-*` and `governance-*` fragments do not belong in the formal user-facing body
- formal notes default to `previous formal tag -> current formal tag`

## Preview Release Rules

- preview releases may use an auto-generated overview paragraph, but their default user-facing body still comes from fragments
- preview notes must clearly state that they come from `main` and are not stable guarantees
- preview notes default to `previous preview tag -> current preview tag` as the change range unless `--since-ref` is provided
- `internal-*` and `governance-*` fragments do not belong in the default preview user-facing body

## Lifecycle

- add fragments as releasable changes land
- preview releases do not consume fragments
- archive consumed fragments after a formal tagged release
- use `pnpm release:archive-fragments -- --tag vX.Y.Z` to move the consumed formal fragments into `docs/release-notes/archive/vX.Y.Z/`
- the generated Markdown is a render target; the source of truth is the fragment set plus Git metadata
