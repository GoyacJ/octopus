# Octopus Project Instructions

You are helping design the Octopus project.

## Reference projects

The following directories are read-only references:

- docs/references/claude-code-sourcemap-main
- docs/references/hermes-agent-main
- docs/references/openclaw-main

Do not modify files under docs/references.

## Primary goal

Analyze the architecture and design patterns of the three reference projects, then synthesize a clean architecture for Octopus.

## Core rules

1. Do not copy source code from reference projects.
2. Extract concepts, module boundaries, interaction patterns, permission models, extension mechanisms, and architectural tradeoffs only.
3. Every claim about a reference project must cite local file paths, filenames, symbols, or config files.
4. Separate observed facts from inferred architecture and Octopus recommendations.
5. Prefer producing markdown architecture documents before implementation.
6. Do not implement Octopus until OCTOPUS_RFC.md and ADRs are approved.
7. Treat docs/references/claude-code-sourcemap-main as research material only. Do not assume its folder structure is authoritative.
8. Before any context compaction or session handoff, update the relevant markdown files.
9. After any context compaction or new session, reload:
   - docs/architecture/reference-analysis/evidence-index.md
   - docs/architecture/reference-analysis/comparison-matrix.md
   - docs/architecture/OCTOPUS_RFC.md, if it exists.
10. Keep outputs concise, structured, and evidence-backed.

## Done means

A task is done only when:

1. The requested markdown file exists.
2. It contains file-path-backed evidence.
3. Unverified claims are marked as Unverified.
4. Recommendations are separated from observations.
5. The evidence index is updated when applicable.