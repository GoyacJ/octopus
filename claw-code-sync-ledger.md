# claw-code Upstream Sync Ledger

## Baseline

- Upstream range: `be561bfdeb92fce7011938e748ee20051460d6a4..8e25611064e5bba49263ff6ebbe8f103be9fcdfb`
- Upstream review source: `/tmp/claw-code-upstream`
- Local sync rule: follow `claw-code` backend semantics for `api / runtime / session / worker / config / tools`; adapt implementation where Octopus keeps a different public contract or persistence model
- Status legend:
  - `done`: backfilled and covered by local tests
  - `tracked-only`: intentionally not backfilled in Octopus backend core

## Sync Batches

| Upstream commits | Theme | Class | Octopus modules | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| `8c6dfe5`, `5851f2d` | preflight fast-fail and count-tokens regression fix | A | `crates/api`, `crates/rusty-claude-cli`, `crates/octopus-runtime-adapter` | done | Restored local preflight guard so oversized or invalid requests fail before remote round-trip masking. |
| `0530c50`, `3ac97e6`, `ff1df4c`, `8dc6580`, `adcea6b` | provider routing and auth-provider error hints | A | `crates/api`, `crates/rusty-claude-cli` | done | Synced `openai/`, `gpt-*`, `grok*`, `qwen/*`, `qwen-*` routing and improved provider/auth copy. |
| `c667d47`, `b513d6e`, `523ce74`, `e7e0fd2`, `e4c3871`, `eb044f0` | OpenAI-compatible request compatibility | A | `crates/api` | done | Added tuning params, reasoning sanitization, Anthropic stop conversion, `/responses` strict object schema, `reasoning_effort`, and `max_completion_tokens` behavior. |
| `ce22d8f`, `ce360e0`, `2a64287` | stream parsing and provider error context hardening | A | `crates/api` | done | Stream/token parsing now tolerates missing usage fields and returns richer provider/model/body context. |
| `861edfc`, `28e6cc0`, `20f3a59` | per-worktree session isolation and workspace-root metadata | B | `crates/runtime` | done | `Session.workspace_root` is persisted, managed session summaries keep the metadata, and `runtime::SessionStore` now exposes namespaced per-worktree session storage with workspace-hash isolation tests. |
| `f03b8dc` | stale-base preflight | B | `crates/runtime` | done | Added `stale_base` module and tests for `.claw-base` / explicit base-commit comparison. |
| `314f0c9`, `fcb5d0c`, `9461522`, `c08f060`, `aee5263`, `eff0765` | worker control plane observability and recovery | A | `crates/runtime`, `crates/tools` | done | Tools parity tests now cover `WorkerObserveCompletion`, `WorkerGet`, `WorkerAwaitReady`, trust-stall recovery via `WorkerResolveTrust` and `WorkerRestart`, non-ready prompt rejection, plus `.claw/worker-state.json` stage transitions with `seconds_since_update`. |
| `172a2ad`, `252536b` | plugin/tool stability and env-lock race fixes | A | `crates/tools`, `crates/plugins` | done | BrokenPipe tolerance and env-var serialization regressions were already backfilled in local tests/tooling paths. |
| `133ed45`, `5dfb1d7`, `bcdc52d`, `6ddfa78` | config validation, trusted roots, aliases, provider fallbacks | B | `crates/runtime`, `crates/tools`, `crates/octopus-runtime-adapter` | done | Parser now accepts `aliases`, `providerFallbacks`, `trustedRoots`, and `plugins.maxOutputTokens`; `WorkerCreate` merges config trusted roots; adapter validation surfaces unknown/deprecated warnings while preserving Octopus scope-based config ownership. |
| `ecdca49` | plugin-level `maxOutputTokens` override | B | `crates/runtime`, `crates/octopus-runtime-adapter` | done | Scoped runtime config `plugins.maxOutputTokens` now overrides registry defaults during target resolution and is consumed by `anthropic_messages`, `openai_chat`, `openai_responses`, and `gemini_native` request payload builders without changing Octopus config truth sources. |
| `1f968b3` | HTTP proxy v2 | B | `crates/api`, `crates/octopus-runtime-adapter` | done | Added upstream env-contract proxy resolution via shared `api::http_client` builders, exported `ProxyConfig`, added proxy integration tests, and routed `AnthropicClient`, `OpenAiCompatClient`, and `LiveRuntimeModelExecutor` through fail-open shared client construction. |
| `5c276c8`, `18d3c19`, `0f2f02a` | PDF extract and tooling-only follow-ups | C | n/a | tracked-only | These upstream commits belong to PDF/tooling scope rather than Octopus backend-core parity; keep tracked only unless Octopus adopts the corresponding workflow. |
| `506ff55`, `275b585`, `8d86607`, `0d8fd51`, `dab16c2` | doctor/CLI banner/stdin/session-export UX | C | n/a | tracked-only | CLI-facing quality-of-life work is not in current backend-core sync scope. |
| `c980c3c` and `ROADMAP.md` / `USAGE.md` / install-script only commits | docs, roadmap, install, display-only updates | C | n/a | tracked-only | Keep as reference only; no backend semantic delta to mirror. |

## Closeout

- Backend-core items in this baseline are fully closed as either `done` or `tracked-only`.
- Octopus runtime config ownership remains `config/runtime/{workspace,projects,users}`; no `.claw/settings*.json` truth source was reintroduced.
