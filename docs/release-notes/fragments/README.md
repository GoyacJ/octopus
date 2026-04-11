This directory stores release note fragments.

Use the governed category-prefix format documented in [`docs/release-notes/README.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/release-notes/README.md).

Quick rules:

- one Markdown fragment per releasable change
- use filenames like `summary-*`, `feature-*`, `fix-*`, `breaking-*`, `migration-*`, `internal-*`, or `governance-*`
- formal releases require at least one `summary-*` fragment
- preview releases do not consume fragments
- archive consumed formal fragments after they are included in a tagged formal release
- use `pnpm release:archive-fragments -- --tag vX.Y.Z` to move the consumed formal fragments into `docs/release-notes/archive/vX.Y.Z/`
